// src-tauri/src/commands/master_portrait.rs
//
// Master Portrait Generator for StoryEngine
// ============================================
// Generates initial character reference portraits via ComfyUI.
// These "master images" are used by IP-Adapter FaceID to maintain
// character consistency across all future scene generations.
//
// ARCHITECTURE NOTE:
// This module does NOT import private internals from comfyui_api.rs.
// Instead it makes its own HTTP calls to ComfyUI directly, keeping the
// module fully self-contained.  This avoids the `ComfyOutputImage is private`
// compiler error while reusing the same ComfyUI REST API.
//
// Pipeline:
//   1. Build a portrait-optimized prompt from character details
//   2. Send to ComfyUI as a batch of 4 (user picks the best)
//   3. Save the selected image to disk as the master reference
//   4. Update the character database with the master_image_path

use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

use crate::state::OllamaState;

// ============================================================================
// CONFIGURATION
// ============================================================================

const DEFAULT_COMFYUI_URL: &str = "http://127.0.0.1:8188";

/// Recommended settings for master portrait generation.
/// These produce clean, detailed portraits ideal for IP-Adapter reference.
const PORTRAIT_STEPS: u32 = 20;
const PORTRAIT_CFG: f64 = 7.0;
const PORTRAIT_WIDTH: u32 = 832;
const PORTRAIT_HEIGHT: u32 = 1216;
const PORTRAIT_BATCH_SIZE: u32 = 4;
const PORTRAIT_SAMPLER: &str = "euler_ancestral";
const PORTRAIT_SCHEDULER: &str = "normal";
const PORTRAIT_TIMEOUT_SECS: u64 = 300;
const POLL_INTERVAL_MS: u64 = 1000;

// ============================================================================
// TYPES
// ============================================================================

/// Character details used to build the portrait prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPortraitRequest {
    pub name: String,
    #[serde(default)]
    pub age: Option<u32>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub skin_tone: Option<String>,
    #[serde(default)]
    pub hair_color: Option<String>,
    #[serde(default)]
    pub hair_style: Option<String>,
    #[serde(default)]
    pub body_type: Option<String>,
    #[serde(default)]
    pub default_clothing: Option<String>,
    #[serde(default)]
    pub physical_features: Option<String>,
    #[serde(default)]
    pub art_style: Option<String>,
    /// Optional: user-edited prompt override. If set, skips auto-generation.
    #[serde(default)]
    pub custom_prompt: Option<String>,
    /// Optional: specific seed for reproducibility. None or negative = random.
    #[serde(default)]
    pub seed: Option<i64>,
    /// Optional: ComfyUI base URL override.
    #[serde(default)]
    pub comfyui_url: Option<String>,
}

impl Default for MasterPortraitRequest {
    fn default() -> Self {
        Self {
            name: String::new(),
            age: None,
            gender: None,
            skin_tone: None,
            hair_color: None,
            hair_style: None,
            body_type: None,
            default_clothing: None,
            physical_features: None,
            art_style: None,
            custom_prompt: None,
            seed: None,
            comfyui_url: None,
        }
    }
}

/// Result returned after generating portrait options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPortraitResult {
    /// Base64-encoded PNG images (batch of 4).
    pub images_base64: Vec<String>,
    /// File paths on disk for each generated image.
    pub image_paths: Vec<String>,
    /// The prompt that was used.
    pub prompt_used: String,
    /// The negative prompt used.
    pub negative_prompt: String,
    /// Seed used (for reproducibility).
    pub seed: i64,
    /// ComfyUI prompt ID.
    pub prompt_id: String,
}

/// Request to save a selected master portrait.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMasterPortraitRequest {
    /// Character database ID.
    pub character_id: i64,
    /// Index of the selected image (0-3) from the batch.
    pub selected_index: usize,
    /// The image paths returned from generation.
    pub image_paths: Vec<String>,
    /// Optional: the character name (for filename).
    #[serde(default)]
    pub character_name: Option<String>,
}

/// Internal: image info parsed from ComfyUI /history response.
#[derive(Debug, Clone, Deserialize)]
struct OutputImage {
    filename: String,
    #[serde(default)]
    subfolder: String,
    #[serde(default = "default_output_type")]
    r#type: String,
}

fn default_output_type() -> String {
    "output".to_string()
}

// ============================================================================
// PROMPT BUILDER
// ============================================================================

/// Build an optimized portrait prompt from character details.
///
/// The prompt is structured for maximum IP-Adapter compatibility:
///   1. Quality tags first (masterpiece, best quality)
///   2. Subject framing (solo, portrait, upper body)
///   3. Gender/age descriptor
///   4. Physical features in SD-friendly order
///   5. Clothing
///   6. Background and lighting (neutral, studio)
pub fn build_portrait_prompt(request: &MasterPortraitRequest) -> String {
    // If user provided a custom prompt, use it directly
    if let Some(custom) = &request.custom_prompt {
        if !custom.trim().is_empty() {
            return custom.clone();
        }
    }

    let mut parts: Vec<String> = Vec::new();

    // 1. Quality prefix
    parts.push("(masterpiece, best quality)".to_string());

    // 2. Subject + framing
    let gender_tag = match request.gender.as_deref() {
        Some("Female") => "1girl",
        Some("Male") => "1boy",
        _ => "1person",
    };
    parts.push(format!("solo, {}", gender_tag));

    // 3. Portrait framing — critical for IP-Adapter reference
    parts.push("portrait, upper body, looking at viewer".to_string());

    // 4. Age
    if let Some(age) = request.age {
        let age_desc = match age {
            0..=12 => "child".to_string(),
            13..=17 => "teenager".to_string(),
            18..=25 => format!("{} year old young adult", age),
            26..=45 => format!("{} year old", age),
            46..=65 => format!("{} year old middle-aged", age),
            _ => format!("{} year old elderly", age),
        };
        parts.push(age_desc);
    }

    // 5. Physical features
    if let Some(skin) = &request.skin_tone {
        if !skin.is_empty() {
            parts.push(format!("{} skin", skin.to_lowercase()));
        }
    }

    // Hair — color before style for emphasis
    match (&request.hair_color, &request.hair_style) {
        (Some(color), Some(style)) if !color.is_empty() && !style.is_empty() => {
            parts.push(format!(
                "{} {} hair",
                color.to_lowercase(),
                style.to_lowercase()
            ));
        }
        (Some(color), _) if !color.is_empty() => {
            parts.push(format!("{} hair", color.to_lowercase()));
        }
        (_, Some(style)) if !style.is_empty() => {
            parts.push(format!("{} hair", style.to_lowercase()));
        }
        _ => {}
    }

    // Body type
    if let Some(body) = &request.body_type {
        let body_desc = match body.as_str() {
            "Slim" => "slim body",
            "Athletic" => "athletic body, fit",
            "Average" => "normal body",
            "Curvy" => "curvy body",
            "Muscular" => "muscular body",
            "Heavyset" => "large body",
            other => other,
        };
        parts.push(body_desc.to_string());
    }

    // Additional physical features (free text from registration)
    if let Some(features) = &request.physical_features {
        if !features.is_empty() {
            for feat in features.split(',') {
                let trimmed = feat.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_lowercase());
                }
            }
        }
    }

    // 6. Clothing
    if let Some(clothing) = &request.default_clothing {
        if !clothing.is_empty() {
            parts.push(format!("wearing {}", clothing));
        }
    }

    // 7. Background and lighting — neutral for best IP-Adapter reference
    parts.push("neutral gray background".to_string());
    parts.push("detailed face, sharp focus".to_string());
    parts.push("soft studio lighting, rim lighting".to_string());

    parts.join(", ")
}

/// Build a comprehensive negative prompt for portrait generation.
fn build_negative_prompt(art_style: Option<&str>) -> String {
    let base = "bad anatomy, bad hands, missing fingers, extra fingers, \
                blurry, low quality, deformed, mutated, watermark, text, \
                signature, cropped, out of frame, worst quality, low resolution, \
                ugly, duplicate, extra limbs, gross proportions, malformed, \
                poorly drawn face, poorly drawn hands, long neck, extra heads, \
                bad proportions, cloned face, disfigured, fused fingers, \
                too many fingers, unclear eyes, cross-eyed";

    let style_neg = match art_style {
        Some("Anime") => ", photorealistic, 3d, realistic",
        Some("Realistic") => ", drawing, anime, sketch, cartoon, graphic, painting",
        Some("3D") => ", sketch, 2d, flat, drawing, anime",
        Some("Painting") => ", photorealistic, 3d, camera, photo",
        Some("Sketch") => ", color, 3d, photo, bright",
        _ => ", drawing, anime, sketch, cartoon, graphic, painting",
    };

    format!("{}{}", base, style_neg)
}

// ============================================================================
// HTTP HELPERS
// ============================================================================

fn http_client(timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Ping ComfyUI's /system_stats to confirm it's reachable.
async fn check_health(base_url: &str) -> Result<(), String> {
    let client = http_client(5);
    let url = format!("{}/system_stats", base_url);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Cannot reach ComfyUI at {}: {}", base_url, e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("ComfyUI returned status {}", resp.status()))
    }
}

/// POST the workflow JSON to /prompt and return the prompt_id.
async fn queue_workflow(base_url: &str, workflow: &Value) -> Result<String, String> {
    let client = http_client(30);
    let url = format!("{}/prompt", base_url);

    let body = json!({ "prompt": workflow });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("POST /prompt failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Queue returned {}: {}", status, text));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid queue response: {}", e))?;

    result["prompt_id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No prompt_id in queue response".to_string())
}

/// Poll GET /history/{prompt_id} until the job completes or times out.
async fn poll_until_complete(
    base_url: &str,
    prompt_id: &str,
    timeout_secs: u64,
) -> Result<Vec<OutputImage>, String> {
    let client = http_client(30);
    let url = format!("{}/history/{}", base_url, prompt_id);
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        if std::time::Instant::now() > deadline {
            return Err(format!(
                "Timed out after {}s waiting for ComfyUI generation",
                timeout_secs
            ));
        }

        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("GET /history failed: {}", e))?;

        if !resp.status().is_success() {
            continue;
        }

        let history: Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid history response: {}", e))?;

        let entry = match history.get(prompt_id) {
            Some(e) => e,
            None => continue, // Not ready yet
        };

        // Check for errors
        if let Some(status) = entry.get("status") {
            if let Some(msgs) = status.get("messages") {
                if let Some(arr) = msgs.as_array() {
                    for msg in arr {
                        if msg.get(0).and_then(|v| v.as_str()) == Some("execution_error") {
                            let detail = msg.get(1).cloned().unwrap_or(Value::Null);
                            return Err(format!("ComfyUI execution error: {}", detail));
                        }
                    }
                }
            }
        }

        // Look for outputs
        if let Some(outputs) = entry.get("outputs") {
            let mut images = Vec::new();

            if let Some(outputs_obj) = outputs.as_object() {
                for (_node_id, node_output) in outputs_obj {
                    if let Some(img_list) = node_output.get("images") {
                        if let Some(arr) = img_list.as_array() {
                            for img_val in arr {
                                if let Ok(img) =
                                    serde_json::from_value::<OutputImage>(img_val.clone())
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

            // Outputs exist but no images found
            if outputs.as_object().map_or(false, |o| !o.is_empty()) {
                return Err(
                    "Workflow completed but no images were found in outputs".to_string()
                );
            }
        }
    }
}

/// Download a single output image from ComfyUI to local disk.
async fn download_output_image(
    base_url: &str,
    image: &OutputImage,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let client = http_client(30);

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
        .map_err(|e| format!("GET /view failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download returned status {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Cannot create output dir: {}", e))?;

    let output_path = output_dir.join(&image.filename);
    std::fs::write(&output_path, &bytes)
        .map_err(|e| format!("Cannot write {}: {}", output_path.display(), e))?;

    Ok(output_path)
}

// ============================================================================
// WORKFLOW BUILDER
// ============================================================================

/// Build a ComfyUI API-format workflow for portrait generation.
fn build_portrait_workflow(
    prompt: &str,
    negative_prompt: &str,
    seed: i64,
    art_style: Option<&str>,
) -> Value {
    let ckpt_name = match art_style {
        Some("Anime") => "animagine-xl-3.1.safetensors",
        _ => "juggernautXL_ragnarokBy.safetensors",
    };

    json!({
        "1": {
            "class_type": "CheckpointLoaderSimple",
            "inputs": {
                "ckpt_name": ckpt_name
            }
        },
        "2": {
            "class_type": "CLIPTextEncode",
            "inputs": {
                "clip": ["1", 1],
                "text": prompt
            }
        },
        "3": {
            "class_type": "CLIPTextEncode",
            "inputs": {
                "clip": ["1", 1],
                "text": negative_prompt
            }
        },
        "4": {
            "class_type": "EmptyLatentImage",
            "inputs": {
                "width": PORTRAIT_WIDTH,
                "height": PORTRAIT_HEIGHT,
                "batch_size": PORTRAIT_BATCH_SIZE
            }
        },
        "5": {
            "class_type": "KSampler",
            "inputs": {
                "model": ["1", 0],
                "positive": ["2", 0],
                "negative": ["3", 0],
                "latent_image": ["4", 0],
                "seed": seed,
                "steps": PORTRAIT_STEPS,
                "cfg": PORTRAIT_CFG,
                "sampler_name": PORTRAIT_SAMPLER,
                "scheduler": PORTRAIT_SCHEDULER,
                "denoise": 1.0
            }
        },
        "6": {
            "class_type": "VAEDecode",
            "inputs": {
                "samples": ["5", 0],
                "vae": ["1", 2]
            }
        },
        "7": {
            "class_type": "SaveImage",
            "inputs": {
                "images": ["6", 0],
                "filename_prefix": "StoryEngine/master_portrait"
            }
        }
    })
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Generate a batch of 4 master portrait options via ComfyUI.
///
/// Frontend usage:
/// ```typescript
/// const result = await invoke('generate_master_portrait', {
///   request: {
///     name: 'Marcus',
///     age: 28,
///     gender: 'Male',
///     hair_color: 'black',
///     hair_style: 'short',
///     skin_tone: 'warm brown',
///     body_type: 'Athletic',
///     physical_features: 'warm brown eyes, short beard',
///     default_clothing: 'fitted black t-shirt, dark jeans, silver watch',
///     art_style: 'Realistic'
///   }
/// });
/// // result.images_base64 = [img1, img2, img3, img4]
/// ```
#[tauri::command]
pub async fn generate_master_portrait(
    request: MasterPortraitRequest,
    app: AppHandle,
) -> Result<MasterPortraitResult, String> {
    let base_url = request
        .comfyui_url
        .as_deref()
        .unwrap_or(DEFAULT_COMFYUI_URL);

    // 1. Health check
    check_health(base_url).await.map_err(|e| {
        format!(
            "ComfyUI is not running at {}. Please start ComfyUI first. ({})",
            base_url, e
        )
    })?;
    println!("[MasterPortrait] ComfyUI connected at {}", base_url);

    // 2. Build prompts
    let prompt = build_portrait_prompt(&request);
    let negative = build_negative_prompt(request.art_style.as_deref());

    // FIX: ComfyUI requires seed >= 0. Generate a random seed if none provided
    // or if a negative value was passed (old sentinel value).
    let seed = match request.seed {
        Some(s) if s >= 0 => s,
        _ => rand::thread_rng().gen_range(0..i64::MAX),
    };

    println!("[MasterPortrait] Prompt: {}", prompt);
    println!("[MasterPortrait] Seed: {}", seed);
    println!(
        "[MasterPortrait] Generating batch of {} portraits...",
        PORTRAIT_BATCH_SIZE
    );

    // 3. Build workflow
    let workflow =
        build_portrait_workflow(&prompt, &negative, seed, request.art_style.as_deref());

    // 4. Queue prompt
    let prompt_id = queue_workflow(base_url, &workflow).await?;
    println!("[MasterPortrait] Queued prompt: {}", prompt_id);

    // 5. Poll for completion
    let output_images =
        poll_until_complete(base_url, &prompt_id, PORTRAIT_TIMEOUT_SECS).await?;
    println!(
        "[MasterPortrait] Generation complete! {} images",
        output_images.len()
    );

    // 6. Download images to app data directory
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let char_dir_name = request
        .name
        .to_lowercase()
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
    let output_dir = app_data.join("master_portraits").join(&char_dir_name);

    let mut image_paths: Vec<String> = Vec::new();
    let mut images_base64: Vec<String> = Vec::new();

    for img in &output_images {
        let local_path = download_output_image(base_url, img, &output_dir).await?;

        let bytes = std::fs::read(&local_path)
            .map_err(|e| format!("Failed to read image {}: {}", local_path.display(), e))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

        image_paths.push(local_path.to_string_lossy().to_string());
        images_base64.push(b64);
    }

    Ok(MasterPortraitResult {
        images_base64,
        image_paths,
        prompt_used: prompt,
        negative_prompt: negative,
        seed,
        prompt_id,
    })
}

/// Save the selected portrait as the character's master reference image.
///
/// Copies the selected image to a permanent location and updates
/// the character database with the master_image_path.
#[tauri::command]
pub async fn save_master_portrait(
    request: SaveMasterPortraitRequest,
    app: AppHandle,
    state: State<'_, OllamaState>,
) -> Result<String, String> {
    if request.selected_index >= request.image_paths.len() {
        return Err(format!(
            "Invalid selection index {} (only {} images available)",
            request.selected_index,
            request.image_paths.len()
        ));
    }

    let source_path = Path::new(&request.image_paths[request.selected_index]);
    if !source_path.exists() {
        return Err(format!(
            "Selected image not found at: {}",
            source_path.display()
        ));
    }

    // Create permanent storage directory
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let master_dir = app_data.join("character_masters");
    std::fs::create_dir_all(&master_dir)
        .map_err(|e| format!("Failed to create master directory: {}", e))?;

    let char_name_safe = request
        .character_name
        .as_deref()
        .unwrap_or("character")
        .to_lowercase()
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
    let master_filename = format!("{}_master.png", char_name_safe);
    let master_path = master_dir.join(&master_filename);

    std::fs::copy(source_path, &master_path).map_err(|e| {
        format!(
            "Failed to copy master image to {}: {}",
            master_path.display(),
            e
        )
    })?;

    let master_path_str = master_path.to_string_lossy().to_string();

    // Update database
    sqlx::query(
        "UPDATE characters SET master_image_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&master_path_str)
    .bind(request.character_id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to update character master image: {}", e))?;

    println!(
        "[MasterPortrait] Saved master for character {} at: {}",
        request.character_id, master_path_str
    );

    // Clean up temporary batch images (keep only the selected one)
    for (i, path) in request.image_paths.iter().enumerate() {
        if i != request.selected_index {
            let _ = std::fs::remove_file(path);
        }
    }

    Ok(master_path_str)
}

/// Build a portrait prompt from character details without generating.
/// Useful for live preview in the frontend.
#[tauri::command]
pub fn preview_portrait_prompt(request: MasterPortraitRequest) -> String {
    build_portrait_prompt(&request)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_full_details() {
        let req = MasterPortraitRequest {
            name: "Marcus".to_string(),
            age: Some(28),
            gender: Some("Male".to_string()),
            skin_tone: Some("warm brown".to_string()),
            hair_color: Some("black".to_string()),
            hair_style: Some("short".to_string()),
            body_type: Some("Athletic".to_string()),
            default_clothing: Some(
                "fitted black t-shirt, dark jeans, silver watch".to_string(),
            ),
            physical_features: Some("warm brown eyes, short beard".to_string()),
            art_style: Some("Realistic".to_string()),
            custom_prompt: None,
            seed: None,
            comfyui_url: None,
        };

        let prompt = build_portrait_prompt(&req);

        assert!(prompt.contains("masterpiece"));
        assert!(prompt.contains("1boy"));
        assert!(prompt.contains("28 year old"));
        assert!(prompt.contains("warm brown skin"));
        assert!(prompt.contains("black short hair"));
        assert!(prompt.contains("athletic body"));
        assert!(prompt.contains("warm brown eyes"));
        assert!(prompt.contains("short beard"));
        assert!(prompt.contains("fitted black t-shirt"));
        assert!(prompt.contains("neutral gray background"));
        assert!(prompt.contains("detailed face"));
    }

    #[test]
    fn test_build_prompt_minimal() {
        let req = MasterPortraitRequest {
            name: "Unknown".to_string(),
            ..Default::default()
        };

        let prompt = build_portrait_prompt(&req);
        assert!(prompt.contains("masterpiece"));
        assert!(prompt.contains("1person"));
        assert!(prompt.contains("portrait"));
        assert!(prompt.contains("neutral gray background"));
    }

    #[test]
    fn test_custom_prompt_overrides() {
        let req = MasterPortraitRequest {
            name: "Test".to_string(),
            age: Some(25),
            gender: Some("Female".to_string()),
            custom_prompt: Some("my custom portrait prompt here".to_string()),
            ..Default::default()
        };

        let prompt = build_portrait_prompt(&req);
        assert_eq!(prompt, "my custom portrait prompt here");
    }

    #[test]
    fn test_negative_prompt_styles() {
        let realistic = build_negative_prompt(Some("Realistic"));
        assert!(realistic.contains("anime"));
        assert!(realistic.contains("bad anatomy"));

        let anime = build_negative_prompt(Some("Anime"));
        assert!(anime.contains("photorealistic"));

        let default_neg = build_negative_prompt(None);
        assert!(default_neg.contains("anime"));
    }

    #[test]
    fn test_portrait_workflow_structure() {
        let workflow =
            build_portrait_workflow("test prompt", "test negative", 42, Some("Realistic"));

        assert_eq!(workflow["1"]["class_type"], "CheckpointLoaderSimple");
        assert_eq!(workflow["2"]["class_type"], "CLIPTextEncode");
        assert_eq!(workflow["2"]["inputs"]["text"], "test prompt");
        assert_eq!(workflow["5"]["class_type"], "KSampler");
        assert_eq!(workflow["5"]["inputs"]["seed"], 42);
        assert_eq!(workflow["5"]["inputs"]["steps"], PORTRAIT_STEPS);
        assert_eq!(workflow["4"]["inputs"]["batch_size"], PORTRAIT_BATCH_SIZE);
        assert_eq!(workflow["7"]["class_type"], "SaveImage");
    }

    #[test]
    fn test_anime_uses_different_checkpoint() {
        let workflow = build_portrait_workflow("test", "neg", 0, Some("Anime"));
        assert_eq!(
            workflow["1"]["inputs"]["ckpt_name"],
            "animagine-xl-3.1.safetensors"
        );

        let workflow_real = build_portrait_workflow("test", "neg", 0, Some("Realistic"));
        assert_eq!(
            workflow_real["1"]["inputs"]["ckpt_name"],
            "juggernautXL_ragnarokBy.safetensors"
        );
    }

    #[test]
    fn test_seed_is_never_negative() {
        // Simulate the seed resolution logic from generate_master_portrait
        let resolve_seed = |opt: Option<i64>| -> i64 {
            match opt {
                Some(s) if s >= 0 => s,
                _ => 12345, // mock random
            }
        };

        assert_eq!(resolve_seed(Some(42)), 42);
        assert_eq!(resolve_seed(Some(0)), 0);
        assert_eq!(resolve_seed(Some(-1)), 12345); // was the bug
        assert_eq!(resolve_seed(None), 12345);
    }
}