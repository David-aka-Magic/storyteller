// src/lib/scene-processor.ts
//
// Example integration: Processing LLM output to generate images with IP-Adapter
// This shows how to use the character database with your ComfyUI workflow

import { invoke } from '@tauri-apps/api/core';
import { lookupSceneCharacters, getCharacterByName } from './stores/character-store';
import type { SceneCharacter, CharacterLookup, CharacterProfile, LLMSceneOutput } from './character-types';

/**
 * Data prepared for ComfyUI/IP-Adapter image generation
 */
interface ImageGenRequest {
  scenePrompt: string;           // The visual description from LLM
  characters: {
    name: string;
    region: string;              // left, center, right
    view: string;                // FULL-BODY, PORTRAIT
    action: string;
    expression: string;
    clothing: string;
    masterImagePath: string | null;  // Path for IP-Adapter reference
    sdPrompt: string | null;     // Character's base SD prompt
  }[];
}

/**
 * Process LLM scene output and prepare data for image generation
 * 
 * This is the main integration point between your LLM and image generation pipeline.
 * It takes the raw LLM output, looks up characters in the database, and prepares
 * everything needed for ComfyUI with IP-Adapter.
 * 
 * Usage:
 * ```typescript
 * const llmOutput = JSON.parse(aiResponse);
 * const request = await prepareSceneForImageGen(llmOutput, currentStoryId);
 * // request.characters now has masterImagePath for each known character
 * ```
 */
export async function prepareSceneForImageGen(
  llmOutput: LLMSceneOutput,
  currentStoryId?: number
): Promise<ImageGenRequest> {
  const sceneCharacters = llmOutput.sd_json?.characters_in_scene || [];
  
  // Batch lookup all characters from the database
  const lookupResults = await lookupSceneCharacters(sceneCharacters, currentStoryId);
  
  // Prepare the request with character reference images
  const characters = lookupResults.map(([sceneChar, dbChar]) => ({
    name: sceneChar.name,
    region: sceneChar.region || 'center',
    view: sceneChar.view || 'FULL-BODY',
    action: sceneChar.action || '',
    expression: sceneChar.expression || 'neutral',
    clothing: sceneChar.clothing || dbChar?.default_clothing || 'casual clothing',
    masterImagePath: dbChar?.master_image_path || null,
    sdPrompt: dbChar?.sd_prompt || null,
  }));
  
  // Log any missing characters (helpful for debugging)
  const missingChars = characters.filter(c => !c.masterImagePath);
  if (missingChars.length > 0) {
    console.warn(
      'Characters without reference images:', 
      missingChars.map(c => c.name).join(', ')
    );
  }
  
  return {
    scenePrompt: llmOutput.sd_json?.look || '',
    characters
  };
}

/**
 * Build a ComfyUI-compatible prompt with character details
 * Adjust this based on your actual ComfyUI workflow structure
 */
export function buildComfyPrompt(request: ImageGenRequest): string {
  // Base scene description
  let prompt = request.scenePrompt;
  
  // Add character-specific details
  for (const char of request.characters) {
    if (char.sdPrompt) {
      // If character has a custom SD prompt, incorporate their visual details
      prompt += `, ${char.name} (${char.sdPrompt})`;
    }
    
    // Add dynamic elements from the scene
    if (char.action) {
      prompt += `, ${char.name} ${char.action}`;
    }
    if (char.expression) {
      prompt += `, ${char.expression}`;
    }
  }
  
  return prompt;
}

/**
 * Get reference images for IP-Adapter from a prepared request
 * Returns only characters that have master reference images
 */
export function getReferenceImages(request: ImageGenRequest): { name: string; path: string }[] {
  return request.characters
    .filter(c => c.masterImagePath !== null)
    .map(c => ({ name: c.name, path: c.masterImagePath! }));
}

/**
 * Example: Full workflow integration
 * 
 * Usage in your ChatArea or message handler:
 * ```typescript
 * // After receiving LLM response with sd_json
 * const imageData = await generateSceneImage(llmOutput, currentStoryId);
 * if (imageData) {
 *   // Display the generated image
 * }
 * ```
 */
export async function generateSceneImage(
  llmOutput: LLMSceneOutput,
  storyId?: number
): Promise<{ imagePath: string; usedReferences: string[] } | null> {
  try {
    // Step 1: Prepare scene data with character lookups
    const request = await prepareSceneForImageGen(llmOutput, storyId);
    
    // Step 2: Build the prompt
    const prompt = buildComfyPrompt(request);
    
    // Step 3: Collect reference images for IP-Adapter
    const referenceImages = getReferenceImages(request);
    
    console.log('Generating scene with:', {
      prompt,
      referenceCount: referenceImages.length,
      references: referenceImages.map(r => r.name)
    });
    
    // Step 4: Call your image generation backend
    // Uncomment and adjust based on your actual implementation
    /*
    const result = await invoke<string>('generate_scene_with_references', {
      prompt,
      referenceImages: referenceImages.map(r => r.path),
      characters: request.characters,
    });
    
    return {
      imagePath: result,
      usedReferences: referenceImages.map(r => r.path)
    };
    */
    
    // Placeholder return for now
    return null;
    
  } catch (error) {
    console.error('Failed to generate scene image:', error);
    return null;
  }
}

/**
 * Quick single-character lookup for portrait generation
 */
export async function getCharacterForPortrait(
  name: string,
  storyId?: number
): Promise<{
  character: CharacterProfile | null;
  hasReferenceImage: boolean;
  canUseIPAdapter: boolean;
}> {
  const character = await getCharacterByName(name, storyId);
  
  return {
    character,
    hasReferenceImage: character?.master_image_path != null,
    canUseIPAdapter: character?.master_image_path != null
  };
}

/**
 * Validate that all characters in a scene exist in the database
 * Useful before starting a new story arc
 */
export async function validateSceneCharacters(
  characterNames: string[],
  storyId?: number
): Promise<{
  valid: boolean;
  found: string[];
  missing: string[];
}> {
  const found: string[] = [];
  const missing: string[] = [];
  
  for (const name of characterNames) {
    const character = await getCharacterByName(name, storyId);
    if (character) {
      found.push(name);
    } else {
      missing.push(name);
    }
  }
  
  return {
    valid: missing.length === 0,
    found,
    missing
  };
}