// src/lib/comfyui/comfyui-client.ts
//
// ComfyUI API client for StoryEngine.
// Builds workflow JSON from the v5 template, submits via ComfyUI API,
// and monitors execution progress.
//
// The workflow uses chained IPAdapterFaceID nodes (1-3) with:
// - Per-character attention masks (generated dynamically)
// - FaceID Plus v2 + LoRA for face consistency
// - CLIPVision + InsightFace for face detection

import { invoke } from '@tauri-apps/api/core';
import type { ParsedCharacter } from '../llm-parser-types';
import { generateSceneMasks, autoAssignRegions, CANVAS_WIDTH, CANVAS_HEIGHT } from './mask-generator';

// ============================================================================
// CONFIGURATION
// ============================================================================

const COMFYUI_BASE_URL = 'http://127.0.0.1:8188';
const COMFYUI_INPUT_DIR = 'C:\\Users\\dcarl\\Documents\\ComfyUI\\input';

/**
 * Node IDs in the v5 workflow template.
 * These map to the node IDs in storyengine_ipadapter_v5.json.
 */
const NODE_IDS = {
  checkpoint: '1',
  lora: '5',
  positivePrompt: '2',
  negativePrompt: '3',
  emptyLatent: '4',
  ipadapterLoader: '10',
  clipVisionLoader: '11',
  insightfaceLoader: '12',
  // Character reference images
  charRef: ['20', '21'],
  // Mask images
  maskLoad: ['40', '41'],
  // ImageToMask converters
  maskConvert: ['45', '46'],
  // FaceID nodes
  faceId: ['30', '31'],
  // Sampling & output
  ksampler: '35',
  vaeDecode: '36',
  saveImage: '37',
  preview: '38',
} as const;

// ============================================================================
// TYPES
// ============================================================================

interface ComfyUIPrompt {
  [nodeId: string]: {
    class_type: string;
    inputs: Record<string, any>;
  };
}

interface SceneGenRequest {
  positivePrompt: string;
  negativePrompt?: string;
  characters: {
    name: string;
    region: string;
    referenceImagePath: string;  // Path to master reference image
  }[];
  seed?: number;
  steps?: number;
  cfg?: number;
  width?: number;
  height?: number;
}

interface SceneGenResult {
  images: string[];  // Output image paths
  promptId: string;
}

// ============================================================================
// WORKFLOW BUILDER
// ============================================================================

/**
 * Build a ComfyUI API prompt (workflow) for scene generation.
 * Dynamically includes only the FaceID nodes needed for the character count.
 */
function buildWorkflowPrompt(request: SceneGenRequest, maskFilenames: string[]): ComfyUIPrompt {
  const numChars = request.characters.length;
  const width = request.width || CANVAS_WIDTH;
  const height = request.height || CANVAS_HEIGHT;
  const seed = request.seed ?? Math.floor(Math.random() * 2 ** 53);

  const negativePrompt = request.negativePrompt ||
    '(worst quality, low quality:1.4), (bad anatomy:1.3), (bad hands:1.4), ' +
    '(missing fingers:1.3), (extra fingers:1.3), (too many fingers:1.4), ' +
    '(fused fingers:1.3), (poorly drawn hands:1.4), (floating limbs:1.3), ' +
    '(disconnected limbs:1.3), (extra limbs:1.3), (missing arms:1.2), ' +
    '(extra arms:1.2), (deformed:1.3), (mutated:1.2), (disfigured:1.2), ' +
    'blurry, lowres, watermark, text, signature, cropped, out of frame, ' +
    'ugly, duplicate, cloned face, poorly drawn face, ' +
    '(floating head:1.4), (detached head:1.4), bad proportions, long neck';

  const prompt: ComfyUIPrompt = {};

  // --- Checkpoint ---
  prompt[NODE_IDS.checkpoint] = {
    class_type: 'CheckpointLoaderSimple',
    inputs: { ckpt_name: 'juggernautXL_ragnarokBy.safetensors' },
  };

  // --- LoRA ---
  prompt[NODE_IDS.lora] = {
    class_type: 'LoraLoader',
    inputs: {
      model: [NODE_IDS.checkpoint, 0],
      clip: [NODE_IDS.checkpoint, 1],
      lora_name: 'ip-adapter-faceid-plusv2_sdxl_lora.safetensors',
      strength_model: 0.85,
      strength_clip: 0.85,
    },
  };

  // --- Prompts ---
  prompt[NODE_IDS.positivePrompt] = {
    class_type: 'CLIPTextEncode',
    inputs: {
      clip: [NODE_IDS.lora, 1],
      text: request.positivePrompt,
    },
  };

  prompt[NODE_IDS.negativePrompt] = {
    class_type: 'CLIPTextEncode',
    inputs: {
      clip: [NODE_IDS.lora, 1],
      text: negativePrompt,
    },
  };

  // --- Empty Latent ---
  prompt[NODE_IDS.emptyLatent] = {
    class_type: 'EmptyLatentImage',
    inputs: { width, height, batch_size: 1 },
  };

  // --- Shared loaders ---
  prompt[NODE_IDS.ipadapterLoader] = {
    class_type: 'IPAdapterModelLoader',
    inputs: { ipadapter_file: 'ip-adapter-faceid-plusv2_sdxl.bin' },
  };

  prompt[NODE_IDS.clipVisionLoader] = {
    class_type: 'CLIPVisionLoader',
    inputs: { clip_name: 'CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors' },
  };

  prompt[NODE_IDS.insightfaceLoader] = {
    class_type: 'IPAdapterInsightFaceLoader',
    inputs: { provider: 'CPU' },
  };

  // --- Per-character nodes ---
  let previousModelOutput: [string, number] = [NODE_IDS.lora, 0]; // Start from LoRA model output

  for (let i = 0; i < numChars; i++) {
    const char = request.characters[i];
    const refNodeId = NODE_IDS.charRef[i];
    const maskLoadId = NODE_IDS.maskLoad[i];
    const maskConvertId = NODE_IDS.maskConvert[i];
    const faceIdNodeId = NODE_IDS.faceId[i];

    // Reference image
    prompt[refNodeId] = {
      class_type: 'LoadImage',
      inputs: { image: char.referenceImagePath },
    };

    // Mask image
    prompt[maskLoadId] = {
      class_type: 'LoadImage',
      inputs: { image: maskFilenames[i] },
    };

    // ImageToMask converter
    prompt[maskConvertId] = {
      class_type: 'ImageToMask',
      inputs: {
        image: [maskLoadId, 0],
        channel: 'red',
      },
    };

    // FaceID node
    prompt[faceIdNodeId] = {
      class_type: 'IPAdapterFaceID',
      inputs: {
        model: previousModelOutput,
        ipadapter: [NODE_IDS.ipadapterLoader, 0],
        image: [refNodeId, 0],
        attn_mask: [maskConvertId, 0],
        clip_vision: [NODE_IDS.clipVisionLoader, 0],
        insightface: [NODE_IDS.insightfaceLoader, 0],
        weight: 1.2,
        weight_faceidv2: 1.2,
        weight_type: 'strong middle',
        combine_embeds: 'concat',
        start_at: 0,
        end_at: 1,
        embeds_scaling: 'V only',
      },
    };

    // Chain: this node's model output feeds the next
    previousModelOutput = [faceIdNodeId, 0];
  }

  // --- KSampler (connects to last FaceID in chain) ---
  prompt[NODE_IDS.ksampler] = {
    class_type: 'KSampler',
    inputs: {
      model: previousModelOutput,
      positive: [NODE_IDS.positivePrompt, 0],
      negative: [NODE_IDS.negativePrompt, 0],
      latent_image: [NODE_IDS.emptyLatent, 0],
      seed,
      steps: request.steps || 30,
      cfg: request.cfg || 5.5,
      sampler_name: 'dpmpp_2m_sde',
      scheduler: 'karras',
      denoise: 1.0,
    },
  };

  // --- VAE Decode ---
  prompt[NODE_IDS.vaeDecode] = {
    class_type: 'VAEDecode',
    inputs: {
      samples: [NODE_IDS.ksampler, 0],
      vae: [NODE_IDS.checkpoint, 2],
    },
  };

  // --- Save Image ---
  prompt[NODE_IDS.saveImage] = {
    class_type: 'SaveImage',
    inputs: {
      images: [NODE_IDS.vaeDecode, 0],
      filename_prefix: 'StoryEngine/scene',
    },
  };

  return prompt;
}

// ============================================================================
// API FUNCTIONS
// ============================================================================

/**
 * Upload an image to ComfyUI's input directory via the API.
 */
async function uploadImage(
  imagePath: string,
  filename: string
): Promise<string> {
  // Read image as bytes from the local filesystem via Tauri
  const imageBytes = await invoke<number[]>('read_file_bytes', { path: imagePath });
  const blob = new Blob([new Uint8Array(imageBytes)], { type: 'image/png' });

  const formData = new FormData();
  formData.append('image', blob, filename);

  const response = await fetch(`${COMFYUI_BASE_URL}/upload/image`, {
    method: 'POST',
    body: formData,
  });

  if (!response.ok) {
    throw new Error(`Failed to upload image: ${response.statusText}`);
  }

  const result = await response.json();
  return result.name; // Returns the filename as stored by ComfyUI
}

/**
 * Queue a prompt (workflow) for execution.
 */
async function queuePrompt(prompt: ComfyUIPrompt): Promise<string> {
  const response = await fetch(`${COMFYUI_BASE_URL}/prompt`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ prompt }),
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(`Failed to queue prompt: ${JSON.stringify(error)}`);
  }

  const result = await response.json();
  return result.prompt_id;
}

/**
 * Poll for prompt completion and get output images.
 */
async function waitForCompletion(
  promptId: string,
  timeoutMs: number = 120000
): Promise<string[]> {
  const startTime = Date.now();

  while (Date.now() - startTime < timeoutMs) {
    const response = await fetch(`${COMFYUI_BASE_URL}/history/${promptId}`);
    const history = await response.json();

    if (history[promptId]) {
      const outputs = history[promptId].outputs;
      const images: string[] = [];

      // Collect output images from SaveImage node
      const saveOutput = outputs[NODE_IDS.saveImage];
      if (saveOutput?.images) {
        for (const img of saveOutput.images) {
          images.push(`${COMFYUI_BASE_URL}/view?filename=${img.filename}&subfolder=${img.subfolder || ''}&type=${img.type || 'output'}`);
        }
      }

      return images;
    }

    // Wait 1 second before polling again
    await new Promise(resolve => setTimeout(resolve, 1000));
  }

  throw new Error('Prompt execution timed out');
}

// ============================================================================
// MAIN PUBLIC API
// ============================================================================

/**
 * Generate a scene image with 1-3 characters using FaceID.
 * This is the main entry point for the StoryEngine image pipeline.
 * 
 * @param positivePrompt - Scene description with quality tags
 * @param characters - Characters with their regions and reference image paths
 * @param options - Optional overrides for seed, steps, cfg, dimensions
 * @returns URLs to the generated images
 * 
 * Usage:
 * ```typescript
 * const result = await generateScene(
 *   '(masterpiece, best quality), cozy coffee shop, warm lighting, two women talking',
 *   [
 *     { name: 'Elena', region: 'left', referenceImagePath: '/path/to/elena_ref.png' },
 *     { name: 'Sophie', region: 'right', referenceImagePath: '/path/to/sophie_ref.png' },
 *   ]
 * );
 * ```
 */
export async function generateScene(
  positivePrompt: string,
  characters: { name: string; region: string; referenceImagePath: string }[],
  options?: {
    negativePrompt?: string;
    seed?: number;
    steps?: number;
    cfg?: number;
    width?: number;
    height?: number;
  }
): Promise<SceneGenResult> {
  if (characters.length === 0 || characters.length > 3) {
    throw new Error('Workflow supports 1-3 characters');
  }

  // Step 1: Auto-assign regions if needed
  const assigned = autoAssignRegions(characters);

  // Step 2: Upload reference images to ComfyUI
  const refFilenames: string[] = [];
  for (const char of characters) {
    const filename = `ref_${char.name.toLowerCase().replace(/\s+/g, '_')}.png`;
    const uploaded = await uploadImage(char.referenceImagePath, filename);
    refFilenames.push(uploaded);
  }

  // Step 3: Generate and save attention masks
  const masks = await generateSceneMasks(
    assigned,
    COMFYUI_INPUT_DIR,
    options?.width || CANVAS_WIDTH,
    options?.height || CANVAS_HEIGHT
  );
  const maskFilenames = masks.map(m => m.maskFilename);

  // Step 4: Build the workflow
  const request: SceneGenRequest = {
    positivePrompt,
    negativePrompt: options?.negativePrompt,
    characters: characters.map((c, i) => ({
      ...c,
      region: assigned[i].region,
      referenceImagePath: refFilenames[i],
    })),
    seed: options?.seed,
    steps: options?.steps,
    cfg: options?.cfg,
    width: options?.width,
    height: options?.height,
  };

  const workflow = buildWorkflowPrompt(request, maskFilenames);

  // Step 5: Queue and wait
  const promptId = await queuePrompt(workflow);
  const images = await waitForCompletion(promptId);

  return { images, promptId };
}

/**
 * Build prompt string from scene data + character details.
 * Integrates with the LLM parser output format.
 */
export function buildScenePrompt(
  sceneDescription: string,
  characters: ParsedCharacter[],
  mood?: string,
  lighting?: string
): string {
  const parts = ['(masterpiece, best quality, highly detailed)'];
  parts.push('detailed eyes, clear eyes, realistic eyes, perfect teeth');

  if (sceneDescription) parts.push(sceneDescription);
  if (lighting) parts.push(lighting);
  if (mood) parts.push(`${mood} atmosphere`);

  // Add character descriptions
  for (const char of characters) {
    const charParts: string[] = [];
    if (char.expression) charParts.push(char.expression);
    if (char.clothing) charParts.push(char.clothing);
    if (char.action) charParts.push(char.action);
    if (charParts.length > 0) {
      parts.push(`person ${char.region || ''} ${charParts.join(', ')}`.trim());
    }
  }

  parts.push('looking at camera, natural expression');

  return parts.join(', ');
}

/**
 * Check if ComfyUI is running and accessible.
 */
export async function isComfyUIRunning(): Promise<boolean> {
  try {
    const response = await fetch(`${COMFYUI_BASE_URL}/system_stats`, {
      signal: AbortSignal.timeout(3000),
    });
    return response.ok;
  } catch {
    return false;
  }
}

export { COMFYUI_BASE_URL, NODE_IDS };