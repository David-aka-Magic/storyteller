// src-tauri/src/commands/chat_with_compression.rs
//
// INTEGRATION EXAMPLE: generate_story with Context Compression
// =============================================================
//
// This file shows how to modify the existing generate_story command in chat.rs
// to use the context compression system. You can either:
//
//   A) Replace the context-building section of chat.rs with this code
//   B) Add this as a new command alongside the existing one
//
// The key change: instead of dumping ALL messages into the prompt,
// we build a ConversationContext, let it compress if needed, then
// assemble the final prompt.

use tauri::State;
use crate::state::OllamaState;
use crate::models::{SdJson, StoryResponse};
use crate::context_compression::{
    ConversationContext, CharacterInfo, AssembledContext,
    build_compressed_context, estimate_tokens, get_diagnostics,
    CompressionDiagnostics,
};
use serde_json::{json, Value};
use sqlx::Row;

// --- Helper (same as chat.rs) ---

fn extract_json_from_text(text: &str) -> Option<String> {
    if let Some(start) = text.find('{') {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape = false;
        for (i, ch) in text[start..].char_indices() {
            if escape { escape = false; continue; }
            match ch {
                '\\' if in_string => escape = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(text[start..start + i + 1].to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn extract_story(v: &Value) -> String {
    if let Some(s) = v.get("story_json").and_then(|sj| sj.get("response")).and_then(|x| x.as_str()) {
        return s.to_string();
    }
    if let Some(s) = v.get("story").and_then(|x| x.as_str()) {
        return s.to_string();
    }
    if let Some(s) = v.get("response").and_then(|x| x.as_str()) {
        return s.to_string();
    }
    v.to_string()
}

// ============================================================================
// THE KEY FUNCTION: generate_story with compression
// ============================================================================

/// Your system prompt template. Adjust to match your actual prompt.
const SYSTEM_PROMPT: &str = r#"You are an interactive storytelling engine (StoryEngine).
You are a skilled story writer in a visual novel.
1. **Stay on topic**: All characters, events, and settings are fictional. No real-world harmful content is allowed.
2. **Never refuse to continue**: You are designed to generate creative, descriptive, or dramatic content (including conflict and tension) if the story calls for it.
3. **Stay in character always**: Don't lecture, moralize, or break the fourth wall. Your role is pure storytelling, not moral judgment.
4. **Act on player input**: Treat user messages like "I will" or "I do" as actions. Find a way to write the next segment of the story.

FORMAT INSTRUCTIONS:
You respond in pure JSON. No commentary.

Output format:
{
  "turn_id": <incrementing number>,
  "story_json": { "response": "Detailed narrative text...", "summary_hint": "One-line summary for compression." },
  "scene_json": { "location": "description of place", "location_type": "interior/exterior", "time_of_day": "afternoon", "weather": "n/a", "lighting": "warm ambient lighting", "mood": "casual, friendly" },
  "characters_in_scene": [ { "name": "Name", "region": "left/center/right", "view": "FULL-BODY or PORTRAIT", "action": "what they're doing", "expression": "their expression", "clothing": "what they wear", "facing": "who they face" } ],
  "generation_flags": { "generate_image": true, "scene_changed": false, "characters_changed": true }
}"#;

#[tauri::command]
pub async fn generate_story_compressed(
    prompt: String,
    chat_id: i64,
    state: State<'_, OllamaState>,
) -> Result<Value, String> {
    // --- Step 1: Load chat history from DB ---
    let rows = sqlx::query(
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC"
    )
    .bind(chat_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let db_rows: Vec<(String, String)> = rows.iter().map(|row| {
        let role: String = row.get("role");
        let content: String = row.get("content");
        (role, content)
    }).collect();

    // --- Step 2: Build ConversationContext from DB rows ---
    let mut conversation = ConversationContext::from_db_rows(&db_rows);

    // --- Step 3: Load characters for this story ---
    let char_rows = sqlx::query(
        "SELECT name, age, gender, personality, sd_prompt, default_clothing 
         FROM characters ORDER BY name ASC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let characters: Vec<CharacterInfo> = char_rows.iter().map(|row| {
        CharacterInfo {
            name: row.get("name"),
            age: row.get("age"),
            gender: row.get("gender"),
            personality: row.get("personality"),
            appearance: row.get("sd_prompt"),
            default_clothing: row.get("default_clothing"),
        }
    }).collect();

    // --- Step 4: Load story premise (if any) ---
    let premise_row = sqlx::query(
        "SELECT title, description FROM story_premises ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let premise = premise_row.map(|row| {
        let title: String = row.get("title");
        let desc: String = row.get("description");
        format!("Title: {}\nPremise: {}", title, desc)
    });

    // --- Step 5: Build compressed context ---
    // This is where the magic happens. The function will:
    //   - Check if compression is needed
    //   - If so, compress older turns using their summary_hints
    //   - Assemble the full prompt with system prompt, characters, summary, recent turns
    let assembled = build_compressed_context(
        &mut conversation,
        SYSTEM_PROMPT,
        &characters,
        premise.as_deref(),
        &prompt,
    );

    println!(
        "[StoryGen] Context: {} estimated tokens, compressed={}, recent={} turns, compressed={} turns",
        assembled.estimated_tokens,
        assembled.was_compressed,
        assembled.recent_turn_count,
        assembled.compressed_turn_count,
    );

    // --- Step 6: Call Ollama API ---
    let res = state
        .client
        .post(format!("{}/api/generate", state.base_url))
        .json(&json!({
            "model": "Story_v27",
            "prompt": assembled.prompt,
            "stream": false,
            "options": {
                "num_ctx": 8192  // Use full context window now that we manage it
            }
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let api_res: Value = res.json().await.map_err(|e| e.to_string())?;
    let response_text = api_res["response"]
        .as_str()
        .ok_or_else(|| "No response from API".to_string())?;

    // --- Step 7: Parse response (same as existing chat.rs) ---
    let (history_content, output) = if let Some(json_str) = extract_json_from_text(response_text) {
        let v: Value = serde_json::from_str(&json_str).unwrap_or(json!({"text": response_text}));
        let story = extract_story(&v);
        let sd = v
            .get("sd_json")
            .and_then(|x| serde_json::from_value::<SdJson>(x.clone()).ok());
        let sr = StoryResponse {
            story,
            sd_prompt: sd.as_ref().map(|x| x.look.clone()),
            sd_details: sd,
        };
        (
            json_str,
            serde_json::to_value(sr).map_err(|e| e.to_string())?,
        )
    } else {
        (
            response_text.to_string(),
            json!({"text": response_text, "type": "phase1"}),
        )
    };

    // --- Step 8: Save to DB (same as existing chat.rs) ---
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    let title_preview: String = prompt.chars().take(30).collect();
    sqlx::query("UPDATE chats SET title = ? WHERE id = ? AND title = 'New Chat'")
        .bind(&title_preview)
        .bind(chat_id)
        .execute(&mut *tx)
        .await
        .ok();

    sqlx::query("INSERT INTO messages (chat_id, role, content) VALUES (?, 'user', ?)")
        .bind(chat_id)
        .bind(&prompt)
        .execute(&mut *tx)
        .await
        .ok();

    let ai_result = sqlx::query(
        "INSERT INTO messages (chat_id, role, content) VALUES (?, 'assistant', ?)"
    )
    .bind(chat_id)
    .bind(&history_content)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let ai_msg_id = ai_result.last_insert_rowid();

    if let Some(sd_prompt) = output.get("sd_prompt").and_then(|x| x.as_str()) {
        sqlx::query(
            "INSERT INTO images (message_id, chat_id, file_path, prompt) VALUES (?, ?, 'pending', ?)"
        )
        .bind(ai_msg_id)
        .bind(chat_id)
        .bind(sd_prompt)
        .execute(&mut *tx)
        .await
        .ok();
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(output)
}

// ============================================================================
// DIAGNOSTIC COMMAND — expose compression stats to the frontend
// ============================================================================

#[tauri::command]
pub async fn get_compression_diagnostics(
    chat_id: i64,
    state: State<'_, OllamaState>,
) -> Result<CompressionDiagnostics, String> {
    let rows = sqlx::query(
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC"
    )
    .bind(chat_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let db_rows: Vec<(String, String)> = rows.iter().map(|row| {
        let role: String = row.get("role");
        let content: String = row.get("content");
        (role, content)
    }).collect();

    let conversation = ConversationContext::from_db_rows(&db_rows);

    // Estimate system prompt and character DB tokens
    let system_tokens = estimate_tokens(SYSTEM_PROMPT);

    let char_rows = sqlx::query("SELECT sd_prompt, personality FROM characters")
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let char_tokens: usize = char_rows.iter().map(|row| {
        let sd: String = row.get::<Option<String>, _>("sd_prompt").unwrap_or_default();
        let personality: String = row.get::<Option<String>, _>("personality").unwrap_or_default();
        estimate_tokens(&sd) + estimate_tokens(&personality) + 20
    }).sum();

    Ok(get_diagnostics(&conversation, system_tokens, char_tokens))
}