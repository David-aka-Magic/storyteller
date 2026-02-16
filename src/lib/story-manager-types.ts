// src/lib/story-manager-types.ts
//
// TypeScript types for the Story Manager system
// Mirrors the Rust structs in src-tauri/src/commands/story_manager.rs
//
// These types are used by:
//   - The story store (story-store.ts) for reactive state management
//   - Frontend components for displaying story lists, session data, exports
//   - The orchestrator integration for auto-save coordination

import type { CharacterProfile } from './character-types';
import type { CompressedHistory, StoryTurn } from './context-compression-types';

// ============================================================================
// CORE SESSION TYPES
// ============================================================================

/**
 * Full story session returned by `load_story`.
 * Contains everything the frontend needs to resume a story.
 */
export interface StorySession {
  story_id: number;
  title: string;
  description: string;
  characters: CharacterProfile[];
  compressed_history: CompressedHistory;
  recent_turns: StoryTurn[];
  current_location: string | null;
  total_turns: number;
  created_at: string;
  last_played_at: string;
  /** The chat_id associated with this story (for message storage) */
  chat_id: number | null;
}

/**
 * Lightweight summary for the story list view.
 */
export interface StorySummary {
  story_id: number;
  title: string;
  description: string;
  character_count: number;
  turn_count: number;
  last_played_at: string;
  created_at: string;
  thumbnail_path: string | null;
  current_location: string | null;
}

// ============================================================================
// REQUEST / RESPONSE TYPES
// ============================================================================

/**
 * Parameters for creating a new story.
 */
export interface CreateStoryRequest {
  title: string;
  description: string;
  initial_character_ids?: number[];
}

/**
 * Export format options.
 */
export type ExportFormat = 'json' | 'html';

/**
 * Exported story data (JSON format).
 */
export interface ExportedStory {
  meta: ExportedMeta;
  characters: CharacterProfile[];
  compressed_history: CompressedHistory;
  turns: ExportedTurn[];
}

export interface ExportedMeta {
  story_id: number;
  title: string;
  description: string;
  total_turns: number;
  created_at: string;
  last_played_at: string;
  current_location: string | null;
  exported_at: string;
}

export interface ExportedTurn {
  turn_number: number;
  user_input: string;
  story_text: string;
  summary_hint: string;
  timestamp: string;
  image_path: string | null;
}

// ============================================================================
// STORE STATE TYPE
// ============================================================================

/**
 * The shape of the story store's internal state.
 * Used by the writable store in story-store.ts.
 */
export interface StoryStoreState {
  /** The currently loaded story session (null if no story is loaded) */
  currentStory: StorySession | null;
  /** List of all story summaries */
  stories: StorySummary[];
  /** Whether a story operation is in progress */
  isLoading: boolean;
  /** Last error message (null if no error) */
  lastError: string | null;
  /** Whether auto-save has unsaved changes */
  isDirty: boolean;
}