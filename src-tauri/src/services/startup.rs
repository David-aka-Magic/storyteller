// src-tauri/src/services/startup.rs
//
// Auto-start logic for Ollama and Stable Diffusion WebUI
// =======================================================
// Extracted from main.rs setup closure to keep startup lean.

use std::path::PathBuf;
use super::ServiceManager;

pub async fn auto_start_services(sd_path: PathBuf) {
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
}
