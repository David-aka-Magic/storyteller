use serde_json::json;
use tauri::{AppHandle, State};

use crate::models::{Chat, CharacterProfile, Message, SdJson, StoryResponse, StoryPremise, SDRequest, SDResponse};
use crate::state::OllamaState;
use crate::utils::{extract_json, extract_story};

// --- Image Generation ---

#[tauri::command]
pub async fn generate_image(
    prompt: String,
    chat_id: u64,
    msg_index: usize,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<String, String> {
    let url = "http://127.0.0.1:7860/sdapi/v1/txt2img";

    let payload = SDRequest {
        prompt: prompt.clone(),
        negative_prompt: "low quality, bad anatomy, worst quality, text, watermark, signature, ugly, deformed".to_string(),
        steps: 25,
        width: 1024,
        height: 1024,
        cfg_scale: 7.0,
        sampler_name: "Euler a".to_string(),
        batch_size: 1,
    };

    // 1. Generate Image
    let client = reqwest::Client::new();
    let res = client.post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to SD (is it running?): {}", e))?;

    if !res.status().is_success() {
        return Err(format!("SD Error: {}", res.status()));
    }

    let sd_res: SDResponse = res.json().await.map_err(|e| format!("Failed to parse SD response: {}", e))?;

    let base64_image = sd_res.images.first()
        .ok_or("No image returned from SD")?
        .clone();

    {
        let mut chats = state.chats.lock().map_err(|e| e.to_string())?;
        
        if let Some(chat) = chats.iter_mut().find(|c| c.id == chat_id) {
            if let Some(msg) = chat.messages.get_mut(msg_index) {
                msg.images = Some(vec![base64_image.clone()]);
            } else {
                return Err("Message index out of bounds".to_string());
            }
        } else {
            return Err("Chat ID not found".to_string());
        }
    } 

    state.save(&app)?;

    Ok(base64_image)
}

#[tauri::command]
pub fn save_character(
    character: CharacterProfile,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut chars = state.characters.lock().map_err(|e| format!("{}", e))?;
        if let Some(existing_char) = chars.iter_mut().find(|c| c.id == character.id) {
            *existing_char = character;
        } else {
            chars.push(character);
        }
    } 
    state.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn delete_character(
    id: String,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut chars = state.characters.lock().map_err(|e| format!("{}", e))?;
        chars.retain(|c| c.id != id);
    } 
    state.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn get_character_list(state: State<'_, OllamaState>) -> Result<Vec<CharacterProfile>, String> {
    let chars = state.characters.lock().map_err(|e| format!("{}", e))?;
    Ok(chars.clone())
}


#[tauri::command]
pub fn save_story_premise(
    story: StoryPremise,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut stories = state.stories.lock().map_err(|e| format!("{}", e))?;
        if let Some(existing_story) = stories.iter_mut().find(|s| s.id == story.id) {
            *existing_story = story;
        } else {
            stories.push(story);
        }
    } 
    state.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn delete_stories(
    ids: Vec<String>,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut stories = state.stories.lock().map_err(|e| format!("{}", e))?;
        stories.retain(|story| !ids.contains(&story.id));
    } 
    state.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn get_story_list(state: State<'_, OllamaState>) -> Result<Vec<StoryPremise>, String> {
    let stories = state.stories.lock().map_err(|e| format!("{}", e))?;
    Ok(stories.clone())
}


#[tauri::command]
pub fn get_chat_list(state: State<'_, OllamaState>) -> Result<Vec<Chat>, String> {
    let c = state.chats.lock().map_err(|e| format!("{}", e))?;
    Ok(c.iter()
        .map(|x| Chat {
            id: x.id,
            title: x.title.clone(),
            messages: vec![],
        })
        .collect())
}

#[tauri::command]
pub fn load_chat(id: u64, state: State<'_, OllamaState>) -> Result<Vec<Message>, String> {
    let c = state.chats.lock().map_err(|e| format!("{}", e))?;
    if let Some(ch) = c.iter().find(|x| x.id == id) {
        *state.current_chat_id.lock().map_err(|e| format!("{}", e))? = id;
        Ok(ch.messages.clone())
    } else {
        Err("Chat not found".into())
    }
}

#[tauri::command]
pub fn new_chat(state: State<'_, OllamaState>) -> Result<u64, String> {
    let mut n = state.next_chat_id.lock().map_err(|e| format!("{}", e))?;
    let id = *n;
    *n += 1;
    let mut c = state.chats.lock().map_err(|e| format!("{}", e))?;
    c.push(Chat {
        id,
        title: "New Chat".into(),
        messages: vec![],
    });
    *state.current_chat_id.lock().map_err(|e| format!("{}", e))? = id;
    Ok(id)
}

#[tauri::command]
pub fn clear_history(state: State<'_, OllamaState>) -> Result<(), String> {
    let id = *state.current_chat_id.lock().map_err(|e| format!("{}", e))?;
    let mut c = state.chats.lock().map_err(|e| format!("{}", e))?;
    if let Some(ch) = c.iter_mut().find(|x| x.id == id) {
        ch.messages.clear();
        ch.title = "New Chat".into();
        Ok(())
    } else {
        Err("Current chat not found".into())
    }
}

#[tauri::command]
pub fn delete_chats(
    ids: Vec<u64>,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<bool, String> {
    let mut chats = state.chats.lock().map_err(|e| format!("{}", e))?;

    if ids.is_empty() { return Ok(false); }

    let original_len = chats.len();
    chats.retain(|chat| !ids.contains(&chat.id));

    let mut current_id = state.current_chat_id.lock().map_err(|e| format!("{}", e))?;
    if ids.contains(&*current_id) {
        if let Some(first_chat) = chats.first() {
            *current_id = first_chat.id;
        } else {
            let new_id = 1;
            chats.push(Chat { id: new_id, title: "New Chat".into(), messages: vec![] });
            *current_id = new_id;
        }
    }

    let mut next_id = state.next_chat_id.lock().map_err(|e| format!("{}", e))?;
    let max_id = chats.iter().map(|c| c.id).max().unwrap_or(0);
    *next_id = max_id + 1;

    drop(chats);
    drop(current_id);
    drop(next_id);

    state.save(&app)?;
    Ok(original_len > 0 && ids.len() > 0)
}

#[tauri::command]
pub async fn regenerate_story(
    state: State<'_, OllamaState>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/api/chat", state.base_url);
    let id = *state.current_chat_id.lock().map_err(|e| e.to_string())?;

    let (prompt, character_context, story_context) = {
        let mut chats = state.chats.lock().map_err(|e| e.to_string())?;
        let ch = chats.iter_mut().find(|x| x.id == id).ok_or("Chat not found")?;

        if let Some(last) = ch.messages.last() {
            if last.role == "assistant" {
                ch.messages.pop();
            }
        }

        let last_user_msg = ch.messages.last().ok_or("No user message found to regenerate from")?;
        let prompt_text = last_user_msg.content.clone();
        
        let chars = state.characters.lock().map_err(|e| e.to_string())?;
        let stories = state.stories.lock().map_err(|e| e.to_string())?;
        
        let mut c_ctx = String::new();
        if !chars.is_empty() {
            for c in chars.iter() {
                c_ctx.push_str(&format!(
                    "- Name: {}, Age: {}, Gender: {}, Appearance: {}. Personality: {}\n",
                    c.name, c.age, c.gender, c.sd_prompt, c.personality
                ));
            }
        }
        let s_ctx = if let Some(s) = stories.first() {
            format!("Title: {}\nPremise: {}", s.title, s.description)
        } else {
            "Freeform".to_string()
        };
        (prompt_text, c_ctx, s_ctx)
    };

    let system_prompt = format!(
        r#"STORY CONTEXT:
        Setting: {story_context}
        Characters: {character_context}

        FORMAT INSTRUCTIONS:
        You must output JSON only.
        {{
            "story_json": {{ "response": "Story text here..." }},
            "sd_json": {{ "look": "Visual description for image generator..." }}
        }}"#,
        story_context = story_context,
        character_context = character_context
    );

    let mut api_messages = Vec::new();
    api_messages.push(json!({ "role": "system", "content": system_prompt }));

    {
        let chats = state.chats.lock().map_err(|e| e.to_string())?;
        if let Some(ch) = chats.iter().find(|x| x.id == id) {
            for msg in &ch.messages {
                let content = if msg.role == "assistant" {
                    if let Some(j) = extract_json(&msg.content) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&j) {
                            extract_story(&v) 
                        } else { msg.content.clone() }
                    } else { msg.content.clone() }
                } else {
                    msg.content.clone()
                };
                api_messages.push(json!({ "role": msg.role, "content": content }));
            }
        }
    }

    let body = json!({ 
        "model": "Story_v27", 
        "messages": api_messages, 
        "stream": false,
        "format": "json", 
        "options": { "num_ctx": 4096 }
    });

    let r: reqwest::Response = state.client.post(url).json(&body).send().await.map_err(|e| e.to_string())?;
    let text: String = r.text().await.map_err(|e| e.to_string())?;
    
    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let resp = val["message"]["content"].as_str().unwrap_or(&text);

    let hist;
    let out;

    match serde_json::from_str::<serde_json::Value>(resp) {
        Ok(v) => {
            hist = resp.to_string();
            let story_text = if let Some(s) = v.get("story_json") {
                s.get("response").and_then(|t| t.as_str()).unwrap_or("...").to_string()
            } else if let Some(r) = v.get("response") {
                 r.as_str().unwrap_or("...").to_string()
            } else {
                v.to_string() 
            };
            let sd = v.get("sd_json").and_then(|x| serde_json::from_value::<SdJson>(x.clone()).ok());
            
            let sr = StoryResponse {
                story: story_text,
                sd_prompt: sd.as_ref().map(|x| x.look.clone()),
                sd_details: sd,
            };
            out = serde_json::to_value(sr).map_err(|e| e.to_string())?;
        },
        Err(_) => {
            hist = resp.to_string();
            out = json!({"text": resp, "type": "phase1"});
        }
    }

    let mut c = state.chats.lock().map_err(|e| e.to_string())?;
    if let Some(ch) = c.iter_mut().find(|x| x.id == id) {
        ch.messages.push(Message { role: "assistant".into(), content: hist, images: None });
    }

    Ok(out)
}

#[tauri::command]
pub async fn generate_story(
    prompt: String,
    state: State<'_, OllamaState>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/api/chat", state.base_url);

    let (character_context, story_context) = {
        let chars = state.characters.lock().map_err(|e| e.to_string())?;
        let stories = state.stories.lock().map_err(|e| e.to_string())?;
        
        let mut c_ctx = String::new();
        if !chars.is_empty() {
            for c in chars.iter() {
                c_ctx.push_str(&format!(
                    "- Name: {}, Age: {}, Gender: {}, Appearance: {}. Personality: {}\n",
                    c.name, c.age, c.gender, c.sd_prompt, c.personality
                ));
            }
        }

        let s_ctx = if let Some(s) = stories.first() {
            format!("Title: {}\nPremise: {}", s.title, s.description)
        } else {
            "Freeform".to_string()
        };
        (c_ctx, s_ctx)
    };

    let system_prompt = format!(
        r#"STORY CONTEXT:
        Setting: {story_context}
        Characters: {character_context}

        FORMAT INSTRUCTIONS:
        You must output JSON only.
        {{
            "story_json": {{ "response": "Story text here..." }},
            "sd_json": {{ "look": "Visual description for image generator..." }}
        }}"#,
        story_context = story_context,
        character_context = character_context
    );

    let id = *state.current_chat_id.lock().map_err(|e| e.to_string())?;
    
    let mut api_messages = Vec::new();
    api_messages.push(json!({ "role": "system", "content": system_prompt }));

    {
        let chats = state.chats.lock().map_err(|e| e.to_string())?;
        if let Some(ch) = chats.iter().find(|x| x.id == id) {
            for msg in &ch.messages {
                let content = if msg.role == "assistant" {
                    if let Some(j) = extract_json(&msg.content) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&j) {
                            extract_story(&v) 
                        } else { msg.content.clone() }
                    } else { msg.content.clone() }
                } else {
                    msg.content.clone()
                };
                api_messages.push(json!({ "role": msg.role, "content": content }));
            }
        }
    }

    api_messages.push(json!({ "role": "user", "content": prompt }));

    let body = json!({ 
        "model": "Story_v27", 
        "messages": api_messages, 
        "stream": false,
        "format": "json", 
        "options": { "num_ctx": 4096 }
    });

    let r: reqwest::Response = state.client.post(url).json(&body).send().await.map_err(|e| e.to_string())?;
    let text: String = r.text().await.map_err(|e| e.to_string())?;
    
    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let resp = val["message"]["content"].as_str().unwrap_or(&text);

    let hist;
    let out;

    match serde_json::from_str::<serde_json::Value>(resp) {
        Ok(v) => {
            hist = resp.to_string();
            let story_text = if let Some(s) = v.get("story_json") {
                s.get("response").and_then(|t| t.as_str()).unwrap_or("...").to_string()
            } else if let Some(r) = v.get("response") {
                 r.as_str().unwrap_or("...").to_string()
            } else {
                v.to_string() 
            };
            let sd = v.get("sd_json").and_then(|x| serde_json::from_value::<SdJson>(x.clone()).ok());
            
            let sr = StoryResponse {
                story: story_text,
                sd_prompt: sd.as_ref().map(|x| x.look.clone()),
                sd_details: sd,
            };
            out = serde_json::to_value(sr).map_err(|e| e.to_string())?;
        },
        Err(_) => {
            hist = resp.to_string();
            out = json!({"text": resp, "type": "phase1"});
        }
    }

    let mut c = state.chats.lock().map_err(|e| e.to_string())?;
    if let Some(ch) = c.iter_mut().find(|x| x.id == id) {
        if ch.messages.is_empty() {
            let t = prompt.split_whitespace().take(5).collect::<Vec<_>>().join(" ");
            ch.title = if t.is_empty() { "New Chat".into() } else { t };
        }
        ch.messages.push(Message { role: "user".into(), content: prompt, images: None });
        ch.messages.push(Message { role: "assistant".into(), content: hist, images: None });
    }

    Ok(out)
}