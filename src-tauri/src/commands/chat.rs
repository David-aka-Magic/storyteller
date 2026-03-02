use tauri::State;
use crate::state::OllamaState;
use crate::models::{Message, ChatResponse};
use sqlx::Row;

#[tauri::command]
pub async fn get_chat_list(state: State<'_, OllamaState>) -> Result<Vec<ChatResponse>, String> {
    let rows = sqlx::query("SELECT id, title FROM chats ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|row| ChatResponse {
            id: row.get("id"),
            title: row.get("title"),
        })
        .collect())
}

#[tauri::command]
pub async fn new_chat(state: State<'_, OllamaState>) -> Result<i64, String> {
    let result = sqlx::query("INSERT INTO chats (title) VALUES ('New Chat')")
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let new_id = result.last_insert_rowid();
    *state.current_chat_id.lock().unwrap() = Some(new_id);
    Ok(new_id)
}

#[tauri::command]
pub async fn load_chat(id: i64, state: State<'_, OllamaState>) -> Result<Vec<Message>, String> {
    let rows = sqlx::query(
        "SELECT m.id as message_id, m.role, m.content, i.file_path as image_path \
         FROM messages m \
         LEFT JOIN images i ON i.message_id = m.id \
         WHERE m.chat_id = ? \
         ORDER BY m.timestamp ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let messages: Vec<Message> = rows
        .iter()
        .map(|row| {
            let image_path: Option<String> = row.get("image_path");
            Message {
                role: row.get("role"),
                content: row.get("content"),
                images: image_path.map(|p| vec![p]),
                db_id: Some(row.get::<i64, _>("message_id")),
            }
        })
        .collect();

    *state.current_chat_id.lock().unwrap() = Some(id);
    Ok(messages)
}

#[tauri::command]
pub async fn delete_chats(ids: Vec<i64>, state: State<'_, OllamaState>) -> Result<(), String> {
    for id in ids {
        sqlx::query("DELETE FROM images WHERE chat_id = ?")
            .bind(id)
            .execute(&state.db)
            .await
            .ok();
        sqlx::query("DELETE FROM messages WHERE chat_id = ?")
            .bind(id)
            .execute(&state.db)
            .await
            .ok();
        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(id)
            .execute(&state.db)
            .await
            .ok();
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_history(id: i64, state: State<'_, OllamaState>) -> Result<(), String> {
    sqlx::query("DELETE FROM images WHERE chat_id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .ok();
    sqlx::query("DELETE FROM messages WHERE chat_id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .ok();
    sqlx::query("UPDATE chats SET title = 'New Chat' WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .ok();
    Ok(())
}

/// Save (or replace) the generated image for a specific assistant message.
/// Deletes any existing image record for the message first, then inserts the new one.
/// Used by "Illustrate Scene" and "Redraw Image" in the story view.
#[tauri::command]
pub async fn save_image_for_message(
    message_id: i64,
    chat_id: i64,
    file_path: String,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    // Remove any existing image for this message (handles the "redraw" case)
    sqlx::query("DELETE FROM images WHERE message_id = ?")
        .bind(message_id)
        .execute(&state.db)
        .await
        .ok();

    sqlx::query(
        "INSERT INTO images (message_id, chat_id, file_path) VALUES (?, ?, ?)",
    )
    .bind(message_id)
    .bind(chat_id)
    .bind(&file_path)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to save image record: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn set_chat_character(
    chat_id: i64,
    character_id: Option<i64>,
    state: State<'_, OllamaState>,
) -> Result<(), String> {
    sqlx::query("UPDATE chats SET character_id = ? WHERE id = ?")
        .bind(character_id)
        .bind(chat_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
