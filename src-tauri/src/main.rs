#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod models;
mod state;
mod utils;
mod commands; 

use state::OllamaState;
use tauri::Manager;

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            
            // Initialize the SQLite-backed state asynchronously
            tauri::async_runtime::block_on(async move {
                let state = OllamaState::new(&handle).await;
                handle.manage(state);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Core Chat
            commands::chat::generate_story,
            commands::chat::regenerate_story,
            commands::chat::set_chat_character,
            commands::chat::clear_history,
            commands::chat::new_chat,
            commands::chat::load_chat,
            commands::chat::get_chat_list,
            commands::chat::delete_chats,
            
            // Story Premises
            commands::story::save_story_premise,
            commands::story::delete_stories,
            commands::story::get_story_list,
            
            // Characters
            commands::chat::save_character,
            commands::chat::delete_character,
            commands::chat::get_character_list,
            
            // Image Generation
            commands::chat::generate_image,
            commands::chat::generate_image_variation,
            commands::chat::generate_character_portrait
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}