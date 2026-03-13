// src-tauri/src/commands/pose_loras.rs
//
// CRUD commands for pose LoRA definitions.

use tauri::State;
use crate::state::OllamaState;
use crate::models::PoseLora;
use sqlx::Row;

fn row_to_pose_lora(r: &sqlx::sqlite::SqliteRow) -> PoseLora {
    let enabled_int: i64 = r.get("enabled");
    PoseLora {
        id: r.get("id"),
        name: r.get("name"),
        keywords: r.get("keywords"),
        lora_filename: r.get("lora_filename"),
        trigger_words: r.get("trigger_words"),
        strength: r.get("strength"),
        enabled: enabled_int != 0,
        created_at: r.get("created_at"),
    }
}

/// Return all pose LoRAs ordered by name.
#[tauri::command]
pub async fn list_pose_loras(state: State<'_, OllamaState>) -> Result<Vec<PoseLora>, String> {
    let rows = sqlx::query(
        "SELECT id, name, keywords, lora_filename, trigger_words, strength, enabled, created_at
         FROM pose_loras ORDER BY name"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_pose_lora).collect())
}

/// Insert a new pose LoRA and return the created row.
#[tauri::command]
pub async fn create_pose_lora(
    name: String,
    keywords: String,
    lora_filename: String,
    trigger_words: String,
    strength: f64,
    state: State<'_, OllamaState>,
) -> Result<PoseLora, String> {
    let result = sqlx::query(
        "INSERT INTO pose_loras (name, keywords, lora_filename, trigger_words, strength)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&name)
    .bind(&keywords)
    .bind(&lora_filename)
    .bind(&trigger_words)
    .bind(strength)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();

    let row = sqlx::query(
        "SELECT id, name, keywords, lora_filename, trigger_words, strength, enabled, created_at
         FROM pose_loras WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row_to_pose_lora(&row))
}

/// Update an existing pose LoRA.
#[tauri::command]
pub async fn update_pose_lora(
    id: i64,
    name: String,
    keywords: String,
    lora_filename: String,
    trigger_words: String,
    strength: f64,
    enabled: bool,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE pose_loras
         SET name = ?, keywords = ?, lora_filename = ?, trigger_words = ?, strength = ?, enabled = ?
         WHERE id = ?"
    )
    .bind(&name)
    .bind(&keywords)
    .bind(&lora_filename)
    .bind(&trigger_words)
    .bind(strength)
    .bind(enabled as i64)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete a pose LoRA by id.
#[tauri::command]
pub async fn delete_pose_lora(id: i64, state: State<'_, OllamaState>) -> Result<(), String> {
    sqlx::query("DELETE FROM pose_loras WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Seed default pose LoRAs if the table is empty.
#[tauri::command]
pub async fn seed_default_pose_loras(state: State<'_, OllamaState>) -> Result<(), String> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pose_loras")
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    if count > 0 {
        return Ok(());
    }

    let defaults: &[(&str, &str, &str, &str, f64)] = &[
        ("Sitting",  "sitting, sat, seated, chair, bench, booth",                           "sitting_pose.safetensors",  "person sitting down",                    0.7),
        ("Lying Down", "lying, laying, bed, sleeping, resting, napping",                    "lying_pose.safetensors",    "person lying down",                      0.7),
        ("Running",  "running, sprinting, rushing, dashing, chasing",                       "running_pose.safetensors",  "person running",                         0.65),
        ("Kneeling", "kneeling, crouching, bending, ducking",                               "kneeling_pose.safetensors", "person kneeling",                        0.7),
        ("Leaning",  "leaning, propped, resting against, slouching",                        "leaning_pose.safetensors",  "person leaning against wall",            0.65),
        ("Driving",  "driving, steering, behind the wheel, car, truck",                     "driving_pose.safetensors",  "person sitting in vehicle driving",       0.7),
        ("Cooking",  "cooking, kitchen, stove, preparing food, chopping",                   "cooking_pose.safetensors",  "person cooking in kitchen",              0.65),
        ("Fighting", "fighting, punching, kicking, combat, sparring",                       "fighting_pose.safetensors", "person in fighting stance",              0.7),
    ];

    for (name, keywords, lora_filename, trigger_words, strength) in defaults {
        sqlx::query(
            "INSERT OR IGNORE INTO pose_loras (name, keywords, lora_filename, trigger_words, strength)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(name)
        .bind(keywords)
        .bind(lora_filename)
        .bind(trigger_words)
        .bind(strength)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}
