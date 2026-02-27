// src/lib/comfyui/mask-generator.ts
//
// Dynamic attention mask generation for ComfyUI IP-Adapter FaceID workflow.
// Generates per-character black/white PNG masks based on region assignments.
// These masks tell each FaceID node which part of the image to influence.

import { invoke } from '@tauri-apps/api/core';

/**
 * Region definitions for character placement.
 * Each region defines what portion of the canvas (1152x896) the character occupies.
 * x, y are top-left corner. w, h are dimensions. All values as fractions (0-1).
 */
const REGION_PRESETS: Record<string, { x: number; y: number; w: number; h: number }> = {
  // Standing positions
  'left':                { x: 0.00, y: 0.00, w: 0.40, h: 1.00 },
  'center':              { x: 0.25, y: 0.00, w: 0.50, h: 1.00 },
  'right':               { x: 0.60, y: 0.00, w: 0.40, h: 1.00 },

  // Seated positions (lower portion, slightly wider)
  'left-seated':         { x: 0.00, y: 0.30, w: 0.45, h: 0.70 },
  'center-seated':       { x: 0.20, y: 0.30, w: 0.60, h: 0.70 },
  'right-seated':        { x: 0.55, y: 0.30, w: 0.45, h: 0.70 },

  // Background positions (smaller, upper area)
  'left-background':     { x: 0.05, y: 0.05, w: 0.25, h: 0.50 },
  'center-background':   { x: 0.35, y: 0.05, w: 0.30, h: 0.50 },
  'right-background':    { x: 0.70, y: 0.05, w: 0.25, h: 0.50 },

  // Two-character simple split
  'left-half':           { x: 0.00, y: 0.00, w: 0.50, h: 1.00 },
  'right-half':          { x: 0.50, y: 0.00, w: 0.50, h: 1.00 },

  // Three-character split
  'left-third':          { x: 0.00, y: 0.00, w: 0.35, h: 1.00 },
  'center-third':        { x: 0.30, y: 0.00, w: 0.40, h: 1.00 },
  'right-third':         { x: 0.65, y: 0.00, w: 0.35, h: 1.00 },
};

/**
 * Canvas dimensions matching the workflow's EmptyLatentImage
 */
const CANVAS_WIDTH = 1024;
const CANVAS_HEIGHT = 576;

/**
 * Generate a mask image as a base64-encoded PNG.
 * White (255) = character region, Black (0) = not this character.
 * 
 * Uses canvas API in the frontend, or can call a Rust backend command.
 */
export function generateMaskBase64(
  region: string,
  canvasWidth: number = CANVAS_WIDTH,
  canvasHeight: number = CANVAS_HEIGHT,
  feather: number = 20
): string {
  const canvas = document.createElement('canvas');
  canvas.width = canvasWidth;
  canvas.height = canvasHeight;
  const ctx = canvas.getContext('2d')!;

  // Start with black (no influence)
  ctx.fillStyle = '#000000';
  ctx.fillRect(0, 0, canvasWidth, canvasHeight);

  // Get region bounds
  const preset = REGION_PRESETS[region];
  if (!preset) {
    console.warn(`Unknown region "${region}", using full canvas`);
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(0, 0, canvasWidth, canvasHeight);
    return canvas.toDataURL('image/png').split(',')[1];
  }

  const x = Math.round(preset.x * canvasWidth);
  const y = Math.round(preset.y * canvasHeight);
  const w = Math.round(preset.w * canvasWidth);
  const h = Math.round(preset.h * canvasHeight);

  // Draw white region with optional feathering
  if (feather > 0) {
    // Create gradient edges for smoother blending
    const gradient = ctx.createLinearGradient(x, 0, x + feather, 0);
    
    // Simple approach: draw solid white then apply feathered edges
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(x + feather, y, w - feather * 2, h);

    // Left feather
    const leftGrad = ctx.createLinearGradient(x, 0, x + feather, 0);
    leftGrad.addColorStop(0, 'rgba(255,255,255,0)');
    leftGrad.addColorStop(1, 'rgba(255,255,255,1)');
    ctx.fillStyle = leftGrad;
    ctx.fillRect(x, y, feather, h);

    // Right feather
    const rightGrad = ctx.createLinearGradient(x + w - feather, 0, x + w, 0);
    rightGrad.addColorStop(0, 'rgba(255,255,255,1)');
    rightGrad.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = rightGrad;
    ctx.fillRect(x + w - feather, y, feather, h);

    // Top feather
    const topGrad = ctx.createLinearGradient(0, y, 0, y + feather);
    topGrad.addColorStop(0, 'rgba(255,255,255,0)');
    topGrad.addColorStop(1, 'rgba(255,255,255,1)');
    ctx.fillStyle = topGrad;
    ctx.fillRect(x + feather, y, w - feather * 2, feather);

    // Bottom feather
    const bottomGrad = ctx.createLinearGradient(0, y + h - feather, 0, y + h);
    bottomGrad.addColorStop(0, 'rgba(255,255,255,1)');
    bottomGrad.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = bottomGrad;
    ctx.fillRect(x + feather, y + h - feather, w - feather * 2, feather);
  } else {
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(x, y, w, h);
  }

  return canvas.toDataURL('image/png').split(',')[1];
}

/**
 * Generate mask PNG file and save to ComfyUI's input directory.
 * Calls the Rust backend to write the file.
 */
export async function generateAndSaveMask(
  region: string,
  filename: string,
  comfyInputDir: string,
  canvasWidth: number = CANVAS_WIDTH,
  canvasHeight: number = CANVAS_HEIGHT
): Promise<string> {
  const base64 = generateMaskBase64(region, canvasWidth, canvasHeight);
  
  const filePath = await invoke<string>('save_mask_image', {
    base64Data: base64,
    filename,
    outputDir: comfyInputDir,
  });
  
  return filePath;
}

/**
 * Auto-assign regions based on number of characters and their scene data.
 * Used when the LLM doesn't provide specific region assignments.
 */
export function autoAssignRegions(
  characters: { name: string; region?: string; view?: string }[]
): { name: string; region: string }[] {
  const count = characters.length;

  // If all characters already have regions, use them
  if (characters.every(c => c.region && c.region !== 'off-screen')) {
    return characters.map(c => ({ name: c.name, region: c.region! }));
  }

  // Auto-assign based on count
  const defaultRegions: Record<number, string[]> = {
    1: ['center'],
    2: ['left-half', 'right-half'],
    3: ['left-third', 'center-third', 'right-third'],
  };

  const regions = defaultRegions[count] || defaultRegions[3];

  return characters.map((c, i) => ({
    name: c.name,
    region: c.region && c.region !== 'off-screen' 
      ? c.region 
      : regions[i] || 'center',
  }));
}

/**
 * Generate all masks for a scene and save them to ComfyUI's input directory.
 * Returns the filenames for use in the workflow API call.
 */
export async function generateSceneMasks(
  characters: { name: string; region: string }[],
  comfyInputDir: string,
  canvasWidth: number = CANVAS_WIDTH,
  canvasHeight: number = CANVAS_HEIGHT
): Promise<{ charIndex: number; name: string; maskFilename: string }[]> {
  const results = [];

  for (let i = 0; i < characters.length; i++) {
    const char = characters[i];
    const filename = `scene_mask_char${i + 1}.png`;

    await generateAndSaveMask(
      char.region,
      filename,
      comfyInputDir,
      canvasWidth,
      canvasHeight
    );

    results.push({
      charIndex: i,
      name: char.name,
      maskFilename: filename,
    });
  }

  return results;
}

/**
 * Get all available region presets (for UI display or debugging)
 */
export function getAvailableRegions(): string[] {
  return Object.keys(REGION_PRESETS);
}

export { REGION_PRESETS, CANVAS_WIDTH, CANVAS_HEIGHT };