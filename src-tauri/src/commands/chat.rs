use tauri::State;
use crate::state::OllamaState;
use crate::models::{Message, ChatResponse, SdJson, StoryResponse}; 
use serde_json::{json, Value};

// --- Internal Helper Functions ---

/// Safely extracts the first valid JSON object from a string.
/// This is crucial for Phase 2 where the AI might wrap JSON in conversational text.
fn extract_json_from_text(text: &str) -> Option<String> {
    if let Some(start) = text.find('{') {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape = false;
        for (i, ch) in text[start..].char_indices() {
            match ch {
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(text[start..start + i + 1].to_string());
                    }
                },
                '"' if !escape => in_string = !in_string,
                '\\' => escape = !escape,
                _ => escape = false,
            }
        }
    }
    None
}

// --- Chat Management Commands ---

#[tauri::command]
pub async fn get_chat_list(state: State<'_, OllamaState>) -> Result<Vec<ChatResponse>, String> {
    let chats = sqlx::query_as!(
        ChatResponse,
        "SELECT id, title FROM chats ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(chats)
}

#[tauri::command]
pub async fn new_chat(state: State<'_, OllamaState>) -> Result<i64, String> {
    let result = sqlx::query!("INSERT INTO chats (title) VALUES (?)", "New Chat")
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let new_id = result.last_insert_rowid();
    
    let mut current_id = state.current_chat_id.lock().unwrap();
    *current_id = Some(new_id);

    Ok(new_id)
}

#[tauri::command]
pub async fn load_chat(id: i64, state: State<'_, OllamaState>) -> Result<Vec<Message>, String> {
    let messages = sqlx::query_as!(
        Message,
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC",
        id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let mut current_id = state.current_chat_id.lock().unwrap();
    *current_id = Some(id);

    Ok(messages)
}

#[tauri::command]
pub async fn delete_chats(ids: Vec<i64>, state: State<'_, OllamaState>) -> Result<(), String> {
    for id in ids {
        // Delete related images first, then messages, then the chat
        sqlx::query!("DELETE FROM images WHERE chat_id = ?", id).execute(&state.db).await.ok();
        sqlx::query!("DELETE FROM messages WHERE chat_id = ?", id).execute(&state.db).await.ok();
        sqlx::query!("DELETE FROM chats WHERE id = ?", id).execute(&state.db).await.ok();
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_history(state: State<'_, OllamaState>) -> Result<(), String> {
    let current_id = *state.current_chat_id.lock().unwrap();
    if let Some(id) = current_id {
        sqlx::query!("DELETE FROM images WHERE chat_id = ?", id).execute(&state.db).await.ok();
        sqlx::query!("DELETE FROM messages WHERE chat_id = ?", id).execute(&state.db).await.ok();
        sqlx::query!("UPDATE chats SET title = 'New Chat' WHERE id = ?", id).execute(&state.db).await.ok();
    }
    Ok(())
}

// --- Story Generation Commands ---

#[tauri::command]
pub async fn generate_story(
    prompt: String,
    state: State<'_, OllamaState>,
) -> Result<Value, String> {
    let current_id = *state.current_chat_id.lock().unwrap();
    let chat_id = current_id.ok_or("No active chat selected")?;

    // Fetch history for context
    let history = sqlx::query_as!(
        Message,
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC",
        chat_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Build Prompt (Llama 3.1 format)
    let mut full_prompt = String::new();
    for msg in history {
        full_prompt.push_str(&format!("<|start_header_id|>{}<|end_header_id|>\n\n{}<|eot_id|>", msg.role, msg.content));
    }
    full_prompt.push_str(&format!("<|start_header_id|>user<|end_header_id|>\n\n{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n", prompt));

    // Request to Ollama
    let res = state.client.post(format!("{}/api/generate", state.base_url))
        .json(&json!({ "model": "Story_v27", "prompt": full_prompt, "stream": false }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let api_res: Value = res.json().await.map_err(|e| e.to_string())?;
    let raw_ai_text = api_res["response"].as_str().ok_or("Invalid AI response")?;

    let mut ai_history_content = raw_ai_text.to_string();
    let final_ui_response: Value;

    if let Some(json_str) = extract_json_from_text(raw_ai_text) {
        let val: Value = serde_json::from_str(&json_str).unwrap_or(json!({}));
        let story_text = val["story_json"]["response"].as_str().or(val["response"].as_str()).unwrap_or(raw_ai_text);
        ai_history_content = json_str; 

        final_ui_response = json!({
            "story": story_text,
            "sd_prompt": val["sd_json"]["look"],
            "sd_details": val["sd_json"]
        });
    } else {
        final_ui_response = json!({ "text": raw_ai_text, "type": "phase1" });
    }

    // Database updates via Transaction
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    // Auto-update title if it's a new chat
    sqlx::query!(
        "UPDATE chats SET title = ? WHERE id = ? AND title = 'New Chat'",
        prompt.chars().take(30).collect::<String>(),
        chat_id
    ).execute(&mut *tx).await.ok();

    sqlx::query!("INSERT INTO messages (chat_id, role, content) VALUES (?, 'user', ?)", chat_id, prompt).execute(&mut *tx).await.ok();
    let ai_msg = sqlx::query!("INSERT INTO messages (chat_id, role, content) VALUES (?, 'assistant', ?) RETURNING id", chat_id, ai_history_content).fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;

    // Save image metadata if present
    if let Some(sd_prompt) = final_ui_response.get("sd_prompt").and_then(|v| v.as_str()) {
        sqlx::query!("INSERT INTO images (message_id, chat_id, file_path, prompt) VALUES (?, ?, 'pending', ?)", ai_msg.id, chat_id, sd_prompt).execute(&mut *tx).await.ok();
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(final_ui_response)
}

#[tauri::command]
pub async fn regenerate_story(state: State<'_, OllamaState>) -> Result<Value, String> {
    // Logic: Delete the last AI message in the current chat and re-run generate_story
    let current_id = *state.current_chat_id.lock().unwrap();
    if let Some(id) = current_id {
        let last_msg = sqlx::query!("SELECT id, content FROM messages WHERE chat_id = ? AND role = 'user' ORDER BY timestamp DESC LIMIT 1", id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?;
            
        if let Some(msg) = last_msg {
            sqlx::query!("DELETE FROM messages WHERE chat_id = ? AND timestamp > (SELECT timestamp FROM messages WHERE id = ?)", id, msg.id).execute(&state.db).await.ok();
            return generate_story(msg.content, state).await;
        }
    }
    Err("No message to regenerate".into())
}

// --- Character & Settings Commands ---

#[tauri::command]
pub async fn set_chat_character(character_id: i64, state: State<'_, OllamaState>) -> Result<(), String> {
    // This could update a 'character_id' column in the chats table if you add one
    Ok(())
}

#[tauri::command]
pub async fn save_character(name: String, bio: String, state: State<'_, OllamaState>) -> Result<(), String> {
    // Implementation for saving standalone character cards
    Ok(())
}

#[tauri::command] pub async fn delete_character(_id: i64) -> Result<(), String> { Ok(()) }
#[tauri::command] pub async fn get_character_list() -> Result<Vec<Value>, String> { Ok(vec![]) }

// --- Placeholder Image Generation Commands ---
// These will eventually bridge to your SD generator (Automatic1111/ComfyUI)

#[tauri::command] pub async fn generate_image(_prompt: String) -> Result<String, String> { Ok("path/to/img".into()) }
#[tauri::command] pub async fn generate_image_variation(_image_path: String) -> Result<String, String> { Ok("path/to/var".into()) }
#[tauri::command] pub async fn generate_character_portrait(_name: String) -> Result<String, String> { Ok("path/to/portrait".into()) }