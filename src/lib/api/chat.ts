// src/lib/api/chat.ts — Tauri command wrappers for chat operations
import { invoke } from '@tauri-apps/api/core';
import type { ChatSummary, ChatMessage } from '$lib/types';

export async function getChatList(): Promise<ChatSummary[]> {
  return invoke('get_chat_list');
}

export async function newChat(): Promise<number> {
  return invoke('new_chat');
}

export async function loadChat(id: number): Promise<ChatMessage[]> {
  return invoke('load_chat', { id });
}

export async function deleteChats(ids: number[]): Promise<void> {
  return invoke('delete_chats', { ids });
}

export async function clearHistory(id: number): Promise<void> {
  return invoke('clear_history', { id });
}

export async function setChatCharacter(chatId: number, characterId: number | null): Promise<void> {
  return invoke('set_chat_character', { chatId, characterId });
}

export async function saveImageForMessage(messageId: number, chatId: number, filePath: string): Promise<void> {
  return invoke('save_image_for_message', { messageId, chatId, filePath });
}
