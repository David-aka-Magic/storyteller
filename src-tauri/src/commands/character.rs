// src-tauri/src/commands/character.rs
//
// Character Database Commands for StoryEngine
// Provides CRUD operations and exact name matching for LLM integration

use tauri::State;
use crate::state::OllamaState;
use crate::models::{CharacterProfile, CharacterLookup, SceneCharacter};
use sqlx::Row;

// ============================================================================
// CORE CRUD COMMANDS
// ============================================================================

/// Add a new character to the database
#[tauri::command]
pub async fn add_character(
    character: CharacterProfile,
    state: State<'_, OllamaState>,
) -> Result<i64, String> {
    let art_style = character.art_style.clone().unwrap_or_else(|| "Realistic".to_string());
    
    let result = sqlx::query(
        r#"
        INSERT INTO characters (
            story_id, name, age, gender, skin_tone, hair_style, hair_color,
            body_type, personality, additional_notes, default_clothing,
            sd_prompt, image, master_image_path, seed, art_style
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&character.story_id)
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
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to add character: {}", e))?;

    Ok(result.last_insert_rowid())
}

/// Get a character by exact name match (for LLM integration)
/// This is the key function for matching "name": "David" from your Ollama output
#[tauri::command]
pub async fn get_character_by_name(
    name: String,
    story_id: Option<i64>,
    state: State<'_, OllamaState>,
) -> Result<Option<CharacterProfile>, String> {
    let row = if let Some(sid) = story_id {
        // Look for character in specific story first
        sqlx::query(
            r#"
            SELECT id, story_id, name, age, gender, skin_tone, hair_style, 
                   hair_color, body_type, personality, additional_notes, 
                   default_clothing, sd_prompt, image, master_image_path, seed, art_style
            FROM characters 
            WHERE name = ? AND story_id = ?
            "#
        )
        .bind(&name)
        .bind(sid)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
    } else {
        // Global search (returns first match)
        sqlx::query(
            r#"
            SELECT id, story_id, name, age, gender, skin_tone, hair_style, 
                   hair_color, body_type, personality, additional_notes, 
                   default_clothing, sd_prompt, image, master_image_path, seed, art_style
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

    Ok(row.map(|r| CharacterProfile {
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
    }))
}

/// Get a character by ID
#[tauri::command]
pub async fn get_character_by_id(
    id: i64,
    state: State<'_, OllamaState>,
) -> Result<Option<CharacterProfile>, String> {
    let row = sqlx::query(
        r#"
        SELECT id, story_id, name, age, gender, skin_tone, hair_style, 
               hair_color, body_type, personality, additional_notes, 
               default_clothing, sd_prompt, image, master_image_path, seed, art_style
        FROM characters 
        WHERE id = ?
        "#
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|r| CharacterProfile {
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
    }))
}

/// Update an existing character
#[tauri::command]
pub async fn update_character(
    character: CharacterProfile,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    let art_style = character.art_style.clone().unwrap_or_else(|| "Realistic".to_string());

    sqlx::query(
        r#"
        UPDATE characters SET
            story_id = ?,
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
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#
    )
    .bind(&character.story_id)
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
    .bind(&character.id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to update character: {}", e))?;

    Ok(())
}

/// Delete a character by ID
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

/// List all characters for a specific story (or all if story_id is None)
#[tauri::command]
pub async fn list_characters_for_story(
    story_id: Option<i64>,
    state: State<'_, OllamaState>,
) -> Result<Vec<CharacterProfile>, String> {
    let rows = if let Some(sid) = story_id {
        sqlx::query(
            r#"
            SELECT id, story_id, name, age, gender, skin_tone, hair_style, 
                   hair_color, body_type, personality, additional_notes, 
                   default_clothing, sd_prompt, image, master_image_path, seed, art_style
            FROM characters 
            WHERE story_id = ?
            ORDER BY name ASC
            "#
        )
        .bind(sid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query(
            r#"
            SELECT id, story_id, name, age, gender, skin_tone, hair_style, 
                   hair_color, body_type, personality, additional_notes, 
                   default_clothing, sd_prompt, image, master_image_path, seed, art_style
            FROM characters 
            ORDER BY name ASC
            "#
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    };

    let characters = rows.iter().map(|r| CharacterProfile {
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
    }).collect();

    Ok(characters)
}

// ============================================================================
// LLM INTEGRATION COMMANDS
// ============================================================================

/// Batch lookup characters by names (for processing LLM scene output)
/// Takes the characters_in_scene array from your Ollama model and returns
/// matched characters with their master image paths for IP-Adapter
#[tauri::command]
pub async fn lookup_scene_characters(
    scene_characters: Vec<SceneCharacter>,
    story_id: Option<i64>,
    state: State<'_, OllamaState>,
) -> Result<Vec<(SceneCharacter, Option<CharacterLookup>)>, String> {
    let mut results = Vec::new();
    
    for scene_char in scene_characters {
        let row = if let Some(sid) = story_id {
            sqlx::query(
                r#"
                SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style
                FROM characters 
                WHERE name = ? AND story_id = ?
                "#
            )
            .bind(&scene_char.name)
            .bind(sid)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx::query(
                r#"
                SELECT id, name, master_image_path, sd_prompt, default_clothing, art_style
                FROM characters 
                WHERE name = ?
                LIMIT 1
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
        });
        
        results.push((scene_char, lookup));
    }
    
    Ok(results)
}

/// Update the master reference image path for a character
/// Call this after generating/selecting a reference image for IP-Adapter
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

/// Search characters by partial name match (for autocomplete/search UI)
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
            SELECT id, story_id, name, age, gender, skin_tone, hair_style, 
                   hair_color, body_type, personality, additional_notes, 
                   default_clothing, sd_prompt, image, master_image_path, seed, art_style
            FROM characters 
            WHERE name LIKE ? AND story_id = ?
            ORDER BY name ASC
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
                   default_clothing, sd_prompt, image, master_image_path, seed, art_style
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

    let characters = rows.iter().map(|r| CharacterProfile {
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
    }).collect();

    Ok(characters)
}

#[tauri::command]
pub async fn link_character_to_story(
    character_id: i64,
    story_id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE characters SET story_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(story_id)
    .bind(character_id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to link character to story: {}", e))?;

    Ok(())
}