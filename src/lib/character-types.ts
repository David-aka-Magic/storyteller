// src/lib/character-types.ts
//
// TypeScript types for the Character Database system
// Import these in your components or add to your existing types.ts

/**
 * Extended CharacterProfile with multi-story support
 * This replaces/extends your existing CharacterProfile
 */
export interface CharacterProfile {
  id: number;
  story_id?: number;           // Links character to a specific story
  name: string;
  age?: number;
  gender?: string;
  skin_tone?: string;
  hair_style?: string;
  hair_color?: string;
  body_type?: string;
  personality?: string;
  additional_notes?: string;
  default_clothing?: string;   // Default clothing for scene generation
  sd_prompt?: string;
  image?: string;              // Base64 preview image
  master_image_path?: string;  // File path for IP-Adapter reference
  seed?: number;
  art_style?: string;
}

/**
 * Lightweight lookup result for LLM integration
 * Used when processing scenes for image generation
 */
export interface CharacterLookup {
  id: number;
  name: string;
  master_image_path?: string;
  sd_prompt?: string;
  default_clothing?: string;
  art_style?: string;
}

/**
 * Scene character from LLM output
 * Matches your Ollama model's characters_in_scene JSON structure
 */
export interface SceneCharacter {
  name: string;
  region?: string;       // "left", "center", "right"
  view?: string;         // "FULL-BODY", "PORTRAIT", etc.
  action?: string;       // "walking toward table"
  expression?: string;   // "friendly smile"
  clothing?: string;     // "blue jacket, white t-shirt"
  facing?: string;       // "Elena"
}

/**
 * Result of scene character lookup
 * Pairs scene data with database character (if found)
 */
export type SceneCharacterLookupResult = [SceneCharacter, CharacterLookup | null];

/**
 * LLM scene output structure
 * The format your Ollama model produces
 */
export interface LLMSceneOutput {
  story_json?: {
    response: string;
  };
  sd_json?: {
    look: string;
    characters_in_scene?: SceneCharacter[];
  };
}