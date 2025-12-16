export interface SdDetails {
    name: string;
    view: string;
    look: string;
    features?: string;
    action_context?: string;
    clothing?: string;
}

export interface StoryResponse {
    story: string;
    sd_prompt?: string;
    sd_details?: SdDetails;
}

export interface Phase1Response {
    text: string;
    type: 'phase1';
}

export interface ChatMessage {
    id: number;
    text: string;
    sender: 'user' | 'ai';
    data?: StoryResponse;
  image?: string;
}

export interface ChatSummary {
    id: number;
    title: string;
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

export interface CharacterProfile {
    id: string;  //using string for UUID, Need to look into fixing the type error it causes when I dont.
    name: string;
    age: number;
    gender: string;
    skin_tone: string;
    hair_style: string;
    hair_color: string;
    body_type: string;
    personality: string;
    additional_notes: string;
    sd_prompt: string;
}

export interface StoryPremise {
    id: string;
    title: string;
    description: string;
}