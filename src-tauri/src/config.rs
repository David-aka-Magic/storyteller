use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    /// Path to the stable-diffusion-webui folder
    pub sd_webui_path: String,
    
    /// Ollama base URL (default: http://localhost:11434)
    pub ollama_url: String,
    
    /// Stable Diffusion API URL (default: http://127.0.0.1:7860)
    pub sd_api_url: String,
    
    /// Auto-start services when app launches
    pub auto_start_services: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            // Default path - user can change in config.json
            sd_webui_path: "C:\\Users\\dcarl\\stable-diffusion-webui".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            sd_api_url: "http://127.0.0.1:7860".to_string(),
            auto_start_services: true,
        }
    }
}

impl AppConfig {
    fn config_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not get app data dir: {}", e))?;
        
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).map_err(|e| format!("Could not create app dir: {}", e))?;
        }
        
        Ok(app_dir.join("config.json"))
    }

    pub fn load(app_handle: &AppHandle) -> Self {
        let path = match Self::config_path(app_handle) {
            Ok(p) => p,
            Err(e) => {
                println!("[Config] Could not get config path: {}", e);
                return Self::default();
            }
        };

        println!("[Config] Config path: {:?}", path);

        if !path.exists() {
            println!("[Config] Config file doesn't exist, creating default...");
            let config = Self::default();
            if let Err(e) = config.save(app_handle) {
                println!("[Config] Failed to save default config: {}", e);
            } else {
                println!("[Config] Default config saved successfully");
            }
            return config;
        }

        println!("[Config] Loading existing config...");
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(config) => {
                        println!("[Config] Config loaded successfully");
                        config
                    }
                    Err(e) => {
                        println!("[Config] Failed to parse config: {}, using default", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                println!("[Config] Failed to read config: {}, using default", e);
                Self::default()
            }
        }
    }

    pub fn save(&self, app_handle: &AppHandle) -> Result<(), String> {
        let path = Self::config_path(app_handle)?;
        println!("[Config] Saving config to: {:?}", path);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        println!("[Config] Config saved successfully");
        Ok(())
    }

    pub fn sd_path(&self) -> PathBuf {
        PathBuf::from(&self.sd_webui_path)
    }
}

// Tauri commands for config management

use tauri::State;
use std::sync::Mutex;

pub struct ConfigState(pub Mutex<AppConfig>);

#[tauri::command]
pub fn get_config(state: State<'_, ConfigState>) -> Result<AppConfig, String> {
    let config = state.0.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
pub fn update_config(
    new_config: AppConfig,
    state: State<'_, ConfigState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut config = state.0.lock().map_err(|e| e.to_string())?;
    *config = new_config;
    config.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn set_sd_path(
    path: String,
    state: State<'_, ConfigState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut config = state.0.lock().map_err(|e| e.to_string())?;
    config.sd_webui_path = path;
    config.save(&app)?;
    Ok(())
}