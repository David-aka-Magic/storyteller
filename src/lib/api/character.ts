// src/lib/api/character.ts — Tauri command wrappers for character operations
import { invoke } from '@tauri-apps/api/core';
import type { CharacterProfile, SceneCharacter, SceneCharacterLookupResult } from '$lib/types';

export async function listCharactersForStory(storyId?: number, contentRatingFilter?: string): Promise<CharacterProfile[]> {
  return invoke('list_characters_for_story', { storyId: storyId ?? null, contentRatingFilter: contentRatingFilter ?? null });
}

/** The backend ignores the `id` field and assigns a new one. Pass `id: 0` for new characters. */
export async function addCharacter(character: CharacterProfile): Promise<number> {
  return invoke('add_character', { character });
}

export async function updateCharacter(character: CharacterProfile): Promise<void> {
  return invoke('update_character', { character });
}

export async function deleteCharacterById(id: number): Promise<void> {
  return invoke('delete_character_by_id', { id });
}

/** Legacy: delete by id using the 'delete_character' command name. */
export async function deleteCharacter(id: number): Promise<void> {
  return invoke('delete_character', { id });
}

export async function getCharacterByName(name: string, storyId?: number): Promise<CharacterProfile | null> {
  return invoke('get_character_by_name', { name, storyId: storyId ?? null });
}

export async function getCharacterById(id: number): Promise<CharacterProfile | null> {
  return invoke('get_character_by_id', { id });
}

export async function searchCharacters(
  query: string,
  storyId?: number,
  limit?: number
): Promise<CharacterProfile[]> {
  return invoke('search_characters', { query, storyId: storyId ?? null, limit: limit ?? null });
}

export async function lookupSceneCharacters(
  sceneCharacters: SceneCharacter[],
  storyId?: number
): Promise<SceneCharacterLookupResult[]> {
  return invoke('lookup_scene_characters', { sceneCharacters, storyId: storyId ?? null });
}

export async function setCharacterMasterImage(id: number, imagePath: string): Promise<void> {
  return invoke('set_character_master_image', { id, imagePath });
}

export async function linkCharacterToStory(characterId: number, storyId: number): Promise<void> {
  return invoke('link_character_to_story', { characterId, storyId });
}

/** Add a character to a story via the junction table. Safe to call multiple times. */
export async function addCharacterToStory(characterId: number, storyId: number): Promise<void> {
  return invoke('add_character_to_story', { characterId, storyId });
}

/** Remove a character from a story (deletes only the junction row; character is preserved). */
export async function removeCharacterFromStory(characterId: number, storyId: number): Promise<void> {
  return invoke('remove_character_from_story', { characterId, storyId });
}

/** List ALL characters in the database, not filtered by story. */
export async function listAllCharacters(contentRatingFilter?: string): Promise<CharacterProfile[]> {
  return invoke('list_all_characters', { contentRatingFilter: contentRatingFilter ?? null });
}

/**
 * List characters matching a given art_style (pass null to get all).
 * Characters whose IDs are in excludeIds are omitted (already in the scene).
 */
export async function listCharactersByArtStyle(
  artStyle: string | null,
  excludeIds: number[],
): Promise<CharacterProfile[]> {
  return invoke('list_characters_by_art_style', { artStyle, excludeIds });
}
