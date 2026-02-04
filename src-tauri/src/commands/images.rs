use tauri::{AppHandle, State};
use serde_json::json;
use crate::state::OllamaState;
use crate::models::{SDRequest, SDResponse, Img2ImgRequest};

// --- Helpers ---

async fn switch_model_if_needed(client: &reqwest::Client, style: &str) -> Result<(), String> {
    let model_filename = match style {
        "Anime" => "animagineXLV31_v31.safetensors",
        "Realistic" => "juggernautXL_ragnarokBy.safetensors",
        "3D" => "juggernautXL_ragnarokBy.safetensors",
        "Painting" => "juggernautXL_ragnarokBy.safetensors",
        "Sketch" => "juggernautXL_ragnarokBy.safetensors",
        _ => return Ok(()),
    };

    let url = "http://127.0.0.1:7860/sdapi/v1/options";
    let payload = json!({ "sd_model_checkpoint": model_filename });

    let res = client.post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to SD Options: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Failed to switch model: {}", res.status()));
    }

    Ok(())
}

fn get_style_prompts(style: &str) -> (String, String) {
    match style {
        "Anime" => (", anime style, key visual, vibrant, cel shaded, studio ghibli".to_string(), "photorealistic, 3d, realistic".to_string()),
        "Realistic" => (", photorealistic, raw photo, 8k uhd, dslr, soft lighting, high fidelity".to_string(), "drawing, anime, sketch, cartoon, graphic, text, painting".to_string()),
        "3D" => (", 3d render, unreal engine 5, octane render, ray tracing".to_string(), "sketch, 2d, flat, drawing, anime".to_string()),
        "Painting" => (", digital painting, oil painting, heavy strokes, concept art".to_string(), "photorealistic, 3d, camera, photo".to_string()),
        "Sketch" => (", pencil sketch, graphite, monochrome, rough lines".to_string(), "color, 3d, photo, bright".to_string()),
        _ => ("".to_string(), "".to_string())
    }
}

// --- Commands ---

#[tauri::command]
pub async fn generate_image(
    prompt: String,
    chat_id: u64,
    msg_index: usize,
    character_id: Option<String>,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<String, String> {
    
    let (seed, art_style) = {
        let chars = state.characters.lock().map_err(|e| e.to_string())?;
        let target_char = if let Some(id) = character_id {
            chars.iter().find(|c| c.id == id).or(chars.first())
        } else {
            chars.first()
        };

        if let Some(c) = target_char {
            let style = if c.art_style.is_empty() { "Realistic".to_string() } else { c.art_style.clone() };
            (c.seed.unwrap_or(-1), style)
        } else {
            (-1, "Realistic".to_string())
        }
    };

    let client = reqwest::Client::new();
    switch_model_if_needed(&client, &art_style).await?;

    let url = "http://127.0.0.1:7860/sdapi/v1/txt2img";
    let (style_suffix, style_negative) = get_style_prompts(&art_style);
    let default_neg = "low quality, bad anatomy, worst quality, text, watermark, signature, ugly, deformed";
    let combined_negative = format!("{}, {}", default_neg, style_negative);

    let payload = SDRequest {
        prompt: format!("{}{}", prompt, style_suffix),
        negative_prompt: combined_negative,
        steps: 25,
        width: 1024,
        height: 1024,
        cfg_scale: 7.0,
        sampler_name: "Euler a".to_string(),
        batch_size: 1,
        seed: seed,
    };

    let res = client.post(url).json(&payload).send().await.map_err(|e| format!("SD API Error: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("SD Generation Failed: {}", res.status()));
    }

    let sd_res: SDResponse = res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;
    let base64_image = sd_res.images.first().ok_or("No image returned")?.clone();

    {
        let mut chats = state.chats.lock().map_err(|e| e.to_string())?;
        if let Some(chat) = chats.iter_mut().find(|c| c.id == chat_id) {
            if let Some(msg) = chat.messages.get_mut(msg_index) {
                msg.images = Some(vec![base64_image.clone()]);
            } else {
                return Err(format!("Save failed: Message index {} out of bounds", msg_index));
            }
        } else {
            return Err(format!("Save failed: Chat ID {} not found", chat_id));
        }
    } 

    state.save(&app)?;
    Ok(base64_image)
}

#[tauri::command]
pub async fn generate_image_variation(
    prompt: String,
    source_image: String,
    chat_id: u64,
    msg_index: usize,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<String, String> {
    let url = "http://127.0.0.1:7860/sdapi/v1/img2img";

    let payload = Img2ImgRequest {
        prompt: prompt.clone(),
        negative_prompt: "low quality, bad anatomy, worst quality, text, watermark, signature, ugly, deformed".to_string(),
        init_images: vec![source_image],
        denoising_strength: 0.75,
        steps: 25,
        width: 1024,
        height: 1024,
        cfg_scale: 7.0,
        sampler_name: "Euler a".to_string(),
        batch_size: 1,
    };

    let client = reqwest::Client::new();
    let res = client.post(url).json(&payload).send().await.map_err(|e| format!("Failed to connect: {}", e))?;

    if !res.status().is_success() { return Err(format!("SD Error: {}", res.status())); }
    let sd_res: SDResponse = res.json().await.map_err(|e| format!("Failed to parse: {}", e))?;
    let base64_image = sd_res.images.first().ok_or("No image")?.clone();

    {
        let mut chats = state.chats.lock().map_err(|e| e.to_string())?;
        if let Some(chat) = chats.iter_mut().find(|c| c.id == chat_id) {
            if let Some(msg) = chat.messages.get_mut(msg_index) {
                if let Some(imgs) = &mut msg.images { imgs[0] = base64_image.clone(); } 
                else { msg.images = Some(vec![base64_image.clone()]); }
            }
        }
    }
    state.save(&app)?;
    Ok(base64_image)
}

#[tauri::command]
pub async fn generate_character_portrait(
    prompt: String,
    style: String,
    state: State<'_, OllamaState>,
) -> Result<(String, String), String> {
    let client = reqwest::Client::new();
    switch_model_if_needed(&client, &style).await?;

    let url = "http://127.0.0.1:7860/sdapi/v1/txt2img";
    let (style_suffix, style_negative) = get_style_prompts(&style);
    let default_neg = "low quality, bad anatomy, text, watermark, signature, ugly, deformed, blurry";
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

    let res = client.post(url).json(&payload).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() { return Err(format!("SD Error: {}", res.status())); }
    
    let sd_res: SDResponse = res.json().await.map_err(|e| e.to_string())?;
    let base64 = sd_res.images.first().ok_or("No image")?.clone();
    let info_json: serde_json::Value = serde_json::from_str(&sd_res.info).map_err(|_| "Failed to parse info")?;
    let seed = info_json["seed"].as_i64().unwrap_or(-1);

    Ok((base64, seed.to_string()))
}