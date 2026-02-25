// src/lib/types.ts

export interface ChatMessage {
  id: number;
  text: string;
  sender: "user" | "ai";
  data?: StoryResponse;
  image?: string;
}

export interface ChatSummary {
  id: number;
  title: string;
  messages: any[];
  character_id?: string;
}

export interface Phase1Response {
  text: string;
  type: "phase1";
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
}

export interface CharacterProfile {
  id: number;                   // ← FIXED: was string, Rust backend expects i64
  story_id?: number;
  name: string;
  age: number;
  gender: string;
  skin_tone: string;
  hair_style: string;
  hair_color: string;
  body_type: string;
  personality: string;
  additional_notes: string;
  default_clothing?: string;
  sd_prompt: string;
  image?: string;
  master_image_path?: string;
  seed?: number;
  art_style?: string;
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