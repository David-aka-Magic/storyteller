#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod utils;
mod state;
mod commands;

use state::OllamaState;
use tauri::{AppHandle, Manager};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle: AppHandle = app.handle().clone();
            
            app.manage(OllamaState::new(&handle));
            
            let win = handle.get_webview_window("main").unwrap();
            let h = handle.clone();
            
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let st = h.state::<OllamaState>();
                    let _ = st.save(&h);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Core Chat
            commands::generate_story,
            commands::regenerate_story,
            commands::clear_history,
            commands::new_chat,
            commands::load_chat,
            commands::get_chat_list,
            commands::delete_chats,
            
            // Story Premises
            commands::save_story_premise,
            commands::delete_stories,
            commands::get_story_list,
            
            // Characters
            commands::save_character,
            commands::delete_character,
            commands::get_character_list,
            
            // Image Generation
            commands::generate_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}