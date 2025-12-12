#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ollama;

use ollama::*;
use tauri::{Manager, AppHandle};

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
            generate_story,
            clear_history,
            new_chat,
            load_chat,
            get_chat_list
        ])
        .run(tauri::generate_context!())
        .expect("error");
}
