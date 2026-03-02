// src-tauri/src/image_gen/comfyui/client.rs
//
// Low-level ComfyUI HTTP operations
// ===================================
// Types, error handling, and raw HTTP calls against the ComfyUI REST API.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

// ============================================================================
// CONFIGURATION
// ============================================================================

pub(super) const DEFAULT_COMFYUI_URL: &str = "http://127.0.0.1:8188";
const HEALTH_CHECK_TIMEOUT_SECS: u64 = 3;
const POLL_INTERVAL_MS: u64 = 1000;
pub(super) const DEFAULT_GENERATION_TIMEOUT_SECS: u64 = 180;

// ============================================================================
// TYPES
// ============================================================================

/// Lightweight status returned by health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComfyUIStatus {
    pub running: bool,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// System stats if available (GPU info, queue size, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_stats: Option<Value>,
}

/// Progress info for a running job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub prompt_id: String,
    pub status: JobStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_node: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    TimedOut,
}

/// Info about a single output image from ComfyUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComfyOutputImage {
    pub filename: String,
    #[serde(default)]
    pub subfolder: String,
    #[serde(default = "default_image_type")]
    pub r#type: String,
}

fn default_image_type() -> String {
    "output".to_string()
}

// ============================================================================
// ERROR TYPE
// ============================================================================

#[derive(Debug)]
pub enum ComfyError {
    NotRunning(String),
    UploadFailed(String),
    QueueFailed(String),
    PollFailed(String),
    GenerationFailed(String),
    Timeout(String),
    DownloadFailed(String),
    WorkflowLoadFailed(String),
    IoError(String),
}

impl std::fmt::Display for ComfyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning(msg) => write!(f, "ComfyUI not running: {}", msg),
            Self::UploadFailed(msg) => write!(f, "Image upload failed: {}", msg),
            Self::QueueFailed(msg) => write!(f, "Failed to queue prompt: {}", msg),
            Self::PollFailed(msg) => write!(f, "Failed to poll status: {}", msg),
            Self::GenerationFailed(msg) => write!(f, "Generation failed: {}", msg),
            Self::Timeout(msg) => write!(f, "Generation timed out: {}", msg),
            Self::DownloadFailed(msg) => write!(f, "Image download failed: {}", msg),
            Self::WorkflowLoadFailed(msg) => write!(f, "Workflow load failed: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl From<ComfyError> for String {
    fn from(e: ComfyError) -> String {
        e.to_string()
    }
}

// ============================================================================
// HTTP HELPERS
// ============================================================================

/// Build a reqwest client with appropriate timeouts for ComfyUI calls.
fn comfy_client(timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Check if ComfyUI is reachable at the given URL.
pub async fn check_comfyui_health(base_url: &str) -> ComfyUIStatus {
    let client = comfy_client(HEALTH_CHECK_TIMEOUT_SECS);
    let url = format!("{}/system_stats", base_url);

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let stats = resp.json::<Value>().await.ok();
            ComfyUIStatus {
                running: true,
                url: base_url.to_string(),
                error: None,
                system_stats: stats,
            }
        }
        Ok(resp) => ComfyUIStatus {
            running: false,
            url: base_url.to_string(),
            error: Some(format!("ComfyUI returned status {}", resp.status())),
            system_stats: None,
        },
        Err(e) => ComfyUIStatus {
            running: false,
            url: base_url.to_string(),
            error: Some(format!(
                "Cannot connect to ComfyUI at {}. Is it running? ({})",
                base_url, e
            )),
            system_stats: None,
        },
    }
}

/// Upload an image file to ComfyUI's /upload/image endpoint.
/// Returns the filename as stored by ComfyUI.
pub async fn upload_image_to_comfyui(
    base_url: &str,
    file_path: &Path,
    upload_name: &str,
) -> Result<String, ComfyError> {
    let client = comfy_client(30);

    let file_bytes = std::fs::read(file_path)
        .map_err(|e| ComfyError::IoError(format!("Cannot read {}: {}", file_path.display(), e)))?;

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(upload_name.to_string())
        .mime_str("image/png")
        .map_err(|e| ComfyError::UploadFailed(e.to_string()))?;

    let form = reqwest::multipart::Form::new()
        .part("image", part)
        .text("overwrite", "true");

    let url = format!("{}/upload/image", base_url);
    let resp = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| ComfyError::UploadFailed(format!("POST /upload/image failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(ComfyError::UploadFailed(format!(
            "Upload returned {}: {}",
            status, body
        )));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| ComfyError::UploadFailed(format!("Invalid upload response: {}", e)))?;

    result["name"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ComfyError::UploadFailed("No 'name' field in upload response".into()))
}

/// Queue a workflow prompt for execution.
/// Returns the prompt_id on success.
pub async fn queue_prompt(base_url: &str, workflow: &Value) -> Result<String, ComfyError> {
    let client = comfy_client(15);
    let url = format!("{}/prompt", base_url);

    let payload = json!({
        "prompt": workflow
    });

    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ComfyError::QueueFailed(format!("POST /prompt failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(ComfyError::QueueFailed(format!(
            "Queue returned {}: {}",
            status, body
        )));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| ComfyError::QueueFailed(format!("Invalid queue response: {}", e)))?;

    result["prompt_id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ComfyError::QueueFailed("No 'prompt_id' in response".into()))
}

/// Poll /history/{prompt_id} until the job completes or times out.
/// Returns the list of output images on success.
pub async fn poll_for_completion(
    base_url: &str,
    prompt_id: &str,
    timeout_secs: u64,
) -> Result<Vec<ComfyOutputImage>, ComfyError> {
    let client = comfy_client(10);
    let url = format!("{}/history/{}", base_url, prompt_id);
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            return Err(ComfyError::Timeout(format!(
                "Generation did not complete within {} seconds for prompt {}",
                timeout_secs, prompt_id
            )));
        }

        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                println!("[ComfyUI] Poll request failed (retrying): {}", e);
                continue;
            }
        };

        if !resp.status().is_success() {
            continue;
        }

        let history: Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Check if our prompt_id exists in the history response
        if let Some(entry) = history.get(prompt_id) {
            // Check for execution error
            if let Some(status) = entry.get("status") {
                if let Some(status_str) = status.get("status_str").and_then(|s| s.as_str()) {
                    if status_str == "error" {
                        let messages = status
                            .get("messages")
                            .and_then(|m| serde_json::to_string(m).ok())
                            .unwrap_or_else(|| "unknown error".to_string());
                        return Err(ComfyError::GenerationFailed(format!(
                            "ComfyUI execution error: {}",
                            messages
                        )));
                    }
                }
            }

            // Look for outputs
            if let Some(outputs) = entry.get("outputs") {
                let mut images = Vec::new();

                // Iterate all output nodes to find images
                if let Some(outputs_obj) = outputs.as_object() {
                    for (_node_id, node_output) in outputs_obj {
                        if let Some(img_list) = node_output.get("images") {
                            if let Some(arr) = img_list.as_array() {
                                for img_val in arr {
                                    if let Ok(img) =
                                        serde_json::from_value::<ComfyOutputImage>(img_val.clone())
                                    {
                                        images.push(img);
                                    }
                                }
                            }
                        }
                    }
                }

                if !images.is_empty() {
                    return Ok(images);
                }

                // Outputs exist but no images — might still be processing
                // or the workflow didn't produce image outputs
                if outputs.as_object().map_or(false, |o| !o.is_empty()) {
                    return Err(ComfyError::GenerationFailed(
                        "Workflow completed but no images were found in outputs".into(),
                    ));
                }
            }
        }
    }
}

/// Download a generated image from ComfyUI and save it to disk.
/// Returns the local file path.
pub async fn download_image(
    base_url: &str,
    image: &ComfyOutputImage,
    output_dir: &Path,
) -> Result<PathBuf, ComfyError> {
    let client = comfy_client(30);

    let url = format!(
        "{}/view?filename={}&subfolder={}&type={}",
        base_url,
        urlencoding::encode(&image.filename),
        urlencoding::encode(&image.subfolder),
        urlencoding::encode(&image.r#type),
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ComfyError::DownloadFailed(format!("GET /view failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(ComfyError::DownloadFailed(format!(
            "Download returned status {}",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ComfyError::DownloadFailed(format!("Failed to read image bytes: {}", e)))?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| ComfyError::IoError(format!("Cannot create output dir: {}", e)))?;

    let output_path = output_dir.join(&image.filename);
    std::fs::write(&output_path, &bytes)
        .map_err(|e| ComfyError::IoError(format!("Cannot write {}: {}", output_path.display(), e)))?;

    Ok(output_path)
}
