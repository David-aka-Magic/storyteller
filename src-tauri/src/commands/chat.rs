use tauri::State;
use crate::state::OllamaState;
use crate::models::{Message, ChatResponse, SdJson, StoryResponse};
use serde_json::{json, Value};
use sqlx::Row;

// --- Internal Helper Functions ---

fn extract_json_from_text(text: &str) -> Option<String> {
    if let Some(start) = text.find('{') {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape = false;
        for (i, ch) in text[start..].char_indices() {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' if in_string => escape = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(text[start..start + i + 1].to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn extract_story(v: &Value) -> String {
    if let Some(s) = v.get("story").and_then(|x| x.as_str()) {
        return s.to_string();
    }
    if let Some(s) = v.get("text").and_then(|x| x.as_str()) {
        return s.to_string();
    }
    v.to_string()
}

// --- Chat Management Commands ---

#[tauri::command]
pub async fn get_chat_list(state: State<'_, OllamaState>) -> Result<Vec<ChatResponse>, String> {
    let rows = sqlx::query("SELECT id, title FROM chats ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let chats: Vec<ChatResponse> = rows
        .iter()
        .map(|row| ChatResponse {
            id: row.get("id"),
            title: row.get("title"),
        })
        .collect();

    Ok(chats)
}

#[tauri::command]
pub async fn new_chat(state: State<'_, OllamaState>) -> Result<i64, String> {
    let result = sqlx::query("INSERT INTO chats (title) VALUES ('New Chat')")
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
    let rows = sqlx::query("SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC")
        .bind(id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let messages: Vec<Message> = rows
        .iter()
        .map(|row| Message {
            role: row.get("role"),
            content: row.get("content"),
            images: None,
        })
        .collect();

    let mut current_id = state.current_chat_id.lock().unwrap();
    *current_id = Some(id);

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

// --- Story Generation ---

#[tauri::command]
pub async fn generate_story(
    prompt: String,
    chat_id: i64,
    state: State<'_, OllamaState>,
) -> Result<Value, String> {
    // Load chat history
    let rows = sqlx::query("SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC")
        .bind(chat_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    // Build context string
    let mut context = String::new();
    for row in &rows {
        let role: String = row.get("role");
        let content: String = row.get("content");
        let r = if role == "user" { "user" } else { "assistant" };
        context.push_str(&format!(
            "<|start_header_id|>{}<|end_header_id|>\n\n{}<|eot_id|>",
            r, content
        ));
    }
    context.push_str(&format!(
        "<|start_header_id|>user<|end_header_id|>\n\n{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n",
        prompt
    ));

    // Call Ollama API
    let res = state
        .client
        .post(format!("{}/api/generate", state.base_url))
        .json(&json!({
            "model": "Story_v27",
            "prompt": context,
            "stream": false
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let api_res: Value = res.json().await.map_err(|e| e.to_string())?;
    let response_text = api_res["response"]
        .as_str()
        .ok_or_else(|| "No response from API".to_string())?;

    // Parse response
    let (history_content, output) = if let Some(json_str) = extract_json_from_text(response_text) {
        let v: Value = serde_json::from_str(&json_str).unwrap_or(json!({"text": response_text}));
        let story = extract_story(&v);
        let sd = v
            .get("sd_json")
            .and_then(|x| serde_json::from_value::<SdJson>(x.clone()).ok());
        let sr = StoryResponse {
            story,
            sd_prompt: sd.as_ref().map(|x| x.look.clone()),
            sd_details: sd,
        };
        (
            json_str,
            serde_json::to_value(sr).map_err(|e| e.to_string())?,
        )
    } else {
        (
            response_text.to_string(),
            json!({"text": response_text, "type": "phase1"}),
        )
    };

    // Start transaction
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    // Update chat title if it's still "New Chat"
    let title_preview: String = prompt.chars().take(30).collect();
    sqlx::query("UPDATE chats SET title = ? WHERE id = ? AND title = 'New Chat'")
        .bind(&title_preview)
        .bind(chat_id)
        .execute(&mut *tx)
        .await
        .ok();

    // Save user message
    sqlx::query("INSERT INTO messages (chat_id, role, content) VALUES (?, 'user', ?)")
        .bind(chat_id)
        .bind(&prompt)
        .execute(&mut *tx)
        .await
        .ok();

    // Save assistant message
    let ai_result = sqlx::query("INSERT INTO messages (chat_id, role, content) VALUES (?, 'assistant', ?)")
        .bind(chat_id)
        .bind(&history_content)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let ai_msg_id = ai_result.last_insert_rowid();

    // If there's an SD prompt, save pending image
    if let Some(sd_prompt) = output.get("sd_prompt").and_then(|x| x.as_str()) {
        sqlx::query("INSERT INTO images (message_id, chat_id, file_path, prompt) VALUES (?, ?, 'pending', ?)")
            .bind(ai_msg_id)
            .bind(chat_id)
            .bind(sd_prompt)
            .execute(&mut *tx)
            .await
            .ok();
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(output)
}

#[tauri::command]
pub async fn regenerate_story(id: i64, state: State<'_, OllamaState>) -> Result<Value, String> {
    // Find the last user message
    let row = sqlx::query(
        "SELECT id, content FROM messages WHERE chat_id = ? AND role = 'user' ORDER BY timestamp DESC LIMIT 1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(msg) = row {
        let msg_id: i64 = msg.get("id");
        let content: String = msg.get("content");

        // Delete messages after this one
        sqlx::query(
            "DELETE FROM messages WHERE chat_id = ? AND timestamp > (SELECT timestamp FROM messages WHERE id = ?)"
        )
        .bind(id)
        .bind(msg_id)
        .execute(&state.db)
        .await
        .ok();

        // Regenerate
        return generate_story(content, id, state).await;
    }

    Err("No message to regenerate".into())
}

// --- Character & Settings Commands ---

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