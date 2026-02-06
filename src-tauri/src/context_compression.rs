// src-tauri/src/context_compression.rs
//
// Context Compression System for StoryEngine
// =============================================
//
// Prevents the Llama 3.1 8B model (8192-token context window) from running out
// of context by compressing older turns into a "story so far" summary while
// keeping recent turns in full detail.
//
// Strategy:
//   1. Track approximate token count of the full conversation
//   2. When approaching ~75% of the limit (~6144 tokens), trigger compression
//   3. Collect `summary_hint` fields from older turns into a compact summary
//   4. Optionally call the LLM itself to produce a higher-quality summary
//   5. Keep the last N turns in full detail
//   6. Always preserve: system prompt, character DB info, current location
//
// Usage:
//   The `generate_story` command in chat.rs should call `build_compressed_context`
//   instead of naively dumping all messages into the prompt. That function returns
//   a ready-to-send string (or message array) that fits within the token budget.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Llama 3.1 8B context window size
pub const MAX_CONTEXT_TOKENS: usize = 8192;

/// Trigger compression when estimated tokens exceed this fraction of the window
pub const COMPRESSION_THRESHOLD: f64 = 0.75; // 75% ≈ 6144 tokens

/// Number of recent turns (user+assistant pairs) to always keep in full detail
pub const RECENT_TURNS_TO_KEEP: usize = 6;

/// Tokens reserved for: system prompt + character DB + generation overhead.
/// The LLM also needs room to *produce* its response (~500-800 tokens for
/// a typical story turn with JSON), so we budget generously here.
pub const RESERVED_TOKENS: usize = 1500;

/// Effective token budget for conversation history (compressed + recent)
pub const HISTORY_TOKEN_BUDGET: usize = MAX_CONTEXT_TOKENS - RESERVED_TOKENS; // ~6692

// ============================================================================
// 1. TOKEN ESTIMATION
// ============================================================================

/// Rough token estimate: ~1 token per 4 characters for English text.
/// This matches the commonly-cited Llama tokenizer ratio and is intentionally
/// conservative (slightly over-counts) so we compress a little early rather
/// than a little late.
///
/// For more accuracy you could shell out to `ollama show --modelfile` and
/// count the actual BPE tokens, but for compression triggers this is fine.
pub fn estimate_tokens(text: &str) -> usize {
    // Ceiling division: (len + 3) / 4
    (text.len() + 3) / 4
}

/// Estimate tokens for an array of Ollama chat messages (role + content).
pub fn estimate_messages_tokens(messages: &[Value]) -> usize {
    messages.iter().map(|msg| {
        let role_tokens = 4; // "user" / "assistant" / "system" ≈ 1-2 tokens + framing
        let content = msg["content"].as_str().unwrap_or("");
        role_tokens + estimate_tokens(content)
    }).sum()
}

// ============================================================================
// 2. STORY TURN — one user action + one assistant response
// ============================================================================

/// A single conversation turn extracted from the DB's messages table.
/// We pair each user message with the assistant response that follows it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryTurn {
    /// 1-based turn number within this chat session
    pub turn_number: usize,
    /// The user's input text
    pub user_input: String,
    /// The raw assistant response (full JSON string as stored in the DB)
    pub assistant_response: String,
    /// One-line summary extracted from story_json.summary_hint (may be empty)
    pub summary_hint: String,
    /// Cached token estimate for this turn (user + assistant combined)
    pub token_estimate: usize,
}

impl StoryTurn {
    /// Build a StoryTurn from a user message and the following assistant message.
    /// Parses the assistant JSON to extract the summary_hint.
    pub fn from_messages(turn_number: usize, user_input: &str, assistant_response: &str) -> Self {
        let summary_hint = extract_summary_hint(assistant_response);
        let token_estimate = estimate_tokens(user_input) + estimate_tokens(assistant_response) + 8;
        Self {
            turn_number,
            user_input: user_input.to_string(),
            assistant_response: assistant_response.to_string(),
            summary_hint,
            token_estimate,
        }
    }
}

/// Extract the summary_hint from a raw assistant response string.
/// Handles the full LLM JSON format: { "story_json": { "summary_hint": "..." } }
/// Also checks top-level "summary_hint" as a fallback.
fn extract_summary_hint(raw: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        // Primary path: story_json.summary_hint
        if let Some(hint) = v.get("story_json")
            .and_then(|sj| sj.get("summary_hint"))
            .and_then(|h| h.as_str())
        {
            if !hint.is_empty() {
                return hint.to_string();
            }
        }
        // Fallback: top-level summary_hint
        if let Some(hint) = v.get("summary_hint").and_then(|h| h.as_str()) {
            if !hint.is_empty() {
                return hint.to_string();
            }
        }
    }
    // If we can't parse JSON at all, try to find the key with a simple search.
    // This handles cases where the JSON is embedded in surrounding text.
    if let Some(start) = raw.find("\"summary_hint\"") {
        if let Some(colon_pos) = raw[start..].find(':') {
            let after_colon = &raw[start + colon_pos + 1..];
            let trimmed = after_colon.trim();
            if trimmed.starts_with('"') {
                if let Some(end_quote) = trimmed[1..].find('"') {
                    return trimmed[1..1 + end_quote].to_string();
                }
            }
        }
    }
    String::new()
}

/// Extract just the story text from a raw assistant response (for display in recent turns).
/// Pulls from story_json.response, falling back to the raw text.
fn extract_story_text(raw: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        // story_json.response (main LLM output format)
        if let Some(text) = v.get("story_json")
            .and_then(|sj| sj.get("response"))
            .and_then(|r| r.as_str())
        {
            if !text.is_empty() {
                return text.to_string();
            }
        }
        // Fallback: top-level "response"
        if let Some(text) = v.get("response").and_then(|r| r.as_str()) {
            if !text.is_empty() {
                return text.to_string();
            }
        }
        // Fallback: "story" key (used by StoryResponse)
        if let Some(text) = v.get("story").and_then(|r| r.as_str()) {
            if !text.is_empty() {
                return text.to_string();
            }
        }
    }
    // Last resort: return raw text, trimmed
    raw.trim().to_string()
}

// ============================================================================
// 3. COMPRESSED HISTORY — the "story so far" block
// ============================================================================

/// The compressed summary that replaces older turns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressedHistory {
    /// The combined summary text (built from summary_hints or LLM summarization)
    pub story_so_far: String,
    /// Which turn numbers have been compressed into this summary
    pub compressed_turn_ids: Vec<usize>,
    /// Approximate token count of the compressed summary
    pub token_estimate: usize,
}

// ============================================================================
// 4. CONVERSATION CONTEXT — manages the full state for one chat
// ============================================================================

/// Full conversation state for a single chat session.
/// This is the main struct that `generate_story` works with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// All turns in order (only the recent, uncompressed ones after compression)
    pub turns: Vec<StoryTurn>,
    /// The current compressed summary (empty if no compression has happened yet)
    pub compressed: CompressedHistory,
    /// Total estimated tokens across all uncompressed turns
    pub total_turn_tokens: usize,
}

impl ConversationContext {
    pub fn new() -> Self {
        Self {
            turns: Vec::new(),
            compressed: CompressedHistory::default(),
            total_turn_tokens: 0,
        }
    }

    /// Build a ConversationContext from raw DB message rows.
    /// Messages should be in chronological order, alternating user/assistant.
    pub fn from_message_pairs(messages: &[(String, String)]) -> Self {
        let mut ctx = Self::new();
        for (i, (user_msg, assistant_msg)) in messages.iter().enumerate() {
            let turn = StoryTurn::from_messages(i + 1, user_msg, assistant_msg);
            ctx.total_turn_tokens += turn.token_estimate;
            ctx.turns.push(turn);
        }
        ctx
    }

    /// Build from the raw rows as they come from SQLite (role, content pairs in order).
    /// Groups consecutive user+assistant messages into turns.
    pub fn from_db_rows(rows: &[(String, String)]) -> Self {
        let mut pairs: Vec<(String, String)> = Vec::new();
        let mut i = 0;
        while i < rows.len() {
            let (ref role, ref content) = rows[i];
            if role == "user" {
                // Look ahead for the assistant response
                let assistant = if i + 1 < rows.len() && rows[i + 1].0 == "assistant" {
                    i += 1;
                    rows[i].1.clone()
                } else {
                    String::new()
                };
                pairs.push((content.clone(), assistant));
            }
            i += 1;
        }
        Self::from_message_pairs(&pairs)
    }

    /// Add a new turn to the context.
    pub fn add_turn(&mut self, user_input: &str, assistant_response: &str) {
        let turn_number = self.turns.len() + 1;
        let turn = StoryTurn::from_messages(turn_number, user_input, assistant_response);
        self.total_turn_tokens += turn.token_estimate;
        self.turns.push(turn);
    }

    /// Check if compression is needed based on the total token estimate.
    pub fn needs_compression(&self, system_prompt_tokens: usize, character_db_tokens: usize) -> bool {
        let total = system_prompt_tokens
            + character_db_tokens
            + self.compressed.token_estimate
            + self.total_turn_tokens;
        let threshold = (MAX_CONTEXT_TOKENS as f64 * COMPRESSION_THRESHOLD) as usize;
        total > threshold
    }

    /// Returns how many tokens the full context would use (estimated).
    pub fn estimated_total_tokens(&self, system_prompt_tokens: usize, character_db_tokens: usize) -> usize {
        system_prompt_tokens
            + character_db_tokens
            + self.compressed.token_estimate
            + self.total_turn_tokens
    }

    /// Perform compression using the fast method (summary_hint concatenation).
    /// Compresses all turns except the most recent `RECENT_TURNS_TO_KEEP`.
    pub fn compress_with_hints(&mut self) {
        if self.turns.len() <= RECENT_TURNS_TO_KEEP {
            return; // Nothing to compress
        }

        let split_point = self.turns.len() - RECENT_TURNS_TO_KEEP;
        let turns_to_compress = &self.turns[..split_point];

        // Build the "story so far" from summary_hints
        let mut story_lines: Vec<String> = Vec::new();

        // If there's an existing compressed summary, start with it
        if !self.compressed.story_so_far.is_empty() {
            story_lines.push(self.compressed.story_so_far.clone());
        }

        // Add each turn's summary_hint
        for turn in turns_to_compress {
            if !turn.summary_hint.is_empty() {
                story_lines.push(format!("Turn {}: {}", turn.turn_number, turn.summary_hint));
            } else {
                // Fallback: use a truncated version of the story text
                let story_text = extract_story_text(&turn.assistant_response);
                let truncated: String = story_text.chars().take(100).collect();
                let suffix = if story_text.len() > 100 { "..." } else { "" };
                story_lines.push(format!("Turn {}: {}{}", turn.turn_number, truncated, suffix));
            }
        }

        let new_summary = story_lines.join("\n");
        let compressed_ids: Vec<usize> = turns_to_compress.iter().map(|t| t.turn_number).collect();

        // Calculate token savings
        let old_tokens: usize = turns_to_compress.iter().map(|t| t.token_estimate).sum();
        let new_tokens = estimate_tokens(&new_summary);

        // Update the compressed history
        self.compressed = CompressedHistory {
            story_so_far: new_summary,
            compressed_turn_ids: compressed_ids,
            token_estimate: new_tokens,
        };

        // Remove compressed turns and recalculate
        self.turns = self.turns[split_point..].to_vec();
        self.total_turn_tokens = self.turns.iter().map(|t| t.token_estimate).sum();

        println!(
            "[ContextCompression] Compressed {} turns: {} tokens -> {} tokens (saved {})",
            split_point, old_tokens, new_tokens, old_tokens.saturating_sub(new_tokens)
        );
    }

    /// Generate an LLM-based summary prompt for higher quality compression.
    /// Returns the prompt string to send to Ollama. The caller should send this,
    /// get the response, and call `apply_llm_summary` with the result.
    pub fn build_llm_summary_prompt(&self) -> Option<String> {
        if self.turns.len() <= RECENT_TURNS_TO_KEEP {
            return None;
        }

        let split_point = self.turns.len() - RECENT_TURNS_TO_KEEP;
        let turns_to_compress = &self.turns[..split_point];

        let mut story_texts = Vec::new();

        // Include existing compressed summary
        if !self.compressed.story_so_far.is_empty() {
            story_texts.push(format!("Previous summary:\n{}", self.compressed.story_so_far));
        }

        // Add each turn's narrative text
        for turn in turns_to_compress {
            let story_text = extract_story_text(&turn.assistant_response);
            story_texts.push(format!(
                "Turn {} - Player said: \"{}\"\nStory: {}",
                turn.turn_number, turn.user_input, story_text
            ));
        }

        let all_text = story_texts.join("\n\n");

        Some(format!(
            r#"Summarize the following story events into a concise paragraph (3-5 sentences).
Preserve: character names, key plot points, current location, and any unresolved conflicts.
Do NOT add new story content. Only summarize what happened.

{}

SUMMARY:"#,
            all_text
        ))
    }

    /// Apply an LLM-generated summary to replace older turns.
    /// Call this after sending the prompt from `build_llm_summary_prompt` to Ollama.
    pub fn apply_llm_summary(&mut self, summary: &str) {
        if self.turns.len() <= RECENT_TURNS_TO_KEEP {
            return;
        }

        let split_point = self.turns.len() - RECENT_TURNS_TO_KEEP;
        let compressed_ids: Vec<usize> = self.turns[..split_point]
            .iter()
            .map(|t| t.turn_number)
            .collect();

        let new_tokens = estimate_tokens(summary);

        self.compressed = CompressedHistory {
            story_so_far: summary.trim().to_string(),
            compressed_turn_ids: compressed_ids,
            token_estimate: new_tokens,
        };

        // Remove compressed turns
        self.turns = self.turns[split_point..].to_vec();
        self.total_turn_tokens = self.turns.iter().map(|t| t.token_estimate).sum();
    }
}

// ============================================================================
// 5. CONTEXT BUILDER — assembles the final prompt for Ollama
// ============================================================================

/// Information about registered characters to include in the system prompt.
#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub name: String,
    pub age: Option<i32>,
    pub gender: Option<String>,
    pub personality: Option<String>,
    pub appearance: Option<String>, // sd_prompt
    pub default_clothing: Option<String>,
}

/// The assembled context ready to send to Ollama.
#[derive(Debug, Clone, Serialize)]
pub struct AssembledContext {
    /// The full prompt string (for /api/generate endpoint)
    pub prompt: String,
    /// Estimated total token count
    pub estimated_tokens: usize,
    /// Whether compression was applied during assembly
    pub was_compressed: bool,
    /// Number of turns in full detail
    pub recent_turn_count: usize,
    /// Number of turns that were compressed
    pub compressed_turn_count: usize,
}

/// Build the character database section of the system prompt.
fn build_character_section(characters: &[CharacterInfo]) -> String {
    if characters.is_empty() {
        return String::new();
    }
    let mut section = String::from("REGISTERED CHARACTERS:\n");
    for c in characters {
        section.push_str(&format!(
            "- Name: {}, Age: {}, Gender: {}, Personality: {}. Appearance: {}. Default clothing: {}\n",
            c.name,
            c.age.map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string()),
            c.gender.as_deref().unwrap_or("unknown"),
            c.personality.as_deref().unwrap_or("not specified"),
            c.appearance.as_deref().unwrap_or("not specified"),
            c.default_clothing.as_deref().unwrap_or("not specified"),
        ));
    }
    section
}

/// Build the complete context string using Llama 3's chat template format.
///
/// The output uses the `<|start_header_id|>...<|end_header_id|>` format
/// that the existing `generate_story` in chat.rs already uses, so this is
/// a drop-in replacement for the context-building portion of that function.
///
/// Layout:
/// ```text
/// <|start_header_id|>system<|end_header_id|>
/// [system prompt + character DB + story premise]
///
/// === STORY SO FAR ===
/// [compressed summary of older turns]
/// <|eot_id|>
///
/// <|start_header_id|>user<|end_header_id|> [recent turn user input] <|eot_id|>
/// <|start_header_id|>assistant<|end_header_id|> [recent turn story text] <|eot_id|>
/// ...
/// <|start_header_id|>user<|end_header_id|> [current input] <|eot_id|>
/// <|start_header_id|>assistant<|end_header_id|>
/// ```
pub fn build_compressed_context(
    conversation: &mut ConversationContext,
    system_prompt: &str,
    characters: &[CharacterInfo],
    story_premise: Option<&str>,
    current_user_input: &str,
) -> AssembledContext {
    // --- Step 1: Build the system prompt section ---
    let character_section = build_character_section(characters);
    let premise_section = story_premise
        .map(|p| format!("\nSTORY PREMISE:\n{}\n", p))
        .unwrap_or_default();

    let full_system = format!("{}\n{}{}", system_prompt, character_section, premise_section);
    let system_tokens = estimate_tokens(&full_system);
    let input_tokens = estimate_tokens(current_user_input) + 8; // +framing

    // --- Step 2: Check if compression is needed ---
    let was_compressed = if conversation.needs_compression(system_tokens, input_tokens) {
        conversation.compress_with_hints();
        true
    } else {
        false
    };

    // --- Step 3: Assemble the prompt string ---
    let mut prompt = String::new();

    // System message (includes compressed summary if present)
    prompt.push_str(&format!(
        "<|start_header_id|>system<|end_header_id|>\n\n{}",
        full_system
    ));

    // Append compressed history to system message if it exists
    if !conversation.compressed.story_so_far.is_empty() {
        prompt.push_str(&format!(
            "\n\n=== STORY SO FAR ===\n{}\n=== END STORY SO FAR ===",
            conversation.compressed.story_so_far
        ));
    }
    prompt.push_str("<|eot_id|>");

    // Recent turns in full detail
    for turn in &conversation.turns {
        // User message
        prompt.push_str(&format!(
            "<|start_header_id|>user<|end_header_id|>\n\n{}<|eot_id|>",
            turn.user_input
        ));
        // Assistant message (extract just the story text for context efficiency)
        if !turn.assistant_response.is_empty() {
            let story_text = extract_story_text(&turn.assistant_response);
            prompt.push_str(&format!(
                "<|start_header_id|>assistant<|end_header_id|>\n\n{}<|eot_id|>",
                story_text
            ));
        }
    }

    // Current user input
    prompt.push_str(&format!(
        "<|start_header_id|>user<|end_header_id|>\n\n{}<|eot_id|>\
         <|start_header_id|>assistant<|end_header_id|>\n\n",
        current_user_input
    ));

    let estimated_tokens = estimate_tokens(&prompt);

    AssembledContext {
        prompt,
        estimated_tokens,
        was_compressed,
        recent_turn_count: conversation.turns.len(),
        compressed_turn_count: conversation.compressed.compressed_turn_ids.len(),
    }
}

/// Alternative builder that outputs a JSON message array for the /api/chat endpoint.
/// Use this if you prefer the chat API over the raw generate API.
pub fn build_compressed_chat_messages(
    conversation: &mut ConversationContext,
    system_prompt: &str,
    characters: &[CharacterInfo],
    story_premise: Option<&str>,
    current_user_input: &str,
) -> (Vec<Value>, bool) {
    let character_section = build_character_section(characters);
    let premise_section = story_premise
        .map(|p| format!("\nSTORY PREMISE:\n{}\n", p))
        .unwrap_or_default();

    let full_system = format!("{}\n{}{}", system_prompt, character_section, premise_section);
    let system_tokens = estimate_tokens(&full_system);
    let input_tokens = estimate_tokens(current_user_input) + 8;

    let was_compressed = if conversation.needs_compression(system_tokens, input_tokens) {
        conversation.compress_with_hints();
        true
    } else {
        false
    };

    let mut messages: Vec<Value> = Vec::new();

    // System message with compressed history embedded
    let system_content = if !conversation.compressed.story_so_far.is_empty() {
        format!(
            "{}\n\n=== STORY SO FAR ===\n{}\n=== END STORY SO FAR ===",
            full_system, conversation.compressed.story_so_far
        )
    } else {
        full_system
    };
    messages.push(json!({"role": "system", "content": system_content}));

    // Recent turns
    for turn in &conversation.turns {
        messages.push(json!({"role": "user", "content": turn.user_input}));
        if !turn.assistant_response.is_empty() {
            let story_text = extract_story_text(&turn.assistant_response);
            messages.push(json!({"role": "assistant", "content": story_text}));
        }
    }

    // Current input
    messages.push(json!({"role": "user", "content": current_user_input}));

    (messages, was_compressed)
}

// ============================================================================
// 6. LLM SUMMARIZATION HELPER
// ============================================================================

/// Call Ollama to produce a high-quality summary of older turns.
/// This is async and meant to be called from a Tauri command.
///
/// Returns the summary text on success.
pub async fn summarize_with_llm(
    client: &reqwest::Client,
    base_url: &str,
    prompt: &str,
) -> Result<String, String> {
    let body = json!({
        "model": "Story_v27",
        "prompt": prompt,
        "stream": false,
        "options": {
            "num_ctx": 4096,
            "temperature": 0.3  // Low temperature for factual summarization
        }
    });

    let res = client
        .post(format!("{}/api/generate", base_url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LLM summarization request failed: {}", e))?;

    let api_res: Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse LLM summarization response: {}", e))?;

    let summary = api_res["response"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if summary.is_empty() {
        Err("LLM returned empty summary".to_string())
    } else {
        Ok(summary)
    }
}

// ============================================================================
// 7. DIAGNOSTICS
// ============================================================================

/// Compression diagnostics for the frontend to display.
#[derive(Debug, Serialize)]
pub struct CompressionDiagnostics {
    pub total_turns: usize,
    pub compressed_turns: usize,
    pub recent_turns: usize,
    pub estimated_total_tokens: usize,
    pub max_context_tokens: usize,
    pub compression_threshold: usize,
    pub needs_compression: bool,
    pub compressed_summary_preview: String,
}

/// Build diagnostics from a conversation context.
pub fn get_diagnostics(
    conversation: &ConversationContext,
    system_prompt_tokens: usize,
    character_db_tokens: usize,
) -> CompressionDiagnostics {
    let total_est = conversation.estimated_total_tokens(system_prompt_tokens, character_db_tokens);
    let threshold = (MAX_CONTEXT_TOKENS as f64 * COMPRESSION_THRESHOLD) as usize;

    CompressionDiagnostics {
        total_turns: conversation.turns.len() + conversation.compressed.compressed_turn_ids.len(),
        compressed_turns: conversation.compressed.compressed_turn_ids.len(),
        recent_turns: conversation.turns.len(),
        estimated_total_tokens: total_est,
        max_context_tokens: MAX_CONTEXT_TOKENS,
        compression_threshold: threshold,
        needs_compression: total_est > threshold,
        compressed_summary_preview: conversation.compressed.story_so_far
            .chars()
            .take(200)
            .collect::<String>(),
    }
}

// ============================================================================
// 8. TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        // "Hello world" = 11 chars ≈ 3 tokens
        assert_eq!(estimate_tokens("Hello world"), 3);
        // Empty string
        assert_eq!(estimate_tokens(""), 0);
        // 100 chars ≈ 25 tokens
        let text: String = "a".repeat(100);
        assert_eq!(estimate_tokens(&text), 25);
    }

    #[test]
    fn test_extract_summary_hint_full_json() {
        let json = r#"{
            "turn_id": 3,
            "story_json": {
                "response": "The full narrative text...",
                "summary_hint": "Marcus and Elena argue about the missing letter."
            }
        }"#;
        assert_eq!(
            extract_summary_hint(json),
            "Marcus and Elena argue about the missing letter."
        );
    }

    #[test]
    fn test_extract_summary_hint_top_level() {
        let json = r#"{"summary_hint": "They found the treasure."}"#;
        assert_eq!(extract_summary_hint(json), "They found the treasure.");
    }

    #[test]
    fn test_extract_summary_hint_missing() {
        let json = r#"{"story_json": {"response": "Something happened."}}"#;
        assert_eq!(extract_summary_hint(json), "");
    }

    #[test]
    fn test_extract_summary_hint_not_json() {
        let text = "This is just plain text with no JSON.";
        assert_eq!(extract_summary_hint(text), "");
    }

    #[test]
    fn test_story_turn_from_messages() {
        let user = "Open the door";
        let assistant = r#"{"story_json":{"response":"You push open the heavy oak door.","summary_hint":"Player opens the oak door."}}"#;
        let turn = StoryTurn::from_messages(1, user, assistant);
        assert_eq!(turn.turn_number, 1);
        assert_eq!(turn.summary_hint, "Player opens the oak door.");
        assert!(turn.token_estimate > 0);
    }

    #[test]
    fn test_conversation_context_from_pairs() {
        let pairs = vec![
            ("Hello".to_string(), r#"{"story_json":{"response":"World","summary_hint":"Greeting."}}"#.to_string()),
            ("Go north".to_string(), r#"{"story_json":{"response":"You head north.","summary_hint":"Went north."}}"#.to_string()),
        ];
        let ctx = ConversationContext::from_message_pairs(&pairs);
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[0].summary_hint, "Greeting.");
        assert_eq!(ctx.turns[1].summary_hint, "Went north.");
    }

    #[test]
    fn test_compression_not_needed_for_few_turns() {
        let pairs = vec![
            ("A".to_string(), r#"{"story_json":{"response":"B","summary_hint":"C"}}"#.to_string()),
        ];
        let ctx = ConversationContext::from_message_pairs(&pairs);
        assert!(!ctx.needs_compression(200, 100));
    }

    #[test]
    fn test_compress_with_hints() {
        // Create 10 turns with ~200 tokens each to simulate a large conversation
        let mut pairs = Vec::new();
        for i in 0..10 {
            let user = format!("Action for turn {}", i + 1);
            let long_response = "x".repeat(600); // ~150 tokens per response
            let assistant = format!(
                r#"{{"story_json":{{"response":"{}","summary_hint":"Turn {} summary."}}}}"#,
                long_response, i + 1
            );
            pairs.push((user, assistant));
        }
        let mut ctx = ConversationContext::from_message_pairs(&pairs);
        assert_eq!(ctx.turns.len(), 10);

        // Force compression
        ctx.compress_with_hints();

        // Should keep RECENT_TURNS_TO_KEEP turns
        assert_eq!(ctx.turns.len(), RECENT_TURNS_TO_KEEP);
        // Should have compressed the rest
        assert_eq!(ctx.compressed.compressed_turn_ids.len(), 10 - RECENT_TURNS_TO_KEEP);
        // Summary should contain turn summaries
        assert!(ctx.compressed.story_so_far.contains("Turn 1 summary."));
        assert!(ctx.compressed.story_so_far.contains("Turn 2 summary."));
    }

    #[test]
    fn test_build_compressed_context() {
        let pairs = vec![
            ("Hello".to_string(), r#"{"story_json":{"response":"Welcome!","summary_hint":"Greeting."}}"#.to_string()),
            ("Go north".to_string(), r#"{"story_json":{"response":"Forest.","summary_hint":"Went north."}}"#.to_string()),
        ];
        let mut ctx = ConversationContext::from_message_pairs(&pairs);
        let characters = vec![CharacterInfo {
            name: "Marcus".to_string(),
            age: Some(30),
            gender: Some("Male".to_string()),
            personality: Some("Brave".to_string()),
            appearance: Some("Tall, dark hair".to_string()),
            default_clothing: Some("Leather armor".to_string()),
        }];

        let result = build_compressed_context(
            &mut ctx,
            "You are a story engine.",
            &characters,
            Some("A fantasy adventure"),
            "Look around",
        );

        assert!(result.prompt.contains("You are a story engine."));
        assert!(result.prompt.contains("Marcus"));
        assert!(result.prompt.contains("fantasy adventure"));
        assert!(result.prompt.contains("Look around"));
        assert!(result.prompt.contains("Welcome!"));
        assert!(result.estimated_tokens > 0);
    }

    #[test]
    fn test_build_compressed_chat_messages() {
        let pairs = vec![
            ("Hi".to_string(), r#"{"story_json":{"response":"Hello traveler.","summary_hint":"Met traveler."}}"#.to_string()),
        ];
        let mut ctx = ConversationContext::from_message_pairs(&pairs);

        let (messages, compressed) = build_compressed_chat_messages(
            &mut ctx,
            "System prompt",
            &[],
            None,
            "Go east",
        );

        assert!(!compressed);
        assert_eq!(messages.len(), 4); // system + user + assistant + current user
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "Hi");
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(messages[3]["role"], "user");
        assert_eq!(messages[3]["content"], "Go east");
    }

    #[test]
    fn test_from_db_rows() {
        let rows = vec![
            ("user".to_string(), "Hello".to_string()),
            ("assistant".to_string(), r#"{"story_json":{"response":"Hi","summary_hint":"Greeting"}}"#.to_string()),
            ("user".to_string(), "Go north".to_string()),
            ("assistant".to_string(), r#"{"story_json":{"response":"Forest","summary_hint":"Went north"}}"#.to_string()),
        ];
        let ctx = ConversationContext::from_db_rows(&rows);
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[0].user_input, "Hello");
        assert_eq!(ctx.turns[0].summary_hint, "Greeting");
        assert_eq!(ctx.turns[1].user_input, "Go north");
    }

    #[test]
    fn test_llm_summary_prompt_generation() {
        let mut pairs = Vec::new();
        for i in 0..10 {
            pairs.push((
                format!("Action {}", i + 1),
                format!(r#"{{"story_json":{{"response":"Story {}","summary_hint":"Summary {}"}}}}"#, i + 1, i + 1),
            ));
        }
        let ctx = ConversationContext::from_message_pairs(&pairs);

        let prompt = ctx.build_llm_summary_prompt();
        assert!(prompt.is_some());
        let prompt_text = prompt.unwrap();
        assert!(prompt_text.contains("Summarize"));
        assert!(prompt_text.contains("Action 1"));
        assert!(prompt_text.contains("Story 1"));
        // Should NOT contain the most recent turns (they stay uncompressed)
        assert!(!prompt_text.contains(&format!("Action {}", 10)));
    }

    #[test]
    fn test_extract_story_text_variants() {
        // Full LLM format
        let full = r#"{"story_json":{"response":"The door opens.","summary_hint":"Door opened."}}"#;
        assert_eq!(extract_story_text(full), "The door opens.");

        // Top-level response
        let simple = r#"{"response":"A simple response."}"#;
        assert_eq!(extract_story_text(simple), "A simple response.");

        // StoryResponse format
        let sr = r#"{"story":"Story text here."}"#;
        assert_eq!(extract_story_text(sr), "Story text here.");

        // Plain text
        assert_eq!(extract_story_text("Just plain text."), "Just plain text.");
    }

    #[test]
    fn test_diagnostics() {
        let pairs = vec![
            ("A".to_string(), r#"{"story_json":{"response":"B","summary_hint":"C"}}"#.to_string()),
        ];
        let ctx = ConversationContext::from_message_pairs(&pairs);
        let diag = get_diagnostics(&ctx, 200, 100);
        assert_eq!(diag.total_turns, 1);
        assert_eq!(diag.compressed_turns, 0);
        assert_eq!(diag.recent_turns, 1);
        assert_eq!(diag.max_context_tokens, MAX_CONTEXT_TOKENS);
        assert!(!diag.needs_compression);
    }
}