// src-tauri/src/services/startup.rs
//
// Auto-start logic for Ollama, Stable Diffusion WebUI, and ComfyUI
// =================================================================
// Extracted from main.rs setup closure to keep startup lean.

use std::path::PathBuf;
use super::ServiceManager;

pub struct StartedServices {
    pub ollama_pid: Option<u32>,
    pub comfyui_pid: Option<u32>,
}

pub async fn auto_start_services(sd_path: PathBuf, comfyui_path: PathBuf) -> StartedServices {
    let mut started = StartedServices {
        ollama_pid: None,
        comfyui_pid: None,
    };

    // Small delay to let app fully initialize
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Start Ollama if not running
    if !ServiceManager::is_ollama_running().await {
        println!("[Startup] Starting Ollama...");
        let ollama_bin = super::setup::find_ollama_binary()
            .unwrap_or_else(|| std::path::PathBuf::from("ollama"));
        match std::process::Command::new(&ollama_bin)
            .arg("serve")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => {
                let pid = child.id();
                println!("[Startup] Ollama started with PID {}", pid);
                started.ollama_pid = Some(pid);
                std::mem::forget(child);
            }
            Err(e) => println!("[Startup] Failed to start Ollama: {}", e),
        }
    } else {
        println!("[Startup] Ollama already running — not tracking PID");
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
                Ok(child) => {
                    println!("[Startup] SD process started (PID {})", child.id());
                    std::mem::forget(child);
                }
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
                    Ok(child) => {
                        println!("[Startup] SD batch file started in new window");
                        std::mem::forget(child);
                    }
                    Err(e) => println!("[Startup] Failed to start SD batch: {}", e),
                }
            } else {
                println!("[Startup] ERROR: No valid SD startup method found!");
            }
        }
    } else {
        println!("[Startup] Stable Diffusion already running");
    }

    // Start ComfyUI if not running and path is configured
    if !comfyui_path.as_os_str().is_empty() {
        if !ServiceManager::is_comfyui_running().await {
            println!("[Startup] Starting ComfyUI...");

            // Detect layout: standalone (embedded python) vs venv
            let embedded_python = comfyui_path.join("python_embeded").join("python.exe");
            let standalone_main = comfyui_path.join("ComfyUI").join("main.py");
            let venv_python = comfyui_path.join("venv").join("Scripts").join("python.exe");
            let venv_main = comfyui_path.join("main.py");

            let result = if embedded_python.exists() && standalone_main.exists() {
                std::process::Command::new(&embedded_python)
                    .arg("-s")
                    .arg(&standalone_main)
                    .arg("--windows-standalone-build")
                    .current_dir(&comfyui_path)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
            } else if venv_python.exists() && venv_main.exists() {
                std::process::Command::new(&venv_python)
                    .arg(&venv_main)
                    .current_dir(&comfyui_path)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
            } else {
                println!("[Startup] ComfyUI not found at {:?}", comfyui_path);
                return started;
            };

            match result {
                Ok(child) => {
                    let pid = child.id();
                    println!("[Startup] ComfyUI started with PID {}", pid);
                    started.comfyui_pid = Some(pid);
                    std::mem::forget(child);
                }
                Err(e) => println!("[Startup] Failed to start ComfyUI: {}", e),
            }
        } else {
            println!("[Startup] ComfyUI already running — not tracking PID");
        }
    } else {
        println!("[Startup] ComfyUI path not configured, skipping");
    }

    started
}
