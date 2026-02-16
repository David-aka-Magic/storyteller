#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod comfyui_api;
mod commands;
mod context_compression;
mod config;
mod llm_parser;
mod mask_generator;
mod models;
mod services;
mod state;

use config::{AppConfig, ConfigState};
use services::ServiceManager;
use state::OllamaState;
use std::sync::Mutex;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // Load config first
            let config = AppConfig::load(&handle);
            let sd_path = config.sd_path();
            let auto_start = config.auto_start_services;

            // Store config in state
            app.manage(ConfigState(Mutex::new(config)));

            // Create and store service manager
            let service_manager = ServiceManager::new(sd_path.clone());
            app.manage(service_manager);

            // Initialize database state in separate thread
            let state = std::thread::spawn({
                let handle = handle.clone();
                move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(OllamaState::new(&handle))
                }
            })
            .join()
            .expect("Failed to initialize state");

            app.manage(state);

            // Auto-start services if enabled (after all state is managed)
            if auto_start {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        // Small delay to let app fully initialize
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        
                        // Start Ollama if not running
                        if !ServiceManager::is_ollama_running().await {
                            println!("[Startup] Starting Ollama...");
                            let _ = std::process::Command::new("ollama")
                                .arg("serve")
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                        } else {
                            println!("[Startup] Ollama already running");
                        }

                        // Start SD if not running
                        if !ServiceManager::is_sd_running().await {
                            println!("[Startup] Starting Stable Diffusion...");
                            
                            let venv_python = sd_path.join("venv").join("Scripts").join("python.exe");
                            let launch_py = sd_path.join("launch.py");
                            
                            println!("[Startup] Checking venv at: {:?}", venv_python);
                            println!("[Startup] Checking launch.py at: {:?}", launch_py);
                            
                            if venv_python.exists() && launch_py.exists() {
                                println!("[Startup] Starting SD via venv Python...");
                                match std::process::Command::new(&venv_python)
                                    .arg(&launch_py)
                                    .arg("--api")
                                    .arg("--xformers")
                                    .arg("--skip-python-version-check")
                                    .current_dir(&sd_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn() 
                                {
                                    Ok(_) => println!("[Startup] SD process started successfully"),
                                    Err(e) => println!("[Startup] Failed to start SD: {}", e),
                                }
                            } else {
                                let webui_bat = sd_path.join("webui-user.bat");
                                println!("[Startup] Venv not found, trying batch file at: {:?}", webui_bat);
                                
                                if webui_bat.exists() {
                                    match std::process::Command::new("cmd")
                                        .args(["/c", "start", "StableDiffusion", "cmd", "/k", "webui-user.bat"])
                                        .current_dir(&sd_path)
                                        .spawn()
                                    {
                                        Ok(_) => println!("[Startup] SD batch file started in new window"),
                                        Err(e) => println!("[Startup] Failed to start SD batch: {}", e),
                                    }
                                } else {
                                    println!("[Startup] ERROR: No valid SD startup method found!");
                                }
                            }
                        } else {
                            println!("[Startup] Stable Diffusion already running");
                        }
                    });
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Chat commands
            commands::chat::get_chat_list,
            commands::chat::new_chat,
            commands::chat::load_chat,
            commands::chat::delete_chats,
            commands::chat::clear_history,
            commands::chat::generate_story,
            commands::chat::regenerate_story,
            commands::chat::set_chat_character,
            // Story commands (legacy — kept for backward compatibility)
            commands::story::save_story_premise,
            commands::story::get_story_list,
            commands::story::delete_stories,
            // Story Manager commands (full session lifecycle)
            commands::story_manager::create_story,
            commands::story_manager::load_story,
            commands::story_manager::save_story_state,
            commands::story_manager::list_stories,
            commands::story_manager::delete_story,
            commands::story_manager::export_story,
            // Character & Image commands
            commands::images::save_character,
            commands::images::delete_character,
            commands::images::get_character_list,
            commands::images::generate_image,
            commands::images::generate_image_variation,
            commands::images::generate_character_portrait,
            // Service commands
            services::check_services_status,
            services::start_services,
            services::stop_services,
            // Config commands
            config::get_config,
            config::update_config,
            config::set_sd_path,
            // LLM Parser commands
            llm_parser::parse_story_turn,
            llm_parser::get_story_text,
            llm_parser::get_character_names,
            llm_parser::check_generation_flags,
            // Mask Generator commands
            mask_generator::generate_color_mask,
            mask_generator::save_mask_image,
            // ComfyUI API commands
            comfyui_api::check_comfyui_status,
            comfyui_api::upload_to_comfyui,
            comfyui_api::queue_comfyui_prompt,
            comfyui_api::poll_comfyui_result,
            comfyui_api::download_comfyui_image,
            comfyui_api::generate_comfyui_scene,
            comfyui_api::read_file_bytes,
            comfyui_api::read_file_base64,
            // Master Portrait commands
            commands::master_portrait::generate_master_portrait,
            commands::master_portrait::save_master_portrait,
            commands::master_portrait::preview_portrait_prompt,
            // Orchestrator (unified story turn pipeline)
            commands::orchestrator::process_story_turn,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}