// src/lib/types.ts
//
// Consolidated TypeScript type definitions for StoryEngine
// =========================================================
// All types previously spread across character-types.ts, llm-parser-types.ts,
// orchestrator-types.ts, story-manager-types.ts, and context-compression-types.ts
// are merged here.

// ============================================================================
// CHAT
// ============================================================================

export interface ChatMessage {
  id: number;
  text: string;
  sender: 'user' | 'ai';
  data?: StoryResponse;
  image?: string;
  /** DB primary key of the messages row — used to persist generated images. */
  dbMessageId?: number;
  /** Names of characters that were in this scene (needs_render=true). Used to filter image gen. */
  sceneCharacterNames?: string[];
}

export interface ChatSummary {
  id: number;
  title: string;
  messages: any[];
  character_id?: string;
}

export interface Phase1Response {
  text: string;
  type: 'phase1';
}

export interface SdDetails {
  name: string;
  view: string;
  features: string;
  action_context: string;
  clothing: string;
  look: string;
}

export interface StoryResponse {
  story: string;
  sd_prompt?: string;
  sd_details?: SdDetails;
}

export interface StoryPremise {
  id: string;
  title: string;
  description: string;
  /** The chat_id linked to this story. Populated when loaded via listStories/loadStory. */
  chat_id?: number;
}

export interface SelectionState {
  selectedIds: Set<number>;
  isSelecting: boolean;
}

export interface ContextMenuData {
  show: boolean;
  x: number;
  y: number;
  chatId: number | null;
}

// ============================================================================
// CHARACTER
// ============================================================================

/** Full character profile as stored in the database. */
export interface CharacterProfile {
  id: number;
  story_id?: number;           // Links character to a specific story
  name: string;
  age: number;
  gender: string;
  skin_tone: string;
  hair_style: string;
  hair_color: string;
  body_type: string;
  personality: string;
  additional_notes: string;
  default_clothing?: string;   // Default clothing for scene generation
  sd_prompt: string;
  image?: string;              // Base64 preview image
  master_image_path?: string;  // File path for IP-Adapter reference
  seed?: number;
  art_style?: string;
  eye_color?: string;
  height_scale?: number;   // 1-5 (1=Very Short … 5=Very Tall), default 3
  weight_scale?: number;   // 1-5 (1=Very Slim … 5=Very Heavyset), default 3
}

/** Lightweight lookup result for LLM integration. */
export interface CharacterLookup {
  id: number;
  name: string;
  master_image_path?: string;
  sd_prompt?: string;
  default_clothing?: string;
  art_style?: string;
}

/** Scene character from LLM output. */
export interface SceneCharacter {
  name: string;
  region?: string;       // "left", "center", "right"
  view?: string;         // "FULL-BODY", "PORTRAIT", etc.
  pose?: string;         // "STANDING", "SITTING", "RUNNING", etc.
  action?: string;       // "walking toward table"
  expression?: string;   // "friendly smile"
  clothing?: string;     // "blue jacket, white t-shirt"
  facing?: string;       // "Elena"
}

/** Result of scene character lookup — pairs scene data with database character (if found). */
export type SceneCharacterLookupResult = [SceneCharacter, CharacterLookup | null];

/** LLM scene output structure. */
export interface LLMSceneOutput {
  story_json?: {
    response: string;
  };
  sd_json?: {
    look: string;
    characters_in_scene?: SceneCharacter[];
  };
}

// ============================================================================
// STORY
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

/** Lightweight summary for the story list view. */
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

/** A generated scene image belonging to a story. */
export interface StoryImage {
  id: number;
  file_path: string;
  message_id: number;
  timestamp: string;
  caption: string;
}

/** Parameters for creating a new story. */
export interface CreateStoryRequest {
  title: string;
  description: string;
  initial_character_ids?: number[];
}

/** Export format options. */
export type ExportFormat = 'json' | 'html';

/** Exported story data (JSON format). */
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

/** The shape of the story store's internal state. */
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

// ============================================================================
// SCENE
// ============================================================================

export interface Scene {
  id: number;
  name: string;
  description?: string;
  location?: string;
  location_type?: string;
  time_of_day?: string;
  mood?: string;
  created_at: string;
}

export interface SceneWithCharacters {
  scene: Scene;
  characters: CharacterProfile[];
}

// ============================================================================
// LLM / PARSING
// ============================================================================

/** Scene environment details from the LLM. */
export interface SceneJson {
  location: string;
  location_type: string;
  time_of_day: string;
  weather: string;
  lighting: string;
  mood: string;
}

/** Image generation control flags. */
export interface GenerationFlags {
  generate_image: boolean;
  scene_changed: boolean;
  characters_changed: boolean;
}

/** A character in the scene as returned by the parser. */
export interface ParsedCharacter {
  name: string;
  region: string;
  view: string;         // "PORTRAIT" | "UPPER-BODY" | "FULL-BODY" | "NONE"
  pose: string;         // "STANDING" | "SITTING" | "RUNNING" | etc.
  action: string;
  expression: string;
  clothing: string;
  facing: string;
  needs_render: boolean;
}

/** The full parsed turn result from the backend. */
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

export const REGIONS = [
  'left', 'center', 'right',
  'left-seated', 'center-seated', 'right-seated',
  'left-background', 'center-background', 'right-background',
  'off-screen',
] as const;

export type CharacterRegion = typeof REGIONS[number] | string;

export const VIEWS = ['PORTRAIT', 'UPPER-BODY', 'FULL-BODY', 'NONE'] as const;

export type CharacterViewType = typeof VIEWS[number] | string;

// ---- Client-side helpers ----

/** Check if a character region is a seated position. */
export function isSeatedRegion(region: string): boolean {
  return region.includes('seated');
}

/** Check if a character region is in the background. */
export function isBackgroundRegion(region: string): boolean {
  return region.includes('background');
}

/** Check if a character is off-screen. */
export function isOffScreen(region: string): boolean {
  return region === 'off-screen';
}

/** Get only the ParsedCharacters that need image rendering. */
export function getRenderableCharacters(characters: ParsedCharacter[]): ParsedCharacter[] {
  return characters.filter(c => c.needs_render);
}

/** Check if the parse result has any issues worth showing the user. */
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

// ============================================================================
// ORCHESTRATOR
// ============================================================================

/** A character as they appear in a scene, enriched with database info. */
export interface CharacterInScene {
  name: string;
  region: string;
  view: string;
  pose: string;
  action: string;
  expression: string;
  clothing: string;
  facing: string;
  needs_render: boolean;
  /** Database character ID (null if character wasn't found in DB). */
  db_id: number | null;
  /** Whether this character has a master reference image for IP-Adapter. */
  has_reference_image: boolean;
  /** Text description for characters without a reference image (e.g. animals). */
  prompt_only_description: string | null;
}

/**
 * Compression diagnostics from the orchestrator.
 * Mirrors OrchestratorCompressionInfo in Rust.
 */
export interface OrchestratorCompressionInfo {
  total_turns: number;
  compressed_turns: number;
  recent_turns: number;
  estimated_total_tokens: number;
  max_context_tokens: number;
  compression_threshold: number;
  needs_compression: boolean;
  compressed_summary_preview: string;
}

/** Complete result of a single story turn from the orchestrator. */
export interface StoryTurnResult {
  /** Turn sequence number from the LLM. */
  turn_id: number;
  /** The narrative text to display to the player. */
  story_text: string;
  /** One-line summary for future context compression. */
  summary_hint: string;
  /** Scene environment data (location, lighting, mood, etc.). */
  scene: SceneJson | null;
  /** Characters in this scene with enriched database info. */
  characters: CharacterInScene[];
  /** Absolute path to the generated scene image (if one was produced). */
  generated_image_path: string | null;
  /** Parse quality indicator. */
  parse_status: 'ok' | 'partial' | 'fallback';
  /** Any warnings from the LLM parser (empty on "ok"). */
  parse_warnings: string[];
  /** Context compression diagnostics for the token meter UI. */
  compression_info: OrchestratorCompressionInfo;
  /** Whether image generation was attempted this turn. */
  image_generation_attempted: boolean;
  /** Error message if image generation failed (null on success or not attempted). */
  image_generation_error: string | null;
  /** LoRA filename used for pose this turn (null if none matched). */
  pose_lora_used: string | null;
  /** DB message_id of the assistant message saved this turn. Used to persist images later. */
  assistant_message_id: number | null;
  /** Scene DB id active after this turn (auto-created or matched). Null if no story selected. */
  active_scene_id: number | null;
}

// ---- Helpers ----

/** Check if the turn result has any parse quality issues. */
export function hasParseIssues(result: StoryTurnResult): boolean {
  return result.parse_status !== 'ok';
}

/** Check if an image was successfully generated this turn. */
export function hasGeneratedImage(result: StoryTurnResult): boolean {
  return result.generated_image_path !== null;
}

/** Get only the CharacterInScene entries that are actually rendered. */
export function getSceneRenderableCharacters(result: StoryTurnResult): CharacterInScene[] {
  return result.characters.filter(c => c.needs_render);
}

/** Get characters that are in the scene but couldn't be rendered (no reference image). */
export function getMissingReferenceCharacters(result: StoryTurnResult): CharacterInScene[] {
  return result.characters.filter(c => c.needs_render && !c.has_reference_image);
}

/** Check if context compression is getting close to triggering. */
export function isContextNearLimit(result: StoryTurnResult): boolean {
  return result.compression_info.needs_compression;
}

/** Get image generation status as a simple string. */
export function imageGenStatus(result: StoryTurnResult): 'not-requested' | 'success' | 'failed' {
  if (!result.image_generation_attempted) return 'not-requested';
  if (result.generated_image_path) return 'success';
  return 'failed';
}

// ============================================================================
// CONTEXT COMPRESSION
// ============================================================================

/** A single story turn (user action + assistant response pair). */
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
  /** DB message_id of the assistant message (null if not loaded from DB) */
  message_id: number | null;
  /** File path of the generated scene image for this turn (null if no image) */
  image_path: string | null;
}

/** The compressed "story so far" block that replaces older turns. */
export interface CompressedHistory {
  /** Combined summary text */
  story_so_far: string;
  /** Turn numbers that have been compressed */
  compressed_turn_ids: number[];
  /** Approximate token count of the summary */
  token_estimate: number;
}

/** Full conversation context state for one chat session. */
export interface ConversationContext {
  /** Recent uncompressed turns */
  turns: StoryTurn[];
  /** Compressed summary of older turns */
  compressed: CompressedHistory;
  /** Total tokens across uncompressed turns */
  total_turn_tokens: number;
}

/** Compression diagnostics returned by the backend. */
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

/** Result from the context assembly function. */
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

export interface PoseLora {
  id: number;
  name: string;
  keywords: string;
  lora_filename: string;
  trigger_words: string;
  strength: number;
  enabled: boolean;
  created_at: string;
}

/** Llama 3.1 8B context window */
export const MAX_CONTEXT_TOKENS = 8192;

/** Compression triggers at this fraction of the window */
export const COMPRESSION_THRESHOLD = 0.75;

/** Number of recent turns always kept in full detail */
export const RECENT_TURNS_TO_KEEP = 6;

/** Tokens reserved for system prompt + character DB + response generation */
export const RESERVED_TOKENS = 1500;

/** Rough token estimate: ~1 token per 4 characters. */
export function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

/** Calculate what percentage of the context window is used (0–1). */
export function contextUsagePercent(estimatedTokens: number): number {
  return Math.min(estimatedTokens / MAX_CONTEXT_TOKENS, 1.0);
}

/** Get a human-readable status for the current context usage. */
export function contextStatus(estimatedTokens: number): 'ok' | 'warning' | 'critical' {
  const usage = estimatedTokens / MAX_CONTEXT_TOKENS;
  if (usage < 0.5) return 'ok';
  if (usage < COMPRESSION_THRESHOLD) return 'warning';
  return 'critical';
}