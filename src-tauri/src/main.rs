#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod config;
mod image_gen;
mod models;
mod services;
mod state;
mod text_gen;

use config::{AppConfig, ConfigState};
use services::ServiceManager;
use state::{OllamaState, SceneHintState, ServicePidState};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;

/// Kill a process and all its child processes by PID.
fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let result = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output();
        match result {
            Ok(output) if output.status.success() => {
                println!("[Shutdown] Process tree {} killed successfully", pid);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("[Shutdown] taskkill PID {} returned: {}", pid, stderr.trim());
            }
            Err(e) => println!("[Shutdown] Failed to kill PID {}: {}", pid, e),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
        let _ = std::process::Command::new("pkill")
            .args(["-P", &pid.to_string()])
            .output();
        println!("[Shutdown] Process {} and children killed", pid);
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // Load config first
            let config = AppConfig::load(&handle);
            let sd_path = config.sd_path();
            let comfyui_path = std::path::PathBuf::from(&config.comfyui_path);
            let auto_start = config.auto_start_services;

            // Store config in state
            app.manage(ConfigState(Mutex::new(config)));

            // Create and store service manager
            let service_manager = ServiceManager::new(sd_path.clone(), comfyui_path.clone());
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
            app.manage(SceneHintState(Mutex::new(HashMap::new())));
            app.manage(ServicePidState {
                ollama_pid: Mutex::new(None),
                comfyui_pid: Mutex::new(None),
            });

            // Pre-generate pose skeleton PNGs (fast no-op if already present)
            if let Ok(app_data_dir) = app.path().app_data_dir() {
                if let Err(e) = image_gen::pose_skeletons::ensure_pose_skeletons(&app_data_dir) {
                    eprintln!("[PoseSkeletons] Warning: {}", e);
                }
            }

            // Auto-start services if enabled (after all state is managed)
            if auto_start {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let started = rt.block_on(services::startup::auto_start_services(sd_path, comfyui_path));

                    // Store PIDs so we only kill what we started on exit
                    if let Some(pid_state) = app_handle.try_state::<ServicePidState>() {
                        if let Some(pid) = started.ollama_pid {
                            *pid_state.ollama_pid.lock().unwrap() = Some(pid);
                            println!("[ServiceManager] Stored Ollama PID: {}", pid);
                        }
                        if let Some(pid) = started.comfyui_pid {
                            *pid_state.comfyui_pid.lock().unwrap() = Some(pid);
                            println!("[ServiceManager] Stored ComfyUI PID: {}", pid);
                        }
                    }
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
            commands::chat::set_chat_character,
            commands::chat::save_image_for_message,
            // Story commands
            commands::story::save_story_premise,
            commands::story::get_story_list,
            commands::story::delete_stories,
            commands::story::create_story,
            commands::story::load_story,
            commands::story::save_story_state,
            commands::story::list_stories,
            commands::story::delete_story,
            commands::story::export_story,
            commands::story::get_story_images,
            commands::story::get_story_for_chat,
            commands::story::update_story_rating,
            // Character & Image commands
            image_gen::sd_webui::generate_image,
            image_gen::sd_webui::generate_image_variation,
            image_gen::sd_webui::generate_character_portrait,
            image_gen::sd_webui::diagnose_sd_connection,
            // Service commands
            services::check_services_status,
            services::start_services,
            services::stop_services,
            // Config commands
            config::get_config,
            config::update_config,
            config::set_sd_path,
            // LLM Parser commands
            text_gen::parser::parse_story_turn,
            text_gen::parser::get_story_text,
            text_gen::parser::get_character_names,
            text_gen::parser::check_generation_flags,
            // Mask Generator commands
            image_gen::masks::generate_color_mask,
            image_gen::masks::save_mask_image,
            // ComfyUI API commands
            image_gen::comfyui::check_comfyui_status,
            image_gen::comfyui::upload_to_comfyui,
            image_gen::comfyui::queue_comfyui_prompt,
            image_gen::comfyui::poll_comfyui_result,
            image_gen::comfyui::download_comfyui_image,
            image_gen::comfyui::generate_comfyui_scene,
            image_gen::comfyui::read_file_bytes,
            image_gen::comfyui::read_file_base64,
            // Character commands
            commands::character::add_character,
            commands::character::update_character,
            commands::character::delete_character_by_id,
            commands::character::delete_character,
            commands::character::get_character_by_name,
            commands::character::get_character_by_id,
            commands::character::list_characters_for_story,
            commands::character::list_all_characters,
            commands::character::list_characters_by_art_style,
            commands::character::search_characters,
            commands::character::set_character_master_image,
            commands::character::lookup_scene_characters,
            commands::character::link_character_to_story,
            commands::character::add_character_to_story,
            commands::character::remove_character_from_story,
            // Master Portrait commands
            image_gen::portrait::generate_master_portrait,
            image_gen::portrait::save_master_portrait,
            image_gen::portrait::preview_portrait_prompt,
            // Orchestrator (unified story turn pipeline)
            text_gen::orchestrator::process_story_turn,
            text_gen::orchestrator::generate_scene_image_for_turn,
            text_gen::orchestrator::get_compression_diagnostics,
            text_gen::orchestrator::regenerate_story,
            text_gen::orchestrator::regenerate_story_with_input,
            text_gen::orchestrator::free_vram,
            text_gen::orchestrator::preview_scene_prompt,
            text_gen::orchestrator::illustrate_scene_custom,
            // Scene commands
            commands::scene::create_scene,
            commands::scene::update_scene,
            commands::scene::delete_scene,
            commands::scene::list_scenes_for_story,
            commands::scene::list_all_scenes,
            commands::scene::link_scene_to_story,
            commands::scene::unlink_scene_from_story,
            commands::scene::add_character_to_scene,
            commands::scene::remove_character_from_scene,
            commands::scene::get_scene_characters,
            commands::scene::get_scene_with_characters,
            commands::scene::set_active_scene,
            commands::scene::get_active_scene,
            commands::scene::create_scene_from_llm_output,
            commands::scene::set_scene_hint,
            // Setup / dependency installer commands
            services::setup::check_setup_status,
            services::setup::install_dependency,
            services::setup::install_all_dependencies,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application")
        .run(|app, event| {
            match event {
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                    println!("[Shutdown] App closing — stopping services we started...");

                    if let Some(pid_state) = app.try_state::<ServicePidState>() {
                        if let Some(pid) = pid_state.ollama_pid.lock().unwrap().take() {
                            println!("[Shutdown] Killing Ollama (PID {})...", pid);
                            kill_process_tree(pid);
                        } else {
                            println!("[Shutdown] Ollama was not started by us — leaving it alone");
                        }

                        if let Some(pid) = pid_state.comfyui_pid.lock().unwrap().take() {
                            println!("[Shutdown] Killing ComfyUI (PID {})...", pid);
                            kill_process_tree(pid);
                        } else {
                            println!("[Shutdown] ComfyUI was not started by us — leaving it alone");
                        }
                    }

                    println!("[Shutdown] Cleanup complete");
                }
                _ => {}
            }
        });
}