// src/lib/api/text-gen.ts — Tauri command wrappers for LLM / text generation
import { invoke } from '@tauri-apps/api/core';
import type {
  ParsedTurn,
  GenerationFlags,
  StoryTurnResult,
  OrchestratorCompressionInfo,
} from '$lib/types';

// ---- Orchestrator ----

export async function processStoryTurn(
  chatId: number,
  userInput: string,
  storyId?: number
): Promise<StoryTurnResult> {
  return invoke('process_story_turn', { chatId, userInput, storyId: storyId ?? null });
}

export async function generateSceneImageForTurn(
  scenePrompt: string,
  storyId?: number,
  characterNames?: string[],
  characterPoses?: string[],
  positivePromptOverride?: string,
  negativePromptOverride?: string,
): Promise<string> {
  return invoke('generate_scene_image_for_turn', {
    scenePrompt,
    storyId: storyId ?? null,
    characterNames: (characterNames && characterNames.length > 0) ? characterNames : null,
    characterPoses: (characterPoses && characterPoses.length > 0) ? characterPoses : null,
    positivePromptOverride: positivePromptOverride ?? null,
    negativePromptOverride: negativePromptOverride ?? null,
  });
}

export async function previewScenePrompt(
  scenePrompt: string,
  storyId?: number,
  characterNames?: string[],
  characterPoses?: string[],
): Promise<{ positive: string; negative: string }> {
  return invoke('preview_scene_prompt', {
    scenePrompt,
    storyId: storyId ?? null,
    characterNames: (characterNames && characterNames.length > 0) ? characterNames : null,
    characterPoses: (characterPoses && characterPoses.length > 0) ? characterPoses : null,
  });
}

export async function illustrateSceneCustom(
  storyId: number,
  chatId: number,
  messageId: number,
  positivePrompt: string,
  negativePrompt: string,
): Promise<string> {
  return invoke('illustrate_scene_custom', {
    storyId,
    chatId,
    messageId,
    positivePrompt,
    negativePrompt,
  });
}

export async function getCompressionDiagnostics(chatId: number): Promise<OrchestratorCompressionInfo> {
  return invoke('get_compression_diagnostics', { chatId });
}

export async function regenerateStory(id: number, storyId?: number): Promise<StoryTurnResult> {
  return invoke('regenerate_story', { id, storyId: storyId ?? null });
}

// ---- LLM Parser ----

export async function parseStoryTurn(rawOutput: string): Promise<ParsedTurn> {
  return invoke('parse_story_turn', { rawOutput });
}

export async function getStoryText(rawOutput: string): Promise<string> {
  return invoke('get_story_text', { rawOutput });
}

export async function getCharacterNames(rawOutput: string): Promise<string[]> {
  return invoke('get_character_names', { rawOutput });
}

export async function checkGenerationFlags(rawOutput: string): Promise<GenerationFlags> {
  return invoke('check_generation_flags', { rawOutput });
}
