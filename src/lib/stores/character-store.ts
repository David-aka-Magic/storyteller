// src/lib/stores/character-store.ts
//
// Reactive state only — no API calls.
// Components compose this store with src/lib/api/character.ts directly.

import { writable, derived, get } from 'svelte/store';
import type { CharacterProfile } from '$lib/types';

// ============================================================================
// STORES
// ============================================================================

/** All characters for the current story */
export const characters = writable<CharacterProfile[]>([]);

/** Currently selected story ID (for filtering characters) */
export const currentStoryId = writable<number | undefined>(undefined);

/** Loading state */
export const isLoading = writable(false);

/** Error state */
export const lastError = writable<string | null>(null);

/** Characters indexed by name for fast local lookups */
export const charactersByName = derived(characters, ($characters) => {
  const map = new Map<string, CharacterProfile>();
  for (const char of $characters) {
    map.set(char.name, char);
  }
  return map;
});

// ============================================================================
// LOCAL UTILITIES (no API calls — read from store only)
// ============================================================================

/** Quick check if a character name exists in the currently loaded list */
export function characterExistsLocally(name: string): boolean {
  return get(charactersByName).has(name);
}

/** Get character from the currently loaded list by name */
export function getCharacterLocally(name: string): CharacterProfile | undefined {
  return get(charactersByName).get(name);
}

/** Generate a default SD prompt from character attributes */
export function generateDefaultSdPrompt(char: Partial<CharacterProfile>): string {
  const parts = ['(masterpiece, best quality), solo'];

  if (char.gender) parts.push(char.gender.toLowerCase());
  if (char.age) parts.push(`${char.age} years old`);
  if (char.skin_tone) parts.push(`${char.skin_tone} skin`);
  if (char.hair_color && char.hair_style) {
    parts.push(`${char.hair_color} ${char.hair_style} hair`);
  }
  if (char.body_type) parts.push(`${char.body_type} build`);
  if (char.default_clothing) parts.push(char.default_clothing);

  parts.push('detailed face, looking at viewer');

  return parts.join(', ');
}
