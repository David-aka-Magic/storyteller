// src/lib/stores/character-store.ts
//
// Svelte store and helper functions for Character Database
// Provides reactive state management and Tauri command wrappers

import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { 
  CharacterProfile, 
  CharacterLookup, 
  SceneCharacter,
  SceneCharacterLookupResult 
} from '../character-types';

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

// Derived store: characters indexed by name for fast lookup
export const charactersByName = derived(characters, ($characters) => {
  const map = new Map<string, CharacterProfile>();
  for (const char of $characters) {
    map.set(char.name, char);
  }
  return map;
});

// ============================================================================
// TAURI COMMAND WRAPPERS
// ============================================================================

/**
 * Load all characters for a story (or all characters if storyId is undefined)
 */
export async function loadCharactersForStory(storyId?: number): Promise<CharacterProfile[]> {
  isLoading.set(true);
  lastError.set(null);
  
  try {
    const result = await invoke<CharacterProfile[]>('list_characters_for_story', { 
      storyId: storyId ?? null 
    });
    characters.set(result);
    return result;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    lastError.set(error);
    console.error('Failed to load characters:', error);
    return [];
  } finally {
    isLoading.set(false);
  }
}

/**
 * Add a new character to the database
 */
export async function addCharacter(character: Omit<CharacterProfile, 'id'> & { id?: number }): Promise<number | null> {
  isLoading.set(true);
  lastError.set(null);
  
  try {
    // For new characters, pass id as 0 (backend will auto-generate)
    const charToSave = { ...character, id: character.id ?? 0 };
    const id = await invoke<number>('add_character', { character: charToSave });
    
    // Refresh the character list
    const storyId = get(currentStoryId);
    await loadCharactersForStory(storyId);
    
    return id;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    lastError.set(error);
    console.error('Failed to add character:', error);
    return null;
  } finally {
    isLoading.set(false);
  }
}

/**
 * Update an existing character
 */
export async function updateCharacter(character: CharacterProfile): Promise<boolean> {
  isLoading.set(true);
  lastError.set(null);
  
  try {
    await invoke('update_character', { character });
    
    // Refresh the character list
    const storyId = get(currentStoryId);
    await loadCharactersForStory(storyId);
    
    return true;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    lastError.set(error);
    console.error('Failed to update character:', error);
    return false;
  } finally {
    isLoading.set(false);
  }
}

/**
 * Delete a character by ID
 */
export async function deleteCharacter(id: number): Promise<boolean> {
  isLoading.set(true);
  lastError.set(null);
  
  try {
    await invoke('delete_character_by_id', { id });
    
    // Refresh the character list
    const storyId = get(currentStoryId);
    await loadCharactersForStory(storyId);
    
    return true;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    lastError.set(error);
    console.error('Failed to delete character:', error);
    return false;
  } finally {
    isLoading.set(false);
  }
}

/**
 * Get a character by exact name match
 * Critical for LLM integration - matches "name": "David" from scene output
 */
export async function getCharacterByName(
  name: string, 
  storyId?: number
): Promise<CharacterProfile | null> {
  try {
    const result = await invoke<CharacterProfile | null>('get_character_by_name', { 
      name, 
      storyId: storyId ?? null 
    });
    return result;
  } catch (e) {
    console.error('Failed to get character by name:', e);
    return null;
  }
}

/**
 * Get a character by ID
 */
export async function getCharacterById(id: number): Promise<CharacterProfile | null> {
  try {
    const result = await invoke<CharacterProfile | null>('get_character_by_id', { id });
    return result;
  } catch (e) {
    console.error('Failed to get character by ID:', e);
    return null;
  }
}

/**
 * Search characters by partial name match
 * Useful for autocomplete/search UI
 */
export async function searchCharacters(
  query: string,
  storyId?: number,
  limit?: number
): Promise<CharacterProfile[]> {
  try {
    const result = await invoke<CharacterProfile[]>('search_characters', { 
      query, 
      storyId: storyId ?? null,
      limit: limit ?? null
    });
    return result;
  } catch (e) {
    console.error('Failed to search characters:', e);
    return [];
  }
}

/**
 * Batch lookup characters for a scene
 * Use this when processing LLM output with characters_in_scene array
 */
export async function lookupSceneCharacters(
  sceneCharacters: SceneCharacter[],
  storyId?: number
): Promise<SceneCharacterLookupResult[]> {
  try {
    const result = await invoke<SceneCharacterLookupResult[]>('lookup_scene_characters', { 
      sceneCharacters, 
      storyId: storyId ?? null 
    });
    return result;
  } catch (e) {
    console.error('Failed to lookup scene characters:', e);
    // Return the original scene characters with null lookups
    return sceneCharacters.map(sc => [sc, null]);
  }
}

/**
 * Update a character's master reference image path
 */
export async function setCharacterMasterImage(
  id: number, 
  imagePath: string
): Promise<boolean> {
  try {
    await invoke('set_character_master_image', { id, imagePath });
    
    // Update local store
    characters.update(chars => 
      chars.map(c => c.id === id ? { ...c, master_image_path: imagePath } : c)
    );
    
    return true;
  } catch (e) {
    console.error('Failed to set master image:', e);
    return false;
  }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Process LLM scene output and get character data for image generation
 * Returns characters with their master reference images for IP-Adapter
 */
export async function processSceneForImageGen(
  charactersInScene: SceneCharacter[],
  storyId?: number
): Promise<{
  character: SceneCharacter;
  dbCharacter: CharacterLookup | null;
  hasReferenceImage: boolean;
}[]> {
  const lookupResults = await lookupSceneCharacters(charactersInScene, storyId);
  
  return lookupResults.map(([sceneChar, dbChar]) => ({
    character: sceneChar,
    dbCharacter: dbChar,
    hasReferenceImage: dbChar?.master_image_path != null
  }));
}

/**
 * Quick check if a character name exists in the current story
 * Uses the local store for fast lookups
 */
export function characterExistsLocally(name: string): boolean {
  const map = get(charactersByName);
  return map.has(name);
}

/**
 * Get character from local store by name
 * Faster than database lookup when data is already loaded
 */
export function getCharacterLocally(name: string): CharacterProfile | undefined {
  const map = get(charactersByName);
  return map.get(name);
}

/**
 * Generate a default SD prompt from character attributes
 */
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