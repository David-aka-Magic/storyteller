pub mod startup;
pub mod setup;

use std::process::{Command, Child, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use std::path::PathBuf;

pub struct ServiceManager {
    ollama_process: Mutex<Option<Child>>,
    sd_process: Mutex<Option<Child>>,
    comfyui_process: Mutex<Option<Child>>,
    sd_path: PathBuf,
    comfyui_path: PathBuf,
}

impl ServiceManager {
    pub fn new(sd_path: PathBuf, comfyui_path: PathBuf) -> Self {
        Self {
            ollama_process: Mutex::new(None),
            sd_process: Mutex::new(None),
            comfyui_process: Mutex::new(None),
            sd_path,
            comfyui_path,
        }
    }

    /// Check if Ollama is already running by trying to connect
    pub async fn is_ollama_running() -> bool {
        let client = reqwest::Client::new();
        client
            .get("http://localhost:11434/api/tags")
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok()
    }

    /// Check if Stable Diffusion is already running
    pub async fn is_sd_running() -> bool {
        let client = reqwest::Client::new();
        client
            .get("http://127.0.0.1:7860/sdapi/v1/sd-models")
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok()
    }

    /// Check if ComfyUI is already running
    pub async fn is_comfyui_running() -> bool {
        let client = reqwest::Client::new();
        client
            .get("http://127.0.0.1:8188/system_stats")
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .map_or(false, |r| r.status().is_success())
    }

    /// Start Ollama if not already running
    pub fn start_ollama(&self) -> Result<(), String> {
        let mut process = self.ollama_process.lock().map_err(|e| e.to_string())?;

        if process.is_some() {
            return Ok(());
        }

        let ollama_bin = setup::find_ollama_binary()
            .unwrap_or_else(|| PathBuf::from("ollama"));

        let child = Command::new(&ollama_bin)
            .arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start Ollama: {}. Make sure Ollama is installed and in PATH.", e))?;

        *process = Some(child);
        println!("[ServiceManager] Started Ollama");
        Ok(())
    }

    /// Start Stable Diffusion WebUI
    /// This runs the venv Python directly with launch.py to avoid batch file issues
    pub fn start_sd(&self) -> Result<(), String> {
        let mut process = self.sd_process.lock().map_err(|e| e.to_string())?;

        if process.is_some() {
            return Ok(());
        }

        // Check if venv Python exists (preferred method)
        let venv_python = self.sd_path.join("venv").join("Scripts").join("python.exe");
        let launch_py = self.sd_path.join("launch.py");

        if venv_python.exists() && launch_py.exists() {
            println!("[ServiceManager] Starting SD via venv Python: {:?}", venv_python);

            let child = Command::new(&venv_python)
                .arg(&launch_py)
                .arg("--api")
                .arg("--xformers")
                .arg("--autolaunch")
                .current_dir(&self.sd_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to start Stable Diffusion: {}", e))?;

            *process = Some(child);
            println!("[ServiceManager] Started Stable Diffusion WebUI");
            return Ok(());
        }

        // Fallback: try running the batch file via cmd
        let webui_bat = self.sd_path.join("webui-user.bat");

        if !webui_bat.exists() {
            return Err(format!(
                "Stable Diffusion not found at: {:?}\nExpected either venv/Scripts/python.exe or webui-user.bat",
                self.sd_path
            ));
        }

        let child = Command::new("cmd")
            .args(["/c", "start", "/min", "cmd", "/c", "webui-user.bat"])
            .current_dir(&self.sd_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start Stable Diffusion: {}", e))?;

        *process = Some(child);
        println!("[ServiceManager] Started Stable Diffusion WebUI (via batch file)");
        Ok(())
    }

    /// Start ComfyUI if not already running.
    /// Supports the Windows standalone (embedded Python) layout and venv layouts.
    pub fn start_comfyui(&self) -> Result<(), String> {
        let mut process = self.comfyui_process.lock().map_err(|e| e.to_string())?;

        if process.is_some() {
            return Ok(());
        }

        // Windows standalone build: python_embeded\python.exe + ComfyUI\main.py
        let embedded_python = self.comfyui_path.join("python_embeded").join("python.exe");
        let standalone_main = self.comfyui_path.join("ComfyUI").join("main.py");

        // venv build: venv\Scripts\python.exe + main.py
        let venv_python = self.comfyui_path.join("venv").join("Scripts").join("python.exe");
        let venv_main = self.comfyui_path.join("main.py");

        let (python_cmd, main_py, standalone) = if embedded_python.exists() && standalone_main.exists() {
            (embedded_python, standalone_main, true)
        } else if venv_python.exists() && venv_main.exists() {
            (venv_python, venv_main, false)
        } else {
            return Err(format!(
                "ComfyUI not found at: {:?}\nExpected python_embeded\\python.exe + ComfyUI\\main.py (standalone) or venv\\Scripts\\python.exe + main.py (venv)",
                self.comfyui_path
            ));
        };

        println!("[ServiceManager] Starting ComfyUI via {:?}", python_cmd);

        let mut cmd = Command::new(&python_cmd);
        if standalone {
            cmd.arg("-s"); // don't import user site-packages
        }
        cmd.arg(&main_py);
        if standalone {
            cmd.arg("--windows-standalone-build");
        }
        // Performance flags for both layouts
        cmd.arg("--fast").arg("--fp8_e4m3fn-unet");
        cmd.current_dir(&self.comfyui_path)
           .stdout(Stdio::null())
           .stderr(Stdio::null());

        let child = cmd.spawn()
            .map_err(|e| format!("Failed to start ComfyUI: {}", e))?;

        *process = Some(child);
        println!("[ServiceManager] Started ComfyUI");
        Ok(())
    }

    /// Start all services, checking if they're already running first
    pub async fn start_all(&self) -> Result<ServiceStatus, String> {
        let mut status = ServiceStatus::default();

        // Check and start Ollama
        if Self::is_ollama_running().await {
            status.ollama = ServiceState::AlreadyRunning;
            println!("[ServiceManager] Ollama already running");
        } else {
            match self.start_ollama() {
                Ok(_) => {
                    status.ollama = ServiceState::Started;
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(e) => {
                    status.ollama = ServiceState::Failed(e);
                }
            }
        }

        // Check and start SD
        if Self::is_sd_running().await {
            status.sd = ServiceState::AlreadyRunning;
            println!("[ServiceManager] Stable Diffusion already running");
        } else {
            match self.start_sd() {
                Ok(_) => {
                    status.sd = ServiceState::Started;
                }
                Err(e) => {
                    status.sd = ServiceState::Failed(e);
                }
            }
        }

        // Check and start ComfyUI
        if Self::is_comfyui_running().await {
            status.comfyui = ServiceState::AlreadyRunning;
            println!("[ServiceManager] ComfyUI already running");
        } else if !self.comfyui_path.as_os_str().is_empty() {
            match self.start_comfyui() {
                Ok(_) => {
                    status.comfyui = ServiceState::Started;
                }
                Err(e) => {
                    status.comfyui = ServiceState::Failed(e.clone());
                    println!("[ServiceManager] Failed to start ComfyUI: {}", e);
                }
            }
        } else {
            status.comfyui = ServiceState::Failed("ComfyUI path not configured".to_string());
        }

        Ok(status)
    }

    /// Stop services we started (called on app exit)
    pub fn stop_all(&self) {
        if let Ok(mut process) = self.ollama_process.lock() {
            if let Some(mut child) = process.take() {
                let _ = child.kill();
                println!("[ServiceManager] Stopped Ollama");
            }
        }

        if let Ok(mut process) = self.sd_process.lock() {
            if let Some(mut child) = process.take() {
                let _ = child.kill();
                println!("[ServiceManager] Stopped Stable Diffusion");
            }
        }

        if let Ok(mut process) = self.comfyui_process.lock() {
            if let Some(mut child) = process.take() {
                let _ = child.kill();
                println!("[ServiceManager] Stopped ComfyUI");
            }
        }
    }
}

impl Drop for ServiceManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}

#[derive(Debug, Clone, Default)]
pub struct ServiceStatus {
    pub ollama: ServiceState,
    pub sd: ServiceState,
    pub comfyui: ServiceState,
}

#[derive(Debug, Clone, Default)]
pub enum ServiceState {
    #[default]
    NotStarted,
    Started,
    AlreadyRunning,
    Failed(String),
}

// Tauri commands for service management

use tauri::State;
use serde::Serialize;

#[derive(Serialize)]
pub struct ServiceStatusResponse {
    pub ollama_running: bool,
    pub sd_running: bool,
    pub comfyui_running: bool,
    pub ollama_error: Option<String>,
    pub sd_error: Option<String>,
    pub comfyui_error: Option<String>,
}

#[tauri::command]
pub async fn check_services_status() -> Result<ServiceStatusResponse, String> {
    let ollama_running = ServiceManager::is_ollama_running().await;
    let sd_running = ServiceManager::is_sd_running().await;
    let comfyui_running = ServiceManager::is_comfyui_running().await;

    Ok(ServiceStatusResponse {
        ollama_running,
        sd_running,
        comfyui_running,
        ollama_error: None,
        sd_error: None,
        comfyui_error: None,
    })
}

#[tauri::command]
pub async fn start_services(state: State<'_, ServiceManager>) -> Result<ServiceStatusResponse, String> {
    let status = state.start_all().await?;

    let ollama_error = match &status.ollama {
        ServiceState::Failed(e) => Some(e.clone()),
        _ => None,
    };

    let sd_error = match &status.sd {
        ServiceState::Failed(e) => Some(e.clone()),
        _ => None,
    };

    let comfyui_error = match &status.comfyui {
        ServiceState::Failed(e) => Some(e.clone()),
        _ => None,
    };

    // Check actual status after starting
    let ollama_running = ServiceManager::is_ollama_running().await;
    let sd_running = ServiceManager::is_sd_running().await;
    let comfyui_running = ServiceManager::is_comfyui_running().await;

    Ok(ServiceStatusResponse {
        ollama_running,
        sd_running,
        comfyui_running,
        ollama_error,
        sd_error,
        comfyui_error,
    })
}

#[tauri::command]
pub fn stop_services(state: State<'_, ServiceManager>) -> Result<(), String> {
    state.stop_all();
    Ok(())
}
