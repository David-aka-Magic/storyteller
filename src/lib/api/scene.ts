// src/lib/api/scene.ts — Scene management API layer
// All Tauri invoke() calls for scenes live here. Components never call invoke() directly.
//
// Tauri v2 converts snake_case Rust parameter names to camelCase for JS IPC.
// e.g. `story_id: i64` in Rust → send `{ storyId }` from JS.

import { invoke } from '@tauri-apps/api/core';
import type { Scene, SceneWithCharacters, CharacterProfile } from '../types';

// ─── CRUD ────────────────────────────────────────────────────────────────────

export async function createScene(
  name: string,
  description?: string,
  location?: string,
  location_type?: string,
  time_of_day?: string,
  mood?: string,
): Promise<number> {
  return invoke('create_scene', {
    name,
    description,
    location,
    locationType: location_type,
    timeOfDay: time_of_day,
    mood,
  });
}

export async function updateScene(
  id: number,
  name: string,
  description?: string,
  location?: string,
  location_type?: string,
  time_of_day?: string,
  mood?: string,
): Promise<void> {
  return invoke('update_scene', {
    id,
    name,
    description,
    location,
    locationType: location_type,
    timeOfDay: time_of_day,
    mood,
  });
}

export async function deleteScene(id: number): Promise<void> {
  return invoke('delete_scene', { id });
}

// ─── QUERIES ─────────────────────────────────────────────────────────────────

export async function listScenesForStory(storyId: number): Promise<Scene[]> {
  return invoke('list_scenes_for_story', { storyId });
}

export async function listAllScenes(): Promise<Scene[]> {
  return invoke('list_all_scenes');
}

// ─── STORY <-> SCENE ─────────────────────────────────────────────────────────

export async function linkSceneToStory(sceneId: number, storyId: number): Promise<void> {
  return invoke('link_scene_to_story', { sceneId, storyId });
}

export async function unlinkSceneFromStory(sceneId: number, storyId: number): Promise<void> {
  return invoke('unlink_scene_from_story', { sceneId, storyId });
}

// ─── SCENE <-> CHARACTER ──────────────────────────────────────────────────────

export async function addCharacterToScene(sceneId: number, characterId: number): Promise<void> {
  return invoke('add_character_to_scene', { sceneId, characterId });
}

export async function removeCharacterFromScene(sceneId: number, characterId: number): Promise<void> {
  return invoke('remove_character_from_scene', { sceneId, characterId });
}

export async function getSceneCharacters(sceneId: number): Promise<CharacterProfile[]> {
  return invoke('get_scene_characters', { sceneId });
}

export async function getSceneWithCharacters(sceneId: number): Promise<SceneWithCharacters> {
  return invoke('get_scene_with_characters', { sceneId });
}

// ─── ACTIVE SCENE ────────────────────────────────────────────────────────────

export async function setActiveScene(storyId: number, sceneId: number | null): Promise<void> {
  return invoke('set_active_scene', { storyId, sceneId });
}

export async function getActiveScene(storyId: number): Promise<Scene | null> {
  return invoke('get_active_scene', { storyId });
}

// ─── CONVENIENCE ─────────────────────────────────────────────────────────────

export async function setSceneHint(storyId: number, sceneId: number): Promise<void> {
  return invoke('set_scene_hint', { storyId, sceneId });
}

export async function createSceneFromLlmOutput(
  name: string,
  storyId: number,
  description?: string,
  location?: string,
  location_type?: string,
  time_of_day?: string,
  mood?: string,
): Promise<number> {
  return invoke('create_scene_from_llm_output', {
    name,
    storyId,
    description,
    location,
    locationType: location_type,
    timeOfDay: time_of_day,
    mood,
  });
}
