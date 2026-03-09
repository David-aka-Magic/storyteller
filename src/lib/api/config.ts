// src/lib/api/config.ts — Tauri command wrappers for app configuration
import { invoke } from '@tauri-apps/api/core';

export interface AppConfig {
  sd_webui_path: string;
  ollama_url: string;
  sd_api_url: string;
  auto_start_services: boolean;
  content_rating: 'sfw' | 'nsfw';
  comfyui_path: string;
  setup_completed: boolean;
}

export async function getConfig(): Promise<AppConfig> {
  return invoke('get_config');
}

export async function updateConfig(newConfig: Partial<AppConfig>): Promise<void> {
  return invoke('update_config', { newConfig });
}

export async function setSDPath(path: string): Promise<void> {
  return invoke('set_sd_path', { path });
}
