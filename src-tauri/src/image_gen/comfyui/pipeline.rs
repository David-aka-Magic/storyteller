// src-tauri/src/image_gen/comfyui/pipeline.rs
//
// High-level image generation pipeline
// ======================================
// Request/result types and the full upload → queue → poll → download pipeline.

use serde::{Deserialize, Serialize};
use std::path::Path;

use super::client::{
    check_comfyui_health, download_image, poll_for_completion, queue_prompt,
    upload_image_to_comfyui, ComfyError, DEFAULT_COMFYUI_URL, DEFAULT_GENERATION_TIMEOUT_SECS,
};
use super::workflow::{build_workflow_modifications, load_workflow_template, modify_workflow};

// ============================================================================
// REQUEST / RESULT TYPES
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
    /// Per-character mask PNGs. Empty for 1-char workflow.
    /// For 2-char: vec of 2 paths, one per character.
    pub mask_paths: Vec<String>,
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
    /// Optional: pose LoRA filename (e.g. "sitting_pose.safetensors")
    #[serde(default)]
    pub pose_lora_filename: Option<String>,
    /// Optional: pose LoRA strength (0.0-1.0)
    #[serde(default)]
    pub pose_lora_strength: Option<f64>,
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

// ============================================================================
// PIPELINE
// ============================================================================

/// Run the full image generation pipeline:
///   1. Check ComfyUI health
///   2. Upload reference images + mask
///   3. Load & modify workflow template
///   4. Queue the prompt
///   5. Poll for completion
///   6. Download output images
///
/// This is the main function the orchestrator calls.
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

    // 2. Upload reference images
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

    // Upload mask images
    let mut uploaded_masks: Vec<String> = Vec::new();
    for (i, mask_path_str) in request.mask_paths.iter().enumerate() {
        if !mask_path_str.is_empty() {
            let mask_path = Path::new(mask_path_str);
            let mask_upload_name = format!("scene_mask_char{}.png", i);
            let mask_name = upload_image_to_comfyui(base_url, mask_path, &mask_upload_name).await?;
            println!("[ComfyUI] Uploaded mask {}: {}", i, mask_name);
            uploaded_masks.push(mask_name);
        }
    }

    // 3. Load and modify workflow
    let template_path = Path::new(&request.workflow_template);
    let mut workflow = load_workflow_template(template_path)?;

    let modifications = build_workflow_modifications(request, &uploaded_refs, &uploaded_masks);
    modify_workflow(&mut workflow, &modifications)?;
    println!("[ComfyUI] Workflow prepared with {} modifications", modifications.len());

    // Inject pose LoRA if specified
    if let Some(ref lora_file) = request.pose_lora_filename {
        let strength = request.pose_lora_strength.unwrap_or(0.7);
        super::workflow::inject_pose_lora(&mut workflow, lora_file, strength)?;
    }

    println!(
        "[ComfyUI][DEBUG] Final workflow JSON:\n{}",
        serde_json::to_string_pretty(&workflow)
            .unwrap_or_else(|_| "SERIALIZATION_FAILED".to_string())
    );

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
