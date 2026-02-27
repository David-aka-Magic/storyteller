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
//      b. Call ComfyUI using comfyui_api.rs (selects 1-char or 2-char workflow)
//      c. Wait for and retrieve the generated image
//   7. Save the turn to the messages table
//   8. Return everything to the frontend

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;

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
const DEFAULT_IMAGE_WIDTH: u32 = 1152;
const DEFAULT_IMAGE_HEIGHT: u32 = 768;

/// Workflow filenames (relative to app data dir's "workflows" folder).
/// Single character: uses IPAdapter FaceID without masks.
/// Multi character: uses IPAdapter FaceID with per-character attention masks.
const WORKFLOW_1CHAR: &str = "workflows/scene_workflow_1char.json";
const WORKFLOW_2CHAR: &str = "workflows/scene_workflow_2char.json";

// ============================================================================
// SYSTEM PROMPT
// ============================================================================

const SYSTEM_PROMPT: &str = r#"You are an RP-API (Roleplay Application Interface) — a creative story engine that outputs structured data for an interactive visual novel system.

You are in PHASE 2: STORY GENERATION. Output raw JSON only. No markdown, no preamble, no explanation.
You MUST include ALL of these fields every single turn. Never omit any of them.

{
  "turn_id": <integer, incrementing>,
  "story_json": { "response": "<narrative text, 2-4 paragraphs>", "summary_hint": "<one sentence summary>" },
  "scene_json": { "location": "<place>", "location_type": "interior or exterior", "time_of_day": "<time>", "weather": "<weather or n/a>", "lighting": "<lighting>", "mood": "<atmosphere>" },
  "characters_in_scene": [ { "name": "<EXACT registered name>", "region": "<left|center|right|left-seated|center-seated|right-seated|left-background|center-background|right-background|off-screen>", "view": "<PORTRAIT|UPPER-BODY|FULL-BODY|NONE>", "action": "<action>", "expression": "<expression>", "clothing": "<clothing>", "facing": "<facing>" } ],
  "generation_flags": { "generate_image": <true if characters present or scene is visual>, "scene_changed": <true if location changed>, "characters_changed": <true if characters entered or exited> }
}

CHARACTER NAME RULES: Use EXACT names as registered. Names are case-sensitive. Never invent new characters."#;

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
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC",
    )
    .bind(chat_id)
    .fetch_all(db)
    .await
    .map_err(|e| format!("Failed to load messages: {}", e))?;

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

    Ok(pairs)
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

        println!("[Orchestrator] Looking up character '{}' (story_id={:?})", scene_char.name, story_id);

        // Try story-scoped lookup first
        let row = if let Some(sid) = story_id {
            let r = sqlx::query(
                "SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style, gender \
                 FROM characters WHERE name = ? AND story_id = ? LIMIT 1",
            )
            .bind(&scene_char.name)
            .bind(sid)
            .fetch_optional(db)
            .await
            .map_err(|e| format!("Character lookup failed for '{}': {}", scene_char.name, e))?;

            if r.is_none() {
                // Fallback to global search
                println!("[Orchestrator] '{}' not found in story {}, trying global lookup", scene_char.name, sid);
                sqlx::query(
                    "SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style \
                     FROM characters WHERE name = ? LIMIT 1",
                )
                .bind(&scene_char.name)
                .fetch_optional(db)
                .await
                .map_err(|e| format!("Global lookup failed for '{}': {}", scene_char.name, e))?
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
    println!("[Orchestrator] User input: {:?}", &user_input[..user_input.len().min(80)]);

    // ── Step 1: Build compressed context ─────────────────────────────

    let context_start = std::time::Instant::now();

    let message_rows = load_conversation_history(&state.db, chat_id).await?;
    let characters = load_characters_for_context(&state.db, story_id).await?;
    let story_premise = load_story_premise(&state.db).await?;

    let mut conversation = ConversationContext::from_message_pairs(&message_rows);

    let assembled = build_compressed_context(
        &mut conversation,
        SYSTEM_PROMPT,
        &characters,
        story_premise.as_deref(),
        &user_input,
    );

    let system_prompt_tokens = estimate_tokens(SYSTEM_PROMPT);
    let char_token_estimate = estimate_character_db_tokens(&characters);
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
        characters.len(),
        assembled.was_compressed,
    );

    // ── Step 2: Call Ollama ───────────────────────────────────────────

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
        ParseStatus::Partial(w) => {
            for warn in w {
                println!("[Orchestrator] Parse warning: {}", warn);
            }
            ("partial".to_string(), w.clone())
        }
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

#[tauri::command]
pub async fn generate_scene_image_for_turn(
    scene_prompt: String,
    story_id: Option<i64>,
    app: AppHandle,
    state: State<'_, OllamaState>,
) -> Result<String, String> {
    let app_data = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let characters = get_characters_with_references(story_id, &state).await?;

    if characters.is_empty() {
        return Err("No characters with reference images found".to_string());
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

    // Extract pose/action from the scene prompt to place at the front of the
    // enriched prompt.  SDXL weights early tokens most heavily, so the pose
    // emphasis overrides the model's default "standing facing camera" pose.
    let pose_emphasis = extract_pose_emphasis(&scene_prompt);

    println!("[Orchestrator] Subject count tag: {}", subject_count_tag);
    if !pose_emphasis.is_empty() {
        println!("[Orchestrator] Pose emphasis: {}", pose_emphasis);
    }

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
            "left" => "full body shot of a person on the left,",
            "right" => "full body shot of a person on the right,",
            _ => "full body shot of a person standing in the scene,",
        };
        char_segments.push(format!("{} {}", region_prefix, char_desc));
    }

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
        "(masterpiece, best quality, highly detailed, full body, wide angle, cinematic composition, environmental portrait), {}{}, {}, {}, detailed face, clear face, realistic face",
        pose_prefix,
        subject_count_tag,
        scene_prompt,
        char_segments.join(", ")
    );

    println!(
        "[Orchestrator] Enriched scene prompt: {}",
        &enriched_prompt[..enriched_prompt.len().min(200)]
    );

    // Generate per-character masks for 2-char workflow
    let mask_paths: Vec<String> = if num_chars > 1 {
        let masks_dir = app_data.join("masks");
        let mut paths = Vec::new();

        for (i, region) in regions.iter().enumerate().take(num_chars) {
            let mask_chars = vec![crate::mask_generator::MaskCharacter {
                name: characters.get(i).map(|c| c.name.clone()).unwrap_or_default(),
                region: region.to_string(),
                color_index: 0, // MUST be 0 (red) — ImageToMask reads red channel
            }];

            let filename = format!("scene_mask_char{}.png", i);
            match crate::mask_generator::generate_mask(
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

    let char_inputs: Vec<crate::comfyui_api::CharacterInput> = characters
        .iter()
        .enumerate()
        .take(num_chars)
        .map(|(i, c)| crate::comfyui_api::CharacterInput {
            name: c.name.clone(),
            reference_image_path: c.master_image_path.clone().unwrap_or_default(),
            region: regions.get(i).cloned().unwrap_or_else(|| "center".to_string()),
            prompt: String::new(),
        })
        .collect();

    let request = crate::comfyui_api::ImageGenRequest {
        scene_prompt: enriched_prompt,
        characters: char_inputs,
        mask_paths,
        workflow_template: workflow_path,
        comfyui_url: None,
        seed: None,
        steps: None,
        cfg: None,
        width: Some(DEFAULT_IMAGE_WIDTH),
        height: Some(DEFAULT_IMAGE_HEIGHT),
        negative_prompt: None,
        timeout_secs: Some(600),
    };

    let output_dir = app_data.join("generated_images");
    let result = crate::comfyui_api::generate_scene_image(&request, &output_dir)
        .await
        .map_err(|e| e.to_string())?;

    result.image_paths.into_iter().next()
        .ok_or_else(|| "No image generated".to_string())
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
            "SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style, gender
             FROM characters WHERE story_id = ? AND master_image_path IS NOT NULL"
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
            prompt_only_description: None,
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
            prompt_only_description: None,
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