use tauri::State;
use crate::state::OllamaState;
use crate::models::StoryPremise;
use sqlx::Row;

#[tauri::command]
pub async fn save_story_premise(
    title: String,
    description: String,
    id: Option<i64>,
    state: State<'_, OllamaState>,
) -> Result<i64, String> {
    if let Some(existing_id) = id {
        // Update existing
        sqlx::query("UPDATE story_premises SET title = ?, description = ? WHERE id = ?")
            .bind(&title)
            .bind(&description)
            .bind(existing_id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(existing_id)
    } else {
        // Insert new
        let result = sqlx::query("INSERT INTO story_premises (title, description) VALUES (?, ?)")
            .bind(&title)
            .bind(&description)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.last_insert_rowid())
    }
}

#[tauri::command]
pub async fn get_story_list(state: State<'_, OllamaState>) -> Result<Vec<StoryPremise>, String> {
    let rows = sqlx::query("SELECT id, title, description FROM story_premises ORDER BY title ASC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let stories: Vec<StoryPremise> = rows
        .iter()
        .map(|row| StoryPremise {
            id: row.get("id"),
            title: row.get("title"),
            description: row.get("description"),
        })
        .collect();

    Ok(stories)
}

#[tauri::command]
pub async fn delete_stories(ids: Vec<i64>, state: State<'_, OllamaState>) -> Result<(), String> {
    for id in ids {
        sqlx::query("DELETE FROM story_premises WHERE id = ?")
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}