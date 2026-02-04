use tauri::State;
use crate::state::OllamaState;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct StoryPremise {
    pub id: Option<i64>,
    pub title: String,
    pub description: String,
}

#[tauri::command]
pub async fn save_story_premise(
    title: String, 
    description: String, 
    state: State<'_, OllamaState>
) -> Result<i64, String> {
    // We check if the table exists (though handled in state.rs) and insert the new premise
    let result = sqlx::query!(
        "INSERT INTO story_premises (title, description) VALUES (?, ?)",
        title,
        description
    )
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn get_story_list(state: State<'_, OllamaState>) -> Result<Vec<StoryPremise>, String> {
    let stories = sqlx::query_as!(
        StoryPremise,
        "SELECT id, title, description FROM story_premises ORDER BY title ASC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(stories)
}

#[tauri::command]
pub async fn delete_stories(ids: Vec<i64>, state: State<'_, OllamaState>) -> Result<(), String> {
    for id in ids {
        sqlx::query!("DELETE FROM story_premises WHERE id = ?", id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}