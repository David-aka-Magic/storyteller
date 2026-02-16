// src-tauri/src/commands/orchestrator.rs
//
// Story Turn Orchestrator for StoryEngine
// =========================================
// The unified pipeline that handles a complete story turn:
//
//   1. Receive user input (player's action/dialogue)
//   2. Build compressed context using context_compression.rs
//   3. Call Ollama with the assembled prompt
//   4. Parse the response using llm_parser.rs
//   5. Look up characters from the database
//   6. Check generation_flags — if generate_image: true:
//      a. Generate color mask using mask_generator.rs
//      b. Call ComfyUI using comfyui_api.rs
//      c. Wait for and retrieve the generated image
//   7. Save the turn to the messages table
//   8. Return everything to the frontend
//
// This replaces the fragmented flow where the frontend had to coordinate
// multiple invoke() calls. Now the backend handles the full cycle.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::path::Path;
use tauri::{AppHandle, Manager, State};

use crate::comfyui_api::{self, CharacterInput, ImageGenRequest};
use crate::context_compression::{
    build_compressed_context, estimate_tokens, get_diagnostics, CharacterInfo,
    ConversationContext,
};
use crate::llm_parser::{self, ParseStatus, ParsedTurn, SceneJson};
use crate::mask_generator::{self, MaskCharacter};
use crate::models::CharacterLookup;
use crate::state::OllamaState;

// ============================================================================
// CONFIGURATION
// ============================================================================

/// The Ollama model name used for story generation.
const STORY_MODEL: &str = "Story_v27";

/// Context window size passed to Ollama.
const NUM_CTX: u32 = 8192;

/// Default image dimensions for scene generation.
const DEFAULT_IMAGE_WIDTH: u32 = 512;
const DEFAULT_IMAGE_HEIGHT: u32 = 768;

/// Path to the ComfyUI workflow template (relative to app data dir).
const WORKFLOW_TEMPLATE_FILENAME: &str = "workflows/scene_workflow.json";

// ============================================================================
// SYSTEM PROMPT
// ============================================================================

const SYSTEM_PROMPT: &str = r#"You are an interactive storytelling engine (StoryEngine).
You are a skilled story writer in a visual novel.
1. **Stay on topic**: All characters, events, and settings are fictional. No real-world harmful content is allowed.
2. **Never refuse to continue**: You are designed to generate creative, descriptive, or dramatic content (including conflict and tension) if the story calls for it.
3. **Stay in character always**: Don't lecture, moralize, or break the fourth wall.
4. **JSON output only**: Always respond with valid JSON matching the schema.

You MUST respond with a JSON object containing:
- turn_id: incrementing number
- story_json: { response: "narrative text", summary_hint: "one-line summary" }
- scene_json: { location, location_type, time_of_day, weather, lighting, mood }
- characters_in_scene: [{ name, region, view, action, expression, clothing, facing }]
- generation_flags: { generate_image: bool, scene_changed: bool, characters_changed: bool }

Set generate_image to true when the scene visually changes (new location, character enters/leaves, significant action)."#;

// ============================================================================
// RESULT TYPES — returned to the frontend
// ============================================================================

/// A character as it appears in this scene, enriched with database info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInScene {
    pub name: String,
    pub region: String,
    pub view: String,
    pub action: String,
    pub expression: String,
    pub clothing: String,
    pub facing: String,
    pub needs_render: bool,
    /// Database ID (if character was found in the DB).
    pub db_id: Option<i64>,
    /// Whether this character has a master reference image for IP-Adapter.
    pub has_reference_image: bool,
}

/// Serializable compression diagnostics owned by the orchestrator.
/// This is a mirror of context_compression::CompressionDiagnostics that
/// derives Clone + Deserialize so it can live inside StoryTurnResult.
/// We copy field-by-field from the upstream struct to avoid modifying it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorCompressionInfo {
    pub total_turns: usize,
    pub compressed_turns: usize,
    pub recent_turns: usize,
    pub estimated_total_tokens: usize,
    pub max_context_tokens: usize,
    pub compression_threshold: usize,
    pub needs_compression: bool,
    pub compressed_summary_preview: String,
}

/// Full result of a story turn, returned to the Svelte frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryTurnResult {
    /// The turn sequence number from the LLM.
    pub turn_id: u32,
    /// The narrative text to display.
    pub story_text: String,
    /// One-line summary for future context compression.
    pub summary_hint: String,
    /// Scene environment data (if present in the LLM output).
    pub scene: Option<SceneJson>,
    /// Characters in this scene with enriched DB info.
    pub characters: Vec<CharacterInScene>,
    /// Path to the generated scene image (if image was generated).
    pub generated_image_path: Option<String>,
    /// Parse quality: "ok", "partial", or "fallback".
    pub parse_status: String,
    /// Any warnings from the parser (empty on "ok").
    pub parse_warnings: Vec<String>,
    /// Context compression diagnostics for the frontend token meter.
    pub compression_info: OrchestratorCompressionInfo,
    /// Whether image generation was attempted.
    pub image_generation_attempted: bool,
    /// If image generation failed, the error message.
    pub image_generation_error: Option<String>,
}

// ============================================================================
// HELPERS
// ============================================================================

/// Extract JSON from text that might have preamble or markdown fences.
fn extract_json_from_text(text: &str) -> Option<String> {
    if let Some(start) = text.find('{') {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape = false;
        for (i, ch) in text[start..].char_indices() {
            if escape {
                escape = false;
                continue;
            }
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

/// Load message history from the DB for a given chat.
async fn load_message_history(
    db: &sqlx::SqlitePool,
    chat_id: i64,
) -> Result<Vec<(String, String)>, String> {
    let rows = sqlx::query(
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC",
    )
    .bind(chat_id)
    .fetch_all(db)
    .await
    .map_err(|e| format!("Failed to load messages: {}", e))?;

    Ok(rows
        .iter()
        .map(|r| {
            let role: String = r.get("role");
            let content: String = r.get("content");
            (role, content)
        })
        .collect())
}

/// Load characters for a story (or all characters if no story_id).
async fn load_characters_for_context(
    db: &sqlx::SqlitePool,
    story_id: Option<i64>,
) -> Result<Vec<CharacterInfo>, String> {
    let rows = if let Some(sid) = story_id {
        sqlx::query(
            "SELECT name, age, gender, personality, sd_prompt, default_clothing \
             FROM characters WHERE story_id = ? ORDER BY name",
        )
        .bind(sid)
        .fetch_all(db)
        .await
    } else {
        sqlx::query(
            "SELECT name, age, gender, personality, sd_prompt, default_clothing \
             FROM characters ORDER BY name",
        )
        .fetch_all(db)
        .await
    }
    .map_err(|e| format!("Failed to load characters: {}", e))?;

    Ok(rows
        .iter()
        .map(|r| CharacterInfo {
            name: r.get("name"),
            age: r.get("age"),
            gender: r.get("gender"),
            personality: r.get("personality"),
            appearance: r.get("sd_prompt"),
            default_clothing: r.get("default_clothing"),
        })
        .collect())
}

/// Load the active story premise (most recent).
async fn load_story_premise(db: &sqlx::SqlitePool) -> Result<Option<String>, String> {
    let row = sqlx::query(
        "SELECT title, description FROM story_premises ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Failed to load story premise: {}", e))?;

    Ok(row.map(|r| {
        let title: String = r.get("title");
        let desc: String = r.get("description");
        format!("Title: {}\nPremise: {}", title, desc)
    }))
}

/// Look up scene characters in the database.
async fn lookup_characters_in_db(
    db: &sqlx::SqlitePool,
    parsed: &ParsedTurn,
    story_id: Option<i64>,
) -> Result<Vec<(llm_parser::SceneCharacterRaw, Option<CharacterLookup>)>, String> {
    let mut results = Vec::new();

    for scene_char in &parsed.turn.characters_in_scene {
        if scene_char.name.is_empty() {
            continue;
        }

        let row = if let Some(sid) = story_id {
            sqlx::query(
                "SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style \
                 FROM characters WHERE name = ? AND story_id = ? LIMIT 1",
            )
            .bind(&scene_char.name)
            .bind(sid)
            .fetch_optional(db)
            .await
        } else {
            sqlx::query(
                "SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style \
                 FROM characters WHERE name = ? LIMIT 1",
            )
            .bind(&scene_char.name)
            .fetch_optional(db)
            .await
        }
        .map_err(|e| format!("Character lookup failed for '{}': {}", scene_char.name, e))?;

        let lookup = row.map(|r| CharacterLookup {
            id: r.get("id"),
            name: r.get("name"),
            master_image_path: r.get("master_image_path"),
            sd_prompt: r.get("sd_prompt"),
            default_clothing: r.get("default_clothing"),
            art_style: r.get("art_style"),
        });

        results.push((scene_char.clone(), lookup));
    }

    Ok(results)
}

/// Build the enriched CharacterInScene list from parsed + DB data.
fn build_characters_in_scene(
    lookup_results: &[(llm_parser::SceneCharacterRaw, Option<CharacterLookup>)],
) -> Vec<CharacterInScene> {
    lookup_results
        .iter()
        .map(|(raw, db_char)| {
            let typed = raw.typed();
            CharacterInScene {
                name: raw.name.clone(),
                region: raw.region.clone(),
                view: raw.view.clone(),
                action: raw.action.clone(),
                expression: raw.expression.clone(),
                clothing: if raw.clothing.is_empty() {
                    db_char
                        .as_ref()
                        .and_then(|c| c.default_clothing.clone())
                        .unwrap_or_default()
                } else {
                    raw.clothing.clone()
                },
                facing: raw.facing.clone(),
                needs_render: typed.needs_render(),
                db_id: db_char.as_ref().map(|c| c.id),
                has_reference_image: db_char
                    .as_ref()
                    .and_then(|c| c.master_image_path.as_ref())
                    .map(|p| !p.is_empty())
                    .unwrap_or(false),
            }
        })
        .collect()
}

/// Build a per-character prompt fragment for image generation.
fn character_prompt_fragment(
    char_in_scene: &CharacterInScene,
    db_char: Option<&CharacterLookup>,
) -> String {
    let mut parts = Vec::new();

    if let Some(db) = db_char {
        if let Some(ref sd) = db.sd_prompt {
            if !sd.is_empty() {
                parts.push(sd.clone());
            }
        }
    }

    if !char_in_scene.view.is_empty() && char_in_scene.view != "NONE" {
        parts.push(char_in_scene.view.to_lowercase().replace('-', " "));
    }
    if !char_in_scene.expression.is_empty() {
        parts.push(char_in_scene.expression.clone());
    }
    if !char_in_scene.clothing.is_empty() {
        parts.push(char_in_scene.clothing.clone());
    }
    if !char_in_scene.action.is_empty() {
        parts.push(char_in_scene.action.clone());
    }

    parts.join(", ")
}

/// Estimate token cost of the character DB section for diagnostics.
fn estimate_character_db_tokens(characters: &[CharacterInfo]) -> usize {
    if characters.is_empty() {
        return 0;
    }
    let mut total = estimate_tokens("REGISTERED CHARACTERS:\n");
    for c in characters {
        let line = format!(
            "- Name: {}, Age: {}, Gender: {}, Personality: {}. Appearance: {}. Default clothing: {}\n",
            c.name,
            c.age.map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string()),
            c.gender.as_deref().unwrap_or("unknown"),
            c.personality.as_deref().unwrap_or("not specified"),
            c.appearance.as_deref().unwrap_or("not specified"),
            c.default_clothing.as_deref().unwrap_or("not specified"),
        );
        total += estimate_tokens(&line);
    }
    total
}

// ============================================================================
// IMAGE GENERATION SUB-PIPELINE
// ============================================================================

/// Run the image generation sub-pipeline:
///   1. Filter to renderable characters with reference images
///   2. Generate a color mask
///   3. Build the ComfyUI request
///   4. Call generate_scene_image
///
/// Returns (image_path, None) on success, (None, Some(error)) on failure.
async fn generate_image_for_scene(
    parsed: &ParsedTurn,
    characters_in_scene: &[CharacterInScene],
    lookup_results: &[(llm_parser::SceneCharacterRaw, Option<CharacterLookup>)],
    app: &AppHandle,
) -> (Option<String>, Option<String>) {
    // 1. Filter to renderable characters that have reference images
    let renderable: Vec<_> = characters_in_scene
        .iter()
        .zip(lookup_results.iter())
        .filter(|(cis, _)| cis.needs_render && cis.has_reference_image)
        .collect();

    if renderable.is_empty() {
        println!(
            "[Orchestrator] No renderable characters with reference images — skipping image gen"
        );
        return (
            None,
            Some("No characters with reference images available for rendering".to_string()),
        );
    }

    let app_data = match app.path().app_data_dir() {
        Ok(p) => p,
        Err(e) => {
            return (None, Some(format!("Failed to get app data dir: {}", e)));
        }
    };

    // 2. Generate color mask
    let mask_characters: Vec<MaskCharacter> = renderable
        .iter()
        .enumerate()
        .map(|(i, (cis, _))| MaskCharacter {
            name: cis.name.clone(),
            region: cis.region.clone(),
            color_index: i.min(2), // Max 3 colors (0, 1, 2)
        })
        .collect();

    let masks_dir = app_data.join("masks");
    let mask_result = match mask_generator::generate_mask(
        &mask_characters,
        DEFAULT_IMAGE_WIDTH,
        DEFAULT_IMAGE_HEIGHT,
        &masks_dir,
        None,
    ) {
        Ok(r) => {
            println!(
                "[Orchestrator] Mask generated: {} ({} regions)",
                r.path, r.regions_drawn
            );
            r
        }
        Err(e) => {
            return (None, Some(format!("Mask generation failed: {}", e)));
        }
    };

    // 3. Build ComfyUI character inputs
    let comfy_characters: Vec<CharacterInput> = renderable
        .iter()
        .map(|(cis, (_, db_char))| {
            let ref_path = db_char
                .as_ref()
                .and_then(|c| c.master_image_path.clone())
                .unwrap_or_default();

            CharacterInput {
                name: cis.name.clone(),
                reference_image_path: ref_path,
                region: cis.region.clone(),
                prompt: character_prompt_fragment(cis, db_char.as_ref()),
            }
        })
        .collect();

    // 4. Build the scene prompt
    let scene_prompt = parsed.scene_prompt_fragment();

    // 5. Find workflow template
    let workflow_path = app_data.join(WORKFLOW_TEMPLATE_FILENAME);
    let workflow_template = if workflow_path.exists() {
        workflow_path.to_string_lossy().to_string()
    } else {
        // Fallback: check in resources directory
        let resources_path = app
            .path()
            .resource_dir()
            .ok()
            .map(|p| p.join(WORKFLOW_TEMPLATE_FILENAME));

        match resources_path {
            Some(p) if p.exists() => p.to_string_lossy().to_string(),
            _ => {
                return (
                    None,
                    Some(format!(
                        "Workflow template not found at: {}",
                        workflow_path.display()
                    )),
                );
            }
        }
    };

    // 6. Build the full image generation request
    let request = ImageGenRequest {
        scene_prompt,
        characters: comfy_characters,
        mask_path: mask_result.path,
        workflow_template,
        comfyui_url: None,
        seed: None,
        steps: None,
        cfg: None,
        width: Some(DEFAULT_IMAGE_WIDTH),
        height: Some(DEFAULT_IMAGE_HEIGHT),
        negative_prompt: None,
        timeout_secs: None,
    };

    // 7. Run the ComfyUI pipeline
    let output_dir = app_data.join("scene_images");
    match comfyui_api::generate_scene_image(&request, &output_dir).await {
        Ok(result) => {
            let primary_image = result.image_paths.first().cloned();
            println!(
                "[Orchestrator] Image generated: {:?} ({} images total)",
                primary_image,
                result.image_paths.len()
            );
            (primary_image, None)
        }
        Err(e) => {
            println!("[Orchestrator] Image generation failed: {}", e);
            (None, Some(e.to_string()))
        }
    }
}

// ============================================================================
// DATABASE PERSISTENCE
// ============================================================================

/// Save the user message and assistant response to the messages table.
/// Also saves the generated image path to the images table if present.
/// Returns the assistant message ID.
async fn save_turn_to_db(
    db: &sqlx::SqlitePool,
    chat_id: i64,
    user_input: &str,
    raw_assistant_response: &str,
    image_path: Option<&str>,
) -> Result<i64, String> {
    let mut tx = db
        .begin()
        .await
        .map_err(|e| format!("Transaction start failed: {}", e))?;

    // Update chat title if still "New Chat"
    let title_preview: String = user_input.chars().take(40).collect();
    sqlx::query("UPDATE chats SET title = ? WHERE id = ? AND title = 'New Chat'")
        .bind(&title_preview)
        .bind(chat_id)
        .execute(&mut *tx)
        .await
        .ok();

    // Save user message
    sqlx::query("INSERT INTO messages (chat_id, role, content) VALUES (?, 'user', ?)")
        .bind(chat_id)
        .bind(user_input)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to save user message: {}", e))?;

    // Save assistant message
    let result = sqlx::query(
        "INSERT INTO messages (chat_id, role, content) VALUES (?, 'assistant', ?)",
    )
    .bind(chat_id)
    .bind(raw_assistant_response)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to save assistant message: {}", e))?;

    let assistant_msg_id = result.last_insert_rowid();

    // Save image reference if an image was generated
    if let Some(path) = image_path {
        sqlx::query(
            "INSERT INTO images (message_id, chat_id, file_path) VALUES (?, ?, ?)",
        )
        .bind(assistant_msg_id)
        .bind(chat_id)
        .bind(path)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to save image record: {}", e))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Transaction commit failed: {}", e))?;

    Ok(assistant_msg_id)
}

// ============================================================================
// THE MAIN ORCHESTRATOR COMMAND
// ============================================================================

/// Process a complete story turn. This is the single Tauri command that the
/// frontend calls to advance the story by one step.
///
/// ## Frontend usage
/// ```typescript
/// const result = await invoke('process_story_turn', {
///   chatId: currentChatId,
///   userInput: playerAction,
///   storyId: currentStoryId  // optional, null for global
/// });
/// ```
#[tauri::command]
pub async fn process_story_turn(
    chat_id: i64,
    user_input: String,
    story_id: Option<i64>,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<StoryTurnResult, String> {
    let start_time = std::time::Instant::now();
    println!(
        "\n[Orchestrator] ========== TURN START (chat={}, story={:?}) ==========",
        chat_id, story_id
    );
    println!(
        "[Orchestrator] User input: {}",
        &user_input[..user_input.len().min(80)]
    );

    // ── Step 1: Load history and build compressed context ──────────────

    let message_rows = load_message_history(&state.db, chat_id).await?;
    let mut conversation = ConversationContext::from_db_rows(&message_rows);

    let characters = load_characters_for_context(&state.db, story_id).await?;
    let premise = load_story_premise(&state.db).await?;

    let assembled = build_compressed_context(
        &mut conversation,
        SYSTEM_PROMPT,
        &characters,
        premise.as_deref(),
        &user_input,
    );

    println!(
        "[Orchestrator] Context: ~{} tokens, compressed={}, recent={} turns, compressed={} turns",
        assembled.estimated_tokens,
        assembled.was_compressed,
        assembled.recent_turn_count,
        assembled.compressed_turn_count,
    );

    // Build compression diagnostics — get_diagnostics takes 3 args:
    //   (&conversation, system_prompt_tokens, character_db_tokens)
    let system_prompt_tokens = estimate_tokens(SYSTEM_PROMPT);
    let character_db_tokens = estimate_character_db_tokens(&characters);
    let diag = get_diagnostics(&conversation, system_prompt_tokens, character_db_tokens);

    // Copy into our own serializable struct to avoid needing Clone/Deserialize
    // on the upstream CompressionDiagnostics which only derives Debug + Serialize
    let compression_info = OrchestratorCompressionInfo {
        total_turns: diag.total_turns,
        compressed_turns: diag.compressed_turns,
        recent_turns: diag.recent_turns,
        estimated_total_tokens: diag.estimated_total_tokens,
        max_context_tokens: diag.max_context_tokens,
        compression_threshold: diag.compression_threshold,
        needs_compression: diag.needs_compression,
        compressed_summary_preview: diag.compressed_summary_preview,
    };

    // ── Step 2: Call Ollama ────────────────────────────────────────────

    let ollama_start = std::time::Instant::now();

    let res = state
        .client
        .post(format!("{}/api/generate", state.base_url))
        .json(&json!({
            "model": STORY_MODEL,
            "prompt": assembled.prompt,
            "stream": false,
            "options": {
                "num_ctx": NUM_CTX
            }
        }))
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    let api_res: Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    let response_text = api_res["response"]
        .as_str()
        .ok_or_else(|| "No 'response' field in Ollama API result".to_string())?;

    println!(
        "[Orchestrator] Ollama responded in {:.1}s ({} chars)",
        ollama_start.elapsed().as_secs_f64(),
        response_text.len()
    );

    // ── Step 3: Parse the LLM output ──────────────────────────────────

    let parsed = llm_parser::parse_llm_output(response_text);

    let (parse_status, parse_warnings) = match &parsed.status {
        ParseStatus::Ok => ("ok".to_string(), vec![]),
        ParseStatus::Partial(w) => ("partial".to_string(), w.clone()),
        ParseStatus::Fallback => (
            "fallback".to_string(),
            vec!["LLM output was not valid JSON; raw text was preserved".to_string()],
        ),
    };

    println!(
        "[Orchestrator] Parse status: {} (turn_id={}, {} characters, generate_image={})",
        parse_status,
        parsed.turn.turn_id,
        parsed.turn.characters_in_scene.len(),
        parsed.should_generate_image()
    );

    // ── Step 4: Look up characters in the database ────────────────────

    let lookup_results = lookup_characters_in_db(&state.db, &parsed, story_id).await?;
    let characters_in_scene = build_characters_in_scene(&lookup_results);

    let found_count = lookup_results
        .iter()
        .filter(|(_, db)| db.is_some())
        .count();
    let ref_count = characters_in_scene
        .iter()
        .filter(|c| c.has_reference_image)
        .count();
    println!(
        "[Orchestrator] Characters: {} in scene, {} found in DB, {} with reference images",
        characters_in_scene.len(),
        found_count,
        ref_count
    );

    // ── Step 5: Conditional image generation ──────────────────────────

    let flags = parsed.flags();
    let mut generated_image_path: Option<String> = None;
    let mut image_generation_attempted = false;
    let mut image_generation_error: Option<String> = None;

    if flags.generate_image {
        image_generation_attempted = true;
        println!("[Orchestrator] Image generation requested — starting pipeline...");

        let img_start = std::time::Instant::now();
        let (path, error) =
            generate_image_for_scene(&parsed, &characters_in_scene, &lookup_results, &app).await;

        generated_image_path = path;
        image_generation_error = error;

        println!(
            "[Orchestrator] Image pipeline completed in {:.1}s (success={})",
            img_start.elapsed().as_secs_f64(),
            generated_image_path.is_some()
        );
    } else {
        println!("[Orchestrator] No image generation requested for this turn");
    }

    // ── Step 6: Save to database ──────────────────────────────────────

    let raw_content =
        extract_json_from_text(response_text).unwrap_or_else(|| response_text.to_string());

    save_turn_to_db(
        &state.db,
        chat_id,
        &user_input,
        &raw_content,
        generated_image_path.as_deref(),
    )
    .await?;

    // ── Step 7: Build and return the result ───────────────────────────

    let total_elapsed = start_time.elapsed();
    println!(
        "[Orchestrator] ========== TURN COMPLETE in {:.1}s ==========\n",
        total_elapsed.as_secs_f64()
    );

    Ok(StoryTurnResult {
        turn_id: parsed.turn.turn_id,
        story_text: parsed.story_text().to_string(),
        summary_hint: parsed.summary_hint().to_string(),
        scene: parsed.turn.scene_json.clone(),
        characters: characters_in_scene,
        generated_image_path,
        parse_status,
        parse_warnings,
        compression_info,
        image_generation_attempted,
        image_generation_error,
    })
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_text() {
        let input =
            r#"Here is the story: {"turn_id": 1, "story_json": {"response": "Hello"}} end"#;
        let json = extract_json_from_text(input).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.contains("turn_id"));
    }

    #[test]
    fn test_extract_json_no_json() {
        assert!(extract_json_from_text("no json here").is_none());
    }

    #[test]
    fn test_character_prompt_fragment_full() {
        let cis = CharacterInScene {
            name: "Elena".into(),
            region: "left".into(),
            view: "FULL-BODY".into(),
            action: "walking toward table".into(),
            expression: "warm smile".into(),
            clothing: "blue jacket, white t-shirt".into(),
            facing: "Marcus".into(),
            needs_render: true,
            db_id: Some(1),
            has_reference_image: true,
        };
        let db = CharacterLookup {
            id: 1,
            name: "Elena".into(),
            master_image_path: Some("/path/to/ref.png".into()),
            sd_prompt: Some("young woman, brown hair".into()),
            default_clothing: Some("casual dress".into()),
            art_style: Some("Realistic".into()),
        };
        let fragment = character_prompt_fragment(&cis, Some(&db));
        assert!(fragment.contains("young woman, brown hair"));
        assert!(fragment.contains("warm smile"));
        assert!(fragment.contains("blue jacket"));
    }

    #[test]
    fn test_character_prompt_fragment_no_db() {
        let cis = CharacterInScene {
            name: "Unknown".into(),
            region: "center".into(),
            view: "PORTRAIT".into(),
            action: "".into(),
            expression: "neutral".into(),
            clothing: "dark suit".into(),
            facing: "".into(),
            needs_render: true,
            db_id: None,
            has_reference_image: false,
        };
        let fragment = character_prompt_fragment(&cis, None);
        assert!(fragment.contains("portrait"));
        assert!(fragment.contains("neutral"));
        assert!(fragment.contains("dark suit"));
    }

    #[test]
    fn test_build_characters_in_scene_clothing_fallback() {
        let raw = llm_parser::SceneCharacterRaw {
            name: "Marcus".into(),
            region: "left".into(),
            view: "FULL-BODY".into(),
            action: "standing".into(),
            expression: "serious".into(),
            clothing: "".into(),
            facing: "Elena".into(),
        };
        let db = Some(CharacterLookup {
            id: 1,
            name: "Marcus".into(),
            master_image_path: Some("/ref.png".into()),
            sd_prompt: None,
            default_clothing: Some("leather armor".into()),
            art_style: None,
        });
        let results = vec![(raw, db)];
        let chars = build_characters_in_scene(&results);

        assert_eq!(chars[0].clothing, "leather armor");
        assert!(chars[0].has_reference_image);
    }

    #[test]
    fn test_estimate_character_db_tokens() {
        let chars = vec![CharacterInfo {
            name: "Marcus".to_string(),
            age: Some(30),
            gender: Some("Male".to_string()),
            personality: Some("Brave".to_string()),
            appearance: Some("Tall, dark hair".to_string()),
            default_clothing: Some("Leather armor".to_string()),
        }];
        let tokens = estimate_character_db_tokens(&chars);
        assert!(tokens > 0);
        assert!(tokens < 100);

        assert_eq!(estimate_character_db_tokens(&[]), 0);
    }
}