// src/lib/stores/story-store.ts
//
// Svelte store and helper functions for Story Manager
// Provides reactive state management and Tauri command wrappers
//
// Usage in components:
//   import { storyStore, currentStory, loadStory, newStory, saveStory } from '$lib/stores/story-store';
//
// The store auto-saves compressed history and location changes,
// and coordinates with the orchestrator for turn-by-turn persistence.

import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type {
  StorySession,
  StorySummary,
  StoryStoreState,
  ExportFormat,
} from '../story-manager-types';
import type { CompressedHistory } from '../context-compression-types';

// ============================================================================
// STORES
// ============================================================================

/** Internal store holding the full story manager state */
const storyState = writable<StoryStoreState>({
  currentStory: null,
  stories: [],
  isLoading: false,
  lastError: null,
  isDirty: false,
});

// --- Derived stores for convenient access ---

/** The currently loaded story session (null if none) */
export const currentStory = derived(storyState, ($s) => $s.currentStory);

/** All story summaries for the list view */
export const stories = derived(storyState, ($s) => $s.stories);

/** Whether a story operation is in progress */
export const isLoading = derived(storyState, ($s) => $s.isLoading);

/** Last error message */
export const lastError = derived(storyState, ($s) => $s.lastError);

/** Whether there are unsaved changes */
export const isDirty = derived(storyState, ($s) => $s.isDirty);

/** Quick accessor: current story ID or null */
export const currentStoryId = derived(storyState, ($s) => $s.currentStory?.story_id ?? null);

/** Quick accessor: current chat ID or null */
export const currentChatId = derived(storyState, ($s) => $s.currentStory?.chat_id ?? null);

/** Quick accessor: current location */
export const currentLocation = derived(storyState, ($s) => $s.currentStory?.current_location ?? null);

// ============================================================================
// AUTO-SAVE CONFIGURATION
// ============================================================================

/** Debounce timer for auto-save (ms) */
const AUTO_SAVE_DEBOUNCE_MS = 5000;
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

// ============================================================================
// STORY LIFECYCLE FUNCTIONS
// ============================================================================

/**
 * Create a new story and optionally link initial characters.
 * Automatically loads the new story after creation.
 * @returns The new story_id
 */
export async function newStory(
  title: string,
  description: string,
  initialCharacterIds?: number[]
): Promise<number | null> {
  storyState.update((s) => ({ ...s, isLoading: true, lastError: null }));

  try {
    const storyId = await invoke<number>('create_story', {
      title,
      description,
      initialCharacterIds: initialCharacterIds ?? null,
    });

    // Load the newly created story
    await loadStory(storyId);

    // Refresh the story list
    await refreshStoryList();

    return storyId;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    storyState.update((s) => ({ ...s, lastError: error, isLoading: false }));
    console.error('[StoryStore] Failed to create story:', error);
    return null;
  }
}

/**
 * Load a story session by ID.
 * Sets it as the current story in the store.
 */
export async function loadStory(storyId: number): Promise<StorySession | null> {
  storyState.update((s) => ({ ...s, isLoading: true, lastError: null }));

  try {
    const session = await invoke<StorySession>('load_story', { storyId });

    storyState.update((s) => ({
      ...s,
      currentStory: session,
      isLoading: false,
      isDirty: false,
    }));

    console.log(
      `[StoryStore] Loaded story "${session.title}" (id=${session.story_id}, turns=${session.total_turns})`
    );

    return session;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    storyState.update((s) => ({ ...s, lastError: error, isLoading: false }));
    console.error('[StoryStore] Failed to load story:', error);
    return null;
  }
}

/**
 * Save the current story state to the database.
 * Called automatically by the auto-save system, or manually.
 */
export async function saveStory(): Promise<boolean> {
  const state = get(storyState);
  if (!state.currentStory) {
    console.warn('[StoryStore] No story loaded, nothing to save');
    return false;
  }

  try {
    await invoke('save_story_state', {
      storyId: state.currentStory.story_id,
      compressedHistory: state.currentStory.compressed_history,
      currentLocation: state.currentStory.current_location,
      thumbnailPath: null, // Updated separately when images are generated
    });

    storyState.update((s) => ({ ...s, isDirty: false }));
    console.log(`[StoryStore] Saved story ${state.currentStory.story_id}`);
    return true;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    storyState.update((s) => ({ ...s, lastError: error }));
    console.error('[StoryStore] Failed to save story:', error);
    return false;
  }
}

/**
 * Refresh the story list from the database.
 */
export async function refreshStoryList(): Promise<StorySummary[]> {
  try {
    const list = await invoke<StorySummary[]>('list_stories');
    storyState.update((s) => ({ ...s, stories: list }));
    return list;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    storyState.update((s) => ({ ...s, lastError: error }));
    console.error('[StoryStore] Failed to list stories:', error);
    return [];
  }
}

/**
 * Delete a story and all associated data.
 * If the deleted story is currently loaded, clears currentStory.
 */
export async function deleteStory(storyId: number): Promise<boolean> {
  storyState.update((s) => ({ ...s, isLoading: true, lastError: null }));

  try {
    await invoke('delete_story', { storyId });

    storyState.update((s) => ({
      ...s,
      currentStory: s.currentStory?.story_id === storyId ? null : s.currentStory,
      isLoading: false,
      isDirty: false,
    }));

    // Refresh the story list
    await refreshStoryList();

    console.log(`[StoryStore] Deleted story ${storyId}`);
    return true;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    storyState.update((s) => ({ ...s, lastError: error, isLoading: false }));
    console.error('[StoryStore] Failed to delete story:', error);
    return false;
  }
}

/**
 * Export a story to a file.
 * @returns The file path of the exported file, or null on failure.
 */
export async function exportStory(
  storyId: number,
  format: ExportFormat = 'json'
): Promise<string | null> {
  storyState.update((s) => ({ ...s, isLoading: true, lastError: null }));

  try {
    const filePath = await invoke<string>('export_story', { storyId, format });

    storyState.update((s) => ({ ...s, isLoading: false }));
    console.log(`[StoryStore] Exported story ${storyId} to ${filePath}`);
    return filePath;
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    storyState.update((s) => ({ ...s, lastError: error, isLoading: false }));
    console.error('[StoryStore] Failed to export story:', error);
    return null;
  }
}

/**
 * Unload the current story (go back to story list).
 * Auto-saves before unloading if dirty.
 */
export async function unloadStory(): Promise<void> {
  const state = get(storyState);
  if (state.isDirty && state.currentStory) {
    await saveStory();
  }

  cancelAutoSave();
  storyState.update((s) => ({
    ...s,
    currentStory: null,
    isDirty: false,
  }));
}

// ============================================================================
// AUTO-SAVE & REACTIVE UPDATE FUNCTIONS
// ============================================================================

/**
 * Update the compressed history in the store and trigger auto-save.
 * Called after each compression event in the orchestrator.
 */
export function updateCompressedHistory(history: CompressedHistory): void {
  storyState.update((s) => {
    if (!s.currentStory) return s;
    return {
      ...s,
      currentStory: {
        ...s.currentStory,
        compressed_history: history,
      },
      isDirty: true,
    };
  });
  scheduleAutoSave();
}

/**
 * Update the current location in the store and trigger auto-save.
 * Called when the scene changes.
 */
export function updateLocation(location: string): void {
  storyState.update((s) => {
    if (!s.currentStory) return s;
    return {
      ...s,
      currentStory: {
        ...s.currentStory,
        current_location: location,
      },
      isDirty: true,
    };
  });
  scheduleAutoSave();
}

/**
 * Update last_played_at and increment turn count locally.
 * Called after each successful story turn.
 */
export function recordTurnPlayed(): void {
  storyState.update((s) => {
    if (!s.currentStory) return s;
    return {
      ...s,
      currentStory: {
        ...s.currentStory,
        total_turns: s.currentStory.total_turns + 1,
        last_played_at: new Date().toISOString(),
      },
      isDirty: true,
    };
  });
  scheduleAutoSave();
}

/**
 * Update the thumbnail path (after image generation).
 */
export function updateThumbnail(thumbnailPath: string): void {
  storyState.update((s) => {
    if (!s.currentStory) return s;
    return {
      ...s,
      isDirty: true,
    };
  });

  // Save thumbnail immediately (not debounced)
  const state = get(storyState);
  if (state.currentStory) {
    invoke('save_story_state', {
      storyId: state.currentStory.story_id,
      compressedHistory: null,
      currentLocation: null,
      thumbnailPath,
    }).catch((e) => console.error('[StoryStore] Failed to save thumbnail:', e));
  }
}

/**
 * Add a character to the current story's character list (local update).
 * The character should already be saved to the DB.
 */
export function addCharacterToSession(character: any): void {
  storyState.update((s) => {
    if (!s.currentStory) return s;
    return {
      ...s,
      currentStory: {
        ...s.currentStory,
        characters: [...s.currentStory.characters, character],
      },
    };
  });
}

/**
 * Remove a character from the current story's character list (local update).
 */
export function removeCharacterFromSession(characterId: number): void {
  storyState.update((s) => {
    if (!s.currentStory) return s;
    return {
      ...s,
      currentStory: {
        ...s.currentStory,
        characters: s.currentStory.characters.filter((c) => c.id !== characterId),
      },
    };
  });
}

// ============================================================================
// AUTO-SAVE INTERNALS
// ============================================================================

/** Schedule a debounced auto-save */
function scheduleAutoSave(): void {
  cancelAutoSave();
  autoSaveTimer = setTimeout(() => {
    saveStory().catch((e) => console.error('[StoryStore] Auto-save failed:', e));
  }, AUTO_SAVE_DEBOUNCE_MS);
}

/** Cancel any pending auto-save */
function cancelAutoSave(): void {
  if (autoSaveTimer) {
    clearTimeout(autoSaveTimer);
    autoSaveTimer = null;
  }
}

// ============================================================================
// EXPORTED STORE (for direct subscription in components)
// ============================================================================

/**
 * The raw store for components that need full state access.
 * Most components should prefer the derived stores above.
 */
export const storyStore = {
  subscribe: storyState.subscribe,

  // Expose all functions for convenience
  newStory,
  loadStory,
  saveStory,
  deleteStory,
  exportStory,
  unloadStory,
  refreshStoryList,
  updateCompressedHistory,
  updateLocation,
  recordTurnPlayed,
  updateThumbnail,
};