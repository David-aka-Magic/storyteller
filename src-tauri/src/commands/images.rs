// src-tauri/src/commands/images.rs
//
// Image generation and character commands
// Updated to include new CharacterProfile fields (story_id, default_clothing, master_image_path)

use tauri::State;
use crate::state::OllamaState;
use crate::models::{CharacterProfile, SDRequest, SDResponse, Img2ImgRequest};
use sqlx::Row;
use serde_json::json;

const SD_URL: &str = "http://127.0.0.1:7860";

// --- Helper Functions ---

async fn switch_model_if_needed(client: &reqwest::Client, style: &str) -> Result<(), String> {
    let model_filename = match style {
        "Anime" => "animagineXLV31_v31.safetensors",
        "Realistic" => "juggernautXL_ragnarokBy.safetensors",
        "3D" => "juggernautXL_ragnarokBy.safetensors",
        "Painting" => "juggernautXL_ragnarokBy.safetensors",
        "Sketch" => "juggernautXL_ragnarokBy.safetensors",
        _ => return Ok(()), // Don't switch for unknown styles
    };

    let url = format!("{}/sdapi/v1/options", SD_URL);
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
pub async fn generate_image(prompt: String, state: State<'_, OllamaState>) -> Result<(String, String), String> {
    let client = &state.client;
    let url = format!("{}/sdapi/v1/txt2img", SD_URL);

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

    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("SD Error: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("SD Error: {}", res.status()));
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
) -> Result<(String, String), String> {
    let client = &state.client;
    let url = format!("{}/sdapi/v1/img2img", SD_URL);

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

    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("SD Error: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("SD Error: {}", res.status()));
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
) -> Result<(String, String), String> {
    let client = &state.client;
    
    // Switch model based on style
    switch_model_if_needed(client, &style).await?;
    
    let url = format!("{}/sdapi/v1/txt2img", SD_URL);
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

    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("SD Error: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("SD Error: {}", res.status()));
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