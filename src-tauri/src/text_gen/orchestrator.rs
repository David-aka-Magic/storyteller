// src-tauri/src/text_gen/orchestrator.rs
//
// Story Turn Orchestrator for StoryEngine
// =========================================
// The unified pipeline that handles a complete story turn:
//
//   1. Receive user input (player's action/dialogue)
//   2. Build compressed context using text_gen::context
//   3. Call Ollama with the assembled prompt
//   4. Parse the response using text_gen::parser
//   5. Look up characters from the database
//   6. Check generation_flags — if generate_image: true:
//      a. Generate color mask using mask_generator.rs
//      b. Call ComfyUI using comfyui_api.rs (selects 1-char or 2-char workflow)
//      c. Wait for and retrieve the generated image
//   7. Save the turn to the messages table
//   8. Return everything to the frontend

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;

use tauri::{AppHandle, Manager, State};

use crate::config::ConfigState;
use crate::image_gen::comfyui::{self as comfyui_api, CharacterInput, ImageGenRequest};
use crate::image_gen::masks::{self as mask_generator, MaskCharacter};
use crate::text_gen::context::{
    build_compressed_context, estimate_tokens, get_diagnostics, CharacterInfo,
    CompressionDiagnostics, ConversationContext,
};
use crate::text_gen::parser::{self as llm_parser, CharacterEmotionalState, ParseStatus, ParsedTurn, SceneJson};
use crate::text_gen::prompts::{
    get_response_length_config, NUM_CTX, OLLAMA_MAX_RETRIES, OLLAMA_REQUEST_TIMEOUT_SECS,
    STORY_MODEL, SYSTEM_PROMPT,
};
use std::time::Duration;
use crate::models::CharacterLookup;
use crate::state::{OllamaState, SceneHintState};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Default image dimensions for scene generation.
const DEFAULT_IMAGE_WIDTH: u32 = 1152;
const DEFAULT_IMAGE_HEIGHT: u32 = 768;

/// Workflow filenames (relative to app data dir's "workflows" folder).
/// Single character: uses IPAdapter FaceID without masks.
/// Multi character: uses IPAdapter FaceID with per-character attention masks.
const WORKFLOW_1CHAR: &str = "workflows/scene_workflow_1char.json";
const WORKFLOW_2CHAR: &str = "workflows/scene_workflow_2char.json";

// ============================================================================
// RESULT TYPES — returned to the frontend
// ============================================================================

/// A character as it appears in this scene, enriched with database info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInScene {
    pub name: String,
    pub region: String,
    pub view: String,
    pub pose: String,
    pub action: String,
    pub expression: String,
    pub clothing: String,
    pub facing: String,
    pub needs_render: bool,
    /// Database ID (if character was found in the DB).
    pub db_id: Option<i64>,
    /// Whether this character has a master reference image for IP-Adapter.
    pub has_reference_image: bool,
    pub prompt_only_description: Option<String>,
}

/// Serializable compression diagnostics owned by the orchestrator.
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
    pub turn_id: u32,
    pub story_text: String,
    pub summary_hint: String,
    pub scene: Option<SceneJson>,
    pub characters: Vec<CharacterInScene>,
    pub generated_image_path: Option<String>,
    pub parse_status: String,
    pub parse_warnings: Vec<String>,
    pub compression_info: OrchestratorCompressionInfo,
    pub image_generation_attempted: bool,
    pub image_generation_error: Option<String>,
    /// DB message_id of the assistant message saved for this turn.
    /// Used by the frontend to persist images generated after the turn completes.
    pub assistant_message_id: Option<i64>,
    /// Scene DB id that is active after this turn (auto-created or matched).
    /// The frontend uses this to refresh the ScenePanel.
    pub active_scene_id: Option<i64>,
    /// Full enriched positive SDXL prompt built for this turn.
    /// None if no renderable characters with reference images exist.
    pub enriched_prompt: Option<String>,
    /// Full negative SDXL prompt built for this turn.
    pub negative_prompt: Option<String>,
    /// Emotional states for each character at the end of this turn.
    pub emotional_states: Vec<CharacterEmotionalState>,
}

/// Preview of the enriched SDXL prompts for a scene, without generating an image.
/// Returned by `preview_scene_prompt` so the frontend can show and edit them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePromptPreview {
    pub positive: String,
    pub negative: String,
}

// ============================================================================
// HELPER: Extract JSON from text that may have surrounding prose
// ============================================================================

fn extract_json_from_text(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end >= start {
        Some(text[start..=end].to_string())
    } else {
        None
    }
}

// ============================================================================
// DATABASE HELPERS
// ============================================================================

/// Load conversation history as (user, assistant) pairs for context building.
/// from_message_pairs expects Vec<(user_input, assistant_response)>.
async fn load_conversation_history(
    db: &sqlx::SqlitePool,
    chat_id: i64,
) -> Result<Vec<(String, String)>, String> {
    let rows = sqlx::query(
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY rowid ASC",
    )
    .bind(chat_id)
    .fetch_all(db)
    .await
    .map_err(|e| format!("Failed to load messages: {}", e))?;

    println!("[DEBUG] Loaded {} raw messages from DB for chat_id={}", rows.len(), chat_id);

    // Pair up user + assistant messages into tuples
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut pending_user: Option<String> = None;

    for row in &rows {
        let role: String = row.get("role");
        let content: String = row.get("content");
        match role.as_str() {
            "user" => {
                pending_user = Some(content);
            }
            "assistant" => {
                if let Some(user_msg) = pending_user.take() {
                    pairs.push((user_msg, content));
                }
            }
            _ => {}
        }
    }

    println!("[DEBUG] Paired into {} turns for chat_id={}", pairs.len(), chat_id);
    Ok(pairs)
}

/// Load all characters for a story (or all characters globally if no story_id).
async fn load_characters_for_context(
    db: &sqlx::SqlitePool,
    story_id: Option<i64>,
) -> Result<Vec<CharacterInfo>, String> {
    let rows = if let Some(sid) = story_id {
        sqlx::query(
            "SELECT c.name, c.age, c.gender, c.personality, c.sd_prompt, c.default_clothing \
             FROM characters c \
             INNER JOIN story_characters sc ON sc.character_id = c.id \
             WHERE sc.story_id = ? ORDER BY c.name",
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

/// Load only the characters pinned to the active scene.
/// Returns an empty vec if scene_id is None or has no characters.
async fn load_scene_characters_for_context(
    db: &sqlx::SqlitePool,
    scene_id: i64,
) -> Result<Vec<CharacterInfo>, String> {
    let rows = sqlx::query(
        "SELECT c.name, c.age, c.gender, c.personality, c.sd_prompt, c.default_clothing \
         FROM characters c \
         INNER JOIN scene_characters sc ON sc.character_id = c.id \
         WHERE sc.scene_id = ? ORDER BY c.name",
    )
    .bind(scene_id)
    .fetch_all(db)
    .await
    .map_err(|e| format!("Failed to load scene characters: {}", e))?;

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

/// Sync the active scene from the LLM's scene_json output.
/// - Matches an existing scene by location (case-insensitive).
/// - Auto-creates a new scene if no match is found.
/// - Syncs scene_characters from the character names in the LLM output.
/// - Sets story_premises.active_scene_id.
///
/// Best-effort: returns Ok(None) if story_id is None or location is empty.
/// Never propagates errors upward — log and continue.
async fn sync_scene_from_turn(
    db: &sqlx::SqlitePool,
    story_id: i64,
    scene_json: &SceneJson,
    character_names: &[String],
) -> Result<Option<i64>, String> {
    let location = scene_json.location.trim().to_string();
    if location.is_empty() {
        return Ok(None);
    }

    // 1. Try to match an existing scene by location (case-insensitive)
    let existing = sqlx::query(
        "SELECT s.id FROM scenes s \
         INNER JOIN story_scenes ss ON ss.scene_id = s.id \
         WHERE ss.story_id = ? AND LOWER(s.location) = LOWER(?)",
    )
    .bind(story_id)
    .bind(&location)
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Scene lookup failed: {}", e))?;

    let scene_id: i64 = if let Some(row) = existing {
        let id: i64 = row.get("id");
        println!("[Scene] Matched existing scene id={} for location='{}'", id, location);
        id
    } else {
        // 2. Auto-create a new scene using LLM data
        let result = sqlx::query(
            "INSERT INTO scenes (name, location, location_type, time_of_day, mood) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&location)           // name = location text
        .bind(&location)           // location field
        .bind(&scene_json.location_type)
        .bind(&scene_json.time_of_day)
        .bind(&scene_json.mood)
        .execute(db)
        .await
        .map_err(|e| format!("Failed to create scene: {}", e))?;

        let new_id = result.last_insert_rowid();
        println!("[Scene] Auto-created scene id={} for location='{}'", new_id, location);

        // Link to story
        sqlx::query("INSERT OR IGNORE INTO story_scenes (story_id, scene_id) VALUES (?, ?)")
            .bind(story_id)
            .bind(new_id)
            .execute(db)
            .await
            .map_err(|e| format!("Failed to link scene to story: {}", e))?;

        new_id
    };

    // 3. Set as active scene for this story
    sqlx::query("UPDATE story_premises SET active_scene_id = ? WHERE id = ?")
        .bind(scene_id)
        .bind(story_id)
        .execute(db)
        .await
        .map_err(|e| format!("Failed to set active scene: {}", e))?;

    // 4. Sync scene_characters from LLM output (clear + re-populate)
    sqlx::query("DELETE FROM scene_characters WHERE scene_id = ?")
        .bind(scene_id)
        .execute(db)
        .await
        .ok(); // best-effort

    for name in character_names {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Resolve name to character id (story-scoped first, then global fallback)
        let char_row = sqlx::query(
            "SELECT c.id FROM characters c \
             INNER JOIN story_characters sc ON sc.character_id = c.id \
             WHERE c.name = ? AND sc.story_id = ? LIMIT 1",
        )
        .bind(trimmed)
        .bind(story_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();

        if let Some(row) = char_row {
            let char_id: i64 = row.get("id");
            sqlx::query(
                "INSERT OR IGNORE INTO scene_characters (scene_id, character_id) VALUES (?, ?)",
            )
            .bind(scene_id)
            .bind(char_id)
            .execute(db)
            .await
            .ok();
        }
    }

    Ok(Some(scene_id))
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

        println!("[Orchestrator] Looking up character '{}' (story_id={:?})", scene_char.name, story_id);

        // Try story-scoped lookup first (via junction table)
        let row = if let Some(sid) = story_id {
            let r = sqlx::query(
                "SELECT c.id, c.name, c.master_image_path, c.sd_prompt, c.default_clothing, c.art_style, c.gender \
                 FROM characters c \
                 INNER JOIN story_characters sc ON sc.character_id = c.id \
                 WHERE c.name = ? AND sc.story_id = ? LIMIT 1",
            )
            .bind(&scene_char.name)
            .bind(sid)
            .fetch_optional(db)
            .await
            .map_err(|e| format!("Character lookup failed for '{}': {}", scene_char.name, e))?;

            if r.is_none() {
                // No global fallback — if the character isn't in this story, skip it
                // to avoid pulling in a same-named character from a different story
                println!("[Orchestrator] '{}' not found in story {} — skipping (no global fallback)", scene_char.name, sid);
                None
            } else {
                r
            }
        } else {
            sqlx::query(
                "SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style, gender \
                 FROM characters WHERE name = ? LIMIT 1",
            )
            .bind(&scene_char.name)
            .fetch_optional(db)
            .await
            .map_err(|e| format!("Character lookup failed for '{}': {}", scene_char.name, e))?
        };

        let lookup = row.map(|r| {
            let master: Option<String> = r.get("master_image_path");
            println!("[Orchestrator] Found '{}' — master_image_path={:?}", scene_char.name, master);
            CharacterLookup {
                id: r.get("id"),
                name: r.get("name"),
                master_image_path: master,
                sd_prompt: r.get("sd_prompt"),
                default_clothing: r.get("default_clothing"),
                art_style: r.get("art_style"),
                gender: r.get("gender"),
            }
        });

        if lookup.is_none() {
            println!("[Orchestrator] '{}' NOT FOUND in database at all", scene_char.name);
        }

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
                pose: raw.pose.clone(),
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
                prompt_only_description: None,
            }
        })
        .collect()
}

/// Tell Ollama to unload the current model from VRAM immediately.
/// This frees GPU memory for ComfyUI image generation.
/// The model will be automatically reloaded on the next /api/generate call.
pub(crate) async fn unload_ollama_model(ollama_url: &str) {
    println!(
        "[VRAM] Requesting Ollama to unload model '{}' from VRAM...",
        STORY_MODEL
    );
    let client = reqwest::Client::new();
    let url = format!("{}/api/generate", ollama_url);

    let result = client
        .post(&url)
        .json(&serde_json::json!({
            "model": STORY_MODEL,
            "keep_alive": 0
        }))
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            println!("[VRAM] Ollama model unloaded successfully — VRAM freed for ComfyUI");
        }
        Ok(resp) => {
            println!(
                "[VRAM] Ollama unload returned status {}, continuing anyway",
                resp.status()
            );
        }
        Err(e) => {
            // Non-fatal — if Ollama is already dead or unresponsive, VRAM is free anyway
            println!(
                "[VRAM] Could not reach Ollama to unload model ({}), continuing anyway",
                e
            );
        }
    }

    // Give the GPU a moment to actually release the memory
    tokio::time::sleep(Duration::from_millis(500)).await;
}

/// Tell ComfyUI to unload all models from VRAM.
/// Uses the /free endpoint to release GPU memory after image generation.
/// Models will be automatically reloaded on the next generation request.
pub(crate) async fn unload_comfyui_models(comfyui_url: &str) {
    println!("[VRAM] Requesting ComfyUI to free GPU memory...");
    let client = reqwest::Client::new();
    let url = format!("{}/free", comfyui_url);

    let result = client
        .post(&url)
        .json(&serde_json::json!({
            "unload_models": true
        }))
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            println!("[VRAM] ComfyUI models unloaded — VRAM freed");
        }
        Ok(resp) => {
            println!(
                "[VRAM] ComfyUI free returned status {}, continuing",
                resp.status()
            );
        }
        Err(e) => {
            println!(
                "[VRAM] Could not reach ComfyUI to free models ({}), continuing",
                e
            );
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
}

/// Free all GPU memory by unloading both Ollama and ComfyUI models.
/// Called when switching stories or from a manual "Free VRAM" button.
#[tauri::command]
pub async fn free_vram(config_state: State<'_, ConfigState>) -> Result<(), String> {
    let ollama_url = {
        let config = config_state.0.lock().map_err(|e| e.to_string())?;
        config.ollama_url.clone()
    };

    unload_ollama_model(&ollama_url).await;
    unload_comfyui_models("http://localhost:8188").await;

    println!("[VRAM] All models unloaded — GPU memory freed");
    Ok(())
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
        let view_tag = match char_in_scene.view.to_uppercase().as_str() {
            "PORTRAIT" => "upper body, close-up".to_string(),
            "UPPER-BODY" | "UPPER_BODY" => "medium shot, from waist up, showing torso and head".to_string(),
            "FULL-BODY" | "FULL_BODY" => "full body, wide shot".to_string(),
            _ => "medium shot, from waist up".to_string(),
        };
        parts.push(view_tag);
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

/// Select the correct workflow file based on how many renderable characters
/// are in the scene.
///
/// - 1 character  → scene_workflow_1char.json (no masks, simpler/faster)
/// - 2+ characters → scene_workflow_2char.json (with per-character masks)
///
/// Both files must be placed in the app data "workflows" folder.
fn select_workflow(num_chars: usize, app_data: &std::path::Path) -> Result<String, String> {
    let filename = match num_chars {
        1 => WORKFLOW_1CHAR,
        _ => WORKFLOW_2CHAR,
    };

    let path = app_data.join(filename);

    if path.exists() {
        println!(
            "[Orchestrator] Selected {} workflow for {} character(s): {}",
            if num_chars == 1 { "single-character" } else { "multi-character" },
            num_chars,
            path.display()
        );
        Ok(path.to_string_lossy().to_string())
    } else {
        Err(format!(
            "Workflow file not found: {}. \
             Please copy your workflow JSON files to: {}\\workflows\\",
            path.display(),
            app_data.display()
        ))
    }
}

/// Run the image generation sub-pipeline:
///   1. Filter to renderable characters with reference images
///   2. Select the right workflow (1-char vs 2-char)
///   3. Generate a color mask (skipped for single character)
///   4. Build the ComfyUI request
///   5. Call generate_scene_image
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

    let num_chars = renderable.len();

    // 2. Select workflow based on character count
    let workflow_template = match select_workflow(num_chars, &app_data) {
        Ok(p) => p,
        Err(e) => return (None, Some(e)),
    };

    // 3. Generate per-character masks (only needed for multi-character scenes)
    let mask_paths: Vec<String> = if num_chars > 1 {
        let masks_dir = app_data.join("masks");
        let mut paths = Vec::new();

        for (i, (cis, _)) in renderable.iter().enumerate().take(2) {
            let mask_chars = vec![MaskCharacter {
                name: cis.name.clone(),
                region: cis.region.clone(),
                color_index: 0, // Always 0 — each mask has one char, red channel
            }];

            let filename = format!("scene_mask_char{}.png", i);
            match mask_generator::generate_mask(
                &mask_chars,
                DEFAULT_IMAGE_WIDTH,
                DEFAULT_IMAGE_HEIGHT,
                &masks_dir,
                Some(&filename),
            ) {
                Ok(r) => {
                    println!("[Orchestrator] Mask for '{}': {} (region={})", cis.name, r.path, cis.region);
                    paths.push(r.path);
                }
                Err(e) => {
                    return (None, Some(format!("Mask gen failed for '{}': {}", cis.name, e)));
                }
            }
        }
        paths
    } else {
        vec![] // 1-char doesn't need masks
    };

    // 4. Build ComfyUI character inputs
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

    // 5. Build the scene prompt, including character descriptions so the
    //    model actually generates people in the image.
    //    IPAdapter applies the face, but the base prompt must mention a person
    //    or the model will only generate the environment.
    let mut scene_prompt = parsed.scene_prompt_fragment();

    // Append ipadapter character descriptions
    for (cis, (_, db_char)) in &renderable {
        let fragment = character_prompt_fragment(cis, db_char.as_ref());
        if !fragment.is_empty() {
            let region_prefix = match cis.region.to_lowercase().as_str() {
                "left" => "person on the left, ",
                "right" => "person on the right, ",
                _ => "person, ",
            };
            scene_prompt = format!("{}, {}{}", scene_prompt, region_prefix, fragment);
        } else {
            // No fragment data — at minimum add "a person" so IPAdapter has something to work with
            scene_prompt = format!("{}, a person", scene_prompt);
        }
    }

    // Append prompt-only character descriptions (no reference image)
    for cis in characters_in_scene.iter().filter(|c| c.needs_render && !c.has_reference_image) {
        if let Some(ref desc) = cis.prompt_only_description {
            let region_prefix = match cis.region.to_lowercase().as_str() {
                "left" => "on the left, ",
                "right" => "on the right, ",
                _ => "",
            };
            scene_prompt = format!("{}, {}{}", scene_prompt, region_prefix, desc);
        }
    }

    println!("[Orchestrator] Final scene prompt: {}...", &scene_prompt[..scene_prompt.len().min(150)]);

    // 6. Build the full image generation request
    let request = ImageGenRequest {
        scene_prompt,
        characters: comfy_characters,
        mask_paths,
        workflow_template,
        comfyui_url: None,
        seed: Some(rand::random::<i64>().abs()),
        steps: None,
        cfg: None,
        width: Some(DEFAULT_IMAGE_WIDTH),
        height: Some(DEFAULT_IMAGE_HEIGHT),
        negative_prompt: Some(
            "(worst quality, low quality:1.4), (bad anatomy:1.3), (bad hands:1.4), \
             close-up, closeup, head shot, headshot, cropped, zoomed in, \
             cowboy hat, cowboy, western clothing".to_string()
        ),
        timeout_secs: None,
        controlnet_image_path: None,
        controlnet_strength: None,
    };

    // 7. Free VRAM: unload Ollama before ComfyUI needs the GPU
    unload_ollama_model("http://localhost:11434").await;

    // 8. Run the ComfyUI pipeline
    let output_dir = app_data.join("scene_images");
    match comfyui_api::generate_scene_image(&request, &output_dir).await {
        Ok(result) => {
            let primary_image = result.image_paths.first().cloned();
            println!(
                "[Orchestrator] Image generated: {:?} ({} images total)",
                primary_image,
                result.image_paths.len()
            );
            unload_comfyui_models("http://localhost:8188").await;
            println!("[VRAM] Image generation complete — both models unloaded, VRAM clean");
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
    config_state: State<'_, ConfigState>,
    hint_state: State<'_, SceneHintState>,
    app: AppHandle,
) -> Result<StoryTurnResult, String> {
    let start_time = std::time::Instant::now();
    println!(
        "\n[Orchestrator] ========== TURN START (chat={}, story={:?}) ==========",
        chat_id, story_id
    );
    println!("[Orchestrator] User input: {:?}", &user_input[..user_input.len().min(80)]);

    // Consume any pending scene hint for this story (one-shot — removed after read)
    let scene_hint: Option<String> = if let Some(sid) = story_id {
        let mut hints = hint_state.0.lock().map_err(|e| e.to_string())?;
        hints.remove(&sid)
    } else {
        None
    };

    if let Some(ref hint) = scene_hint {
        println!("[SceneHint] Injecting scene hint for story_id={:?}: {}...", story_id, &hint[..hint.len().min(80)]);
    }

    // Build effective user input — prepend scene hint if present
    let effective_user_input = if let Some(ref hint) = scene_hint {
        format!("{}\n\nPlayer action: {}", hint, user_input)
    } else {
        user_input.clone()
    };

    // ── Step 1: Build compressed context ─────────────────────────────

    let context_start = std::time::Instant::now();

    // Read config settings and build the effective system prompt
    let (content_rating, response_length) = {
        let config = config_state.0.lock().map_err(|e| e.to_string())?;
        (config.content_rating.clone(), config.response_length.clone())
    };

    let length_config = get_response_length_config(&response_length);
    let max_prompt_tokens = NUM_CTX as usize - length_config.num_predict as usize;

    let content_instruction = if content_rating == "sfw" {
        "\n\nCONTENT POLICY: This is a SFW (Safe For Work) story. Keep all content \
         family-friendly. No explicit sexual content, graphic violence, or mature themes. \
         Characters should remain clothed and scenes should be appropriate for all audiences."
    } else {
        "\n\nCONTENT POLICY: The user has enabled unrestricted content. There are no content \
         restrictions on story themes, descriptions, or scenarios. Write naturally without filtering."
    };
    let full_system_prompt = format!(
        "{}{}",
        SYSTEM_PROMPT.replace("{{PARAGRAPH_RULE}}", length_config.paragraph_instruction),
        content_instruction
    );

    let message_rows = load_conversation_history(&state.db, chat_id).await?;
    let story_premise = load_story_premise(&state.db).await?;

    // Load current active scene (if any) to filter characters and build scene context
    let prior_active_scene_id: Option<i64> = if let Some(sid) = story_id {
        sqlx::query("SELECT active_scene_id FROM story_premises WHERE id = ?")
            .bind(sid)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .and_then(|row| row.get("active_scene_id"))
    } else {
        None
    };

    // All story characters (full roster, always loaded)
    let all_characters = load_characters_for_context(&state.db, story_id).await?;

    // Characters in the active scene (for focused LLM context); falls back to all chars
    let scene_characters = if let Some(scene_id) = prior_active_scene_id {
        let sc = load_scene_characters_for_context(&state.db, scene_id).await?;
        if sc.is_empty() { all_characters.clone() } else { sc }
    } else {
        all_characters.clone()
    };

    // Build scene context string from the active scene record
    let scene_context: Option<String> = if let Some(scene_id) = prior_active_scene_id {
        sqlx::query(
            "SELECT name, location, location_type, time_of_day, mood FROM scenes WHERE id = ?",
        )
        .bind(scene_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .map(|row| {
            let name: String = row.get("name");
            let location: Option<String> = row.get("location");
            let location_type: Option<String> = row.get("location_type");
            let time: Option<String> = row.get("time_of_day");
            let mood: Option<String> = row.get("mood");
            format!(
                "CURRENT SCENE: {} | Location: {} ({}) | Time: {} | Mood: {}",
                name,
                location.as_deref().unwrap_or("unspecified"),
                location_type.as_deref().unwrap_or("interior"),
                time.as_deref().unwrap_or("unspecified"),
                mood.as_deref().unwrap_or("unspecified"),
            )
        })
    } else {
        None
    };

    let mut conversation = ConversationContext::from_message_pairs(&message_rows);

    let assembled = build_compressed_context(
        &mut conversation,
        &full_system_prompt,
        &scene_characters,
        &all_characters,
        story_premise.as_deref(),
        scene_context.as_deref(),
        &effective_user_input,
        max_prompt_tokens,
    );

    let system_prompt_tokens = estimate_tokens(&full_system_prompt);
    let char_token_estimate = estimate_character_db_tokens(&scene_characters);
    let diag = get_diagnostics(&conversation, system_prompt_tokens, char_token_estimate);

    let compression_info = OrchestratorCompressionInfo {
        total_turns: diag.total_turns,
        compressed_turns: diag.compressed_turns,
        recent_turns: diag.recent_turns,
        estimated_total_tokens: diag.estimated_total_tokens,
        max_context_tokens: diag.max_context_tokens,
        compression_threshold: diag.compression_threshold,
        needs_compression: diag.needs_compression,
        compressed_summary_preview: diag.compressed_summary_preview.clone(),
    };

    println!(
        "[Orchestrator] Context built in {:.1}s ({} turns, ~{} tokens, {} chars in DB, compressed={})",
        context_start.elapsed().as_secs_f64(),
        diag.total_turns,
        diag.estimated_total_tokens,
        scene_characters.len(),
        assembled.was_compressed,
    );

    // ── Step 2: Call Ollama ───────────────────────────────────────────

    let ollama_start = std::time::Instant::now();

    println!(
        "[DEBUG] Ollama request — turns in context: {} ({} recent + {} compressed), prompt tokens: ~{}, budget: {}, num_ctx: {}, num_predict: {} (length={})",
        diag.total_turns,
        diag.recent_turns,
        diag.compressed_turns,
        estimate_tokens(&assembled.prompt),
        max_prompt_tokens,
        NUM_CTX,
        length_config.num_predict,
        response_length,
    );

    let request_body = json!({
        "model": STORY_MODEL,
        "prompt": assembled.prompt,
        "raw": true,
        "stream": false,
        "think": false,
        "options": {
            "num_ctx": NUM_CTX,
            "num_predict": length_config.num_predict,
            "temperature": 0.8
        }
    });

    let api_res: Value = {
        let mut last_err = String::new();
        let mut success = None;
        for attempt in 1..=OLLAMA_MAX_RETRIES {
            let result = state
                .client
                .post(format!("{}/api/generate", state.base_url))
                .timeout(Duration::from_secs(OLLAMA_REQUEST_TIMEOUT_SECS))
                .json(&request_body)
                .send()
                .await;
            match result {
                Ok(res) => {
                    match res.json::<Value>().await {
                        Ok(json) => {
                            success = Some(json);
                            break;
                        }
                        Err(e) => {
                            last_err = format!("Failed to parse Ollama response: {}", e);
                            println!("[ERROR] Attempt {}/{} — parse error: {}", attempt, OLLAMA_MAX_RETRIES, last_err);
                        }
                    }
                }
                Err(e) => {
                    last_err = format!("Ollama request failed: {}", e);
                    println!("[ERROR] Attempt {}/{} — {}", attempt, OLLAMA_MAX_RETRIES, last_err);
                }
            }
            if attempt < OLLAMA_MAX_RETRIES {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
        success.ok_or(last_err)?
    };

    let response_text_raw = api_res["response"]
        .as_str()
        .ok_or_else(|| "No 'response' field in Ollama API result".to_string())?;

    // Strip <think>...</think> blocks the model may emit before the JSON
    let response_text = {
        let re = regex::Regex::new(r"(?s)<think>.*?</think>").unwrap();
        let cleaned = re.replace_all(response_text_raw, "");
        cleaned.trim().to_string()
    };

    if response_text_raw.contains("<think>") {
        println!(
            "[DEBUG] Stripped <think> tags from response ({} chars before, {} after)",
            response_text_raw.len(),
            response_text.len()
        );
    }

    println!("[Orchestrator][DEBUG] Raw LLM response JSON:\n{}", &response_text);

    {
        let trimmed = response_text.trim_end();
        let last_char = trimmed.chars().last().unwrap_or('\0');
        let appears_truncated = !matches!(last_char, '.' | '!' | '?' | '"' | '\'' | '}');
        println!(
            "[DEBUG] Ollama response — {:.1}s, {} chars, last_char={:?}, truncated={}",
            ollama_start.elapsed().as_secs_f64(),
            response_text.len(),
            last_char,
            appears_truncated,
        );
    }

    println!(
        "[Orchestrator] Ollama responded in {:.1}s ({} chars)",
        ollama_start.elapsed().as_secs_f64(),
        response_text.len()
    );

    // ── Step 3: Parse the LLM output ──────────────────────────────────

    let parsed = llm_parser::parse_llm_output(&response_text);

    println!(
        "[Orchestrator][DEBUG] Parsed turn JSON:\n{}",
        serde_json::to_string_pretty(&parsed.turn).unwrap_or_else(|_| "SERIALIZATION_FAILED".to_string())
    );

    let (parse_status, parse_warnings) = match &parsed.status {
        ParseStatus::Ok => ("ok".to_string(), vec![]),
        ParseStatus::Partial(w) => {
            for warn in w {
                println!("[Orchestrator] Parse warning: {}", warn);
            }
            ("partial".to_string(), w.clone())
        }
        ParseStatus::Fallback => {
            let preview: String = response_text.chars().take(500).collect();
            println!("[DEBUG] Fallback response preview:\n{}", preview);
            (
                "fallback".to_string(),
                vec!["LLM output was not valid JSON; raw text was preserved".to_string()],
            )
        }
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

    // ── Step 4.5: Build enriched prompt preview for the frontend ─────────────
    // Done here so the frontend has the full SDXL prompt immediately after a turn,
    // without needing a separate `preview_scene_prompt` call.

    let (enriched_prompt_preview, negative_prompt_preview): (Option<String>, Option<String>) = {
        // Filter to renderable characters that have reference images
        let renderable_with_refs: Vec<(&CharacterInScene, &crate::models::CharacterLookup)> =
            characters_in_scene
                .iter()
                .zip(lookup_results.iter())
                .filter(|(cis, _)| cis.needs_render && cis.has_reference_image)
                .filter_map(|(cis, (_, db_opt))| db_opt.as_ref().map(|db| (cis, db)))
                .collect();

        if renderable_with_refs.is_empty() {
            (None, None)
        } else {
            let num_chars = renderable_with_refs.len().min(2);
            let regions: Vec<&str> = if num_chars == 1 { vec!["center"] } else { vec!["left", "right"] };

            let genders: Vec<&'static str> = renderable_with_refs.iter()
                .take(num_chars)
                .map(|(_, db)| infer_gender(db))
                .collect();

            let subject_count_tag = match num_chars {
                1 => match genders[0] { "female" => "1girl", "male" => "1boy", _ => "1person" },
                2 => match (genders[0], genders[1]) {
                    ("female", "female") => "2girls",
                    ("male",   "male")   => "2boys",
                    ("female", "male") | ("male", "female") => "1girl 1boy",
                    _ => "2people",
                },
                _ => "people",
            };

            let mut char_segs: Vec<String> = Vec::new();
            for (i, (_, db)) in renderable_with_refs.iter().enumerate().take(num_chars) {
                let region = regions.get(i).copied().unwrap_or("center");
                let mut parts: Vec<String> = Vec::new();
                if let Some(ref sd) = db.sd_prompt {
                    let clean: String = sd
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| {
                            let l = s.to_lowercase();
                            !l.contains("solo") && !l.contains("portrait")
                                && !l.contains("looking at viewer")
                                && !l.contains("looking at camera")
                                && !l.contains("neutral") && !l.contains("background")
                                && !l.contains("masterpiece") && !l.contains("best quality")
                                && !l.contains("detailed face") && !l.contains("detailed eyes")
                                && !l.contains("1girl") && !l.contains("1boy")
                                && !l.contains("1woman") && !l.contains("1man")
                                && !l.contains("upper body") && !l.contains("close up")
                                && !l.contains("closeup") && !l.contains("headshot")
                        })
                        .collect::<Vec<&str>>()
                        .join(", ");
                    if !clean.is_empty() { parts.push(clean); }
                }
                if let Some(ref cloth) = db.default_clothing {
                    if !cloth.is_empty() { parts.push(cloth.clone()); }
                }
                if infer_gender(db) == "female" {
                    parts.push("soft feminine features, smooth jawline, delicate face, no cleft chin".to_string());
                }
                let desc = if parts.is_empty() { "a person".to_string() } else { parts.join(", ") };
                let prefix = match region {
                    "left"  => "a person on the left side of the scene,",
                    "right" => "a person on the right side of the scene,",
                    _       => "a person in the center of the scene,",
                };
                char_segs.push(format!("{} {}", prefix, desc));
            }

            let sp = parsed.scene_prompt_fragment();
            let ep = format!(
                "(masterpiece, best quality, highly detailed, cinematic composition), \
                 (medium shot, waist up, head and torso visible:1.2), {}, {}, {}, \
                 (detailed face, clear face:1.1)",
                subject_count_tag, sp, char_segs.join(", ")
            );

            let sfw_neg = if content_rating == "sfw" {
                ", nsfw, nude, naked, nudity, bare chest, cleavage, lingerie, underwear, \
                 suggestive, seductive, sexual, explicit, provocative, revealing clothing, \
                 bikini, swimsuit, exposed skin, nipples, breasts"
            } else {
                ""
            };
            let neg = format!(
                "(cropped head:1.5), (head out of frame:1.5), (cut off head:1.5), (headless:1.5), \
                 decapitated, (worst quality, low quality:1.4), (bad anatomy:1.3), (bad hands:1.4), \
                 close-up, closeup, head shot, headshot, cropped, zoomed in, \
                 cowboy hat, cowboy, western clothing{}",
                sfw_neg
            );

            println!(
                "[Orchestrator] Built enriched prompt preview ({} chars, {} chars neg)",
                ep.len(), neg.len()
            );
            (Some(ep), Some(neg))
        }
    };

    // ── Step 5: Conditional image generation ──────────────────────────

    let flags = parsed.flags();
    let generated_image_path: Option<String> = None;
    let image_generation_attempted = false;
    let image_generation_error: Option<String> = None;

    // Image generation is user-initiated via the "Generate Image" button,
    // which calls generate_scene_image_for_turn directly.
    println!(
        "[Orchestrator] Image generation available (generate_image={}, {} renderable chars)",
        flags.generate_image,
        characters_in_scene.iter().filter(|c| c.needs_render && c.has_reference_image).count()
    );

    // ── Step 5.5: Sync scene state from LLM output ────────────────────
    // Best-effort: if story_id is set and the LLM returned scene data,
    // auto-create/match a scene and sync its characters.

    let post_turn_active_scene_id: Option<i64> = if let (Some(sid), Some(scene_json)) =
        (story_id, &parsed.turn.scene_json)
    {
        let char_names: Vec<String> = parsed
            .turn
            .characters_in_scene
            .iter()
            .map(|c| c.name.clone())
            .collect();

        match sync_scene_from_turn(&state.db, sid, scene_json, &char_names).await {
            Ok(id) => id,
            Err(e) => {
                println!("[Scene] sync_scene_from_turn error (non-fatal): {}", e);
                prior_active_scene_id
            }
        }
    } else {
        prior_active_scene_id
    };

    // ── Step 6: Save to database ──────────────────────────────────────

    let raw_content =
        extract_json_from_text(&response_text).unwrap_or_else(|| response_text.to_string());

    let assistant_message_id = match save_turn_to_db(
        &state.db,
        chat_id,
        &user_input,
        &raw_content,
        generated_image_path.as_deref(),
    )
    .await
    {
        Ok(id) => {
            println!("[DEBUG] Saved turn to DB: assistant_message_id={}", id);
            Some(id)
        }
        Err(e) => {
            println!("[ERROR] save_turn_to_db FAILED for chat_id={}: {}", chat_id, e);
            None
        }
    };

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
        assistant_message_id,
        active_scene_id: post_turn_active_scene_id,
        enriched_prompt: enriched_prompt_preview,
        negative_prompt: negative_prompt_preview,
        emotional_states: parsed.emotional_states().to_vec(),
    })
}

#[tauri::command]
pub async fn generate_scene_image_for_turn(
    scene_prompt: String,
    story_id: Option<i64>,
    character_names: Option<Vec<String>>,
    character_poses: Option<Vec<String>>,
    positive_prompt_override: Option<String>,
    negative_prompt_override: Option<String>,
    app: AppHandle,
    state: State<'_, OllamaState>,
    config_state: State<'_, ConfigState>,
) -> Result<String, String> {
    println!(
        "[Orchestrator] generate_scene_image_for_turn called: story_id={:?}, prompt_len={}",
        story_id, scene_prompt.len()
    );
    println!(
        "[Orchestrator] Scene prompt (first 300 chars): {}",
        &scene_prompt[..scene_prompt.len().min(300)]
    );
    println!(
        "[Orchestrator] character_names received: {:?}",
        character_names
    );

    let app_data = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    println!("[Orchestrator] Querying characters with reference images for story_id={:?}", story_id);
    let all_characters = get_characters_with_references(story_id, &state).await?;

    // Load the active scene for this story (used for character filtering + prompt enhancement)
    let active_scene_data: Option<(i64, String, Option<String>, Option<String>, Option<String>)> =
        if let Some(sid) = story_id {
            sqlx::query(
                "SELECT s.id, s.location, s.time_of_day, s.mood, s.name \
                 FROM scenes s \
                 INNER JOIN story_premises sp ON sp.active_scene_id = s.id \
                 WHERE sp.id = ?",
            )
            .bind(sid)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .map(|r| {
                let scene_id: i64 = r.get("id");
                let location: Option<String> = r.get("location");
                let time: Option<String> = r.get("time_of_day");
                let mood: Option<String> = r.get("mood");
                let name: String = r.get("name");
                (scene_id, name, location, time, mood)
            })
        } else {
            None
        };

    // If there's an active scene, load character IDs pinned to it for filtering
    let scene_char_ids: Option<Vec<i64>> = if let Some((scene_id, ..)) = &active_scene_data {
        let rows = sqlx::query(
            "SELECT character_id FROM scene_characters WHERE scene_id = ?",
        )
        .bind(scene_id)
        .fetch_all(&state.db)
        .await
        .ok();
        rows.map(|rs| rs.iter().map(|r| r.get::<i64, _>("character_id")).collect())
    } else {
        None
    };

    // If specific scene character names were provided, filter to only those.
    let characters: Vec<CharacterLookup> = if let Some(ref names) = character_names {
        let names_lower: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
        let filtered: Vec<CharacterLookup> = all_characters
            .iter()
            .filter(|c| names_lower.contains(&c.name.to_lowercase()))
            .cloned()
            .collect();
        if filtered.is_empty() {
            // Names provided but none matched (e.g., character was renamed). Fall back to all.
            println!(
                "[Orchestrator] No DB matches for scene names {:?} — falling back to all {} character(s)",
                names, all_characters.len()
            );
            all_characters
        } else {
            println!(
                "[Orchestrator] Filtered to {} scene character(s) from names: {:?}",
                filtered.len(), names
            );
            filtered
        }
    } else {
        all_characters
    };

    // If the active scene has pinned characters and no explicit names were requested,
    // filter to only scene members (prevents hallucinated extras from being rendered).
    let characters: Vec<CharacterLookup> = if character_names.is_none() {
        if let Some(ref ids) = scene_char_ids {
            if !ids.is_empty() {
                let filtered: Vec<CharacterLookup> = characters
                    .iter()
                    .filter(|c| ids.contains(&c.id))
                    .cloned()
                    .collect();
                println!(
                    "[Orchestrator] Scene filter: {} character(s) in active scene (by ID)",
                    filtered.len()
                );
                // If no scene chars have reference images, fall back to unfiltered list
                if filtered.is_empty() { characters } else { filtered }
            } else {
                characters
            }
        } else {
            characters
        }
    } else {
        characters
    };

    println!("[Orchestrator] Characters with reference images found: {}", characters.len());
    for c in &characters {
        println!(
            "[Orchestrator]   - '{}' (id={}, has_master_image={}, has_sd_prompt={})",
            c.name, c.id, c.master_image_path.is_some(), c.sd_prompt.is_some()
        );
    }

    if characters.is_empty() {
        return Err(
            "No characters with reference images found. \
             To generate scene images, open a character in the Character panel, \
             generate a Master Portrait, then save it as the reference image.".to_string()
        );
    }

    let num_chars = characters.len().min(2); // Max 2 for current workflows
    let workflow_path = select_workflow(num_chars, &app_data)?;

    // Assign regions up-front — used by both mask generation and char_inputs
    let regions: Vec<String> = match num_chars {
        1 => vec!["center".to_string()],
        _ => vec!["left".to_string(), "right".to_string()],
    };

    // ═══════════════════════════════════════════════════════════════════
    // CRITICAL: Enrich the scene prompt with character descriptions.
    // IP-Adapter FaceID only applies a face to a person that already
    // exists in the image. If the prompt doesn't describe a person,
    // the model won't generate one, and the face reference is wasted.
    // ═══════════════════════════════════════════════════════════════════

    // Determine Danbooru-style subject count tags from character genders.
    // These help SDXL place exactly the right number of people.
    let genders: Vec<&'static str> = characters.iter()
        .take(num_chars)
        .map(infer_gender)
        .collect();

    let subject_count_tag = match num_chars {
        1 => match genders[0] {
            "female" => "1girl",
            "male"   => "1boy",
            _        => "1person",
        },
        2 => match (genders[0], genders[1]) {
            ("female", "female")                   => "2girls",
            ("male",   "male")                     => "2boys",
            ("female", "male") | ("male", "female") => "1girl 1boy",
            ("female", _) | (_, "female")          => "1girl 1person",
            ("male",   _) | (_, "male")            => "1boy 1person",
            _                                      => "2people",
        },
        _ => "people",
    };

    // Read config values needed for ControlNet and content rating.
    let (content_rating, controlnet_enabled, controlnet_strength) = {
        let config = config_state.0.lock().map_err(|e| e.to_string())?;
        (config.content_rating.clone(), config.controlnet_pose_enabled, config.controlnet_pose_strength)
    };

    // Extract pose emphasis from scene prompt keywords.
    let pose_emphasis = extract_pose_emphasis(&scene_prompt);
    println!(
        "[Orchestrator][DEBUG] Pose emphasis from scene prompt: '{}'",
        &pose_emphasis
    );

    // Resolve ControlNet pose skeleton — prefer explicit LLM pose, fall back to keyword detection
    let skeletons_dir = app_data.join("pose_skeletons");
    let controlnet_image_path: Option<String> = if controlnet_enabled && skeletons_dir.exists() {
        let llm_pose = character_poses
            .as_ref()
            .and_then(|poses| poses.first())
            .filter(|p| !p.is_empty())
            .map(|p| p.to_uppercase().replace('-', "_"));
        let pose_source = if llm_pose.is_some() { "LLM" } else { "keywords" };
        let detected_pose = llm_pose
            .unwrap_or_else(|| detect_pose_name_from_prompt(&scene_prompt));

        println!(
            "[Orchestrator][DEBUG] Detected pose: {} (from {})",
            detected_pose, pose_source
        );

        let skeleton_path = crate::image_gen::pose_skeletons::get_skeleton_path_for_pose(
            &detected_pose, &skeletons_dir,
        );
        if skeleton_path.exists() {
            println!(
                "[Orchestrator][DEBUG] ControlNet skeleton: pose={}, path={}",
                detected_pose, skeleton_path.display()
            );
            Some(skeleton_path.to_string_lossy().to_string())
        } else {
            println!("[Orchestrator][DEBUG] No skeleton found for pose={}", detected_pose);
            None
        }
    } else {
        if !controlnet_enabled {
            println!("[Orchestrator][DEBUG] ControlNet disabled in settings, skipping skeleton");
        } else {
            println!("[Orchestrator][DEBUG] pose_skeletons directory not found, skipping ControlNet");
        }
        None
    };

    println!("[Orchestrator] Subject count tag: {}", subject_count_tag);

    // Build per-character description segments (collected, not appended inline).
    let mut char_segments: Vec<String> = Vec::new();
    for (i, character) in characters.iter().enumerate().take(num_chars) {
        let region = regions.get(i).map(|s| s.as_str()).unwrap_or("center");

        let mut char_parts: Vec<String> = Vec::new();
        if let Some(ref sd) = character.sd_prompt {
            if !sd.is_empty() {
                // Strip portrait-specific tags that fight scene composition.
                // Keep physical descriptors (age, hair, skin, body type).
                let scene_safe: String = sd
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| {
                        let lower = s.to_lowercase();
                        !lower.contains("solo")
                            && !lower.contains("portrait")
                            && !lower.contains("looking at viewer")
                            && !lower.contains("looking at camera")
                            && !lower.contains("neutral")
                            && !lower.contains("background")
                            && !lower.contains("masterpiece")
                            && !lower.contains("best quality")
                            && !lower.contains("detailed face")
                            && !lower.contains("detailed eyes")
                            && !lower.contains("1girl")
                            && !lower.contains("1boy")
                            && !lower.contains("1woman")
                            && !lower.contains("1man")
                            && !lower.contains("upper body")
                            && !lower.contains("close up")
                            && !lower.contains("closeup")
                            && !lower.contains("headshot")
                    })
                    .collect::<Vec<&str>>()
                    .join(", ");
                if !scene_safe.is_empty() {
                    char_parts.push(scene_safe);
                }
            }
        }
        if let Some(ref clothing) = character.default_clothing {
            if !clothing.is_empty() {
                char_parts.push(clothing.clone());
            }
        }

        // Soften feminine features — InsightFace embeddings can skew masculine
        if infer_gender(character) == "female" {
            char_parts.push("soft feminine features, smooth jawline, delicate face, no cleft chin".to_string());
        }

        let char_desc = if char_parts.is_empty() {
            "a person".to_string()
        } else {
            char_parts.join(", ")
        };

        let region_prefix = match region {
            "left" => "a person on the left side of the scene,",
            "right" => "a person on the right side of the scene,",
            _ => "a person in the center of the scene,",
        };
        char_segments.push(format!("{} {}", region_prefix, char_desc));
    }

    // Append active scene metadata to the prompt as a fallback if the LLM's scene
    // description doesn't already include it (mood, time, location context).
    let scene_prompt = if let Some((_, _, ref location, ref time, ref mood)) = active_scene_data {
        let enhancement_parts: Vec<String> = [
            location.clone(),
            time.clone().map(|t| format!("{} lighting", t)),
            mood.clone().map(|m| format!("{} atmosphere", m)),
        ]
        .into_iter()
        .flatten()
        .collect();

        if !enhancement_parts.is_empty() {
            let enhancement = enhancement_parts.join(", ");
            // Only append if the scene_prompt doesn't already contain these terms
            let lower_prompt = scene_prompt.to_lowercase();
            let new_parts: Vec<String> = enhancement_parts
                .into_iter()
                .filter(|p| !lower_prompt.contains(&p.to_lowercase()))
                .collect();
            if !new_parts.is_empty() {
                let appended = format!("{}, {}", scene_prompt, new_parts.join(", "));
                println!("[Orchestrator] Scene enhancement appended: {}", new_parts.join(", "));
                appended
            } else {
                scene_prompt
            }
        } else {
            scene_prompt
        }
    } else {
        scene_prompt
    };

    // Assemble in SDXL priority order:
    //   (quality tags), (POSE:weight), subject_count, scene_description,
    //   character_descriptions, face_quality
    // Pose goes first so it overrides the model's default standing pose.
    let pose_prefix = if pose_emphasis.is_empty() {
        String::new()
    } else {
        format!("{}, ", pose_emphasis)
    };

    let enriched_prompt = format!(
        "(masterpiece, best quality, highly detailed, cinematic composition), (medium shot, waist up, head and torso visible:1.2), {}{}, {}, {}, (detailed face, clear face:1.1)",
        pose_prefix,
        subject_count_tag,
        scene_prompt,
        char_segments.join(", ")
    );

    println!(
        "[Orchestrator] Enriched scene prompt: {}",
        &enriched_prompt[..enriched_prompt.len().min(200)]
    );
    println!("[Orchestrator][DEBUG] Full enriched prompt:\n{}", &enriched_prompt);

    // Apply user overrides if provided — skip auto-built prompts in favour of edited ones
    let (final_positive, final_negative_override) = if let Some(pos) = positive_prompt_override {
        let neg = negative_prompt_override;
        println!("[Orchestrator] Using custom prompts from user edit ({} chars)", pos.len());
        (pos, neg)
    } else {
        (enriched_prompt, None)
    };

    // Generate per-character masks for 2-char workflow
    let mask_paths: Vec<String> = if num_chars > 1 {
        let masks_dir = app_data.join("masks");
        let mut paths = Vec::new();

        for (i, region) in regions.iter().enumerate().take(num_chars) {
            let mask_chars = vec![mask_generator::MaskCharacter {
                name: characters.get(i).map(|c| c.name.clone()).unwrap_or_default(),
                region: region.to_string(),
                color_index: 0, // MUST be 0 (red) — ImageToMask reads red channel
            }];

            let filename = format!("scene_mask_char{}.png", i);
            match mask_generator::generate_mask(
                &mask_chars,
                DEFAULT_IMAGE_WIDTH,
                DEFAULT_IMAGE_HEIGHT,
                &masks_dir,
                Some(&filename),
            ) {
                Ok(r) => {
                    println!("[Orchestrator] Mask for char {}: {} (region={})", i, r.path, region);
                    paths.push(r.path);
                }
                Err(e) => return Err(format!("Mask gen failed for char {}: {}", i, e)),
            }
        }
        paths
    } else {
        vec![]
    };

    let char_inputs: Vec<comfyui_api::CharacterInput> = characters
        .iter()
        .enumerate()
        .take(num_chars)
        .map(|(i, c)| comfyui_api::CharacterInput {
            name: c.name.clone(),
            reference_image_path: c.master_image_path.clone().unwrap_or_default(),
            region: regions.get(i).cloned().unwrap_or_else(|| "center".to_string()),
            prompt: String::new(),
        })
        .collect();

    let sfw_negative = if content_rating == "sfw" {
        ", nsfw, nude, naked, nudity, bare chest, cleavage, lingerie, underwear, \
         suggestive, seductive, sexual, explicit, provocative, revealing clothing, \
         bikini, swimsuit, exposed skin, nipples, breasts"
    } else {
        ""
    };

    let (scene_width, scene_height) = if num_chars == 1 {
        (896u32, 1152u32)
    } else {
        (1152u32, 896u32)
    };

    let request = comfyui_api::ImageGenRequest {
        scene_prompt: final_positive.clone(),
        characters: char_inputs,
        mask_paths,
        workflow_template: workflow_path,
        comfyui_url: None,
        seed: Some(rand::random::<i64>().abs()),
        steps: None,
        cfg: None,
        width: Some(scene_width),
        height: Some(scene_height),
        negative_prompt: Some(final_negative_override.unwrap_or_else(|| format!(
            "(cropped head:1.5), (head out of frame:1.5), (cut off head:1.5), (headless:1.5), decapitated, \
             (worst quality, low quality:1.4), (bad anatomy:1.3), (bad hands:1.4), \
             close-up, closeup, head shot, headshot, cropped, zoomed in, \
             cowboy hat, cowboy, western clothing{}",
            sfw_negative
        ))),
        timeout_secs: Some(600),
        controlnet_image_path,
        controlnet_strength: Some(controlnet_strength),
    };

    let output_dir = app_data.join("generated_images");
    println!(
        "[Orchestrator][DEBUG] ControlNet: enabled={}, skeleton={}, strength={}",
        controlnet_enabled,
        request.controlnet_image_path.as_deref().unwrap_or("NONE"),
        controlnet_strength
    );
    println!(
        "[Orchestrator][DEBUG] Full scene image generation summary:\n  \
         Pose: {}\n  ControlNet: {}\n  Characters: {}\n  Prompt (first 150): {}",
        character_poses.as_ref().and_then(|p| p.first()).map(|s| s.as_str()).unwrap_or("(keyword fallback)"),
        request.controlnet_image_path.as_deref().unwrap_or("NONE"),
        characters.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", "),
        &request.scene_prompt[..request.scene_prompt.len().min(150)]
    );
    println!(
        "[Orchestrator] Submitting to ComfyUI: {} character(s), workflow={}, output_dir={}",
        request.characters.len(),
        request.workflow_template,
        output_dir.display()
    );
    println!(
        "[Orchestrator][DEBUG] ImageGenRequest JSON:\n{}",
        serde_json::to_string_pretty(&request).unwrap_or_else(|_| "SERIALIZATION_FAILED".to_string())
    );

    // Free VRAM: unload Ollama model before ComfyUI needs the GPU
    {
        let ollama_url = {
            let config = config_state.0.lock().map_err(|e| e.to_string())?;
            config.ollama_url.clone()
        };
        unload_ollama_model(&ollama_url).await;
    }

    let result = comfyui_api::generate_scene_image(&request, &output_dir)
        .await
        .map_err(|e| {
            println!("[Orchestrator] ComfyUI call FAILED: {}", e);
            e.to_string()
        })?;

    let image_path = result.image_paths.into_iter().next()
        .ok_or_else(|| "No image generated".to_string())?;
    println!("[Orchestrator] Scene image generated successfully: {}", image_path);
    unload_comfyui_models("http://localhost:8188").await;
    println!("[VRAM] Image generation complete — both models unloaded, VRAM clean");
    Ok(image_path)
}

/// Generate a scene image using user-provided prompts (skips enrichment).
/// Reuses the same character reference / mask / workflow pipeline as the normal
/// illustration path, but substitutes the caller's positive and negative prompts
/// directly instead of building them from LLM output.
#[tauri::command]
pub async fn illustrate_scene_custom(
    story_id: i64,
    chat_id: i64,
    message_id: i64,
    positive_prompt: String,
    negative_prompt: String,
    config_state: State<'_, ConfigState>,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<String, String> {
    println!(
        "[Orchestrator] illustrate_scene_custom: story={} message={}",
        story_id, message_id
    );
    println!(
        "[Orchestrator] Custom positive (first 150): {}",
        &positive_prompt[..positive_prompt.len().min(150)]
    );

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // Load characters with reference images for this story
    let all_characters = get_characters_with_references(Some(story_id), &state).await?;

    if all_characters.is_empty() {
        return Err(
            "No characters with reference images found. \
             To generate scene images, open a character in the Character panel, \
             generate a Master Portrait, then save it as the reference image."
                .to_string(),
        );
    }

    let num_chars = all_characters.len().min(2);
    let workflow_path = select_workflow(num_chars, &app_data)?;

    let regions: Vec<String> = match num_chars {
        1 => vec!["center".to_string()],
        _ => vec!["left".to_string(), "right".to_string()],
    };

    // Generate per-character masks for 2-char workflow (matches existing pipeline)
    let mask_paths: Vec<String> = if num_chars > 1 {
        let masks_dir = app_data.join("masks");
        let mut paths = Vec::new();
        for (i, region) in regions.iter().enumerate().take(num_chars) {
            let mask_chars = vec![mask_generator::MaskCharacter {
                name: all_characters.get(i).map(|c| c.name.clone()).unwrap_or_default(),
                region: region.to_string(),
                color_index: 0,
            }];
            let filename = format!("custom_mask_char{}.png", i);
            match mask_generator::generate_mask(
                &mask_chars,
                DEFAULT_IMAGE_WIDTH,
                DEFAULT_IMAGE_HEIGHT,
                &masks_dir,
                Some(&filename),
            ) {
                Ok(r) => paths.push(r.path),
                Err(e) => return Err(format!("Mask gen failed for char {}: {}", i, e)),
            }
        }
        paths
    } else {
        vec![]
    };

    let char_inputs: Vec<comfyui_api::CharacterInput> = all_characters
        .iter()
        .enumerate()
        .take(num_chars)
        .map(|(i, c)| comfyui_api::CharacterInput {
            name: c.name.clone(),
            reference_image_path: c.master_image_path.clone().unwrap_or_default(),
            region: regions.get(i).cloned().unwrap_or_else(|| "center".to_string()),
            prompt: String::new(),
        })
        .collect();

    let (scene_width, scene_height) = if num_chars == 1 {
        (896u32, 1152u32)
    } else {
        (1152u32, 896u32)
    };

    let request = comfyui_api::ImageGenRequest {
        scene_prompt: positive_prompt.clone(),
        characters: char_inputs,
        mask_paths,
        workflow_template: workflow_path,
        comfyui_url: None,
        seed: Some(rand::random::<i64>().abs()),
        steps: None,
        cfg: None,
        width: Some(scene_width),
        height: Some(scene_height),
        negative_prompt: Some(negative_prompt),
        timeout_secs: Some(600),
        controlnet_image_path: None,
        controlnet_strength: None,
    };

    // Free VRAM before ComfyUI needs the GPU
    {
        let ollama_url = {
            let config = config_state.0.lock().map_err(|e| e.to_string())?;
            config.ollama_url.clone()
        };
        unload_ollama_model(&ollama_url).await;
    }

    let output_dir = app_data.join("generated_images");
    let result = comfyui_api::generate_scene_image(&request, &output_dir)
        .await
        .map_err(|e| {
            println!("[Orchestrator] Custom illustration FAILED: {}", e);
            e.to_string()
        })?;

    let image_path = result
        .image_paths
        .into_iter()
        .next()
        .ok_or_else(|| "No image generated".to_string())?;

    unload_comfyui_models("http://localhost:8188").await;
    println!("[Orchestrator] Custom illustration complete: {}", image_path);

    // Persist the image — insert or replace so re-illustration overwrites the old entry
    sqlx::query(
        "INSERT OR REPLACE INTO images (message_id, chat_id, file_path) VALUES (?, ?, ?)",
    )
    .bind(message_id)
    .bind(chat_id)
    .bind(&image_path)
    .execute(&state.db)
    .await
    .ok();

    Ok(image_path)
}

/// Returns the full enriched SDXL prompts for a scene without generating an image.
/// Used by the frontend to show and edit prompts before illustration.
#[tauri::command]
pub async fn preview_scene_prompt(
    scene_prompt: String,
    story_id: Option<i64>,
    character_names: Option<Vec<String>>,
    character_poses: Option<Vec<String>>,
    app: AppHandle,
    state: State<'_, OllamaState>,
    config_state: State<'_, ConfigState>,
) -> Result<ScenePromptPreview, String> {
    let app_data = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let (content_rating, _controlnet_enabled, _controlnet_strength) = {
        let config = config_state.0.lock().map_err(|e| e.to_string())?;
        (config.content_rating.clone(), config.controlnet_pose_enabled, config.controlnet_pose_strength)
    };

    let all_characters = get_characters_with_references(story_id, &state).await?;

    // Load active scene for prompt enhancement
    let active_scene_data: Option<(i64, String, Option<String>, Option<String>, Option<String>)> =
        if let Some(sid) = story_id {
            sqlx::query(
                "SELECT s.id, s.location, s.time_of_day, s.mood, s.name \
                 FROM scenes s \
                 INNER JOIN story_premises sp ON sp.active_scene_id = s.id \
                 WHERE sp.id = ?",
            )
            .bind(sid)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .map(|r| {
                let scene_id: i64 = r.get("id");
                let location: Option<String> = r.get("location");
                let time: Option<String> = r.get("time_of_day");
                let mood: Option<String> = r.get("mood");
                let name: String = r.get("name");
                (scene_id, name, location, time, mood)
            })
        } else {
            None
        };

    let scene_char_ids: Option<Vec<i64>> = if let Some((scene_id, ..)) = &active_scene_data {
        let rows = sqlx::query(
            "SELECT character_id FROM scene_characters WHERE scene_id = ?",
        )
        .bind(scene_id)
        .fetch_all(&state.db)
        .await
        .ok();
        rows.map(|rs| rs.iter().map(|r| r.get::<i64, _>("character_id")).collect())
    } else {
        None
    };

    let characters: Vec<CharacterLookup> = if let Some(ref names) = character_names {
        let names_lower: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
        let filtered: Vec<CharacterLookup> = all_characters
            .iter()
            .filter(|c| names_lower.contains(&c.name.to_lowercase()))
            .cloned()
            .collect();
        if filtered.is_empty() { all_characters } else { filtered }
    } else {
        all_characters
    };

    let characters: Vec<CharacterLookup> = if character_names.is_none() {
        if let Some(ref ids) = scene_char_ids {
            if !ids.is_empty() {
                let filtered: Vec<CharacterLookup> = characters
                    .iter()
                    .filter(|c| ids.contains(&c.id))
                    .cloned()
                    .collect();
                if filtered.is_empty() { characters } else { filtered }
            } else {
                characters
            }
        } else {
            characters
        }
    } else {
        characters
    };

    if characters.is_empty() {
        return Err(
            "No characters with reference images found. \
             To generate scene images, open a character, generate a Master Portrait, \
             then save it as the reference image.".to_string()
        );
    }

    let num_chars = characters.len().min(2);
    let regions: Vec<String> = match num_chars {
        1 => vec!["center".to_string()],
        _ => vec!["left".to_string(), "right".to_string()],
    };

    let genders: Vec<&'static str> = characters.iter().take(num_chars).map(infer_gender).collect();
    let subject_count_tag = match num_chars {
        1 => match genders[0] {
            "female" => "1girl",
            "male"   => "1boy",
            _        => "1person",
        },
        2 => match (genders[0], genders[1]) {
            ("female", "female")                   => "2girls",
            ("male",   "male")                     => "2boys",
            ("female", "male") | ("male", "female") => "1girl 1boy",
            ("female", _) | (_, "female")          => "1girl 1person",
            ("male",   _) | (_, "male")            => "1boy 1person",
            _                                      => "2people",
        },
        _ => "people",
    };

    let pose_emphasis = extract_pose_emphasis(&scene_prompt);

    let mut char_segments: Vec<String> = Vec::new();
    for (i, character) in characters.iter().enumerate().take(num_chars) {
        let region = regions.get(i).map(|s| s.as_str()).unwrap_or("center");
        let mut char_parts: Vec<String> = Vec::new();
        if let Some(ref sd) = character.sd_prompt {
            if !sd.is_empty() {
                let scene_safe: String = sd
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| {
                        let lower = s.to_lowercase();
                        !lower.contains("solo")
                            && !lower.contains("portrait")
                            && !lower.contains("looking at viewer")
                            && !lower.contains("looking at camera")
                            && !lower.contains("neutral")
                            && !lower.contains("background")
                            && !lower.contains("masterpiece")
                            && !lower.contains("best quality")
                            && !lower.contains("detailed face")
                            && !lower.contains("detailed eyes")
                            && !lower.contains("1girl")
                            && !lower.contains("1boy")
                            && !lower.contains("1woman")
                            && !lower.contains("1man")
                            && !lower.contains("upper body")
                            && !lower.contains("close up")
                            && !lower.contains("closeup")
                            && !lower.contains("headshot")
                    })
                    .collect::<Vec<&str>>()
                    .join(", ");
                if !scene_safe.is_empty() {
                    char_parts.push(scene_safe);
                }
            }
        }
        if let Some(ref clothing) = character.default_clothing {
            if !clothing.is_empty() {
                char_parts.push(clothing.clone());
            }
        }
        if infer_gender(character) == "female" {
            char_parts.push("soft feminine features, smooth jawline, delicate face, no cleft chin".to_string());
        }
        let char_desc = if char_parts.is_empty() {
            "a person".to_string()
        } else {
            char_parts.join(", ")
        };
        let region_prefix = match region {
            "left" => "a person on the left side of the scene,",
            "right" => "a person on the right side of the scene,",
            _ => "a person in the center of the scene,",
        };
        char_segments.push(format!("{} {}", region_prefix, char_desc));
    }

    // Enhance scene_prompt with active scene metadata
    let scene_prompt = if let Some((_, _, ref location, ref time, ref mood)) = active_scene_data {
        let enhancement_parts: Vec<String> = [
            location.clone(),
            time.clone().map(|t| format!("{} lighting", t)),
            mood.clone().map(|m| format!("{} atmosphere", m)),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !enhancement_parts.is_empty() {
            let lower_prompt = scene_prompt.to_lowercase();
            let new_parts: Vec<String> = enhancement_parts
                .into_iter()
                .filter(|p| !lower_prompt.contains(&p.to_lowercase()))
                .collect();
            if !new_parts.is_empty() {
                format!("{}, {}", scene_prompt, new_parts.join(", "))
            } else {
                scene_prompt
            }
        } else {
            scene_prompt
        }
    } else {
        scene_prompt
    };

    let pose_prefix = if pose_emphasis.is_empty() {
        String::new()
    } else {
        format!("{}, ", pose_emphasis)
    };

    let positive = format!(
        "(masterpiece, best quality, highly detailed, cinematic composition), (medium shot, waist up, head and torso visible:1.2), {}{}, {}, {}, (detailed face, clear face:1.1)",
        pose_prefix,
        subject_count_tag,
        scene_prompt,
        char_segments.join(", ")
    );

    let sfw_negative = if content_rating == "sfw" {
        ", nsfw, nude, naked, nudity, bare chest, cleavage, lingerie, underwear, \
         suggestive, seductive, sexual, explicit, provocative, revealing clothing, \
         bikini, swimsuit, exposed skin, nipples, breasts"
    } else {
        ""
    };

    let negative = format!(
        "(cropped head:1.5), (head out of frame:1.5), (cut off head:1.5), (headless:1.5), decapitated, \
         (worst quality, low quality:1.4), (bad anatomy:1.3), (bad hands:1.4), \
         close-up, closeup, head shot, headshot, cropped, zoomed in, \
         cowboy hat, cowboy, western clothing{}",
        sfw_negative
    );

    let _ = app_data; // used implicitly above
    let _ = character_poses; // accepted for API symmetry, not used in preview
    Ok(ScenePromptPreview { positive, negative })
}

/// Scans the scene prompt for action/pose keywords and returns an emphasized
/// pose tag to place at the front of the enriched prompt.  SDXL weighs early
/// tokens and `(tag:weight)` syntax most heavily, so putting the pose here
/// overrides the model's default "standing facing camera" tendency.
/// Returns an empty string when no recognizable pose is detected.
fn extract_pose_emphasis(scene_prompt: &str) -> String {
    let lower = scene_prompt.to_lowercase();

    // Ordered from most-specific to least-specific so a driving scene doesn't
    // accidentally match the generic "sitting" branch first.
    let checks: &[(&[&str], &str)] = &[
        (
            &["lying", "lay", "nap", "sleep", "bed", "resting"],
            "(person lying down in bed, resting, eyes closed:1.4)",
        ),
        (
            &["driving", "truck", "car", "steering", "behind the wheel"],
            "(person sitting in vehicle, driving:1.3)",
        ),
        (
            &["riding", "horse", "horseback"],
            "(person riding a horse:1.4)",
        ),
        (
            &["sitting", "sat", "seat", "chair", "booth", "diner", "eating"],
            "(person sitting down:1.3)",
        ),
        (
            &["running", "ran", "sprint", "rushing"],
            "(person running, dynamic motion:1.3)",
        ),
        (
            &["kneeling", "crouching", "bending"],
            "(person kneeling down:1.3)",
        ),
        (
            &["cooking", "kitchen", "stove", "preparing"],
            "(person cooking in kitchen, hands busy:1.3)",
        ),
        (
            &["walking", "walked", "strolling", "heading"],
            "(person walking, in motion:1.2)",
        ),
    ];

    for (keywords, emphasis) in checks {
        if keywords.iter().any(|kw| lower.contains(kw)) {
            return emphasis.to_string();
        }
    }
    String::new()
}

/// Detect a pose name from scene prompt keywords.
/// Returns a pose name string that maps to a skeleton PNG in pose_skeletons/.
fn detect_pose_name_from_prompt(scene_prompt: &str) -> String {
    let lower = scene_prompt.to_lowercase();

    let checks: &[(&[&str], &str)] = &[
        (&["lying", "lay", "nap", "sleep", "bed", "resting"], "LYING_DOWN"),
        (&["driving", "truck", "car", "steering", "behind the wheel"], "DRIVING"),
        (&["riding", "horse", "horseback"], "STANDING"), // no riding skeleton yet
        (&["sitting", "sat", "seat", "chair", "booth", "diner", "eating"], "SITTING"),
        (&["running", "ran", "sprint", "rushing", "dashing", "chasing"], "RUNNING"),
        (&["kneeling", "crouching", "bending", "ducking"], "KNEELING"),
        (&["cooking", "kitchen", "stove", "preparing"], "COOKING"),
        (&["leaning", "propped", "resting against", "slouching"], "LEANING"),
        (&["fighting", "punching", "kicking", "combat", "sparring"], "FIGHTING"),
        (&["walking", "walked", "strolling", "heading"], "WALKING"),
    ];

    for (keywords, pose_name) in checks {
        if keywords.iter().any(|kw| lower.contains(kw)) {
            return pose_name.to_string();
        }
    }

    "STANDING".to_string()
}

/// Infer "female" / "male" / "unknown" from the character's DB gender field,
/// falling back to keyword scanning of the sd_prompt if the field is absent.
fn infer_gender(character: &CharacterLookup) -> &'static str {
    // 1. Explicit DB gender field
    if let Some(ref g) = character.gender {
        let gl = g.to_lowercase();
        if gl.contains("female") || gl.contains("woman") || gl.contains("girl") {
            return "female";
        }
        if gl.contains("male") || gl.contains("man") || gl.contains("boy") {
            return "male";
        }
    }
    // 2. Keyword scan of sd_prompt as fallback
    if let Some(ref sd) = character.sd_prompt {
        let lower = sd.to_lowercase();
        if lower.contains("1girl") || lower.contains("female") || lower.contains("woman") || lower.contains("girl") {
            return "female";
        }
        if lower.contains("1boy") || lower.contains("male") || lower.contains("man") || lower.contains("boy") {
            return "male";
        }
    }
    "unknown"
}

async fn get_characters_with_references(
    story_id: Option<i64>,
    state: &State<'_, OllamaState>,
) -> Result<Vec<CharacterLookup>, String> {
    // Query characters that have reference images, story-scoped if possible,
    // falling back to global if story_id is None or returns nothing.
    let rows: Vec<sqlx::sqlite::SqliteRow> = if let Some(sid) = story_id {
        let scoped = sqlx::query(
            "SELECT c.id, c.name, c.master_image_path, c.sd_prompt, c.default_clothing, c.art_style, c.gender
             FROM characters c
             INNER JOIN story_characters sc ON sc.character_id = c.id
             WHERE sc.story_id = ? AND c.master_image_path IS NOT NULL"
        )
        .bind(sid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

        if scoped.is_empty() {
            // Fall back to global if story scope returns nothing
            sqlx::query(
                "SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style, gender
                 FROM characters WHERE master_image_path IS NOT NULL"
            )
            .fetch_all(&state.db)
            .await
            .map_err(|e| e.to_string())?
        } else {
            scoped
        }
    } else {
        // No story_id — query all characters with reference images
        sqlx::query(
            "SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style, gender
             FROM characters WHERE master_image_path IS NOT NULL"
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(rows.iter().map(|r| CharacterLookup {
        id: r.get("id"),
        name: r.get("name"),
        master_image_path: r.get("master_image_path"),
        sd_prompt: r.get("sd_prompt"),
        default_clothing: r.get("default_clothing"),
        art_style: r.get("art_style"),
        gender: r.get("gender"),
    }).collect())
}

// ============================================================================
// REGENERATE COMMAND
// ============================================================================

/// Regenerate the last AI response for a chat turn.
///
/// Deletes the last user + assistant message pair from the DB, then
/// re-runs the full story turn pipeline with the same user input.
///
/// ## Frontend usage
/// ```typescript
/// const result = await invoke('regenerate_story', { id: chatId, storyId });
/// ```
#[tauri::command]
pub async fn regenerate_story(
    id: i64,
    story_id: Option<i64>,
    state: State<'_, OllamaState>,
    config_state: State<'_, ConfigState>,
    hint_state: State<'_, SceneHintState>,
    app: AppHandle,
) -> Result<StoryTurnResult, String> {
    println!(
        "\n[Orchestrator] ========== REGENERATE (chat={}, story={:?}) ==========",
        id, story_id
    );

    // 1. Load all messages to find the last user + assistant pair
    let rows = sqlx::query(
        "SELECT id, role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to load messages: {}", e))?;

    let mut last_user_id: Option<i64> = None;
    let mut last_user_content: Option<String> = None;
    let mut last_assistant_id: Option<i64> = None;

    for row in &rows {
        let role: String = row.get("role");
        let msg_id: i64 = row.get("id");
        let content: String = row.get("content");
        match role.as_str() {
            "user" => {
                last_user_id = Some(msg_id);
                last_user_content = Some(content);
            }
            "assistant" => {
                last_assistant_id = Some(msg_id);
            }
            _ => {}
        }
    }

    let user_input = last_user_content
        .ok_or_else(|| "No user message found to regenerate".to_string())?;
    println!(
        "[Orchestrator] Regenerating from user input: {:?}",
        &user_input[..user_input.len().min(80)]
    );

    // 2. Delete last assistant message and its image record
    if let Some(asst_id) = last_assistant_id {
        sqlx::query("DELETE FROM images WHERE message_id = ?")
            .bind(asst_id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to delete image record: {}", e))?;

        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(asst_id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to delete assistant message: {}", e))?;

        println!("[Orchestrator] Deleted assistant message id={}", asst_id);
    }

    // 3. Delete last user message (process_story_turn will re-save it)
    if let Some(user_id) = last_user_id {
        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(user_id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to delete user message: {}", e))?;

        println!("[Orchestrator] Deleted user message id={}", user_id);
    }

    // 4. Re-run the full turn pipeline with the same user input
    process_story_turn(id, user_input, story_id, state, config_state, hint_state, app).await
}

/// Regenerate the last AI response with modified user input.
///
/// Like `regenerate_story`, deletes the last user + assistant pair,
/// then re-runs the pipeline with the NEW user input provided.
///
/// ## Frontend usage
/// ```typescript
/// const result = await invoke('regenerate_story_with_input', {
///   id: chatId,
///   userInput: "edited text",
///   storyId
/// });
/// ```
#[tauri::command]
pub async fn regenerate_story_with_input(
    id: i64,
    user_input: String,
    story_id: Option<i64>,
    state: State<'_, OllamaState>,
    config_state: State<'_, ConfigState>,
    hint_state: State<'_, SceneHintState>,
    app: AppHandle,
) -> Result<StoryTurnResult, String> {
    println!(
        "\n[Orchestrator] ========== REGENERATE WITH EDIT (chat={}, story={:?}) ==========",
        id, story_id
    );

    // 1. Load all messages to find the last user + assistant pair
    let rows = sqlx::query(
        "SELECT id, role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to load messages: {}", e))?;

    let mut last_user_id: Option<i64> = None;
    let mut last_assistant_id: Option<i64> = None;

    for row in &rows {
        let role: String = row.get("role");
        let msg_id: i64 = row.get("id");
        match role.as_str() {
            "user" => {
                last_user_id = Some(msg_id);
            }
            "assistant" => {
                last_assistant_id = Some(msg_id);
            }
            _ => {}
        }
    }

    if last_user_id.is_none() {
        return Err("No user message found to regenerate".to_string());
    }

    println!(
        "[Orchestrator] Regenerating with edited input: {:?}",
        &user_input[..user_input.len().min(80)]
    );

    // 2. Delete last assistant message and its image record
    if let Some(asst_id) = last_assistant_id {
        sqlx::query("DELETE FROM images WHERE message_id = ?")
            .bind(asst_id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to delete image record: {}", e))?;

        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(asst_id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to delete assistant message: {}", e))?;

        println!("[Orchestrator] Deleted assistant message id={}", asst_id);
    }

    // 3. Delete last user message (process_story_turn will save the NEW input)
    if let Some(user_id) = last_user_id {
        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(user_id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to delete user message: {}", e))?;

        println!("[Orchestrator] Deleted user message id={}", user_id);
    }

    // 4. Re-run the full turn pipeline with the EDITED user input
    process_story_turn(id, user_input, story_id, state, config_state, hint_state, app).await
}

// ============================================================================
// DIAGNOSTIC COMMANDS
// ============================================================================

/// Returns compression stats for a chat session — useful for debugging
/// context window usage in the frontend.
#[tauri::command]
pub async fn get_compression_diagnostics(
    chat_id: i64,
    state: State<'_, OllamaState>,
) -> Result<CompressionDiagnostics, String> {
    let rows = sqlx::query(
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC",
    )
    .bind(chat_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let db_rows: Vec<(String, String)> = rows
        .iter()
        .map(|r| (r.get("role"), r.get("content")))
        .collect();

    let conversation = ConversationContext::from_db_rows(&db_rows);

    let system_tokens = estimate_tokens(SYSTEM_PROMPT);

    let char_rows = sqlx::query("SELECT sd_prompt, personality FROM characters")
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let char_tokens: usize = char_rows
        .iter()
        .map(|r| {
            let sd: String = r.get::<Option<String>, _>("sd_prompt").unwrap_or_default();
            let personality: String = r
                .get::<Option<String>, _>("personality")
                .unwrap_or_default();
            estimate_tokens(&sd) + estimate_tokens(&personality) + 20
        })
        .sum();

    Ok(get_diagnostics(&conversation, system_tokens, char_tokens))
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
            pose: "".into(),
            action: "walking toward table".into(),
            expression: "warm smile".into(),
            clothing: "blue jacket, white t-shirt".into(),
            facing: "Marcus".into(),
            needs_render: true,
            db_id: Some(1),
            has_reference_image: true,
            prompt_only_description: None,
        };
        let db = CharacterLookup {
            id: 1,
            name: "Elena".into(),
            master_image_path: Some("/path/to/ref.png".into()),
            sd_prompt: Some("young woman, brown hair".into()),
            default_clothing: Some("casual dress".into()),
            art_style: Some("Realistic".into()),
            gender: Some("female".into()),
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
            pose: "".into(),
            action: "".into(),
            expression: "neutral".into(),
            clothing: "dark suit".into(),
            facing: "".into(),
            needs_render: true,
            db_id: None,
            has_reference_image: false,
            prompt_only_description: None,
        };
        let fragment = character_prompt_fragment(&cis, None);
        assert!(fragment.contains("full body"));
        assert!(fragment.contains("neutral"));
        assert!(fragment.contains("dark suit"));
    }

    #[test]
    fn test_build_characters_in_scene_clothing_fallback() {
        let raw = llm_parser::SceneCharacterRaw {
            name: "Marcus".into(),
            region: "left".into(),
            view: "FULL-BODY".into(),
            pose: "".into(),
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
            gender: Some("male".into()),
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