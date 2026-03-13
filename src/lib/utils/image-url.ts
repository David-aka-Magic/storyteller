// src/lib/utils/image-url.ts
// Converts absolute file paths to displayable data URIs via read_file_base64.
// Avoids depending on Tauri's asset protocol (which is not enabled in this app).
// Results are cached in memory to avoid re-reading unchanged files.

import { invoke } from '@tauri-apps/api/core';

const cache = new Map<string, string>();

/**
 * Convert an absolute file path to a base64 data URI suitable for <img src>.
 * Returns null if the path is empty or the file cannot be read.
 */
export async function filePathToDataUrl(path: string): Promise<string | null> {
  if (!path) return null;

  const cached = cache.get(path);
  if (cached) return cached;

  try {
    const b64: string = await invoke('read_file_base64', { path });
    const ext = path.split('.').pop()?.toLowerCase() ?? 'png';
    const mime = ext === 'jpg' || ext === 'jpeg' ? 'image/jpeg' : 'image/png';
    const url = `data:${mime};base64,${b64}`;
    cache.set(path, url);
    return url;
  } catch (e) {
    console.error('[image-url] Failed to read:', path, e);
    return null;
  }
}

/**
 * Remove a path (or all paths) from the cache.
 * Call this before regenerating an image that will be saved to the same file path.
 */
export function clearImageCache(path?: string): void {
  if (path) {
    cache.delete(path);
  } else {
    cache.clear();
  }
}
