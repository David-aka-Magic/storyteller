// src/lib/api/image-gen.ts — Tauri command wrappers for image generation
import { invoke } from '@tauri-apps/api/core';

export interface MasterPortraitRequest {
  name: string;
  age?: number | null;
  gender?: string | null;
  skin_tone?: string | null;
  hair_color?: string | null;
  hair_style?: string | null;
  body_type?: string | null;
  default_clothing?: string | null;
  physical_features?: string | null;
  art_style?: string | null;
  custom_prompt?: string | null;
  seed?: number | null;
}

export interface SaveMasterPortraitRequest {
  character_id: number;
  selected_index: number;
  image_paths: string[];
  character_name: string;
}

export interface MasterPortraitResult {
  images_base64: string[];
  image_paths: string[];
  prompt_used: string;
  seed: number;
  prompt_id: string;
}

export async function generateMasterPortrait(request: MasterPortraitRequest): Promise<MasterPortraitResult> {
  return invoke('generate_master_portrait', { request });
}

export async function generateCharacterPortrait(prompt: string, style?: string): Promise<string> {
  return invoke('generate_character_portrait', { prompt, style });
}

export async function readFileBase64(path: string): Promise<string> {
  return invoke('read_file_base64', { path });
}

export async function generateImage(prompt: string, negativePrompt?: string): Promise<string> {
  return invoke('generate_image', { prompt, negativePrompt });
}

export async function generateImageVariation(imagePath: string): Promise<string> {
  return invoke('generate_image_variation', { imagePath });
}

export async function generateComfyScene(request: unknown): Promise<unknown> {
  return invoke('generate_comfyui_scene', { request });
}

export async function generateColorMask(request: unknown): Promise<unknown> {
  return invoke('generate_color_mask', { request });
}

export async function diagnoseSDConnection(): Promise<unknown> {
  return invoke('diagnose_sd_connection');
}

export async function checkComfyUIStatus(): Promise<unknown> {
  return invoke('check_comfyui_status');
}

export async function previewPortraitPrompt(request: MasterPortraitRequest): Promise<string> {
  return invoke('preview_portrait_prompt', { request });
}

export async function saveMasterPortrait(request: SaveMasterPortraitRequest): Promise<string> {
  return invoke('save_master_portrait', { request });
}
