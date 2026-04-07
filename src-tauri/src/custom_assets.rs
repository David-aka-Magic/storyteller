use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use crate::config::ConfigState;
use crate::state::OllamaState;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCheckpoint {
    pub id: i64,
    pub display_name: String,
    pub filename: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPose {
    pub id: i64,
    pub display_name: String,
    pub filename: String,
    pub created_at: String,
}

// ── Helpers ──

fn checkpoints_dir(config_state: &State<'_, ConfigState>) -> PathBuf {
    let cfg = config_state.0.lock().unwrap();
    PathBuf::from(&cfg.comfyui_path)
        .join("models")
        .join("checkpoints")
}

fn pose_skeletons_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join("pose_skeletons")
}

// ── Checkpoint commands ──

#[tauri::command]
pub async fn scan_available_checkpoints(
    config_state: State<'_, ConfigState>,
) -> Result<Vec<String>, String> {
    let dir = checkpoints_dir(&config_state);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let built_in = [
        "juggernautXL_ragnarokBy.safetensors",
        "animagine-xl-3.1.safetensors",
    ];

    let mut found: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if (name.ends_with(".safetensors") || name.ends_with(".ckpt"))
            && !built_in.contains(&name.as_str())
        {
            found.push(name);
        }
    }
    found.sort();
    Ok(found)
}

#[tauri::command]
pub async fn list_custom_checkpoints(
    state: State<'_, OllamaState>,
) -> Result<Vec<CustomCheckpoint>, String> {
    let rows = sqlx::query(
        "SELECT id, display_name, filename, created_at FROM custom_checkpoints ORDER BY display_name"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let result = rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            CustomCheckpoint {
                id: row.get("id"),
                display_name: row.get("display_name"),
                filename: row.get("filename"),
                created_at: row.get::<Option<String>, _>("created_at").unwrap_or_default(),
            }
        })
        .collect();
    Ok(result)
}

#[tauri::command]
pub async fn add_custom_checkpoint(
    display_name: String,
    filename: String,
    state: State<'_, OllamaState>,
) -> Result<CustomCheckpoint, String> {
    let result = sqlx::query(
        "INSERT INTO custom_checkpoints (display_name, filename) VALUES (?, ?)"
    )
    .bind(&display_name)
    .bind(&filename)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();
    let row = sqlx::query("SELECT created_at FROM custom_checkpoints WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    use sqlx::Row;
    let created_at: String = row.get::<Option<String>, _>("created_at").unwrap_or_default();

    Ok(CustomCheckpoint { id, display_name, filename, created_at })
}

#[tauri::command]
pub async fn delete_custom_checkpoint(
    id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query("DELETE FROM custom_checkpoints WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Pose commands ──

#[tauri::command]
pub async fn scan_available_poses(app: AppHandle) -> Result<Vec<String>, String> {
    let dir = pose_skeletons_dir(&app);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let built_in = [
        "standing.png", "sitting.png", "lying_down.png", "running.png",
        "kneeling.png", "leaning.png", "driving.png", "cooking.png",
        "fighting.png", "walking.png",
    ];

    let mut found: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".png") && !built_in.contains(&name.as_str()) {
            found.push(name);
        }
    }
    found.sort();
    Ok(found)
}

#[tauri::command]
pub async fn list_custom_poses(
    state: State<'_, OllamaState>,
) -> Result<Vec<CustomPose>, String> {
    let rows = sqlx::query(
        "SELECT id, display_name, filename, created_at FROM custom_poses ORDER BY display_name"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let result = rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            CustomPose {
                id: row.get("id"),
                display_name: row.get("display_name"),
                filename: row.get("filename"),
                created_at: row.get::<Option<String>, _>("created_at").unwrap_or_default(),
            }
        })
        .collect();
    Ok(result)
}

#[tauri::command]
pub async fn add_custom_pose(
    display_name: String,
    filename: String,
    state: State<'_, OllamaState>,
) -> Result<CustomPose, String> {
    let result = sqlx::query(
        "INSERT INTO custom_poses (display_name, filename) VALUES (?, ?)"
    )
    .bind(&display_name)
    .bind(&filename)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();
    let row = sqlx::query("SELECT created_at FROM custom_poses WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    use sqlx::Row;
    let created_at: String = row.get::<Option<String>, _>("created_at").unwrap_or_default();

    Ok(CustomPose { id, display_name, filename, created_at })
}

#[tauri::command]
pub async fn delete_custom_pose(
    id: i64,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query("DELETE FROM custom_poses WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn import_pose_file(
    source_path: String,
    target_filename: String,
    app: AppHandle,
) -> Result<String, String> {
    let dir = pose_skeletons_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(&target_filename);
    std::fs::copy(&source_path, &dest)
        .map_err(|e| format!("Failed to copy pose file: {}", e))?;
    Ok(target_filename)
}

#[tauri::command]
pub async fn import_checkpoint_file(
    source_path: String,
    target_filename: String,
    config_state: State<'_, ConfigState>,
) -> Result<String, String> {
    let dir = checkpoints_dir(&config_state);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(&target_filename);
    std::fs::copy(&source_path, &dest)
        .map_err(|e| format!("Failed to copy checkpoint file: {}", e))?;
    Ok(target_filename)
}
