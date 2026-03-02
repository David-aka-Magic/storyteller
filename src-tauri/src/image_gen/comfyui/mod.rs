// src-tauri/src/image_gen/comfyui/mod.rs
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
// Sub-modules:
//   client   — low-level HTTP types and operations
//   workflow — workflow template loading and modification
//   pipeline — request/result types and the full generation pipeline
//   commands — #[tauri::command] wrappers for the Svelte frontend

mod client;
mod commands;
mod pipeline;
mod workflow;

// Re-export types that other modules (orchestrator, etc.) need
pub use client::{ComfyError, ComfyOutputImage, ComfyUIStatus};
pub use pipeline::{generate_scene_image, CharacterInput, ImageGenRequest, ImageGenResult};
pub use commands::*;
