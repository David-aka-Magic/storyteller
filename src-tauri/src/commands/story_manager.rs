// src-tauri/src/commands/story_manager.rs
//
// Story Manager for StoryEngine
// ================================
// Provides full story session lifecycle management:
//   - Create new stories with initial characters
//   - Load a full StorySession (premise, characters, compressed history, recent turns)
//   - Auto-save compressed history and location on each turn
//   - List stories with summaries (title, turn count, last played, thumbnail)
//   - Delete stories with cascading cleanup
//   - Export stories as JSON (HTML/PDF planned for future)
//
// This module extends the existing story_premises, chats, messages, characters,
// and images tables. It adds new columns to story_premises for richer session
// tracking (last_played_at, current_location, compressed_history, etc.)

use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;
use tauri::{AppHandle, Manager, State};

use crate::context_compression::{CompressedHistory, ConversationContext, StoryTurn};
use crate::models::CharacterProfile;
use crate::state::OllamaState;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Full story session returned by `load_story`.
/// Contains everything the frontend needs to resume a story.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorySession {
    pub story_id: i64,
    pub title: String,
    pub description: String,
    pub characters: Vec<CharacterProfile>,
    pub compressed_history: CompressedHistory,
    pub recent_turns: Vec<StoryTurn>,
    pub current_location: Option<String>,
    pub total_turns: usize,
    pub created_at: String,
    pub last_played_at: String,
    /// The chat_id associated with this story (for message storage)
    pub chat_id: Option<i64>,
}

/// Lightweight summary for the story list view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorySummary {
    pub story_id: i64,
    pub title: String,
    pub description: String,
    pub character_count: usize,
    pub turn_count: usize,
    pub last_played_at: String,
    pub created_at: String,
    pub thumbnail_path: Option<String>,
    pub current_location: Option<String>,
}

/// Parameters for creating a new story.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStoryRequest {
    pub title: String,
    pub description: String,
    /// Optional character IDs to associate with this story.
    /// These existing characters will have their story_id updated.
    pub initial_character_ids: Option<Vec<i64>>,
}

/// Export format options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Html,
    // Pdf, // Future
}

/// Exported story data (JSON format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedStory {
    pub meta: ExportedMeta,
    pub characters: Vec<CharacterProfile>,
    pub compressed_history: CompressedHistory,
    pub turns: Vec<ExportedTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedMeta {
    pub story_id: i64,
    pub title: String,
    pub description: String,
    pub total_turns: usize,
    pub created_at: String,
    pub last_played_at: String,
    pub current_location: Option<String>,
    pub exported_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedTurn {
    pub turn_number: usize,
    pub user_input: String,
    pub story_text: String,
    pub summary_hint: String,
    pub timestamp: String,
    pub image_path: Option<String>,
}

// ============================================================================
// DATABASE MIGRATION
// ============================================================================

/// Run migration to add story manager columns to story_premises.
/// Called from state.rs setup_database. Safe to call multiple times.
pub async fn run_migrations(pool: &sqlx::SqlitePool) {
    // =========================================================================
    // FIX: Ensure created_at exists on story_premises.
    // On databases created before this column was added to the CREATE TABLE
    // statement, the column may be missing. This ALTER is idempotent —
    // .ok() silently swallows the "duplicate column" error from SQLite.
    // =========================================================================
    sqlx::query(
        "ALTER TABLE story_premises ADD COLUMN created_at DATETIME DEFAULT CURRENT_TIMESTAMP"
    )
    .execute(pool).await.ok();

    // Add new columns to story_premises (silently fails if already exist)
    sqlx::query("ALTER TABLE story_premises ADD COLUMN last_played_at DATETIME DEFAULT CURRENT_TIMESTAMP")
        .execute(pool).await.ok();
    sqlx::query("ALTER TABLE story_premises ADD COLUMN current_location TEXT")
        .execute(pool).await.ok();
    sqlx::query("ALTER TABLE story_premises ADD COLUMN compressed_history_json TEXT DEFAULT '{}'")
        .execute(pool).await.ok();
    sqlx::query("ALTER TABLE story_premises ADD COLUMN chat_id INTEGER REFERENCES chats(id)")
        .execute(pool).await.ok();
    sqlx::query("ALTER TABLE story_premises ADD COLUMN thumbnail_path TEXT")
        .execute(pool).await.ok();

    // Index for fast story lookups by last played
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_story_premises_last_played ON story_premises(last_played_at DESC)")
        .execute(pool).await.ok();

    // Index for chat -> story relationship
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_story_premises_chat ON story_premises(chat_id)")
        .execute(pool).await.ok();
}

// ============================================================================
// COMMANDS
// ============================================================================

/// Create a new story with an associated chat session.
/// Returns the new story_id.
#[tauri::command]
pub async fn create_story(
    title: String,
    description: String,
    initial_character_ids: Option<Vec<i64>>,
    state: State<'_, OllamaState>,
) -> Result<i64, String> {
    // 1. Create a chat for this story
    let chat_result = sqlx::query(
        "INSERT INTO chats (title, created_at) VALUES (?, CURRENT_TIMESTAMP)"
    )
    .bind(&title)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to create chat: {}", e))?;

    let chat_id = chat_result.last_insert_rowid();

    // 2. Create the story premise with the chat link
    let story_result = sqlx::query(
        "INSERT INTO story_premises (title, description, chat_id, created_at, last_played_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
    )
    .bind(&title)
    .bind(&description)
    .bind(chat_id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to create story: {}", e))?;

    let story_id = story_result.last_insert_rowid();

    // 3. Link initial characters to this story
    if let Some(char_ids) = initial_character_ids {
        for cid in char_ids {
            sqlx::query(
                "UPDATE characters SET story_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
            )
            .bind(story_id)
            .bind(cid)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to link character {}: {}", cid, e))?;
        }
    }

    println!(
        "[StoryManager] Created story '{}' (id={}, chat_id={})",
        title, story_id, chat_id
    );

    Ok(story_id)
}

/// Load a full story session by ID.
/// Returns the premise, characters, compressed history, recent turns, and metadata.
#[tauri::command]
pub async fn load_story(
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<StorySession, String> {
    // 1. Load the story premise
    let story_row = sqlx::query(
        "SELECT id, title, description, created_at, last_played_at,
                current_location, compressed_history_json, chat_id, thumbnail_path
         FROM story_premises WHERE id = ?"
    )
    .bind(story_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("DB error: {}", e))?
    .ok_or_else(|| format!("Story {} not found", story_id))?;

    let title: String = story_row.get("title");
    let description: String = story_row.get("description");
    let created_at: String = story_row.get::<Option<String>, _>("created_at").unwrap_or_default();
    let last_played_at: String = story_row.get::<Option<String>, _>("last_played_at").unwrap_or_default();
    let current_location: Option<String> = story_row.get("current_location");
    let compressed_json: Option<String> = story_row.get("compressed_history_json");
    let chat_id: Option<i64> = story_row.get("chat_id");

    // 2. Parse compressed history
    let compressed_history: CompressedHistory = compressed_json
        .and_then(|json_str| serde_json::from_str(&json_str).ok())
        .unwrap_or_default();

    // 3. Load characters for this story
    let char_rows = sqlx::query(
        "SELECT id, story_id, name, age, gender, skin_tone, hair_style, hair_color,
                body_type, personality, additional_notes, default_clothing,
                sd_prompt, image, master_image_path, seed, art_style
         FROM characters WHERE story_id = ? ORDER BY name ASC"
    )
    .bind(story_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to load characters: {}", e))?;

    let characters: Vec<CharacterProfile> = char_rows
        .iter()
        .map(|r| CharacterProfile {
            id: r.get("id"),
            story_id: r.get("story_id"),
            name: r.get("name"),
            age: r.get("age"),
            gender: r.get("gender"),
            skin_tone: r.get("skin_tone"),
            hair_style: r.get("hair_style"),
            hair_color: r.get("hair_color"),
            body_type: r.get("body_type"),
            personality: r.get("personality"),
            additional_notes: r.get("additional_notes"),
            default_clothing: r.get("default_clothing"),
            sd_prompt: r.get("sd_prompt"),
            image: r.get("image"),
            master_image_path: r.get("master_image_path"),
            seed: r.get("seed"),
            art_style: r.get("art_style"),
        })
        .collect();

    // 4. Load recent turns from messages table
    let mut recent_turns: Vec<StoryTurn> = Vec::new();
    let mut total_turns: usize = compressed_history.compressed_turn_ids.len();

    if let Some(cid) = chat_id {
        let msg_rows = sqlx::query(
            "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC"
        )
        .bind(cid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Failed to load messages: {}", e))?;

        // Build turns from message pairs
        let rows: Vec<(String, String)> = msg_rows
            .iter()
            .map(|r| (r.get::<String, _>("role"), r.get::<String, _>("content")))
            .collect();

        let conversation = ConversationContext::from_db_rows(&rows);
        recent_turns = conversation.turns;
        total_turns += recent_turns.len();
    }

    // 5. Update last_played_at
    sqlx::query("UPDATE story_premises SET last_played_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(story_id)
        .execute(&state.db)
        .await
        .ok();

    println!(
        "[StoryManager] Loaded story '{}' (id={}, chars={}, turns={})",
        title,
        story_id,
        characters.len(),
        total_turns
    );

    Ok(StorySession {
        story_id,
        title,
        description,
        characters,
        compressed_history,
        recent_turns,
        current_location,
        total_turns,
        created_at,
        last_played_at,
        chat_id,
    })
}

/// Save the current story state (auto-save).
/// Called after compression events or scene changes.
#[tauri::command]
pub async fn save_story_state(
    story_id: i64,
    compressed_history: Option<CompressedHistory>,
    current_location: Option<String>,
    thumbnail_path: Option<String>,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    // Build dynamic UPDATE query based on what's provided
    let mut updates: Vec<String> = vec![
        "last_played_at = CURRENT_TIMESTAMP".to_string(),
    ];
    let mut binds: Vec<Option<String>> = Vec::new();

    if let Some(ref history) = compressed_history {
        let json = serde_json::to_string(history)
            .map_err(|e| format!("Failed to serialize compressed history: {}", e))?;
        updates.push("compressed_history_json = ?".to_string());
        binds.push(Some(json));
    }

    if let Some(ref loc) = current_location {
        updates.push("current_location = ?".to_string());
        binds.push(Some(loc.clone()));
    }

    if let Some(ref thumb) = thumbnail_path {
        updates.push("thumbnail_path = ?".to_string());
        binds.push(Some(thumb.clone()));
    }

    let sql = format!(
        "UPDATE story_premises SET {} WHERE id = ?",
        updates.join(", ")
    );

    let mut query = sqlx::query(&sql);
    for bind_val in &binds {
        query = query.bind(bind_val.as_deref());
    }
    query = query.bind(story_id);

    query
        .execute(&state.db)
        .await
        .map_err(|e| format!("Failed to save story state: {}", e))?;

    println!(
        "[StoryManager] Auto-saved story {} (location={:?}, has_compression={}, has_thumb={})",
        story_id,
        current_location,
        compressed_history.is_some(),
        thumbnail_path.is_some()
    );

    Ok(())
}

/// List all stories with summary information.
#[tauri::command]
pub async fn list_stories(
    state: State<'_, OllamaState>,
) -> Result<Vec<StorySummary>, String> {
    let rows = sqlx::query(
        "SELECT
            sp.id,
            sp.title,
            sp.description,
            sp.created_at,
            sp.last_played_at,
            sp.current_location,
            sp.thumbnail_path,
            sp.chat_id,
            (SELECT COUNT(*) FROM characters c WHERE c.story_id = sp.id) AS char_count,
            COALESCE(
                (SELECT COUNT(*) FROM messages m WHERE m.chat_id = sp.chat_id AND m.role = 'user'),
                0
            ) AS turn_count
         FROM story_premises sp
         ORDER BY sp.last_played_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to list stories: {}", e))?;

    let summaries: Vec<StorySummary> = rows
        .iter()
        .map(|r| {
            // Try to get last generated image as thumbnail fallback
            let thumbnail: Option<String> = r.get("thumbnail_path");

            StorySummary {
                story_id: r.get("id"),
                title: r.get("title"),
                description: r.get("description"),
                character_count: r.get::<i32, _>("char_count") as usize,
                turn_count: r.get::<i32, _>("turn_count") as usize,
                last_played_at: r.get::<Option<String>, _>("last_played_at").unwrap_or_default(),
                created_at: r.get::<Option<String>, _>("created_at").unwrap_or_default(),
                thumbnail_path: thumbnail,
                current_location: r.get("current_location"),
            }
        })
        .collect();

    Ok(summaries)
}

/// Delete a story and all associated data (cascade).
/// Removes: characters, chat + messages, images, and the story premise.
#[tauri::command]
pub async fn delete_story(
    story_id: i64,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    // 1. Get the chat_id before deleting
    let story_row = sqlx::query(
        "SELECT chat_id, thumbnail_path FROM story_premises WHERE id = ?"
    )
    .bind(story_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    if let Some(row) = story_row {
        let chat_id: Option<i64> = row.get("chat_id");
        let thumbnail: Option<String> = row.get("thumbnail_path");

        // 2. Collect image file paths to clean up from disk
        let mut image_paths: Vec<String> = Vec::new();

        if let Some(cid) = chat_id {
            let img_rows = sqlx::query(
                "SELECT file_path FROM images WHERE chat_id = ?"
            )
            .bind(cid)
            .fetch_all(&state.db)
            .await
            .map_err(|e| format!("Failed to query images: {}", e))?;

            for img_row in &img_rows {
                let path: String = img_row.get("file_path");
                image_paths.push(path);
            }
        }

        // Also collect character master images
        let char_img_rows = sqlx::query(
            "SELECT master_image_path FROM characters WHERE story_id = ? AND master_image_path IS NOT NULL"
        )
        .bind(story_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Failed to query character images: {}", e))?;

        for row in &char_img_rows {
            let path: String = row.get("master_image_path");
            image_paths.push(path);
        }

        if let Some(thumb) = thumbnail {
            image_paths.push(thumb);
        }

        // 3. Delete database records (order matters for foreign keys)
        // Delete images first
        if let Some(cid) = chat_id {
            sqlx::query("DELETE FROM images WHERE chat_id = ?")
                .bind(cid)
                .execute(&state.db)
                .await
                .ok();

            sqlx::query("DELETE FROM messages WHERE chat_id = ?")
                .bind(cid)
                .execute(&state.db)
                .await
                .ok();

            sqlx::query("DELETE FROM chats WHERE id = ?")
                .bind(cid)
                .execute(&state.db)
                .await
                .ok();
        }

        // Delete characters for this story
        sqlx::query("DELETE FROM characters WHERE story_id = ?")
            .bind(story_id)
            .execute(&state.db)
            .await
            .ok();

        // Delete the story premise itself
        sqlx::query("DELETE FROM story_premises WHERE id = ?")
            .bind(story_id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Failed to delete story: {}", e))?;

        // 4. Clean up image files from disk (best-effort)
        for path in &image_paths {
            let _ = std::fs::remove_file(path);
        }

        println!(
            "[StoryManager] Deleted story {} (cleaned {} image files)",
            story_id,
            image_paths.len()
        );
    } else {
        return Err(format!("Story {} not found", story_id));
    }

    Ok(())
}

/// Export a story as JSON or HTML.
#[tauri::command]
pub async fn export_story(
    story_id: i64,
    format: ExportFormat,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<String, String> {
    let session = load_story_internal(&state.db, story_id).await?;

    let now = {
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}", duration.as_secs())
    };

    match format {
        ExportFormat::Json => {
            let exported = ExportedStory {
                meta: ExportedMeta {
                    story_id: session.story_id,
                    title: session.title.clone(),
                    description: session.description.clone(),
                    total_turns: session.total_turns,
                    created_at: session.created_at.clone(),
                    last_played_at: session.last_played_at.clone(),
                    current_location: session.current_location.clone(),
                    exported_at: now.clone(),
                },
                characters: session.characters.clone(),
                compressed_history: session.compressed_history.clone(),
                turns: session
                    .recent_turns
                    .iter()
                    .enumerate()
                    .map(|(i, t)| ExportedTurn {
                        turn_number: i + 1,
                        user_input: t.user_input.clone(),
                        story_text: t.assistant_response.clone(),
                        summary_hint: t.summary_hint.clone(),
                        timestamp: String::new(),
                        image_path: None,
                    })
                    .collect(),
            };

            let json = serde_json::to_string_pretty(&exported)
                .map_err(|e| format!("Failed to serialize export: {}", e))?;

            let export_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app dir: {}", e))?
                .join("exports");

            std::fs::create_dir_all(&export_dir)
                .map_err(|e| format!("Failed to create export dir: {}", e))?;

            let safe_title: String = session
                .title
                .chars()
                .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '_' })
                .collect::<String>()
                .replace(' ', "_");

            let filename = format!("{}_{}.json", safe_title, now.replace(':', "-"));
            let filepath = export_dir.join(&filename);

            std::fs::write(&filepath, json)
                .map_err(|e| format!("Failed to write export file: {}", e))?;

            let path_str = filepath.to_string_lossy().to_string();
            println!("[StoryManager] Exported story {} as JSON to {}", story_id, path_str);

            Ok(path_str)
        }
        ExportFormat::Html => {
            // Build a simple HTML export
            let mut html = String::new();
            html.push_str("<!DOCTYPE html><html><head><meta charset='utf-8'>");
            html.push_str(&format!("<title>{}</title>", session.title));
            html.push_str("<style>body{font-family:Georgia,serif;max-width:800px;margin:0 auto;padding:20px;background:#1a1a2e;color:#e0e0e0}");
            html.push_str("h1{color:#e94560;border-bottom:2px solid #e94560;padding-bottom:10px}");
            html.push_str(".turn{margin:20px 0;padding:15px;background:#16213e;border-radius:8px}");
            html.push_str(".user-input{color:#4ecca3;font-style:italic;margin-bottom:8px}");
            html.push_str(".story-text{line-height:1.6}");
            html.push_str(".meta{color:#888;font-size:0.9em;margin-bottom:20px}");
            html.push_str("</style></head><body>");

            html.push_str(&format!("<h1>{}</h1>", session.title));
            html.push_str(&format!(
                "<div class='meta'><p>{}</p><p>Turns: {} | Exported: {}</p></div>",
                session.description, session.total_turns, now
            ));

            for (i, turn) in session.recent_turns.iter().enumerate() {
                html.push_str("<div class='turn'>");
                html.push_str(&format!(
                    "<div class='user-input'>Turn {}: {}</div>",
                    i + 1,
                    turn.user_input
                ));
                html.push_str(&format!(
                    "<div class='story-text'>{}</div>",
                    turn.assistant_response
                ));
                html.push_str("</div>");
            }

            html.push_str("</body></html>");

            let export_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app dir: {}", e))?
                .join("exports");

            std::fs::create_dir_all(&export_dir)
                .map_err(|e| format!("Failed to create export dir: {}", e))?;

            let safe_title: String = session
                .title
                .chars()
                .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '_' })
                .collect::<String>()
                .replace(' ', "_");

            let filename = format!("{}_{}.html", safe_title, now.replace(':', "-"));
            let filepath = export_dir.join(&filename);

            std::fs::write(&filepath, html)
                .map_err(|e| format!("Failed to write export file: {}", e))?;

            let path_str = filepath.to_string_lossy().to_string();
            println!("[StoryManager] Exported story {} as HTML to {}", story_id, path_str);

            Ok(path_str)
        }
    }
}

// ============================================================================
// INTERNAL HELPERS
// ============================================================================

/// Internal load without State wrapper (for use within this module).
async fn load_story_internal(
    db: &sqlx::SqlitePool,
    story_id: i64,
) -> Result<StorySession, String> {
    let story_row = sqlx::query(
        "SELECT id, title, description, created_at, last_played_at,
                current_location, compressed_history_json, chat_id
         FROM story_premises WHERE id = ?"
    )
    .bind(story_id)
    .fetch_optional(db)
    .await
    .map_err(|e| format!("DB error: {}", e))?
    .ok_or_else(|| format!("Story {} not found", story_id))?;

    let title: String = story_row.get("title");
    let description: String = story_row.get("description");
    let created_at: String = story_row.get::<Option<String>, _>("created_at").unwrap_or_default();
    let last_played_at: String = story_row.get::<Option<String>, _>("last_played_at").unwrap_or_default();
    let current_location: Option<String> = story_row.get("current_location");
    let compressed_json: Option<String> = story_row.get("compressed_history_json");
    let chat_id: Option<i64> = story_row.get("chat_id");

    let compressed_history: CompressedHistory = compressed_json
        .and_then(|json_str| serde_json::from_str(&json_str).ok())
        .unwrap_or_default();

    let char_rows = sqlx::query(
        "SELECT id, story_id, name, age, gender, skin_tone, hair_style, hair_color,
                body_type, personality, additional_notes, default_clothing,
                sd_prompt, image, master_image_path, seed, art_style
         FROM characters WHERE story_id = ? ORDER BY name ASC"
    )
    .bind(story_id)
    .fetch_all(db)
    .await
    .map_err(|e| format!("Failed to load characters: {}", e))?;

    let characters: Vec<CharacterProfile> = char_rows
        .iter()
        .map(|r| CharacterProfile {
            id: r.get("id"),
            story_id: r.get("story_id"),
            name: r.get("name"),
            age: r.get("age"),
            gender: r.get("gender"),
            skin_tone: r.get("skin_tone"),
            hair_style: r.get("hair_style"),
            hair_color: r.get("hair_color"),
            body_type: r.get("body_type"),
            personality: r.get("personality"),
            additional_notes: r.get("additional_notes"),
            default_clothing: r.get("default_clothing"),
            sd_prompt: r.get("sd_prompt"),
            image: r.get("image"),
            master_image_path: r.get("master_image_path"),
            seed: r.get("seed"),
            art_style: r.get("art_style"),
        })
        .collect();

    let mut recent_turns: Vec<StoryTurn> = Vec::new();
    let mut total_turns: usize = compressed_history.compressed_turn_ids.len();

    if let Some(cid) = chat_id {
        let msg_rows = sqlx::query(
            "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC"
        )
        .bind(cid)
        .fetch_all(db)
        .await
        .map_err(|e| format!("Failed to load messages: {}", e))?;

        let rows: Vec<(String, String)> = msg_rows
            .iter()
            .map(|r| (r.get::<String, _>("role"), r.get::<String, _>("content")))
            .collect();

        let conversation = ConversationContext::from_db_rows(&rows);
        recent_turns = conversation.turns;
        total_turns += recent_turns.len();
    }

    Ok(StorySession {
        story_id,
        title,
        description,
        characters,
        compressed_history,
        recent_turns,
        current_location,
        total_turns,
        created_at,
        last_played_at,
        chat_id,
    })
}