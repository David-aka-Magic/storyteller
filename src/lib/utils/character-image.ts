// src/lib/utils/character-image.ts
//
// Shared utility for resolving a character's display image.
// Priority: master_image_path (file, high quality) > image (base64) > null
//
// Uses read_file_base64 Tauri command instead of convertFileSrc because the
// Tauri asset protocol (asset.localhost) is not enabled in this app's config.

import { invoke } from '@tauri-apps/api/core';

/**
 * Async version: resolves a character's display image URL.
 * Priority: master_image_path (file) > image (base64) > null (caller shows fallback avatar)
 */
export async function resolveCharacterImageUrl(character: {
  master_image_path?: string | null;
  image?: string | null;
}): Promise<string | null> {
  if (character.master_image_path) {
    try {
      const b64: string = await invoke('read_file_base64', { path: character.master_image_path });
      return `data:image/png;base64,${b64}`;
    } catch {
      // fall through to image field
    }
  }

  if (character.image) {
    if (character.image.startsWith('data:')) return character.image;
    return `data:image/png;base64,${character.image}`;
  }

  return null;
}

/**
 * Sync version for callers that already have a base64 image or just need the fallback check.
 * Does NOT handle master_image_path (file paths require async I/O — use resolveCharacterImageUrl).
 */
export function getCharacterImageUrl(character: {
  master_image_path?: string | null;
  image?: string | null;
}): string | null {
  if (character.image) {
    if (character.image.startsWith('data:')) return character.image;
    return `data:image/png;base64,${character.image}`;
  }
  return null;
}
