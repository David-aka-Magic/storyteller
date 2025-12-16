use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::models::{Chat, CharacterProfile, StoryPremise};

const STATE_FILE: &str = "ollama_state.json";
const BASE_URL: &str = "http://localhost:11434";

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableOllamaState {
    pub chats: Vec<Chat>,
    pub characters: Vec<CharacterProfile>,
    pub stories: Vec<StoryPremise>,
    pub current_chat_id: u64,
    pub next_chat_id: u64,
    pub base_url: String, 
}

pub struct OllamaState {
    pub chats: Mutex<Vec<Chat>>,
    pub characters: Mutex<Vec<CharacterProfile>>,
    pub stories: Mutex<Vec<StoryPremise>>,
    pub current_chat_id: Mutex<u64>,
    pub next_chat_id: Mutex<u64>,
    pub base_url: String,
    pub client: reqwest::Client,
}

impl OllamaState {
    fn get_state_path(handle: &AppHandle) -> Result<PathBuf, String> {
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;

        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }
        
        Ok(app_data_dir.join(STATE_FILE))
    }

    fn new_default() -> Self {
        Self {
            chats: Mutex::new(vec![Chat {
                id: 1,
                title: "New Chat".into(),
                messages: vec![],
            }]),
            characters: Mutex::new(vec![]),
            stories: Mutex::new(vec![]),
            current_chat_id: Mutex::new(1),
            next_chat_id: Mutex::new(2),
            base_url: BASE_URL.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn new(handle: &AppHandle) -> Self {
        match Self::load(handle) {
            Ok(state) => state,
            Err(e) => {
                println!("Warning: Failed to load state: {}. Creating default state.", e);
                Self::new_default()
            }
        }
    }

    pub fn load(handle: &AppHandle) -> Result<Self, String> {
        let path = Self::get_state_path(handle)?;
        
        if let Ok(data) = fs::read_to_string(&path) {
            let serializable_state: SerializableOllamaState = 
                serde_json::from_str(&data).map_err(|e| format!("Failed to deserialize state: {}", e))?;

            Ok(OllamaState {
                chats: Mutex::new(serializable_state.chats),
                characters: Mutex::new(serializable_state.characters),
                stories: Mutex::new(serializable_state.stories),
                current_chat_id: Mutex::new(serializable_state.current_chat_id),
                next_chat_id: Mutex::new(serializable_state.next_chat_id),
                base_url: serializable_state.base_url,
                client: reqwest::Client::new(),
            })
        } else {
            Err("State file not found or readable".to_string())
        }
    }

    pub fn save(&self, handle: &AppHandle) -> Result<(), String> {
        let path = Self::get_state_path(handle)?;

        let c = self.chats.lock().map_err(|e| format!("{}", e))?;
        let curr_id = self.current_chat_id.lock().map_err(|e| format!("{}", e))?;
        let next_id = self.next_chat_id.lock().map_err(|e| format!("{}", e))?;
        let chars = self.characters.lock().map_err(|e| format!("{}", e))?;
        let stories = self.stories.lock().map_err(|e| format!("{}", e))?;
        
        let st = SerializableOllamaState {
            chats: c.clone(),
            current_chat_id: *curr_id,
            next_chat_id: *next_id,
            characters: chars.clone(),
            stories: stories.clone(),
            base_url: self.base_url.clone(),
        };
        
        let json = serde_json::to_string_pretty(&st).map_err(|e| format!("Failed to serialize state: {}", e))?;
        
        fs::write(&path, json).map_err(|e| format!("Failed to write state file: {}", e))?;

        Ok(())
    }

    pub fn build_context(&self, _prompt: String) -> Result<String, String> {
        Ok(_prompt)
    }
}