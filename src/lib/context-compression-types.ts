// src/lib/context-compression-types.ts
//
// TypeScript types for the Context Compression system
// Mirrors the Rust structs in src-tauri/src/context_compression.rs
//
// These types are used by frontend components to:
//   - Display compression diagnostics (token meter, compression status)
//   - Show the "story so far" summary to the user
//   - Allow manual compression triggers
//   - Configure compression settings

// ============================================================================
// CORE TYPES
// ============================================================================

/**
 * A single story turn (user action + assistant response pair).
 * Extracted from the messages table and paired together.
 */
export interface StoryTurn {
  /** 1-based turn number within this chat session */
  turn_number: number;
  /** The user's input text */
  user_input: string;
  /** The raw assistant response (full JSON as stored in DB) */
  assistant_response: string;
  /** One-line summary from story_json.summary_hint */
  summary_hint: string;
  /** Approximate token count for this turn */
  token_estimate: number;
}

/**
 * The compressed "story so far" block that replaces older turns.
 */
export interface CompressedHistory {
  /** Combined summary text */
  story_so_far: string;
  /** Turn numbers that have been compressed */
  compressed_turn_ids: number[];
  /** Approximate token count of the summary */
  token_estimate: number;
}

/**
 * Full conversation context state for one chat session.
 */
export interface ConversationContext {
  /** Recent uncompressed turns */
  turns: StoryTurn[];
  /** Compressed summary of older turns */
  compressed: CompressedHistory;
  /** Total tokens across uncompressed turns */
  total_turn_tokens: number;
}

// ============================================================================
// DIAGNOSTICS
// ============================================================================

/**
 * Compression diagnostics returned by the backend.
 * Use this to display a token usage meter or compression status to the user.
 */
export interface CompressionDiagnostics {
  /** Total turns including compressed ones */
  total_turns: number;
  /** Number of turns that have been compressed into summary */
  compressed_turns: number;
  /** Number of turns kept in full detail */
  recent_turns: number;
  /** Estimated total token usage */
  estimated_total_tokens: number;
  /** Maximum context window size (8192 for Llama 3.1 8B) */
  max_context_tokens: number;
  /** Token threshold that triggers compression */
  compression_threshold: number;
  /** Whether compression is currently needed */
  needs_compression: boolean;
  /** First 200 chars of the compressed summary for preview */
  compressed_summary_preview: string;
}

/**
 * Result from the context assembly function.
 * Tells the frontend what happened during context building.
 */
export interface AssembledContextInfo {
  /** Estimated total token count of the assembled prompt */
  estimated_tokens: number;
  /** Whether compression was triggered during assembly */
  was_compressed: boolean;
  /** Number of turns in full detail */
  recent_turn_count: number;
  /** Number of turns that were compressed */
  compressed_turn_count: number;
}

// ============================================================================
// CONSTANTS (mirrored from Rust)
// ============================================================================

/** Llama 3.1 8B context window */
export const MAX_CONTEXT_TOKENS = 8192;

/** Compression triggers at this fraction of the window */
export const COMPRESSION_THRESHOLD = 0.75;

/** Number of recent turns always kept in full detail */
export const RECENT_TURNS_TO_KEEP = 6;

/** Tokens reserved for system prompt + character DB + response generation */
export const RESERVED_TOKENS = 1500;

// ============================================================================
// HELPER — Client-side token estimation (matches the Rust implementation)
// ============================================================================

/**
 * Rough token estimate: ~1 token per 4 characters.
 * Matches the Rust implementation for consistent estimates on both sides.
 */
export function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

/**
 * Calculate what percentage of the context window is used.
 * Returns a value between 0 and 1.
 */
export function contextUsagePercent(estimatedTokens: number): number {
  return Math.min(estimatedTokens / MAX_CONTEXT_TOKENS, 1.0);
}

/**
 * Get a human-readable status for the current context usage.
 */
export function contextStatus(estimatedTokens: number): 'ok' | 'warning' | 'critical' {
  const usage = estimatedTokens / MAX_CONTEXT_TOKENS;
  if (usage < 0.5) return 'ok';
  if (usage < COMPRESSION_THRESHOLD) return 'warning';
  return 'critical';
}