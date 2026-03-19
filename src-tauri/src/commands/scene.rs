// src-tauri/src/commands/scene.rs
//
// Scene management commands for StoryEngine.
// Scenes represent named locations/settings (location, time_of_day, mood) that
// can be linked to one or more stories and optionally populated with characters.
//
// Relationships (all many-to-many via junction tables):
//   scene <-> story    : story_scenes(story_id, scene_id)
//   scene <-> character: scene_characters(scene_id, character_id)
// story_premises.active_scene_id tracks which scene is "live" for a given story.

use tauri::State;
use crate::state::{OllamaState, SceneHintState};
use crate::models::{Scene, SceneWithCharacters, CharacterProfile};
use sqlx::Row;

// ============================================================================
// HELPER: map a DB row to Scene
// ============================================================================

fn row_to_scene(r: &sqlx::sqlite::SqliteRow) -> Scene {
    Scene {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
        location: r.get("location"),
        location_type: r.get("location_type"),
        time_of_day: r.get("time_of_day"),
        mood: r.get("mood"),
        created_at: r.get("created_at"),
    }
}

fn row_to_profile(r: &sqlx::sqlite::SqliteRow) -> CharacterProfile {
    CharacterProfile {
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
        eye_color: r.get("eye_color"),
        height_scale: r.get("height_scale"),
        weight_scale: r.get("weight_scale"),
        content_rating: r.get("content_rating"),
    }
}

// ============================================================================
// SCENE CRUD
// ============================================================================

/// Create a new scene and return its id.
#[tauri::command]
pub async fn create_scene(
    name: String,
    description: Option<String>,
    location: Option<String>,
    location_type: Option<String>,
    time_of_day: Option<String>,
    mood: Option<String>,
    state: State<'_, OllamaState>,
) -> Result<i64, String> {
    let result = sqlx::query(
        "INSERT INTO scenes (name, description, location, location_type, time_of_day, mood)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&name)
    .bind(&description)
    .bind(&location)
    .bind(&location_type)
    .bind(&time_of_day)
    .bind(&mood)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.last_insert_rowid())
}

/// Update an existing scene's fields.
#[tauri::command]
pub async fn update_scene(
    id: i64,
    name: String,
    description: Option<String>,
    location: Option<String>,
    location_type: Option<String>,
    time_of_day: Option<String>,
    mood: Option<String>,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE scenes
         SET name=?, description=?, location=?, location_type=?, time_of_day=?, mood=?
         WHERE id=?"
    )
    .bind(&name)
    .bind(&description)
    .bind(&location)
    .bind(&location_type)
    .bind(&time_of_day)
    .bind(&mood)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete a scene by id. Cascade removes story_scenes and scene_characters rows.
#[tauri::command]
pub async fn delete_scene(
    id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query("DELETE FROM scenes WHERE id=?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// SCENE QUERIES
// ============================================================================

/// List all scenes linked to a specific story.
#[tauri::command]
pub async fn list_scenes_for_story(
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<Vec<Scene>, String> {
    let rows = sqlx::query(
        "SELECT s.* FROM scenes s
         JOIN story_scenes ss ON ss.scene_id = s.id
         WHERE ss.story_id = ?
         ORDER BY s.created_at ASC"
    )
    .bind(story_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_scene).collect())
}

/// List every scene in the database.
#[tauri::command]
pub async fn list_all_scenes(
    state: State<'_, OllamaState>,
) -> Result<Vec<Scene>, String> {
    let rows = sqlx::query("SELECT * FROM scenes ORDER BY created_at ASC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_scene).collect())
}

// ============================================================================
// STORY <-> SCENE JUNCTION
// ============================================================================

/// Link a scene to a story (INSERT OR IGNORE — safe to call repeatedly).
#[tauri::command]
pub async fn link_scene_to_story(
    scene_id: i64,
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO story_scenes (story_id, scene_id) VALUES (?, ?)"
    )
    .bind(story_id)
    .bind(scene_id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove the link between a scene and a story. The scene itself is not deleted.
#[tauri::command]
pub async fn unlink_scene_from_story(
    scene_id: i64,
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "DELETE FROM story_scenes WHERE story_id=? AND scene_id=?"
    )
    .bind(story_id)
    .bind(scene_id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// SCENE <-> CHARACTER JUNCTION
// ============================================================================

/// Add a character to a scene (INSERT OR IGNORE).
#[tauri::command]
pub async fn add_character_to_scene(
    scene_id: i64,
    character_id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO scene_characters (scene_id, character_id) VALUES (?, ?)"
    )
    .bind(scene_id)
    .bind(character_id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove a character from a scene.
#[tauri::command]
pub async fn remove_character_from_scene(
    scene_id: i64,
    character_id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "DELETE FROM scene_characters WHERE scene_id=? AND character_id=?"
    )
    .bind(scene_id)
    .bind(character_id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Return all characters pinned to a scene.
#[tauri::command]
pub async fn get_scene_characters(
    scene_id: i64,
    state: State<'_, OllamaState>,
) -> Result<Vec<CharacterProfile>, String> {
    let rows = sqlx::query(
        "SELECT c.* FROM characters c
         JOIN scene_characters sc ON sc.character_id = c.id
         WHERE sc.scene_id = ?
         ORDER BY c.name ASC"
    )
    .bind(scene_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_profile).collect())
}

/// Return a scene together with its pinned characters.
#[tauri::command]
pub async fn get_scene_with_characters(
    scene_id: i64,
    state: State<'_, OllamaState>,
) -> Result<SceneWithCharacters, String> {
    let row = sqlx::query("SELECT * FROM scenes WHERE id=?")
        .bind(scene_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let scene = row_to_scene(&row);

    let char_rows = sqlx::query(
        "SELECT c.* FROM characters c
         JOIN scene_characters sc ON sc.character_id = c.id
         WHERE sc.scene_id = ?
         ORDER BY c.name ASC"
    )
    .bind(scene_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(SceneWithCharacters {
        scene,
        characters: char_rows.iter().map(row_to_profile).collect(),
    })
}

// ============================================================================
// ACTIVE SCENE
// ============================================================================

/// Set (or clear) the active scene for a story.
/// Pass `scene_id = None` to clear the active scene.
#[tauri::command]
pub async fn set_active_scene(
    story_id: i64,
    scene_id: Option<i64>,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query("UPDATE story_premises SET active_scene_id=? WHERE id=?")
        .bind(scene_id)
        .bind(story_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Return the currently active scene for a story, or `None` if none is set.
#[tauri::command]
pub async fn get_active_scene(
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<Option<Scene>, String> {
    let row = sqlx::query(
        "SELECT s.* FROM scenes s
         JOIN story_premises sp ON sp.active_scene_id = s.id
         WHERE sp.id = ?"
    )
    .bind(story_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.as_ref().map(row_to_scene))
}

// ============================================================================
// CONVENIENCE COMMAND
// ============================================================================

/// Create a scene, link it to a story, and set it as the active scene — all in
/// one call. Returns the new scene id.
#[tauri::command]
pub async fn create_scene_from_llm_output(
    name: String,
    story_id: i64,
    description: Option<String>,
    location: Option<String>,
    location_type: Option<String>,
    time_of_day: Option<String>,
    mood: Option<String>,
    state: State<'_, OllamaState>,
) -> Result<i64, String> {
    let result = sqlx::query(
        "INSERT INTO scenes (name, description, location, location_type, time_of_day, mood)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&name)
    .bind(&description)
    .bind(&location)
    .bind(&location_type)
    .bind(&time_of_day)
    .bind(&mood)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let scene_id = result.last_insert_rowid();

    sqlx::query("INSERT OR IGNORE INTO story_scenes (story_id, scene_id) VALUES (?, ?)")
        .bind(story_id)
        .bind(scene_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("UPDATE story_premises SET active_scene_id=? WHERE id=?")
        .bind(scene_id)
        .bind(story_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(scene_id)
}

// ============================================================================
// SCENE HINT (for LLM context injection on next turn)
// ============================================================================

/// Queue a scene-change hint to be injected into the NEXT story turn's context.
///
/// Called when the user manually selects a scene in the ScenePanel. The hint is
/// stored in SceneHintState (keyed by story_id) and consumed (removed) by
/// the next `process_story_turn` call — making it one-shot.
///
/// If the user switches scenes multiple times before sending a message, only the
/// last call wins (HashMap insert overwrites).
#[tauri::command]
pub async fn set_scene_hint(
    story_id: i64,
    scene_id: i64,
    state: State<'_, OllamaState>,
    hint_state: State<'_, SceneHintState>,
) -> Result<(), String> {
    // Load scene details
    let scene_row = sqlx::query(
        "SELECT name, location, time_of_day, mood FROM scenes WHERE id = ?"
    )
    .bind(scene_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Scene not found".to_string())?;

    let name: String = scene_row.get("name");
    let location: Option<String> = scene_row.get("location");
    let time: Option<String> = scene_row.get("time_of_day");
    let mood: Option<String> = scene_row.get("mood");

    // Load characters assigned to this scene
    let char_rows = sqlx::query(
        "SELECT c.name FROM characters c
         INNER JOIN scene_characters sc ON sc.character_id = c.id
         WHERE sc.scene_id = ?"
    )
    .bind(scene_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let char_names: Vec<String> = char_rows.iter().map(|r| r.get("name")).collect();

    let hint = format!(
        "[SCENE CHANGE — The story is transitioning to: {}{}{}\nCharacters present: {}.\nWrite the narrative transition naturally.]",
        location.as_deref().unwrap_or(&name),
        time.map(|t| format!(", Time: {}", t)).unwrap_or_default(),
        mood.map(|m| format!(", Mood: {}", m)).unwrap_or_default(),
        if char_names.is_empty() {
            "none specified".to_string()
        } else {
            char_names.join(", ")
        },
    );

    let mut hints = hint_state.0.lock().map_err(|e| e.to_string())?;
    hints.insert(story_id, hint);
    println!("[SceneHint] Queued hint for story_id={}: location={:?}", story_id, location);

    Ok(())
}
