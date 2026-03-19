// src/lib/api/story.ts — Tauri command wrappers for story operations
import { invoke } from '@tauri-apps/api/core';
import type { StorySession, StorySummary, ExportFormat, StoryPremise, CompressedHistory, StoryImage } from '$lib/types';

export async function createStory(
  title: string,
  description: string,
  initialCharacterIds?: number[],
  contentRating?: string
): Promise<number> {
  return invoke('create_story', {
    title,
    description,
    initialCharacterIds: initialCharacterIds ?? null,
    contentRating: contentRating ?? null,
  });
}

export async function updateStoryRating(storyId: number, contentRating: string): Promise<void> {
  return invoke('update_story_rating', { storyId, contentRating });
}

export async function loadStory(storyId: number): Promise<StorySession> {
  return invoke('load_story', { storyId });
}

export async function saveStoryState(
  storyId: number,
  compressedHistory: CompressedHistory | null,
  currentLocation: string | null,
  thumbnailPath: string | null
): Promise<void> {
  return invoke('save_story_state', { storyId, compressedHistory, currentLocation, thumbnailPath });
}

export async function listStories(contentRatingFilter?: string): Promise<StorySummary[]> {
  return invoke('list_stories', { contentRatingFilter: contentRatingFilter ?? null });
}

export async function deleteStory(storyId: number): Promise<void> {
  return invoke('delete_story', { storyId });
}

export async function exportStory(storyId: number, format: ExportFormat = 'json'): Promise<string> {
  return invoke('export_story', { storyId, format });
}

export async function getStoryImages(storyId: number, chatId?: number): Promise<StoryImage[]> {
  return invoke('get_story_images', { storyId, chatId: chatId ?? null });
}

export async function getStoryForChat(chatId: number): Promise<{ id: number; title: string; description: string } | null> {
  return invoke('get_story_for_chat', { chatId });
}

export async function freeVram(): Promise<void> {
  return invoke('free_vram');
}

// ---- Legacy commands (pre-story-manager) ----

export async function getStoryList(): Promise<StoryPremise[]> {
  return invoke('get_story_list');
}

export async function saveStoryPremise(
  title: string,
  description: string,
  id: number | null
): Promise<number> {
  return invoke('save_story_premise', { title, description, id });
}

export async function deleteStories(ids: number[]): Promise<void> {
  return invoke('delete_stories', { ids });
}
