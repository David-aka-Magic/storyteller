// src-tauri/src/services/setup.rs
//
// Dependency installer for AI Story Writer.
// Manages Ollama, the Story_v27 model, ComfyUI, checkpoints, and custom nodes.

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

// =============================================================================
// Public types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupProgress {
    pub step: String,
    pub progress_pct: f32, // 0.0 - 1.0
    pub message: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// "nvidia", "amd", "intel", or "unknown"
    pub vendor: String,
    /// Human-readable GPU name from the OS
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    pub ollama: DependencyStatus,
    pub ollama_model: DependencyStatus,
    pub comfyui: DependencyStatus,
    pub comfyui_torch: DependencyStatus,
    pub checkpoints: Vec<DependencyStatus>,
    pub custom_nodes: Vec<DependencyStatus>,
    pub gpu_info: GpuInfo,
    pub all_ready: bool,
}

// =============================================================================
// Internal helpers
// =============================================================================

fn emit_progress(app: &AppHandle, step: &str, progress_pct: f32, message: &str, is_error: bool) {
    let _ = app.emit(
        "setup-progress",
        SetupProgress {
            step: step.to_string(),
            progress_pct,
            message: message.to_string(),
            is_error,
        },
    );
}

/// Default ComfyUI install location: <app_data_dir>/comfyui
fn comfyui_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("comfyui"))
        .join("comfyui")
}

/// Detect the primary discrete GPU vendor and name.
///
/// Uses PowerShell's Get-CimInstance on Windows (preferred over deprecated wmic).
/// Falls back to checking for nvidia-smi in PATH as a secondary signal.
pub fn detect_gpu() -> GpuInfo {
    // Try PowerShell Get-CimInstance — works on Windows 10/11
    if let Ok(out) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
        ])
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            // Lines may have multiple adapters; pick the first discrete GPU
            for line in text.lines() {
                let lower = line.to_lowercase();
                if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("quadro") || lower.contains("rtx") || lower.contains("gtx") {
                    return GpuInfo {
                        vendor: "nvidia".to_string(),
                        name: line.trim().to_string(),
                    };
                }
                if lower.contains("amd") || lower.contains("radeon") || lower.contains("rx ") {
                    return GpuInfo {
                        vendor: "amd".to_string(),
                        name: line.trim().to_string(),
                    };
                }
            }
            // If we got output but didn't match above, take the first non-empty line
            if let Some(first) = text.lines().find(|l| !l.trim().is_empty()) {
                let lower = first.to_lowercase();
                let vendor = if lower.contains("intel") { "intel" } else { "unknown" };
                return GpuInfo {
                    vendor: vendor.to_string(),
                    name: first.trim().to_string(),
                };
            }
        }
    }

    // Fallback: check if nvidia-smi is accessible (confirms NVIDIA driver installed)
    if Command::new("nvidia-smi")
        .arg("--query-gpu=name")
        .arg("--format=csv,noheader")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return GpuInfo {
            vendor: "nvidia".to_string(),
            name: "NVIDIA GPU".to_string(),
        };
    }

    GpuInfo {
        vendor: "unknown".to_string(),
        name: "Unknown GPU".to_string(),
    }
}

/// Locate the Ollama binary.
///
/// On Windows, Ollama installs to `%LOCALAPPDATA%\Programs\Ollama\` and adds itself
/// to the *user* PATH. Tauri processes sometimes launch before the shell PATH is refreshed,
/// or inherit a system-level PATH that doesn't include the user-level entry.
/// This function tries bare `"ollama"` first (works when PATH is correct), then falls back
/// to the two canonical Windows install directories.
pub fn find_ollama_binary() -> Option<PathBuf> {
    // 1. PATH — fastest, works in most cases
    if Command::new("ollama")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some(PathBuf::from("ollama"));
    }

    // 2. %LOCALAPPDATA%\Programs\Ollama\ollama.exe  (standard user install)
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local)
            .join("Programs")
            .join("Ollama")
            .join("ollama.exe");
        if p.exists() {
            return Some(p);
        }
    }

    // 3. %ProgramFiles%\Ollama\ollama.exe  (system-wide install)
    if let Ok(pf) = std::env::var("ProgramFiles") {
        let p = PathBuf::from(pf).join("Ollama").join("ollama.exe");
        if p.exists() {
            return Some(p);
        }
    }

    None
}

/// Bundled Modelfile path (dev: src-tauri/resources/Modelfile, prod: resource_dir/resources/Modelfile)
fn modelfile_path(app: &AppHandle) -> PathBuf {
    app.path()
        .resource_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("resources")
        .join("Modelfile")
}

/// Download a file with streaming progress events.
/// Progress is reported in the range [0.0, 0.9]; caller emits the final 1.0.
async fn download_file_with_progress(
    app: &AppHandle,
    url: &str,
    dest: &PathBuf,
    step: &str,
) -> Result<(), String> {
    emit_progress(app, step, 0.0, &format!("Starting download: {}", url), false);

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3600))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed — HTTP {}: {}",
            response.status(),
            url
        ));
    }

    let total_size = response.content_length();
    let mut downloaded: u64 = 0;

    let mut file =
        std::fs::File::create(dest).map_err(|e| format!("Cannot create file {:?}: {}", dest, e))?;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;

        if let Some(total) = total_size {
            let progress = (downloaded as f32 / total as f32).min(0.9);
            let mb_done = downloaded as f32 / 1_048_576.0;
            let mb_total = total as f32 / 1_048_576.0;
            emit_progress(
                app,
                step,
                progress,
                &format!("Downloaded {:.1} / {:.1} MB", mb_done, mb_total),
                false,
            );
        }
    }

    Ok(())
}

// =============================================================================
// Check functions
// =============================================================================

async fn check_ollama() -> DependencyStatus {
    // HTTP check first — if the API responds, Ollama is running (and therefore installed)
    let client = reqwest::Client::new();
    if client
        .get("http://localhost:11434/api/tags")
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
    {
        return DependencyStatus {
            name: "ollama".to_string(),
            installed: true,
            version: Some("running".to_string()),
            path: find_ollama_binary().map(|p| p.to_string_lossy().to_string()),
            error: None,
        };
    }

    // Binary check — installed but not currently serving
    match find_ollama_binary() {
        Some(bin) => DependencyStatus {
            name: "ollama".to_string(),
            installed: true,
            version: None,
            path: Some(bin.to_string_lossy().to_string()),
            error: None,
        },
        None => DependencyStatus {
            name: "ollama".to_string(),
            installed: false,
            version: None,
            path: None,
            error: Some("Ollama not found. Install required.".to_string()),
        },
    }
}

async fn check_ollama_model() -> DependencyStatus {
    let bin = find_ollama_binary().unwrap_or_else(|| PathBuf::from("ollama"));
    match Command::new(&bin).arg("list").output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("Story_v27") {
                DependencyStatus {
                    name: "ollama_model".to_string(),
                    installed: true,
                    version: None,
                    path: None,
                    error: None,
                }
            } else {
                DependencyStatus {
                    name: "ollama_model".to_string(),
                    installed: false,
                    version: None,
                    path: None,
                    error: Some(
                        "Story_v27 model not found. Run setup to pull and create it.".to_string(),
                    ),
                }
            }
        }
        Err(e) => DependencyStatus {
            name: "ollama_model".to_string(),
            installed: false,
            version: None,
            path: None,
            error: Some(format!("Cannot check ollama models: {}", e)),
        },
    }
}

async fn check_comfyui(app: &AppHandle) -> DependencyStatus {
    let dir = comfyui_dir(app);

    // Locally cloned?
    if dir.join("main.py").exists() {
        return DependencyStatus {
            name: "comfyui".to_string(),
            installed: true,
            version: None,
            path: Some(dir.to_string_lossy().to_string()),
            error: None,
        };
    }

    // Running (maybe installed elsewhere)?
    let running = reqwest::Client::new()
        .get("http://127.0.0.1:8188/system_stats")
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if running {
        DependencyStatus {
            name: "comfyui".to_string(),
            installed: true,
            version: None,
            path: None,
            error: None,
        }
    } else {
        DependencyStatus {
            name: "comfyui".to_string(),
            installed: false,
            version: None,
            path: None,
            error: Some("ComfyUI not found at expected path. Install required.".to_string()),
        }
    }
}

fn check_comfyui_torch(app: &AppHandle) -> DependencyStatus {
    let python = comfyui_dir(app).join("venv").join("Scripts").join("python.exe");

    if !python.exists() {
        return DependencyStatus {
            name: "comfyui_torch".to_string(),
            installed: false,
            version: None,
            path: None,
            error: Some("ComfyUI venv not found. Install ComfyUI first.".to_string()),
        };
    }

    // Check for CUDA first, then DirectML (AMD/Windows), then report CPU-only as not ready
    let script = r#"
import sys
try:
    import torch
    if torch.cuda.is_available():
        print('cuda')
        print(torch.__version__)
        sys.exit(0)
    try:
        import torch_directml
        cnt = torch_directml.device_count()
        if cnt > 0:
            print('directml')
            print(str(cnt) + ' device(s)')
            sys.exit(0)
    except Exception:
        pass
    print('cpu')
    print(torch.__version__)
except Exception as e:
    print('error')
    print(str(e))
"#;

    match Command::new(&python).args(["-c", script]).output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = stdout.trim().lines().collect();
            let backend = lines.first().copied().unwrap_or("error");
            let detail = lines.get(1).map(|s| s.to_string());

            match backend {
                "cuda" => DependencyStatus {
                    name: "comfyui_torch".to_string(),
                    installed: true,
                    version: detail.map(|v| format!("torch {v} (CUDA)")),
                    path: None,
                    error: None,
                },
                "directml" => DependencyStatus {
                    name: "comfyui_torch".to_string(),
                    installed: true,
                    version: detail.map(|v| format!("DirectML — {v}")),
                    path: None,
                    error: None,
                },
                _ => DependencyStatus {
                    name: "comfyui_torch".to_string(),
                    installed: false,
                    version: detail,
                    path: None,
                    error: Some(
                        "PyTorch has no GPU backend (CPU-only). \
                         Reinstall with CUDA (NVIDIA) or DirectML (AMD)."
                            .to_string(),
                    ),
                },
            }
        }
        _ => DependencyStatus {
            name: "comfyui_torch".to_string(),
            installed: false,
            version: None,
            path: None,
            error: Some("Cannot check PyTorch in ComfyUI venv.".to_string()),
        },
    }
}

async fn check_checkpoints(app: &AppHandle) -> Vec<DependencyStatus> {
    let dir = comfyui_dir(app).join("models").join("checkpoints");

    [
        (
            "checkpoint_juggernaut",
            "juggernautXL_ragnarokBy.safetensors",
        ),
        ("checkpoint_animagine", "animagine-xl-3.1.safetensors"),
    ]
    .iter()
    .map(|(name, filename)| {
        let path = dir.join(filename);
        if path.exists() {
            DependencyStatus {
                name: name.to_string(),
                installed: true,
                version: None,
                path: Some(path.to_string_lossy().to_string()),
                error: None,
            }
        } else {
            DependencyStatus {
                name: name.to_string(),
                installed: false,
                version: None,
                path: None,
                error: Some(format!("{} not found in checkpoints/", filename)),
            }
        }
    })
    .collect()
}

async fn check_custom_nodes(app: &AppHandle) -> Vec<DependencyStatus> {
    let base = comfyui_dir(app);

    let items: &[(&str, PathBuf, &str)] = &[
        (
            "custom_node_ipadapter",
            base.join("custom_nodes").join("ComfyUI_IPAdapter_plus"),
            "ComfyUI-IPAdapter-Plus not cloned",
        ),
        (
            "ipadapter_faceid_model",
            base.join("models")
                .join("ipadapter")
                .join("ip-adapter-faceid-plusv2_sdxl.bin"),
            "IP-Adapter FaceID model not found",
        ),
        (
            "insightface_buffalo_l",
            base.join("models")
                .join("insightface")
                .join("models")
                .join("buffalo_l"),
            "InsightFace buffalo_l model not found",
        ),
        (
            "clip_vision",
            base.join("models")
                .join("clip_vision")
                .join("CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors"),
            "CLIP-ViT-H-14 model not found",
        ),
        (
            "ipadapter_faceid_lora",
            base.join("models")
                .join("loras")
                .join("ip-adapter-faceid-plusv2_sdxl_lora.safetensors"),
            "IP-Adapter FaceID LoRA not found",
        ),
    ];

    items
        .iter()
        .map(|(name, path, err_msg)| {
            let exists = if path.extension().is_some() {
                // It's a file
                path.exists()
            } else {
                // It's a directory — check it exists and is non-empty
                path.is_dir()
                    && path
                        .read_dir()
                        .map(|mut d| d.next().is_some())
                        .unwrap_or(false)
            };

            if exists {
                DependencyStatus {
                    name: name.to_string(),
                    installed: true,
                    version: None,
                    path: Some(path.to_string_lossy().to_string()),
                    error: None,
                }
            } else {
                DependencyStatus {
                    name: name.to_string(),
                    installed: false,
                    version: None,
                    path: None,
                    error: Some(err_msg.to_string()),
                }
            }
        })
        .collect()
}

// =============================================================================
// Install functions
// =============================================================================

async fn install_ollama(app: &AppHandle) -> Result<(), String> {
    let installer = std::env::temp_dir().join("OllamaSetup.exe");

    download_file_with_progress(
        app,
        "https://ollama.com/download/OllamaSetup.exe",
        &installer,
        "ollama",
    )
    .await?;

    emit_progress(app, "ollama", 0.9, "Running installer silently…", false);

    let status = Command::new(&installer)
        .arg("/S") // Inno/NSIS silent flag
        .status()
        .map_err(|e| format!("Failed to launch Ollama installer: {}", e))?;

    if !status.success() {
        return Err(format!(
            "Ollama installer exited with code: {:?}",
            status.code()
        ));
    }

    emit_progress(app, "ollama", 0.95, "Starting Ollama service…", false);

    // After install the binary is in LOCALAPPDATA; find it rather than relying on PATH
    let ollama_bin = find_ollama_binary().unwrap_or_else(|| PathBuf::from("ollama"));
    Command::new(&ollama_bin)
        .arg("serve")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start ollama serve: {}", e))?;

    // Wait up to 30 s for the API to come up
    let client = reqwest::Client::new();
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if client
            .get("http://localhost:11434/api/tags")
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok()
        {
            break;
        }
    }

    emit_progress(app, "ollama", 1.0, "Ollama installed and running", false);
    Ok(())
}

async fn install_ollama_model(app: &AppHandle) -> Result<(), String> {
    let bin = find_ollama_binary()
        .ok_or_else(|| "Ollama binary not found — install Ollama first.".to_string())?;

    emit_progress(
        app,
        "ollama_model",
        0.0,
        "Pulling jaahas/qwen3.5-uncensored:9b base model (this will take a while)…",
        false,
    );

    let status = Command::new(&bin)
        .args(["pull", "jaahas/qwen3.5-uncensored:9b"])
        .status()
        .map_err(|e| format!("ollama pull failed: {}", e))?;

    if !status.success() {
        return Err("ollama pull jaahas/qwen3.5-uncensored:9b failed".to_string());
    }

    emit_progress(
        app,
        "ollama_model",
        0.7,
        "Creating Story_v27 from Modelfile…",
        false,
    );

    let mf = modelfile_path(app);
    if !mf.exists() {
        return Err(format!(
            "Modelfile not found at {:?}. Ensure the app bundle is complete.",
            mf
        ));
    }

    let status = Command::new(&bin)
        .args([
            "create",
            "Story_v27",
            "-f",
            mf.to_str().unwrap_or("Modelfile"),
        ])
        .status()
        .map_err(|e| format!("ollama create failed: {}", e))?;

    if !status.success() {
        return Err("ollama create Story_v27 failed".to_string());
    }

    emit_progress(app, "ollama_model", 1.0, "Story_v27 model ready", false);
    Ok(())
}

async fn install_comfyui(app: &AppHandle) -> Result<(), String> {
    let dest = comfyui_dir(app);

    emit_progress(app, "comfyui", 0.0, "Checking Python installation…", false);

    let python_cmd = if Command::new("python")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        "python"
    } else if Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        "python3"
    } else {
        return Err(
            "Python 3.10+ is required but not found. Install from https://www.python.org/"
                .to_string(),
        );
    };

    emit_progress(app, "comfyui", 0.05, "Cloning ComfyUI repository…", false);

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let status = Command::new("git")
        .args([
            "clone",
            "https://github.com/comfyanonymous/ComfyUI.git",
            dest.to_str().unwrap_or("comfyui"),
        ])
        .status()
        .map_err(|e| format!("git clone ComfyUI failed: {}", e))?;

    if !status.success() {
        return Err("git clone ComfyUI failed".to_string());
    }

    emit_progress(
        app,
        "comfyui",
        0.3,
        "Creating Python virtual environment…",
        false,
    );

    let venv = dest.join("venv");
    let status = Command::new(python_cmd)
        .args(["-m", "venv", venv.to_str().unwrap_or("venv")])
        .status()
        .map_err(|e| format!("Failed to create venv: {}", e))?;

    if !status.success() {
        return Err("Failed to create Python virtual environment".to_string());
    }

    let pip = venv.join("Scripts").join("pip.exe");

    emit_progress(
        app,
        "comfyui",
        0.4,
        "Installing ComfyUI Python dependencies…",
        false,
    );

    let status = Command::new(&pip)
        .args(["install", "-r", "requirements.txt"])
        .current_dir(&dest)
        .status()
        .map_err(|e| format!("pip install requirements failed: {}", e))?;

    if !status.success() {
        return Err("pip install -r requirements.txt failed".to_string());
    }

    let gpu = detect_gpu();
    let (torch_args, torch_label): (Vec<&str>, &str) = match gpu.vendor.as_str() {
        "amd" => (
            vec!["install", "torch-directml"],
            "PyTorch DirectML (AMD)",
        ),
        _ => (
            vec!["install", "torch", "torchvision", "torchaudio", "--index-url", "https://download.pytorch.org/whl/cu128"],
            "PyTorch CUDA 12.8 (NVIDIA)",
        ),
    };

    emit_progress(
        app,
        "comfyui",
        0.7,
        &format!("Detected {} GPU — installing {}…", gpu.name, torch_label),
        false,
    );

    let status = Command::new(&pip)
        .args(&torch_args)
        .status()
        .map_err(|e| format!("PyTorch install failed: {}", e))?;

    if !status.success() {
        return Err(format!("{} installation failed", torch_label));
    }

    emit_progress(app, "comfyui", 1.0, "ComfyUI installed successfully", false);
    Ok(())
}

async fn install_checkpoint_juggernaut(app: &AppHandle) -> Result<(), String> {
    let dest = comfyui_dir(app)
        .join("models")
        .join("checkpoints")
        .join("juggernautXL_ragnarokBy.safetensors");

    let url = "https://civitai.com/api/download/models/1759168?type=Model&format=SafeTensor&size=full&fp=fp16";

    download_file_with_progress(app, url, &dest, "checkpoint_juggernaut").await?;
    emit_progress(
        app,
        "checkpoint_juggernaut",
        1.0,
        "juggernautXL checkpoint installed",
        false,
    );
    Ok(())
}

async fn install_checkpoint_animagine(app: &AppHandle) -> Result<(), String> {
    let dest = comfyui_dir(app)
        .join("models")
        .join("checkpoints")
        .join("animagine-xl-3.1.safetensors");

    download_file_with_progress(
        app,
        "https://huggingface.co/cagliostrolab/animagine-xl-3.1/resolve/main/animagine-xl-3.1.safetensors",
        &dest,
        "checkpoint_animagine",
    )
    .await?;

    emit_progress(
        app,
        "checkpoint_animagine",
        1.0,
        "animagine-xl-3.1 checkpoint installed",
        false,
    );
    Ok(())
}

async fn install_custom_node_ipadapter(app: &AppHandle) -> Result<(), String> {
    let dest = comfyui_dir(app)
        .join("custom_nodes")
        .join("ComfyUI_IPAdapter_plus");

    emit_progress(
        app,
        "custom_node_ipadapter",
        0.0,
        "Cloning ComfyUI-IPAdapter-Plus…",
        false,
    );

    std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;

    let status = Command::new("git")
        .args([
            "clone",
            "https://github.com/cubiq/ComfyUI_IPAdapter_plus.git",
            dest.to_str().unwrap_or("ComfyUI_IPAdapter_plus"),
        ])
        .status()
        .map_err(|e| format!("git clone IPAdapter failed: {}", e))?;

    if !status.success() {
        return Err("git clone ComfyUI-IPAdapter-Plus failed".to_string());
    }

    let pip = comfyui_dir(app).join("venv").join("Scripts").join("pip.exe");
    if pip.exists() {
        emit_progress(
            app,
            "custom_node_ipadapter",
            0.5,
            "Installing insightface Python package…",
            false,
        );
        let status = Command::new(&pip)
            .args(["install", "insightface", "onnxruntime"])
            .status()
            .map_err(|e| format!("pip install insightface failed: {}", e))?;
        if !status.success() {
            return Err("pip install insightface onnxruntime failed".to_string());
        }
    }

    emit_progress(
        app,
        "custom_node_ipadapter",
        1.0,
        "ComfyUI-IPAdapter-Plus installed",
        false,
    );
    Ok(())
}

async fn install_ipadapter_models(app: &AppHandle) -> Result<(), String> {
    let dest = comfyui_dir(app)
        .join("models")
        .join("ipadapter")
        .join("ip-adapter-faceid-plusv2_sdxl.bin");

    download_file_with_progress(
        app,
        "https://huggingface.co/h94/IP-Adapter-FaceID/resolve/main/ip-adapter-faceid-plusv2_sdxl.bin",
        &dest,
        "ipadapter_faceid_model",
    )
    .await?;

    emit_progress(
        app,
        "ipadapter_faceid_model",
        1.0,
        "IP-Adapter FaceID model installed",
        false,
    );
    Ok(())
}

async fn install_ipadapter_lora(app: &AppHandle) -> Result<(), String> {
    let dest = comfyui_dir(app)
        .join("models")
        .join("loras")
        .join("ip-adapter-faceid-plusv2_sdxl_lora.safetensors");

    download_file_with_progress(
        app,
        "https://huggingface.co/h94/IP-Adapter-FaceID/resolve/main/ip-adapter-faceid-plusv2_sdxl_lora.safetensors",
        &dest,
        "ipadapter_faceid_lora",
    )
    .await?;

    emit_progress(
        app,
        "ipadapter_faceid_lora",
        1.0,
        "IP-Adapter FaceID LoRA installed",
        false,
    );
    Ok(())
}

async fn install_insightface(app: &AppHandle) -> Result<(), String> {
    let models_dir = comfyui_dir(app)
        .join("models")
        .join("insightface")
        .join("models");
    let zip_path = models_dir.join("buffalo_l.zip");
    let extract_dir = models_dir.join("buffalo_l");

    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    download_file_with_progress(
        app,
        "https://github.com/deepinsight/insightface/releases/download/v0.7/buffalo_l.zip",
        &zip_path,
        "insightface_buffalo_l",
    )
    .await?;

    emit_progress(
        app,
        "insightface_buffalo_l",
        0.9,
        "Extracting buffalo_l.zip…",
        false,
    );

    let file =
        std::fs::File::open(&zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {}", e))?;

    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Zip read error: {}", e))?;
        let out = extract_dir.join(entry.name());
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut f =
                std::fs::File::create(&out).map_err(|e| format!("Cannot create file: {}", e))?;
            std::io::copy(&mut entry, &mut f).map_err(|e| format!("Extract error: {}", e))?;
        }
    }

    let _ = std::fs::remove_file(&zip_path);

    emit_progress(
        app,
        "insightface_buffalo_l",
        1.0,
        "InsightFace buffalo_l installed",
        false,
    );
    Ok(())
}

async fn install_clip_vision(app: &AppHandle) -> Result<(), String> {
    let dest = comfyui_dir(app)
        .join("models")
        .join("clip_vision")
        .join("CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors");

    // The CLIP-ViT-H-14 encoder used by IP-Adapter (SD 1.5 / SDXL compatible)
    download_file_with_progress(
        app,
        "https://huggingface.co/h94/IP-Adapter/resolve/main/models/image_encoder/model.safetensors",
        &dest,
        "clip_vision",
    )
    .await?;

    emit_progress(
        app,
        "clip_vision",
        1.0,
        "CLIP-ViT-H-14 model installed",
        false,
    );
    Ok(())
}

async fn install_comfyui_torch(app: &AppHandle) -> Result<(), String> {
    let comfyui = comfyui_dir(app);
    let pip = comfyui.join("venv").join("Scripts").join("pip.exe");

    if !pip.exists() {
        return Err(format!(
            "ComfyUI venv not found. Install ComfyUI first.\nExpected pip at: {:?}",
            pip
        ));
    }

    emit_progress(app, "comfyui_torch", 0.01, "Upgrading pip prerequisites…", false);
    let _ = Command::new(&pip)
        .args(["install", "--upgrade", "typing-extensions", "pip"])
        .status();

    let gpu = detect_gpu();

    // AMD on Windows → DirectML (PyTorch ROCm is Linux-only)
    // NVIDIA / unknown → CUDA 12.8
    let (pip_args, label, manual_cmd) = match gpu.vendor.as_str() {
        "amd" => (
            vec![
                "install".to_string(),
                "torch-directml".to_string(),
            ],
            format!("torch-directml for AMD GPU ({})", gpu.name),
            "pip install torch-directml".to_string(),
        ),
        _ => (
            vec![
                "install".to_string(),
                "torch".to_string(),
                "torchvision".to_string(),
                "torchaudio".to_string(),
                "--index-url".to_string(),
                "https://download.pytorch.org/whl/cu128".to_string(),
                "--force-reinstall".to_string(),
            ],
            format!("PyTorch CUDA 12.8 for {} ({})", gpu.vendor.to_uppercase(), gpu.name),
            "pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu128 --force-reinstall".to_string(),
        ),
    };

    emit_progress(
        app,
        "comfyui_torch",
        0.02,
        &format!("Detected {} — installing {}…", gpu.name, label),
        false,
    );

    let mut child = Command::new(&pip)
        .args(&pip_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run pip: {}", e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_out = app.clone();
    let stdout_thread = std::thread::spawn(move || {
        if let Some(out) = stdout {
            use std::io::{BufRead, BufReader};
            let mut line_count = 0u32;
            for line in BufReader::new(out).lines().flatten() {
                line_count += 1;
                let pct = (0.05_f32 + line_count as f32 * 0.002).min(0.85);
                emit_progress(&app_out, "comfyui_torch", pct, &line, false);
            }
        }
    });

    let app_err = app.clone();
    let stderr_thread = std::thread::spawn(move || {
        if let Some(err) = stderr {
            use std::io::{BufRead, BufReader};
            for line in BufReader::new(err).lines().flatten() {
                if !line.trim().is_empty() && !line.contains('\r') {
                    emit_progress(&app_err, "comfyui_torch", 0.5, &line, false);
                }
            }
        }
    });

    let status = child.wait().map_err(|e| format!("pip wait error: {}", e))?;
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    if !status.success() {
        return Err(format!(
            "PyTorch installation failed.\n\
             You can install it manually:\n\
             1. Open a terminal in your ComfyUI folder\n\
             2. Run: venv\\Scripts\\activate\n\
             3. Run: {}",
            manual_cmd
        ));
    }

    emit_progress(app, "comfyui_torch", 1.0, &format!("{} installed successfully", label), false);
    Ok(())
}

// =============================================================================
// Tauri commands
// =============================================================================

#[tauri::command]
pub async fn check_setup_status(app: AppHandle) -> Result<SetupStatus, String> {
    let ollama = check_ollama().await;
    let ollama_model = check_ollama_model().await;
    let comfyui = check_comfyui(&app).await;
    let comfyui_torch = check_comfyui_torch(&app);
    let checkpoints = check_checkpoints(&app).await;
    let custom_nodes = check_custom_nodes(&app).await;
    let gpu_info = detect_gpu();

    let all_ready = ollama.installed
        && ollama_model.installed
        && comfyui.installed
        && comfyui_torch.installed
        && checkpoints.iter().all(|d| d.installed)
        && custom_nodes.iter().all(|d| d.installed);

    Ok(SetupStatus {
        ollama,
        ollama_model,
        comfyui,
        comfyui_torch,
        checkpoints,
        custom_nodes,
        gpu_info,
        all_ready,
    })
}

#[tauri::command]
pub async fn install_dependency(name: String, app: AppHandle) -> Result<(), String> {
    let result = match name.as_str() {
        "ollama" => install_ollama(&app).await,
        "ollama_model" => install_ollama_model(&app).await,
        "comfyui" => install_comfyui(&app).await,
        "comfyui_torch" => install_comfyui_torch(&app).await,
        "checkpoint_juggernaut" => install_checkpoint_juggernaut(&app).await,
        "checkpoint_animagine" => install_checkpoint_animagine(&app).await,
        "custom_node_ipadapter" => install_custom_node_ipadapter(&app).await,
        "ipadapter_faceid_model" => install_ipadapter_models(&app).await,
        "ipadapter_faceid_lora" => install_ipadapter_lora(&app).await,
        "insightface_buffalo_l" => install_insightface(&app).await,
        "clip_vision" => install_clip_vision(&app).await,
        _ => Err(format!("Unknown dependency: {}", name)),
    };

    if let Err(ref e) = result {
        emit_progress(&app, &name, 0.0, e, true);
    }

    result
}

#[tauri::command]
pub async fn install_all_dependencies(app: AppHandle) -> Result<(), String> {
    let all_steps: &[(&str, &str)] = &[
        ("ollama", "Ollama LLM runtime"),
        ("ollama_model", "Story_v27 model"),
        ("comfyui", "ComfyUI image backend"),
        ("comfyui_torch", "PyTorch CUDA for ComfyUI"),
        ("checkpoint_juggernaut", "JuggernautXL checkpoint"),
        ("checkpoint_animagine", "Animagine XL checkpoint"),
        ("custom_node_ipadapter", "IPAdapter custom node"),
        ("ipadapter_faceid_model", "IPAdapter FaceID model"),
        ("ipadapter_faceid_lora", "IPAdapter FaceID LoRA"),
        ("insightface_buffalo_l", "InsightFace buffalo_l"),
        ("clip_vision", "CLIP vision model"),
    ];

    // Check what's already installed so we don't repeat work
    let current = check_setup_status(app.clone()).await?;
    let is_installed = |name: &str| -> bool {
        match name {
            "ollama" => current.ollama.installed,
            "ollama_model" => current.ollama_model.installed,
            "comfyui" => current.comfyui.installed,
            "comfyui_torch" => current.comfyui_torch.installed,
            _ => current
                .checkpoints
                .iter()
                .chain(current.custom_nodes.iter())
                .any(|d| d.name == name && d.installed),
        }
    };

    let pending: Vec<_> = all_steps
        .iter()
        .filter(|(name, _)| !is_installed(name))
        .collect();

    if pending.is_empty() {
        emit_progress(&app, "all", 1.0, "All dependencies already installed!", false);
        return Ok(());
    }

    let total = pending.len() as f32;

    for (i, (name, label)) in pending.iter().enumerate() {
        let overall = i as f32 / total;
        emit_progress(
            &app,
            "all",
            overall,
            &format!("Installing {} ({}/{})", label, i + 1, pending.len()),
            false,
        );

        if let Err(e) = install_dependency(name.to_string(), app.clone()).await {
            emit_progress(
                &app,
                "all",
                overall,
                &format!("Failed: {} — {}", label, e),
                true,
            );
            return Err(format!("Setup failed at '{}': {}", label, e));
        }
    }

    emit_progress(&app, "all", 1.0, "All dependencies installed!", false);
    Ok(())
}
