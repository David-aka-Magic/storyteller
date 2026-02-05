use std::process::{Command, Child, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use std::path::PathBuf;

pub struct ServiceManager {
    ollama_process: Mutex<Option<Child>>,
    sd_process: Mutex<Option<Child>>,
    sd_path: PathBuf,
}

impl ServiceManager {
    pub fn new(sd_path: PathBuf) -> Self {
        Self {
            ollama_process: Mutex::new(None),
            sd_process: Mutex::new(None),
            sd_path,
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

    /// Start Ollama if not already running
    pub fn start_ollama(&self) -> Result<(), String> {
        let mut process = self.ollama_process.lock().map_err(|e| e.to_string())?;
        
        if process.is_some() {
            return Ok(());
        }

        // On Windows, ollama is typically in PATH after installation
        let child = Command::new("ollama")
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
            // Run the venv Python directly with launch.py
            // This bypasses the batch file complexity
            println!("[ServiceManager] Starting SD via venv Python: {:?}", venv_python);
            
            let child = Command::new(&venv_python)
                .arg(&launch_py)
                .arg("--api")        // Enable API access
                .arg("--xformers")   // Memory optimization (from your setup)
                .arg("--autolaunch") // Don't auto-open browser
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

        // Run batch file in a new cmd window (minimized)
        // Using 'start /min' to run minimized without blocking
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
    pub ollama_error: Option<String>,
    pub sd_error: Option<String>,
}

#[tauri::command]
pub async fn check_services_status() -> Result<ServiceStatusResponse, String> {
    let ollama_running = ServiceManager::is_ollama_running().await;
    let sd_running = ServiceManager::is_sd_running().await;

    Ok(ServiceStatusResponse {
        ollama_running,
        sd_running,
        ollama_error: None,
        sd_error: None,
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

    // Check actual status after starting
    let ollama_running = ServiceManager::is_ollama_running().await;
    let sd_running = ServiceManager::is_sd_running().await;

    Ok(ServiceStatusResponse {
        ollama_running,
        sd_running,
        ollama_error,
        sd_error,
    })
}

#[tauri::command]
pub fn stop_services(state: State<'_, ServiceManager>) -> Result<(), String> {
    state.stop_all();
    Ok(())
}