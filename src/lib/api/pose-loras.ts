// src/lib/api/pose-loras.ts
import { invoke } from '@tauri-apps/api/core';
import type { PoseLora } from '$lib/types';

export async function listPoseLoras(): Promise<PoseLora[]> {
  return invoke('list_pose_loras');
}

export async function createPoseLora(
  name: string,
  keywords: string,
  lora_filename: string,
  trigger_words: string,
  strength: number,
): Promise<PoseLora> {
  return invoke('create_pose_lora', { name, keywords, loraFilename: lora_filename, triggerWords: trigger_words, strength });
}

export async function updatePoseLora(
  id: number,
  name: string,
  keywords: string,
  lora_filename: string,
  trigger_words: string,
  strength: number,
  enabled: boolean,
): Promise<void> {
  return invoke('update_pose_lora', { id, name, keywords, loraFilename: lora_filename, triggerWords: trigger_words, strength, enabled });
}

export async function deletePoseLora(id: number): Promise<void> {
  return invoke('delete_pose_lora', { id });
}

export async function seedDefaultPoseLoras(): Promise<void> {
  return invoke('seed_default_pose_loras');
}
