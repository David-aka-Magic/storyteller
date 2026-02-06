// src/lib/llm-parser-types.ts
//
// TypeScript types and helper functions for the LLM JSON Parser
// These mirror the Rust structs in src-tauri/src/llm_parser.rs
// and provide typed wrappers around the Tauri commands.

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// TYPES — match the Rust ParsedTurnForFrontend / GenerationFlags / etc.
// ============================================================================

/** Scene environment details from the LLM */
export interface SceneJson {
  location: string;
  location_type: string;
  time_of_day: string;
  weather: string;
  lighting: string;
  mood: string;
}

/** Image generation control flags */
export interface GenerationFlags {
  generate_image: boolean;
  scene_changed: boolean;
  characters_changed: boolean;
}

/** A character in the scene as returned by the parser */
export interface ParsedCharacter {
  name: string;
  region: string;
  view: string;         // "PORTRAIT" | "UPPER-BODY" | "FULL-BODY" | "NONE"
  action: string;
  expression: string;
  clothing: string;
  facing: string;
  needs_render: boolean;
}

/** The full parsed turn result from the backend */
export interface ParsedTurn {
  status: 'ok' | 'partial' | 'fallback';
  warnings: string[];
  turn_id: number;
  story_text: string;
  summary_hint: string;
  scene: SceneJson | null;
  characters: ParsedCharacter[];
  flags: GenerationFlags;
  scene_prompt_fragment: string;
  raw_text: string | null;
}

// ============================================================================
// REGION / VIEW CONSTANTS (match Rust enums)
// ============================================================================

export const REGIONS = [
  'left', 'center', 'right',
  'left-seated', 'center-seated', 'right-seated',
  'left-background', 'center-background', 'right-background',
  'off-screen',
] as const;

export type CharacterRegion = typeof REGIONS[number] | string;

export const VIEWS = ['PORTRAIT', 'UPPER-BODY', 'FULL-BODY', 'NONE'] as const;

export type CharacterViewType = typeof VIEWS[number] | string;

// ============================================================================
// TAURI COMMAND WRAPPERS
// ============================================================================

/**
 * Parse raw LLM output into a fully structured turn.
 * This is the main entry point — call after receiving an Ollama response.
 *
 * @param rawOutput - The raw string from the Ollama API response
 * @returns Parsed turn with story text, scene, characters, and flags
 */
export async function parseStoryTurn(rawOutput: string): Promise<ParsedTurn> {
  return invoke<ParsedTurn>('parse_story_turn', { rawOutput });
}

/**
 * Quick extraction of just the story text.
 * Use when you only need the narrative for display and don't need scene/character data.
 */
export async function getStoryText(rawOutput: string): Promise<string> {
  return invoke<string>('get_story_text', { rawOutput });
}

/**
 * Extract character names from the LLM output.
 * Use these with lookupSceneCharacters() from the character store
 * to get master images for IP-Adapter.
 */
export async function getCharacterNames(rawOutput: string): Promise<string[]> {
  return invoke<string[]>('get_character_names', { rawOutput });
}

/**
 * Check just the generation flags without full parsing.
 * Useful for quick "do we need to generate an image?" checks.
 */
export async function checkGenerationFlags(rawOutput: string): Promise<GenerationFlags> {
  return invoke<GenerationFlags>('check_generation_flags', { rawOutput });
}

// ============================================================================
// CLIENT-SIDE HELPERS (no Tauri call needed)
// ============================================================================

/** Check if a character region is a seated position */
export function isSeatedRegion(region: string): boolean {
  return region.includes('seated');
}

/** Check if a character region is in the background */
export function isBackgroundRegion(region: string): boolean {
  return region.includes('background');
}

/** Check if a character is off-screen */
export function isOffScreen(region: string): boolean {
  return region === 'off-screen';
}

/** Get only the characters that need image rendering */
export function getRenderableCharacters(characters: ParsedCharacter[]): ParsedCharacter[] {
  return characters.filter(c => c.needs_render);
}

/** Check if the parse result has any issues worth showing the user */
export function hasParseWarnings(parsed: ParsedTurn): boolean {
  return parsed.status !== 'ok';
}

/**
 * Build a per-character image prompt fragment from parsed character data.
 * Combines expression, clothing, action, and view into a prompt-ready string.
 */
export function characterPromptFragment(char: ParsedCharacter): string {
  const parts: string[] = [];
  if (char.view && char.view !== 'NONE') parts.push(char.view.toLowerCase().replace('-', ' '));
  if (char.expression) parts.push(char.expression);
  if (char.clothing) parts.push(char.clothing);
  if (char.action) parts.push(char.action);
  return parts.join(', ');
}