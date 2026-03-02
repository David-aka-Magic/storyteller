// src-tauri/src/image_gen/comfyui/commands.rs
//
// Tauri command wrappers for the ComfyUI API
// ============================================
// Thin #[tauri::command] functions that delegate to client and pipeline.

use base64::Engine;
use serde_json::Value;
use std::path::Path;
use tauri::{AppHandle, Manager};

use super::client::{
    check_comfyui_health, download_image, poll_for_completion, queue_prompt,
    upload_image_to_comfyui, ComfyOutputImage, ComfyUIStatus, DEFAULT_COMFYUI_URL,
    DEFAULT_GENERATION_TIMEOUT_SECS,
};
use super::pipeline::{generate_scene_image, ImageGenRequest, ImageGenResult};

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
