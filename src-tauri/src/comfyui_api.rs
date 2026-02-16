// src-tauri/src/comfyui_api.rs
//
// ComfyUI API Integration for StoryEngine
// =========================================
// Rust-side client for the ComfyUI local API. Handles:
//   - Health checking (is ComfyUI reachable?)
//   - Uploading reference images and masks to ComfyUI's /input directory
//   - Building workflow JSON from a template with dynamic node modifications
//   - Queuing prompts via POST /prompt
//   - Polling GET /history/{prompt_id} for completion
//   - Downloading generated images via GET /view
//
// All public functions are exposed as Tauri commands so the Svelte frontend
// can call them via `invoke()`.
//
// Architecture note: The existing TypeScript client (comfyui-client.ts) builds
// workflows from scratch in the frontend. This Rust module provides an
// alternative path where the backend handles the full pipeline — useful for
// the orchestrator that chains LLM → mask → ComfyUI → save automatically.

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

use crate::state::OllamaState;

// ============================================================================
// CONFIGURATION
// ============================================================================

const DEFAULT_COMFYUI_URL: &str = "http://127.0.0.1:8188";
const HEALTH_CHECK_TIMEOUT_SECS: u64 = 3;
const POLL_INTERVAL_MS: u64 = 1000;
const DEFAULT_GENERATION_TIMEOUT_SECS: u64 = 180;

// ============================================================================
// TYPES — Request / Response
// ============================================================================

/// A character to include in the scene generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInput {
    pub name: String,
    /// Absolute path to the IP-Adapter reference image on disk.
    pub reference_image_path: String,
    /// Region string (e.g. "left", "center", "right-seated").
    pub region: String,
    /// Character-specific prompt additions (expression, clothing, action).
    pub prompt: String,
}

/// Full request to generate a scene image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenRequest {
    /// Scene description for the positive prompt.
    pub scene_prompt: String,
    /// Characters to render with IP-Adapter FaceID.
    pub characters: Vec<CharacterInput>,
    /// Path to the color mask PNG (from mask_generator).
    pub mask_path: String,
    /// Path to the base workflow JSON template.
    pub workflow_template: String,
    /// Optional: override the ComfyUI base URL.
    #[serde(default)]
    pub comfyui_url: Option<String>,
    /// Optional: random seed (default: random).
    #[serde(default)]
    pub seed: Option<i64>,
    /// Optional: number of sampling steps.
    #[serde(default)]
    pub steps: Option<u32>,
    /// Optional: CFG scale.
    #[serde(default)]
    pub cfg: Option<f64>,
    /// Optional: image width.
    #[serde(default)]
    pub width: Option<u32>,
    /// Optional: image height.
    #[serde(default)]
    pub height: Option<u32>,
    /// Optional: negative prompt override.
    #[serde(default)]
    pub negative_prompt: Option<String>,
    /// Optional: timeout in seconds for generation polling.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Result of a successful image generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenResult {
    /// ComfyUI prompt ID for this job.
    pub prompt_id: String,
    /// Absolute paths to downloaded output images on disk.
    pub image_paths: Vec<String>,
    /// URLs to view images directly from ComfyUI (for preview).
    pub image_urls: Vec<String>,
}

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
struct ComfyOutputImage {
    filename: String,
    #[serde(default)]
    subfolder: String,
    #[serde(default = "default_image_type")]
    r#type: String,
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
// INTERNAL API FUNCTIONS
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

/// Load a workflow JSON template from disk.
pub fn load_workflow_template(path: &Path) -> Result<Value, ComfyError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        ComfyError::WorkflowLoadFailed(format!("Cannot read {}: {}", path.display(), e))
    })?;

    serde_json::from_str(&content).map_err(|e| {
        ComfyError::WorkflowLoadFailed(format!("Invalid JSON in {}: {}", path.display(), e))
    })
}

/// Modify a workflow JSON in-place to set node inputs.
///
/// `modifications` maps node_id → (input_key → new_value).
/// Example: `{ "2": { "text": "cozy coffee shop" }, "4": { "width": 1152 } }`
pub fn modify_workflow(
    workflow: &mut Value,
    modifications: &HashMap<String, HashMap<String, Value>>,
) -> Result<(), ComfyError> {
    let obj = workflow
        .as_object_mut()
        .ok_or_else(|| ComfyError::WorkflowLoadFailed("Workflow root is not an object".into()))?;

    for (node_id, inputs) in modifications {
        if let Some(node) = obj.get_mut(node_id) {
            if let Some(node_inputs) = node.get_mut("inputs") {
                if let Some(input_obj) = node_inputs.as_object_mut() {
                    for (key, value) in inputs {
                        input_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        } else {
            println!(
                "[ComfyUI] Warning: node '{}' not found in workflow, skipping modifications",
                node_id
            );
        }
    }

    Ok(())
}

/// Queue a workflow prompt for execution.
/// Returns the prompt_id on success.
pub async fn queue_prompt(base_url: &str, workflow: &Value) -> Result<String, ComfyError> {
    let client = comfy_client(15);
    let url = format!("{}/prompt", base_url);

    let payload = serde_json::json!({
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
                    // Outputs exist but no images found in any node
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

// ============================================================================
// HIGH-LEVEL PIPELINE
// ============================================================================

/// Run the full image generation pipeline:
///   1. Check ComfyUI health
///   2. Upload reference images + mask
///   3. Load & modify workflow template
///   4. Queue the prompt
///   5. Poll for completion
///   6. Download output images
///
/// This is the main function the orchestrator should call.
pub async fn generate_scene_image(
    request: &ImageGenRequest,
    output_dir: &Path,
) -> Result<ImageGenResult, ComfyError> {
    let base_url = request
        .comfyui_url
        .as_deref()
        .unwrap_or(DEFAULT_COMFYUI_URL);

    // 1. Health check
    let status = check_comfyui_health(base_url).await;
    if !status.running {
        return Err(ComfyError::NotRunning(
            status.error.unwrap_or_else(|| "ComfyUI is not reachable".into()),
        ));
    }
    println!("[ComfyUI] Connected to {}", base_url);

    // 2. Upload reference images and mask
    let mut uploaded_refs: Vec<String> = Vec::new();
    for (i, character) in request.characters.iter().enumerate() {
        let ref_path = Path::new(&character.reference_image_path);
        let upload_name = format!(
            "ref_{}_{}.png",
            character.name.to_lowercase().replace(' ', "_"),
            i
        );
        let stored_name = upload_image_to_comfyui(base_url, ref_path, &upload_name).await?;
        uploaded_refs.push(stored_name);
        println!("[ComfyUI] Uploaded reference for '{}': {}", character.name, upload_name);
    }

    let mask_stored = if !request.mask_path.is_empty() {
        let mask_path = Path::new(&request.mask_path);
        let mask_name = upload_image_to_comfyui(base_url, mask_path, "scene_mask.png").await?;
        println!("[ComfyUI] Uploaded mask: {}", mask_name);
        Some(mask_name)
    } else {
        None
    };

    // 3. Load and modify workflow
    let template_path = Path::new(&request.workflow_template);
    let mut workflow = load_workflow_template(template_path)?;

    let modifications = build_workflow_modifications(request, &uploaded_refs, &mask_stored);
    modify_workflow(&mut workflow, &modifications)?;
    println!("[ComfyUI] Workflow prepared with {} modifications", modifications.len());

    // 4. Queue
    let prompt_id = queue_prompt(base_url, &workflow).await?;
    println!("[ComfyUI] Queued prompt: {}", prompt_id);

    // 5. Poll for completion
    let timeout = request.timeout_secs.unwrap_or(DEFAULT_GENERATION_TIMEOUT_SECS);
    let output_images = poll_for_completion(base_url, &prompt_id, timeout).await?;
    println!(
        "[ComfyUI] Generation complete! {} image(s) produced",
        output_images.len()
    );

    // 6. Download images
    let mut local_paths: Vec<String> = Vec::new();
    let mut view_urls: Vec<String> = Vec::new();

    for img in &output_images {
        let local_path = download_image(base_url, img, output_dir).await?;
        local_paths.push(local_path.to_string_lossy().to_string());

        let view_url = format!(
            "{}/view?filename={}&subfolder={}&type={}",
            base_url,
            urlencoding::encode(&img.filename),
            urlencoding::encode(&img.subfolder),
            urlencoding::encode(&img.r#type),
        );
        view_urls.push(view_url);
    }

    Ok(ImageGenResult {
        prompt_id,
        image_paths: local_paths,
        image_urls: view_urls,
    })
}

/// Build node modifications for the workflow template based on the request.
///
/// This creates a mapping of node_id → { input_key: value } that will
/// be injected into the workflow JSON. Node IDs must match the template.
///
/// This function uses the same node ID conventions as comfyui-client.ts:
///   - "2" = positive prompt, "3" = negative prompt, "4" = empty latent
///   - "50","51","52" = character reference image loaders
///   - "60","61","62" = mask image loaders
///   - "35" = KSampler (seed, steps, cfg)
fn build_workflow_modifications(
    request: &ImageGenRequest,
    uploaded_refs: &[String],
    mask_filename: &Option<String>,
) -> HashMap<String, HashMap<String, Value>> {
    let mut mods: HashMap<String, HashMap<String, Value>> = HashMap::new();

    // --- Positive prompt ---
    let mut positive_inputs = HashMap::new();
    positive_inputs.insert("text".to_string(), Value::String(request.scene_prompt.clone()));
    mods.insert("2".to_string(), positive_inputs);

    // --- Negative prompt ---
    if let Some(neg) = &request.negative_prompt {
        let mut neg_inputs = HashMap::new();
        neg_inputs.insert("text".to_string(), Value::String(neg.clone()));
        mods.insert("3".to_string(), neg_inputs);
    }

    // --- Empty latent dimensions ---
    if request.width.is_some() || request.height.is_some() {
        let mut latent_inputs = HashMap::new();
        if let Some(w) = request.width {
            latent_inputs.insert("width".to_string(), Value::Number(w.into()));
        }
        if let Some(h) = request.height {
            latent_inputs.insert("height".to_string(), Value::Number(h.into()));
        }
        mods.insert("4".to_string(), latent_inputs);
    }

    // --- KSampler ---
    let mut ksampler_inputs = HashMap::new();
    if let Some(seed) = request.seed {
        ksampler_inputs.insert(
            "seed".to_string(),
            Value::Number(serde_json::Number::from(seed)),
        );
    }
    if let Some(steps) = request.steps {
        ksampler_inputs.insert("steps".to_string(), Value::Number(steps.into()));
    }
    if let Some(cfg) = request.cfg {
        if let Some(n) = serde_json::Number::from_f64(cfg) {
            ksampler_inputs.insert("cfg".to_string(), Value::Number(n));
        }
    }
    if !ksampler_inputs.is_empty() {
        mods.insert("35".to_string(), ksampler_inputs);
    }

    // --- Character reference images (nodes 50, 51, 52) ---
    let ref_node_ids = ["50", "51", "52"];
    for (i, ref_filename) in uploaded_refs.iter().enumerate() {
        if i >= ref_node_ids.len() {
            break;
        }
        let mut inputs = HashMap::new();
        inputs.insert("image".to_string(), Value::String(ref_filename.clone()));
        mods.insert(ref_node_ids[i].to_string(), inputs);
    }

    // --- Mask images (nodes 60, 61, 62) ---
    if let Some(mask_name) = mask_filename {
        let mask_node_ids = ["60", "61", "62"];
        for i in 0..request.characters.len().min(mask_node_ids.len()) {
            let mut inputs = HashMap::new();
            inputs.insert("image".to_string(), Value::String(mask_name.clone()));
            mods.insert(mask_node_ids[i].to_string(), inputs);
        }
    }

    mods
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Check if ComfyUI is running and reachable.
///
/// Frontend: `await invoke('check_comfyui_status', { url: 'http://...' })`
#[tauri::command]
pub async fn check_comfyui_status(url: Option<String>) -> Result<ComfyUIStatus, String> {
    let base_url = url.as_deref().unwrap_or(DEFAULT_COMFYUI_URL);
    Ok(check_comfyui_health(base_url).await)
}

/// Upload an image file to ComfyUI's input directory.
///
/// Frontend: `await invoke('upload_to_comfyui', { filePath: '...', uploadName: '...' })`
#[tauri::command]
pub async fn upload_to_comfyui(
    file_path: String,
    upload_name: String,
    url: Option<String>,
) -> Result<String, String> {
    let base_url = url.as_deref().unwrap_or(DEFAULT_COMFYUI_URL);
    let path = Path::new(&file_path);
    upload_image_to_comfyui(base_url, path, &upload_name)
        .await
        .map_err(|e| e.to_string())
}

/// Queue a raw workflow JSON for execution.
///
/// Frontend: `await invoke('queue_comfyui_prompt', { workflow: {...} })`
#[tauri::command]
pub async fn queue_comfyui_prompt(
    workflow: Value,
    url: Option<String>,
) -> Result<String, String> {
    let base_url = url.as_deref().unwrap_or(DEFAULT_COMFYUI_URL);
    queue_prompt(base_url, &workflow)
        .await
        .map_err(|e| e.to_string())
}

/// Poll for a prompt's completion and get the output image info.
///
/// Frontend: `await invoke('poll_comfyui_result', { promptId: '...', timeoutSecs: 120 })`
#[tauri::command]
pub async fn poll_comfyui_result(
    prompt_id: String,
    timeout_secs: Option<u64>,
    url: Option<String>,
) -> Result<Vec<Value>, String> {
    let base_url = url.as_deref().unwrap_or(DEFAULT_COMFYUI_URL);
    let timeout = timeout_secs.unwrap_or(DEFAULT_GENERATION_TIMEOUT_SECS);

    let images = poll_for_completion(base_url, &prompt_id, timeout)
        .await
        .map_err(|e| e.to_string())?;

    // Return as serializable values
    images
        .iter()
        .map(|img| serde_json::to_value(img).map_err(|e| e.to_string()))
        .collect()
}

/// Download a generated image from ComfyUI to local disk.
///
/// Frontend: `await invoke('download_comfyui_image', { filename: '...', subfolder: '', imageType: 'output' })`
#[tauri::command]
pub async fn download_comfyui_image(
    filename: String,
    subfolder: Option<String>,
    image_type: Option<String>,
    url: Option<String>,
    app: AppHandle,
) -> Result<String, String> {
    let base_url = url.as_deref().unwrap_or(DEFAULT_COMFYUI_URL);

    let img = ComfyOutputImage {
        filename,
        subfolder: subfolder.unwrap_or_default(),
        r#type: image_type.unwrap_or_else(|| "output".into()),
    };

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let output_dir = app_data.join("generated_images");

    let path = download_image(base_url, &img, &output_dir)
        .await
        .map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

/// Full pipeline: generate a scene image from a request.
/// This is the main command the frontend orchestrator calls.
///
/// Frontend: `await invoke('generate_comfyui_scene', { request: {...} })`
#[tauri::command]
pub async fn generate_comfyui_scene(
    request: ImageGenRequest,
    app: AppHandle,
) -> Result<ImageGenResult, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let output_dir = app_data.join("generated_images");

    generate_scene_image(&request, &output_dir)
        .await
        .map_err(|e| e.to_string())
}

/// Read a file as raw bytes (used by frontend to upload images).
/// Returns a Vec<u8> that the frontend can use to construct a Blob.
#[tauri::command]
pub fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&path).map_err(|e| format!("Cannot read {}: {}", path, e))
}

/// Read a file as base64 string (useful for displaying images in the UI).
#[tauri::command]
pub fn read_file_base64(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Cannot read {}: {}", path, e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_modify_workflow_sets_inputs() {
        let mut workflow = json!({
            "2": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["1", 1],
                    "text": "placeholder"
                }
            },
            "35": {
                "class_type": "KSampler",
                "inputs": {
                    "seed": 0,
                    "steps": 20
                }
            }
        });

        let mut mods = HashMap::new();
        let mut prompt_mod = HashMap::new();
        prompt_mod.insert("text".to_string(), json!("cozy coffee shop, warm lighting"));
        mods.insert("2".to_string(), prompt_mod);

        let mut sampler_mod = HashMap::new();
        sampler_mod.insert("seed".to_string(), json!(42));
        sampler_mod.insert("steps".to_string(), json!(30));
        mods.insert("35".to_string(), sampler_mod);

        modify_workflow(&mut workflow, &mods).unwrap();

        assert_eq!(workflow["2"]["inputs"]["text"], "cozy coffee shop, warm lighting");
        assert_eq!(workflow["35"]["inputs"]["seed"], 42);
        assert_eq!(workflow["35"]["inputs"]["steps"], 30);
        // Untouched inputs preserved
        assert_eq!(workflow["2"]["inputs"]["clip"], json!(["1", 1]));
    }

    #[test]
    fn test_modify_workflow_missing_node_is_warning_not_error() {
        let mut workflow = json!({
            "2": {
                "class_type": "CLIPTextEncode",
                "inputs": { "text": "hello" }
            }
        });

        let mut mods = HashMap::new();
        let mut missing_mod = HashMap::new();
        missing_mod.insert("text".to_string(), json!("world"));
        mods.insert("999".to_string(), missing_mod);

        // Should not error, just warn
        let result = modify_workflow(&mut workflow, &mods);
        assert!(result.is_ok());
        // Original untouched
        assert_eq!(workflow["2"]["inputs"]["text"], "hello");
    }

    #[test]
    fn test_build_workflow_modifications_basic() {
        let request = ImageGenRequest {
            scene_prompt: "test scene".to_string(),
            characters: vec![CharacterInput {
                name: "Alice".to_string(),
                reference_image_path: "/path/to/alice.png".to_string(),
                region: "left".to_string(),
                prompt: "smiling".to_string(),
            }],
            mask_path: "/path/to/mask.png".to_string(),
            workflow_template: "template.json".to_string(),
            comfyui_url: None,
            seed: Some(123),
            steps: Some(25),
            cfg: Some(5.5),
            width: Some(1152),
            height: Some(896),
            negative_prompt: None,
            timeout_secs: None,
        };

        let uploaded_refs = vec!["ref_alice_0.png".to_string()];
        let mask_filename = Some("scene_mask.png".to_string());

        let mods = build_workflow_modifications(&request, &uploaded_refs, &mask_filename);

        // Positive prompt set
        assert_eq!(mods["2"]["text"], json!("test scene"));
        // KSampler
        assert_eq!(mods["35"]["seed"], json!(123));
        assert_eq!(mods["35"]["steps"], json!(25));
        // Latent
        assert_eq!(mods["4"]["width"], json!(1152));
        assert_eq!(mods["4"]["height"], json!(896));
        // Ref image
        assert_eq!(mods["50"]["image"], json!("ref_alice_0.png"));
        // Mask
        assert_eq!(mods["60"]["image"], json!("scene_mask.png"));
    }
}