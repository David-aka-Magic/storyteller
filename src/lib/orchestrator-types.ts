// src/lib/orchestrator-types.ts
//
// TypeScript types and Tauri command wrapper for the Story Turn Orchestrator
// ==========================================================================
// Mirrors the Rust structs in src-tauri/src/commands/orchestrator.rs
//
// Usage:
// ```typescript
// import { processStoryTurn } from '$lib/orchestrator-types';
//
// const result = await processStoryTurn(chatId, playerAction, storyId);
// // result.story_text  — display this
// // result.scene       — update scene panel
// // result.characters  — update character sprites
// // result.generated_image_path — show generated image
// // result.compression_info — update token meter
// ```

import { invoke } from '@tauri-apps/api/core';
import type { SceneJson } from '$lib/llm-parser-types';

// ============================================================================
// TYPES
// ============================================================================

/** A character as they appear in a scene, enriched with database info. */
export interface CharacterInScene {
  name: string;
  region: string;
  view: string;
  action: string;
  expression: string;
  clothing: string;
  facing: string;
  needs_render: boolean;
  /** Database character ID (null if character wasn't found in DB). */
  db_id: number | null;
  /** Whether this character has a master reference image for IP-Adapter. */
  has_reference_image: boolean;
  /** Text description for characters without a reference image (e.g. animals). */
  prompt_only_description: string | null;
}

/**
 * Compression diagnostics from the orchestrator.
 * Mirrors OrchestratorCompressionInfo in Rust.
 */
export interface OrchestratorCompressionInfo {
  total_turns: number;
  compressed_turns: number;
  recent_turns: number;
  estimated_total_tokens: number;
  max_context_tokens: number;
  compression_threshold: number;
  needs_compression: boolean;
  compressed_summary_preview: string;
}

/** Complete result of a single story turn from the orchestrator. */
export interface StoryTurnResult {
  /** Turn sequence number from the LLM. */
  turn_id: number;
  /** The narrative text to display to the player. */
  story_text: string;
  /** One-line summary for future context compression. */
  summary_hint: string;
  /** Scene environment data (location, lighting, mood, etc.). */
  scene: SceneJson | null;
  /** Characters in this scene with enriched database info. */
  characters: CharacterInScene[];
  /** Absolute path to the generated scene image (if one was produced). */
  generated_image_path: string | null;
  /** Parse quality indicator. */
  parse_status: 'ok' | 'partial' | 'fallback';
  /** Any warnings from the LLM parser (empty on "ok"). */
  parse_warnings: string[];
  /** Context compression diagnostics for the token meter UI. */
  compression_info: OrchestratorCompressionInfo;
  /** Whether image generation was attempted this turn. */
  image_generation_attempted: boolean;
  /** Error message if image generation failed (null on success or not attempted). */
  image_generation_error: string | null;
}

// ============================================================================
// MAIN API
// ============================================================================

/**
 * Process a complete story turn through the backend orchestrator.
 *
 * This single call replaces the old multi-step frontend flow of:
 *   generate_story → parseStoryTurn → lookupSceneCharacters →
 *   generate_color_mask → generate_comfyui_scene
 *
 * @param chatId - The chat/conversation ID
 * @param userInput - The player's action or dialogue text
 * @param storyId - Optional story ID for character scoping
 */
export async function processStoryTurn(
  chatId: number,
  userInput: string,
  storyId?: number
): Promise<StoryTurnResult> {
  return invoke<StoryTurnResult>('process_story_turn', {
    chatId,
    userInput,
    storyId: storyId ?? null,
  });
}

// ============================================================================
// HELPERS
// ============================================================================

/** Check if the turn result has any parse quality issues. */
export function hasParseIssues(result: StoryTurnResult): boolean {
  return result.parse_status !== 'ok';
}

/** Check if an image was successfully generated this turn. */
export function hasGeneratedImage(result: StoryTurnResult): boolean {
  return result.generated_image_path !== null;
}

/** Get only the characters that are actually rendered in the scene image. */
export function getRenderableCharacters(result: StoryTurnResult): CharacterInScene[] {
  return result.characters.filter(c => c.needs_render);
}

/** Get characters that are in the scene but couldn't be rendered (no reference image). */
export function getMissingReferenceCharacters(result: StoryTurnResult): CharacterInScene[] {
  return result.characters.filter(c => c.needs_render && !c.has_reference_image);
}

/** Check if context compression is getting close to triggering. */
export function isContextNearLimit(result: StoryTurnResult): boolean {
  return result.compression_info.needs_compression;
}

/** Get image generation status as a simple string. */
export function imageGenStatus(result: StoryTurnResult): 'not-requested' | 'success' | 'failed' {
  if (!result.image_generation_attempted) return 'not-requested';
  if (result.generated_image_path) return 'success';
  return 'failed';
}