// src-tauri/src/models.rs
//
// Data models for StoryEngine
// Extended with multi-story character support

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chat {
    pub id: i64,
    pub title: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub character_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatResponse {
    pub id: i64,
    pub title: String,
}

/// Extended CharacterProfile with multi-story support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CharacterProfile {
    pub id: i64,
    #[serde(default)]
    pub story_id: Option<i64>,           // NEW: Links character to a story
    pub name: String,
    pub age: Option<i32>,
    pub gender: Option<String>,
    pub skin_tone: Option<String>,
    pub hair_style: Option<String>,
    pub hair_color: Option<String>,
    pub body_type: Option<String>,
    pub personality: Option<String>,
    pub additional_notes: Option<String>,
    #[serde(default)]
    pub default_clothing: Option<String>, // NEW: Default clothing for scene generation
    pub sd_prompt: Option<String>,
    #[serde(default)]
    pub image: Option<String>,            // Base64 preview image
    #[serde(default)]
    pub master_image_path: Option<String>, // NEW: File path for IP-Adapter reference
    #[serde(default)]
    pub seed: Option<i64>,
    #[serde(default)]
    pub art_style: Option<String>,
}

/// Lightweight lookup result for LLM integration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CharacterLookup {
    pub id: i64,
    pub name: String,
    pub master_image_path: Option<String>,
    pub sd_prompt: Option<String>,
    pub default_clothing: Option<String>,
    pub art_style: Option<String>,
    pub gender: Option<String>,
}

/// Scene character from LLM output (matches your Ollama model's JSON)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneCharacter {
    pub name: String,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub view: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub clothing: Option<String>,
    #[serde(default)]
    pub facing: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoryPremise {
    pub id: i64,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SdJson {
    pub name: String,
    pub view: String,
    #[serde(default)]
    pub features: String,
    #[serde(default)]
    pub action_context: String,
    #[serde(default)]
    pub clothing: String,
    pub look: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct StoryResponse {
    pub story: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sd_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sd_details: Option<SdJson>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SDRequest {
    pub prompt: String,
    pub negative_prompt: String,
    pub steps: u32,
    pub width: u32,
    pub height: u32,
    pub cfg_scale: f32,
    pub sampler_name: String,
    pub batch_size: u32,
    #[serde(default)]
    pub seed: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SDResponse {
    pub images: Vec<String>,
    pub info: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Img2ImgRequest {
    pub prompt: String,
    pub negative_prompt: String,
    pub init_images: Vec<String>,
    pub denoising_strength: f32,
    pub steps: u32,
    pub width: u32,
    pub height: u32,
    pub cfg_scale: f32,
    pub sampler_name: String,
    pub batch_size: u32,
}