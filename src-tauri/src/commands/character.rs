// src-tauri/src/commands/character.rs
//
// Character Database Commands for StoryEngine
// Provides CRUD operations and exact name matching for LLM integration.
//
// Characters use a many-to-many relationship with stories via the
// `story_characters` junction table. A character can belong to multiple
// stories and survives story deletion (only junction rows are removed).

use tauri::State;
use crate::state::OllamaState;
use crate::models::{CharacterProfile, CharacterLookup, SceneCharacter};
use sqlx::Row;

// ============================================================================
// SHARED HELPER: map a DB row to CharacterProfile
// ============================================================================

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
// CORE CRUD COMMANDS
// ============================================================================

/// Add a new character to the database.
/// The character is not linked to any story; call `add_character_to_story`
/// afterward to associate it with one or more stories.
#[tauri::command]
pub async fn add_character(
    character: CharacterProfile,
    state: State<'_, OllamaState>,
) -> Result<i64, String> {
    let art_style = character.art_style.clone().unwrap_or_else(|| "Realistic".to_string());

    let result = sqlx::query(
        r#"
        INSERT INTO characters (
            name, age, gender, skin_tone, hair_style, hair_color,
            body_type, personality, additional_notes, default_clothing,
            sd_prompt, image, master_image_path, seed, art_style,
            eye_color, height_scale, weight_scale, content_rating
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&character.name)
    .bind(&character.age)
    .bind(&character.gender)
    .bind(&character.skin_tone)
    .bind(&character.hair_style)
    .bind(&character.hair_color)
    .bind(&character.body_type)
    .bind(&character.personality)
    .bind(&character.additional_notes)
    .bind(&character.default_clothing)
    .bind(&character.sd_prompt)
    .bind(&character.image)
    .bind(&character.master_image_path)
    .bind(&character.seed)
    .bind(&art_style)
    .bind(&character.eye_color)
    .bind(&character.height_scale)
    .bind(&character.weight_scale)
    .bind(character.content_rating.clone().unwrap_or_else(|| "sfw".to_string()))
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to add character: {}", e))?;

    Ok(result.last_insert_rowid())
}

/// Get a character by exact name match (for LLM integration).
/// When story_id is provided, restricts to characters linked to that story.
#[tauri::command]
pub async fn get_character_by_name(
    name: String,
    story_id: Option<i64>,
    state: State<'_, OllamaState>,
) -> Result<Option<CharacterProfile>, String> {
    let row = if let Some(sid) = story_id {
        sqlx::query(
            r#"
            SELECT c.id, c.story_id, c.name, c.age, c.gender, c.skin_tone, c.hair_style,
                   c.hair_color, c.body_type, c.personality, c.additional_notes,
                   c.default_clothing, c.sd_prompt, c.image, c.master_image_path, c.seed, c.art_style,
                   c.eye_color, c.height_scale, c.weight_scale, c.content_rating
            FROM characters c
            INNER JOIN story_characters sc ON sc.character_id = c.id
            WHERE c.name = ? AND sc.story_id = ?
            LIMIT 1
            "#
        )
        .bind(&name)
        .bind(sid)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query(
            r#"
            SELECT id, story_id, name, age, gender, skin_tone, hair_style,
                   hair_color, body_type, personality, additional_notes,
                   default_clothing, sd_prompt, image, master_image_path, seed, art_style,
                   eye_color, height_scale, weight_scale, content_rating
            FROM characters
            WHERE name = ?
            LIMIT 1
            "#
        )
        .bind(&name)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(row.as_ref().map(row_to_profile))
}

/// Get a character by ID.
#[tauri::command]
pub async fn get_character_by_id(
    id: i64,
    state: State<'_, OllamaState>,
) -> Result<Option<CharacterProfile>, String> {
    let row = sqlx::query(
        r#"
        SELECT id, story_id, name, age, gender, skin_tone, hair_style,
               hair_color, body_type, personality, additional_notes,
               default_clothing, sd_prompt, image, master_image_path, seed, art_style,
               eye_color, height_scale, weight_scale, content_rating
        FROM characters
        WHERE id = ?
        "#
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.as_ref().map(row_to_profile))
}

/// Update an existing character's fields (does not affect story membership).
#[tauri::command]
pub async fn update_character(
    character: CharacterProfile,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    let art_style = character.art_style.clone().unwrap_or_else(|| "Realistic".to_string());

    sqlx::query(
        r#"
        UPDATE characters SET
            name = ?,
            age = ?,
            gender = ?,
            skin_tone = ?,
            hair_style = ?,
            hair_color = ?,
            body_type = ?,
            personality = ?,
            additional_notes = ?,
            default_clothing = ?,
            sd_prompt = ?,
            image = ?,
            master_image_path = ?,
            seed = ?,
            art_style = ?,
            eye_color = ?,
            height_scale = ?,
            weight_scale = ?,
            content_rating = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#
    )
    .bind(&character.name)
    .bind(&character.age)
    .bind(&character.gender)
    .bind(&character.skin_tone)
    .bind(&character.hair_style)
    .bind(&character.hair_color)
    .bind(&character.body_type)
    .bind(&character.personality)
    .bind(&character.additional_notes)
    .bind(&character.default_clothing)
    .bind(&character.sd_prompt)
    .bind(&character.image)
    .bind(&character.master_image_path)
    .bind(&character.seed)
    .bind(&art_style)
    .bind(&character.eye_color)
    .bind(&character.height_scale)
    .bind(&character.weight_scale)
    .bind(character.content_rating.clone().unwrap_or_else(|| "sfw".to_string()))
    .bind(&character.id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to update character: {}", e))?;

    Ok(())
}

/// Delete a character by ID (removes the character and all its story links).
#[tauri::command]
pub async fn delete_character_by_id(
    id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query("DELETE FROM characters WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Failed to delete character: {}", e))?;

    Ok(())
}

/// Alias for delete_character_by_id — kept for frontend callers using the shorter name.
#[tauri::command]
pub async fn delete_character(
    id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    delete_character_by_id(id, state).await
}

/// List characters linked to a specific story via the junction table.
/// If story_id is None, returns ALL characters in the database.
/// If content_rating_filter is Some("sfw"), only SFW characters are returned.
#[tauri::command]
pub async fn list_characters_for_story(
    story_id: Option<i64>,
    content_rating_filter: Option<String>,
    state: State<'_, OllamaState>,
) -> Result<Vec<CharacterProfile>, String> {
    use sqlx::QueryBuilder;
    use sqlx::Sqlite;

    let only_sfw = content_rating_filter.as_deref() == Some("sfw");

    let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT c.id, c.story_id, c.name, c.age, c.gender, c.skin_tone, c.hair_style, \
         c.hair_color, c.body_type, c.personality, c.additional_notes, \
         c.default_clothing, c.sd_prompt, c.image, c.master_image_path, c.seed, c.art_style, \
         c.eye_color, c.height_scale, c.weight_scale, c.content_rating \
         FROM characters c"
    );

    if let Some(sid) = story_id {
        builder.push(" INNER JOIN story_characters sc ON sc.character_id = c.id WHERE sc.story_id = ");
        builder.push_bind(sid);
        if only_sfw {
            builder.push(" AND c.content_rating = 'sfw'");
        }
    } else if only_sfw {
        builder.push(" WHERE c.content_rating = 'sfw'");
    }

    builder.push(" ORDER BY c.name ASC");

    let rows = builder.build()
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_profile).collect())
}

/// List ALL characters in the database (not filtered by story).
/// If content_rating_filter is Some("sfw"), only SFW characters are returned.
/// Used by the "Add Existing Character" picker.
#[tauri::command]
pub async fn list_all_characters(
    content_rating_filter: Option<String>,
    state: State<'_, OllamaState>,
) -> Result<Vec<CharacterProfile>, String> {
    let rating_clause = match content_rating_filter.as_deref() {
        Some("sfw") => " WHERE content_rating = 'sfw'",
        _ => "",
    };
    let sql = format!(
        "SELECT id, story_id, name, age, gender, skin_tone, hair_style, \
         hair_color, body_type, personality, additional_notes, \
         default_clothing, sd_prompt, image, master_image_path, seed, art_style, \
         eye_color, height_scale, weight_scale, content_rating \
         FROM characters{} ORDER BY name ASC",
        rating_clause
    );
    let rows = sqlx::query(&sql)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_profile).collect())
}

// ============================================================================
// STORY MEMBERSHIP COMMANDS
// ============================================================================

/// Add a character to a story via the junction table.
/// Safe to call multiple times (INSERT OR IGNORE).
#[tauri::command]
pub async fn add_character_to_story(
    character_id: i64,
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO story_characters (story_id, character_id) VALUES (?, ?)"
    )
    .bind(story_id)
    .bind(character_id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to add character to story: {}", e))?;

    Ok(())
}

/// Remove a character from a story (deletes the junction row only).
/// The character itself is NOT deleted.
#[tauri::command]
pub async fn remove_character_from_story(
    character_id: i64,
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "DELETE FROM story_characters WHERE story_id = ? AND character_id = ?"
    )
    .bind(story_id)
    .bind(character_id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to remove character from story: {}", e))?;

    Ok(())
}

/// Link a character to a story (alias for add_character_to_story).
/// Kept for backward compatibility with older frontend code.
#[tauri::command]
pub async fn link_character_to_story(
    character_id: i64,
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    add_character_to_story(character_id, story_id, state).await
}

// ============================================================================
// LLM INTEGRATION COMMANDS
// ============================================================================

/// Batch lookup characters by names (for processing LLM scene output).
/// When story_id is provided, restricts search to characters in that story,
/// with a global fallback if a name isn't found in the story scope.
#[tauri::command]
pub async fn lookup_scene_characters(
    scene_characters: Vec<SceneCharacter>,
    story_id: Option<i64>,
    state: State<'_, OllamaState>,
) -> Result<Vec<(SceneCharacter, Option<CharacterLookup>)>, String> {
    let mut results = Vec::new();

    for scene_char in scene_characters {
        let row = if let Some(sid) = story_id {
            // Story-scoped lookup via junction table
            let r = sqlx::query(
                r#"
                SELECT c.id, c.name, c.master_image_path, c.sd_prompt, c.default_clothing, c.art_style, c.gender
                FROM characters c
                INNER JOIN story_characters sc ON sc.character_id = c.id
                WHERE c.name = ? AND sc.story_id = ?
                LIMIT 1
                "#
            )
            .bind(&scene_char.name)
            .bind(sid)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?;

            if r.is_none() {
                // No global fallback when story-scoped — avoids same-name collisions
                None
            } else {
                r
            }
        } else {
            sqlx::query(
                r#"
                SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style, gender
                FROM characters WHERE name = ? LIMIT 1
                "#
            )
            .bind(&scene_char.name)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?
        };

        let lookup = row.map(|r| CharacterLookup {
            id: r.get("id"),
            name: r.get("name"),
            master_image_path: r.get("master_image_path"),
            sd_prompt: r.get("sd_prompt"),
            default_clothing: r.get("default_clothing"),
            art_style: r.get("art_style"),
            gender: r.get("gender"),
        });

        results.push((scene_char, lookup));
    }

    Ok(results)
}

/// Update the master reference image path for a character.
#[tauri::command]
pub async fn set_character_master_image(
    id: i64,
    image_path: String,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE characters SET master_image_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(&image_path)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to update master image: {}", e))?;

    Ok(())
}

/// List characters filtered by art_style, excluding already-in-scene IDs.
/// If art_style is None, returns all characters (no style filter).
/// exclude_ids prevents returning characters already present in the scene.
#[tauri::command]
pub async fn list_characters_by_art_style(
    art_style: Option<String>,
    exclude_ids: Vec<i64>,
    state: State<'_, OllamaState>,
) -> Result<Vec<CharacterProfile>, String> {
    use sqlx::QueryBuilder;
    use sqlx::Sqlite;

    let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT id, story_id, name, age, gender, skin_tone, hair_style, \
         hair_color, body_type, personality, additional_notes, \
         default_clothing, sd_prompt, image, master_image_path, seed, art_style, \
         eye_color, height_scale, weight_scale, content_rating \
         FROM characters WHERE 1=1"
    );

    if let Some(ref style) = art_style {
        builder.push(" AND art_style = ");
        builder.push_bind(style.clone());
    }

    if !exclude_ids.is_empty() {
        builder.push(" AND id NOT IN (");
        let mut sep = builder.separated(", ");
        for id in &exclude_ids {
            sep.push_bind(*id);
        }
        builder.push(")");
    }

    builder.push(" ORDER BY name ASC");

    let rows = builder.build()
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_profile).collect())
}

/// Search characters by partial name match (for autocomplete/search UI).
/// When story_id is provided, restricts to characters in that story.
#[tauri::command]
pub async fn search_characters(
    query: String,
    story_id: Option<i64>,
    limit: Option<i32>,
    state: State<'_, OllamaState>,
) -> Result<Vec<CharacterProfile>, String> {
    let search_pattern = format!("%{}%", query);
    let max_results = limit.unwrap_or(10);

    let rows = if let Some(sid) = story_id {
        sqlx::query(
            r#"
            SELECT c.id, c.story_id, c.name, c.age, c.gender, c.skin_tone, c.hair_style,
                   c.hair_color, c.body_type, c.personality, c.additional_notes,
                   c.default_clothing, c.sd_prompt, c.image, c.master_image_path, c.seed, c.art_style,
                   c.eye_color, c.height_scale, c.weight_scale, c.content_rating
            FROM characters c
            INNER JOIN story_characters sc ON sc.character_id = c.id
            WHERE c.name LIKE ? AND sc.story_id = ?
            ORDER BY c.name ASC
            LIMIT ?
            "#
        )
        .bind(&search_pattern)
        .bind(sid)
        .bind(max_results)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query(
            r#"
            SELECT id, story_id, name, age, gender, skin_tone, hair_style,
                   hair_color, body_type, personality, additional_notes,
                   default_clothing, sd_prompt, image, master_image_path, seed, art_style,
                   eye_color, height_scale, weight_scale, content_rating
            FROM characters
            WHERE name LIKE ?
            ORDER BY name ASC
            LIMIT ?
            "#
        )
        .bind(&search_pattern)
        .bind(max_results)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(rows.iter().map(row_to_profile).collect())
}
