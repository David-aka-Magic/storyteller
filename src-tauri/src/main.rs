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

            // Auto-start services if enabled (after all state is managed)
            if auto_start {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(services::startup::auto_start_services(sd_path, comfyui_path));
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
            // Setup / dependency installer commands
            services::setup::check_setup_status,
            services::setup::install_dependency,
            services::setup::install_all_dependencies,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}