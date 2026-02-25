// src-tauri/src/commands/images.rs
//
// Image generation and character commands
// Updated to include new CharacterProfile fields (story_id, default_clothing, master_image_path)
// FIXED: Now reads SD URL from config instead of hardcoded constant

use tauri::State;
use crate::config::ConfigState;
use crate::state::OllamaState;
use crate::models::{CharacterProfile, SDRequest, SDResponse, Img2ImgRequest};
use sqlx::Row;
use serde_json::json;

// ============================================================================
// HELPER: Get SD URL from config (with fallback)
// ============================================================================

fn get_sd_url(config: &State<'_, ConfigState>) -> String {
    config
        .0
        .lock()
        .map(|c| c.sd_api_url.clone())
        .unwrap_or_else(|_| "http://127.0.0.1:7860".to_string())
}

// --- Helper Functions ---

async fn switch_model_if_needed(client: &reqwest::Client, sd_url: &str, style: &str) -> Result<(), String> {
    let model_filename = match style {
        "Anime" => "animagineXLV31_v31.safetensors",
        "Realistic" => "juggernautXL_ragnarokBy.safetensors",
        "3D" => "juggernautXL_ragnarokBy.safetensors",
        "Painting" => "juggernautXL_ragnarokBy.safetensors",
        "Sketch" => "juggernautXL_ragnarokBy.safetensors",
        _ => return Ok(()), // Don't switch for unknown styles
    };

    let url = format!("{}/sdapi/v1/options", sd_url);
    let payload = json!({ "sd_model_checkpoint": model_filename });

    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to SD Options: {}", e))?;

    if !res.status().is_success() {
        // Don't fail if model switch fails - just log and continue with current model
        println!("[SD] Warning: Failed to switch model to {}: {}", model_filename, res.status());
    }

    Ok(())
}

fn get_style_prompts(style: &str) -> (String, String) {
    match style {
        "Anime" => (
            ", anime style, key visual, vibrant, cel shaded, studio ghibli".to_string(),
            "photorealistic, 3d, realistic".to_string()
        ),
        "Realistic" => (
            ", photorealistic, raw photo, 8k uhd, dslr, soft lighting, high fidelity".to_string(),
            "drawing, anime, sketch, cartoon, graphic, text, painting".to_string()
        ),
        "3D" => (
            ", 3d render, unreal engine 5, octane render, ray tracing".to_string(),
            "sketch, 2d, flat, drawing, anime".to_string()
        ),
        "Painting" => (
            ", digital painting, oil painting, heavy strokes, concept art".to_string(),
            "photorealistic, 3d, camera, photo".to_string()
        ),
        "Sketch" => (
            ", pencil sketch, graphite, monochrome, rough lines".to_string(),
            "color, 3d, photo, bright".to_string()
        ),
        _ => ("".to_string(), "".to_string())
    }
}

// ============================================================================
// CHARACTER COMMANDS - Updated with new fields
// ============================================================================

#[tauri::command]
pub async fn save_character(
    character: CharacterProfile,
    state: State<'_, OllamaState>,
) -> Result<i64, String> {
    if character.id > 0 {
        // Update existing - includes all new fields
        sqlx::query(
            "UPDATE characters SET 
                story_id = ?,
                name = ?, 
                age = ?, 
                gender = ?, 
                skin_tone = ?, 
                hair_style = ?, 
                hair_color = ?, 
                body_type = ?, 
                personality = ?, 
                additional_notes = ?, 
                default_clothing = ?,
                sd_prompt = ?,
                image = ?, 
                master_image_path = ?,
                seed = ?, 
                art_style = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?"
        )
        .bind(&character.story_id)
        .bind(&character.name)
        .bind(&character.age)
        .bind(&character.gender)
        .bind(&character.skin_tone)
        .bind(&character.hair_style)
        .bind(&character.hair_color)
        .bind(&character.body_type)
        .bind(&character.personality)
        .bind(&character.additional_notes)
        .bind(&character.default_clothing)
        .bind(&character.sd_prompt)
        .bind(&character.image)
        .bind(&character.master_image_path)
        .bind(&character.seed)
        .bind(&character.art_style)
        .bind(&character.id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
        Ok(character.id)
    } else {
        // Insert new - includes all new fields
        let result = sqlx::query(
            "INSERT INTO characters (
                story_id, name, age, gender, skin_tone, hair_style, hair_color, 
                body_type, personality, additional_notes, default_clothing,
                sd_prompt, image, master_image_path, seed, art_style
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&character.story_id)
        .bind(&character.name)
        .bind(&character.age)
        .bind(&character.gender)
        .bind(&character.skin_tone)
        .bind(&character.hair_style)
        .bind(&character.hair_color)
        .bind(&character.body_type)
        .bind(&character.personality)
        .bind(&character.additional_notes)
        .bind(&character.default_clothing)
        .bind(&character.sd_prompt)
        .bind(&character.image)
        .bind(&character.master_image_path)
        .bind(&character.seed)
        .bind(&character.art_style)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
        Ok(result.last_insert_rowid())
    }
}

#[tauri::command]
pub async fn delete_character(id: i64, state: State<'_, OllamaState>) -> Result<(), String> {
    sqlx::query("DELETE FROM characters WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_character_list(state: State<'_, OllamaState>) -> Result<Vec<CharacterProfile>, String> {
    // Updated to include all new fields
    let rows = sqlx::query(
        "SELECT id, story_id, name, age, gender, skin_tone, hair_style, hair_color, 
                body_type, personality, additional_notes, default_clothing,
                sd_prompt, image, master_image_path, seed, art_style 
         FROM characters ORDER BY name ASC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let characters: Vec<CharacterProfile> = rows
        .iter()
        .map(|row| CharacterProfile {
            id: row.get("id"),
            story_id: row.get("story_id"),
            name: row.get("name"),
            age: row.get("age"),
            gender: row.get("gender"),
            skin_tone: row.get("skin_tone"),
            hair_style: row.get("hair_style"),
            hair_color: row.get("hair_color"),
            body_type: row.get("body_type"),
            personality: row.get("personality"),
            additional_notes: row.get("additional_notes"),
            default_clothing: row.get("default_clothing"),
            sd_prompt: row.get("sd_prompt"),
            image: row.get("image"),
            master_image_path: row.get("master_image_path"),
            seed: row.get("seed"),
            art_style: row.get("art_style"),
        })
        .collect();

    Ok(characters)
}

// ============================================================================
// IMAGE GENERATION COMMANDS
// ============================================================================

#[tauri::command]
pub async fn generate_image(
    prompt: String,
    state: State<'_, OllamaState>,
    config: State<'_, ConfigState>,
) -> Result<(String, String), String> {
    let sd_url = get_sd_url(&config);
    let client = &state.client;
    let url = format!("{}/sdapi/v1/txt2img", sd_url);

    let default_neg = "bad anatomy, bad hands, missing fingers, extra fingers, blurry, low quality";

    let payload = SDRequest {
        prompt,
        negative_prompt: default_neg.to_string(),
        steps: 28,
        width: 832,
        height: 1216,
        cfg_scale: 7.0,
        sampler_name: "Euler a".to_string(),
        batch_size: 1,
        seed: -1,
    };

    let body_bytes = serde_json::to_vec(&payload)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;

    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Content-Length", body_bytes.len().to_string())
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| format!("SD Error: Could not connect to Stable Diffusion at {}. Is it running? ({})", sd_url, e))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("SD Error ({}): {}", status, &err_body[..err_body.len().min(300)]));
    }

    let sd_res: SDResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let base64 = sd_res
        .images
        .first()
        .ok_or("No image returned")?
        .clone();

    let info_json: serde_json::Value = serde_json::from_str(&sd_res.info)
        .map_err(|_| "Failed to parse info JSON")?;

    let seed = info_json["seed"].as_i64().unwrap_or(-1);

    Ok((base64, seed.to_string()))
}

#[tauri::command]
pub async fn generate_image_variation(
    image_base64: String,
    prompt: String,
    state: State<'_, OllamaState>,
    config: State<'_, ConfigState>,
) -> Result<(String, String), String> {
    let sd_url = get_sd_url(&config);
    let client = &state.client;
    let url = format!("{}/sdapi/v1/img2img", sd_url);

    let default_neg = "bad anatomy, bad hands, missing fingers, extra fingers, blurry, low quality";

    let payload = Img2ImgRequest {
        prompt,
        negative_prompt: default_neg.to_string(),
        init_images: vec![image_base64],
        denoising_strength: 0.5,
        steps: 28,
        width: 832,
        height: 1216,
        cfg_scale: 7.0,
        sampler_name: "Euler a".to_string(),
        batch_size: 1,
    };

    let body_bytes = serde_json::to_vec(&payload)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;

    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Content-Length", body_bytes.len().to_string())
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| format!("SD Error: Could not connect to Stable Diffusion at {}. Is it running? ({})", sd_url, e))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("SD Error ({}): {}", status, &err_body[..err_body.len().min(300)]));
    }

    let sd_res: SDResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let base64 = sd_res
        .images
        .first()
        .ok_or("No image returned")?
        .clone();

    let info_json: serde_json::Value = serde_json::from_str(&sd_res.info)
        .map_err(|_| "Failed to parse info JSON")?;

    let seed = info_json["seed"].as_i64().unwrap_or(-1);

    Ok((base64, seed.to_string()))
}

#[tauri::command]
pub async fn generate_character_portrait(
    prompt: String,
    style: String,
    state: State<'_, OllamaState>,
    config: State<'_, ConfigState>,
) -> Result<(String, String), String> {
    let sd_url = get_sd_url(&config);
    let client = &state.client;
    
    // Switch model based on style
    switch_model_if_needed(client, &sd_url, &style).await?;
    
    let url = format!("{}/sdapi/v1/txt2img", sd_url);
    let (style_suffix, style_negative) = get_style_prompts(&style);

    let default_neg = "bad anatomy, bad hands, missing fingers, extra fingers, blurry, low quality";
    let combined_negative = format!("{}, {}", default_neg, style_negative);

    let payload = SDRequest {
        prompt: format!("{}{}", prompt, style_suffix),
        negative_prompt: combined_negative,
        steps: 28,
        width: 832,
        height: 1216,
        cfg_scale: 7.0,
        sampler_name: "Euler a".to_string(),
        batch_size: 1,
        seed: -1,
    };

    // Serialize manually and send with explicit Content-Type + Content-Length
    // (some SD WebUI builds reject chunked transfer encoding from .json())
    let body_bytes = serde_json::to_vec(&payload)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;

    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Content-Length", body_bytes.len().to_string())
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| format!("SD Error: Could not connect to Stable Diffusion at {}. Is it running? ({})", sd_url, e))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("SD Error ({}): {}", status, &err_body[..err_body.len().min(300)]));
    }

    let sd_res: SDResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let base64 = sd_res
        .images
        .first()
        .ok_or("No image returned")?
        .clone();

    let info_json: serde_json::Value = serde_json::from_str(&sd_res.info)
        .map_err(|_| "Failed to parse info JSON")?;

    let seed = info_json["seed"].as_i64().unwrap_or(-1);

    Ok((base64, seed.to_string()))
}

/// Diagnostic command to test SD connectivity from the Rust backend.
#[tauri::command]
pub async fn diagnose_sd_connection(
    state: State<'_, OllamaState>,
    config: State<'_, ConfigState>,
) -> Result<String, String> {
    let sd_url = get_sd_url(&config);
    let client = &state.client;
    let mut report = format!("=== SD Connection Diagnostic ===\nSD URL from config: {}\n\n", sd_url);

    // Test 1: GET base
    match client.get(&sd_url).timeout(std::time::Duration::from_secs(5)).send().await {
        Ok(r) => report.push_str(&format!("✓ GET {} → status {}\n", sd_url, r.status())),
        Err(e) => report.push_str(&format!("✗ GET {} → FAILED: {}\n", sd_url, e)),
    }

    // Test 2: POST with the SAME client (may fail due to HTTP/2)
    let txt2img_url = format!("{}/sdapi/v1/txt2img", sd_url);
    let tiny_payload = serde_json::json!({
        "prompt": "test",
        "negative_prompt": "",
        "steps": 1,
        "width": 64,
        "height": 64,
        "cfg_scale": 1.0,
        "sampler_name": "Euler",
        "batch_size": 1,
        "seed": 42
    });
    let body_bytes = serde_json::to_vec(&tiny_payload).unwrap_or_default();
    
    report.push_str(&format!("\nTest A: POST with default client ({} bytes)...\n", body_bytes.len()));
    match client
        .post(&txt2img_url)
        .header("Content-Type", "application/json")
        .body(body_bytes.clone())
        .send()
        .await
    {
        Ok(r) => report.push_str(&format!("✓ Default client POST → status {}\n", r.status())),
        Err(e) => report.push_str(&format!("✗ Default client POST → FAILED: {:?}\n", e)),
    }

    // Test 3: POST with a FRESH HTTP/1.1-only client
    report.push_str("\nTest B: POST with HTTP/1.1-only client...\n");
    let http11_client = reqwest::Client::builder()
        .http1_only()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    
    match http11_client
        .post(&txt2img_url)
        .header("Content-Type", "application/json")
        .body(body_bytes.clone())
        .send()
        .await
    {
        Ok(r) => {
            let status = r.status();
            if status.is_success() {
                report.push_str(&format!("✓ HTTP/1.1 POST → status {} ← THIS WORKS!\n", status));
            } else {
                let body = r.text().await.unwrap_or_default();
                report.push_str(&format!("✗ HTTP/1.1 POST → status {}: {}\n", status, &body[..body.len().min(300)]));
            }
        },
        Err(e) => report.push_str(&format!("✗ HTTP/1.1 POST → FAILED: {:?}\n", e)),
    }

    // Test 4: POST with a no-proxy client (in case system proxy is interfering)
    report.push_str("\nTest C: POST with no-proxy HTTP/1.1 client...\n");
    let noproxy_client = reqwest::Client::builder()
        .http1_only()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    
    match noproxy_client
        .post(&txt2img_url)
        .header("Content-Type", "application/json")
        .body(body_bytes)
        .send()
        .await
    {
        Ok(r) => {
            let status = r.status();
            if status.is_success() {
                report.push_str(&format!("✓ No-proxy HTTP/1.1 POST → status {} ← THIS WORKS!\n", status));
            } else {
                let body = r.text().await.unwrap_or_default();
                report.push_str(&format!("✗ No-proxy POST → status {}: {}\n", status, &body[..body.len().min(300)]));
            }
        },
        Err(e) => report.push_str(&format!("✗ No-proxy POST → FAILED: {:?}\n", e)),
    }

    // Samplers + model info
    let samplers_url = format!("{}/sdapi/v1/samplers", sd_url);
    match client.get(&samplers_url).timeout(std::time::Duration::from_secs(5)).send().await {
        Ok(r) => {
            if let Ok(samplers) = r.json::<serde_json::Value>().await {
                let names: Vec<String> = samplers.as_array()
                    .map(|arr| arr.iter().filter_map(|s| s["name"].as_str().map(String::from)).collect())
                    .unwrap_or_default();
                report.push_str(&format!("\nSamplers: {}\n", names.join(", ")));
            }
        },
        Err(_) => {},
    }

    let options_url = format!("{}/sdapi/v1/options", sd_url);
    match client.get(&options_url).timeout(std::time::Duration::from_secs(5)).send().await {
        Ok(r) => {
            if let Ok(opts) = r.json::<serde_json::Value>().await {
                let model = opts["sd_model_checkpoint"].as_str().unwrap_or("unknown");
                report.push_str(&format!("Model: {}\n", model));
            }
        },
        Err(_) => {},
    }

    Ok(report)
}