<!-- src/routes/+page.svelte -->
<!--
  StoryEngine — Main Application Page
  =====================================
  Three-mode center panel:
    • Story Home:  Default landing when no story is loaded. Shows grid of
                   saved stories, "New Story" button, empty state.
    • Story Mode:  When a story is loaded via the store → StoryView (VN UI).
    • Chat Mode:   Legacy ChatArea for "Free Write" / direct chat sessions.

  Layout: Sidebar (left) | Center (StoryHome / StoryView / ChatArea) | ConfigPanel (right)
  Modals: CharacterModal, StoryModal, SettingsModal, CompressedHistoryModal
-->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  // ── Components ──
  import Sidebar from '../components/Sidebar.svelte';
  import ChatArea from '../components/ChatArea.svelte';
  import ConfigPanel from '../components/ConfigPanel.svelte';
  import ContextMenu from '../components/ContextMenu.svelte';
  import CharacterModal from '../components/CharacterModal.svelte';
  import StoryModal from '../components/StoryModal.svelte';
  import SettingsModal from '../components/SettingsModal.svelte';
  import StoryView from '../components/story/StoryView.svelte';
  import StoryHome from '../components/story/StoryHome.svelte';

  // ── Stores ──
  import { currentTheme, applyTheme } from '$lib/stores/theme';
  import {
    currentStory,
    currentStoryId as storeStoryId,
    currentChatId as storeChatId,
    loadStory,
    newStory as createNewStory,
    unloadStory,
    refreshStoryList as refreshStoryStore,
  } from '$lib/stores/story-store';

  // ── Types ──
  import {
    processStoryTurn,
    type StoryTurnResult,
  } from '$lib/orchestrator-types';

  import type {
    ChatSummary, ChatMessage, StoryResponse, SdDetails,
    Phase1Response, SelectionState, ContextMenuData,
    CharacterProfile, StoryPremise,
  } from '../lib/types';

  // ═══════════════════════════════════════════════════════════════════════════
  // STATE
  // ═══════════════════════════════════════════════════════════════════════════

  // View Mode: 'home' | 'story' | 'chat'
  type ViewMode = 'home' | 'story' | 'chat';
  let viewMode: ViewMode = 'home';

  // ── Chat / Legacy ──
  let chatList: ChatSummary[] = [];
  let currentChatId: number = 1;
  let messages: ChatMessage[] = [];
  let isLoading = false;

  // ── Selection / Context Menu ──
  let selectionState: SelectionState = { selectedIds: new Set<number>(), isSelecting: false };
  let contextMenu: ContextMenuData = { show: false, x: 0, y: 0, chatId: null };

  // ── Characters ──
  let showCharModal = false;
  let characterToEdit: CharacterProfile | null = null;
  let characters: CharacterProfile[] = [];
  let selectedCharacterIds: Set<string> = new Set();

  // ── Stories ──
  let showStoryModal = false;
  let storyToEdit: StoryPremise | null = null;
  let stories: StoryPremise[] = [];
  let selectedStoryId: string = '';

  // ── Settings ──
  let showSettingsModal = false;
  let configPanelCollapsed = false;

  // ── Orchestrator ──
  let lastTurnResult: StoryTurnResult | null = null;

  // ── Compressed History Preview ──
  let showCompressedHistoryModal = false;
  let compressedHistoryPreview = '';

  // ═══════════════════════════════════════════════════════════════════════════
  // DERIVED
  // ═══════════════════════════════════════════════════════════════════════════

  $: if ($currentStory !== null && $storeChatId !== null) {
    viewMode = 'story';
  }

  $: activeStoryTitle = $currentStory?.title ?? '';
  $: activeStoryDescription = $currentStory?.description ?? '';
  $: activeStoryCharacters = $currentStory?.characters ?? [];

  // ═══════════════════════════════════════════════════════════════════════════
  // LEGACY CHAT HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  function mapMessagesToFrontend(backendMessages: any[]): ChatMessage[] {
    let idCounter = Date.now();
    return backendMessages.map((msg, index) => {
      let chatMsg: ChatMessage = {
        id: idCounter + index,
        text: msg.content,
        sender: msg.role === 'user' ? 'user' : 'ai',
        data: undefined,
        image: undefined,
      };
      if (msg.images && msg.images.length > 0) chatMsg.image = msg.images[0];
      if (chatMsg.sender === 'ai' && msg.content.trim().startsWith('{')) {
        try {
          const parsed = JSON.parse(msg.content);
          if (parsed.story_json || parsed.sd_json) {
            let storyText = parsed.story_json?.response || parsed.story_json || parsed.response || 'Story loaded.';
            if (typeof storyText !== 'string') storyText = 'Story data error.';
            chatMsg.data = {
              story: storyText,
              sd_prompt: parsed.sd_json?.look,
              sd_details: parsed.sd_json,
            };
            chatMsg.text = '';
          }
        } catch (e) { chatMsg.text = msg.content; }
      }
      return chatMsg;
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CHAT LIFECYCLE
  // ═══════════════════════════════════════════════════════════════════════════

  async function fetchChatList() {
    try {
      chatList = await invoke('get_chat_list');
      if (chatList.length > 0 && !chatList.some(c => c.id === currentChatId)) {
        currentChatId = chatList[chatList.length - 1].id;
      }
    } catch (e) { console.error(e); }
  }

  async function loadSelectedChat(id: number) {
    if (isLoading || id === currentChatId) return;
    isLoading = true;
    currentChatId = id;
    messages = [];
    if ($currentStory) await unloadStory();
    viewMode = 'chat';

    try {
      const msgs = await invoke('load_chat', { id }) as any[];
      messages = mapMessagesToFrontend(msgs);
      const chat = chatList.find(c => c.id === id);
      selectedCharacterIds.clear();
      if (chat && chat.character_id) selectedCharacterIds.add(chat.character_id);
      selectedCharacterIds = selectedCharacterIds;
    } catch (e) {
      messages = [{ id: Date.now(), text: `Error: ${e}`, sender: 'ai' }];
    } finally {
      isLoading = false;
    }
  }

  async function startNewChat() {
    if (isLoading) return;
    try {
      if ($currentStory) await unloadStory();
      currentChatId = await invoke('new_chat');
      messages = [];
      selectedCharacterIds.clear();
      selectedCharacterIds = selectedCharacterIds;
      viewMode = 'chat';
      await fetchChatList();
    } catch (e) { console.error(e); }
  }

  async function clearChat() {
    if (isLoading) return;
    try {
      await invoke('clear_history', { id: currentChatId });
      messages = [];
      await fetchChatList();
    } catch (e) { console.error(e); }
  }

  async function deleteSelectedChats() {
    if (selectionState.selectedIds.size === 0) return;
    const ids = Array.from(selectionState.selectedIds);
    try {
      await invoke('delete_chats', { ids });
      selectionState = { isSelecting: false, selectedIds: new Set() };
      await fetchChatList();
      if (ids.includes(currentChatId) && chatList.length > 0) {
        currentChatId = chatList[0].id;
        await loadSelectedChat(currentChatId);
      }
    } catch (e) { console.error(e); }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SEND MESSAGE — Legacy ChatArea path
  // ═══════════════════════════════════════════════════════════════════════════

  async function sendMessage(e: CustomEvent<string>) {
    const text = e.detail;
    const isNew = messages.length === 0;
    isLoading = true;
    messages = [...messages, { id: Date.now(), text, sender: 'user' }];

    try {
      const storyId = selectedStoryId && selectedStoryId !== '1'
        ? parseInt(selectedStoryId, 10) : undefined;

      const result = await processStoryTurn(currentChatId, text, storyId);
      lastTurnResult = result;

      const sdPrompt = result.scene
        ? [result.scene.location, result.scene.lighting, result.scene.mood].filter(Boolean).join(', ')
        : undefined;

      const firstChar = result.characters[0];
      const sdDetails: SdDetails | undefined = result.scene
        ? {
            name: firstChar?.name ?? '', view: firstChar?.view ?? '',
            features: firstChar?.expression ?? '', action_context: firstChar?.action ?? '',
            clothing: firstChar?.clothing ?? '',
            look: [result.scene.location, result.scene.time_of_day, result.scene.lighting, result.scene.mood].filter(Boolean).join(', '),
          }
        : undefined;

      let newMsg: ChatMessage = {
        id: Date.now() + 1, text: '', sender: 'ai',
        data: { story: result.story_text, sd_prompt: sdPrompt, sd_details: sdDetails },
      };
      if (result.generated_image_path) newMsg.image = result.generated_image_path;
      messages = [...messages, newMsg];

      if (result.parse_status !== 'ok') console.warn('[Orchestrator] Parse warnings:', result.parse_warnings);
      if (result.image_generation_error) console.warn('[Orchestrator] Image gen error:', result.image_generation_error);
      if (isNew) await fetchChatList();
    } catch (err) {
      messages = [...messages, { id: Date.now(), text: `Error: ${err}`, sender: 'ai' }];
    } finally {
      isLoading = false;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STORY CRUD
  // ═══════════════════════════════════════════════════════════════════════════

  async function fetchStoryList() {
    try {
      const list = await invoke('get_story_list') as StoryPremise[];
      stories = [{ id: '1', title: 'Free Write', description: 'No constraints.' }, ...list];
      if (!selectedStoryId) selectedStoryId = stories[0].id;
    } catch (e) { console.error(e); }
  }

  async function handleSaveStory(e: CustomEvent<StoryPremise>) {
    try {
      await invoke('save_story_premise', { story: e.detail });
      await fetchStoryList();
      await refreshStoryStore();
      showStoryModal = false;
    } catch (e) { console.error(e); }
  }

  async function deleteStory(id: string) {
    try {
      await invoke('delete_stories', { ids: [id] });
      await fetchStoryList();
      await refreshStoryStore();
      if (selectedStoryId === id) selectedStoryId = stories[0].id;
    } catch (e) { console.error(e); }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STORY SELECTION — ConfigPanel bridge
  // ═══════════════════════════════════════════════════════════════════════════

  async function handleSelectStory(id: string) {
    selectedStoryId = id;
    if (id === '1') {
      if ($currentStory) await unloadStory();
      viewMode = 'chat';
      return;
    }
    const numericId = parseInt(id, 10);
    if (!isNaN(numericId)) {
      const session = await loadStory(numericId);
      if (session && session.chat_id) {
        currentChatId = session.chat_id;
        messages = [];
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STORY HOME HANDLERS
  // ═══════════════════════════════════════════════════════════════════════════

  async function handleHomeLoadStory(e: CustomEvent<number>) {
    const storyId = e.detail;
    const session = await loadStory(storyId);
    if (session && session.chat_id) {
      currentChatId = session.chat_id;
      messages = [];
      selectedStoryId = String(storyId);
    }
  }

  async function handleHomeNewStory(e: CustomEvent<{ title: string; description: string; characterIds: number[] }>) {
    const { title, description, characterIds } = e.detail;
    const charIds = characterIds.length > 0 ? characterIds : undefined;
    const storyId = await createNewStory(title, description, charIds);
    if (storyId !== null) {
      const session = $currentStory;
      if (session?.chat_id) currentChatId = session.chat_id;
      selectedStoryId = String(storyId);
      messages = [];
      await fetchStoryList();
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CHARACTER CRUD
  // ═══════════════════════════════════════════════════════════════════════════

  async function fetchCharacterList() {
    try { characters = await invoke('get_character_list'); } catch (e) { console.error(e); }
  }

  async function handleSaveCharacter(character: CharacterProfile) {
    try {
      await invoke('save_character', { character });
      await fetchCharacterList();
      showCharModal = false;
    } catch (e) { console.error(e); }
  }

  async function deleteCharacter(id: string) {
    try {
      await invoke('delete_character', { id });
      await fetchCharacterList();
      selectedCharacterIds.delete(id);
      selectedCharacterIds = selectedCharacterIds;
    } catch (e) { console.error(e); }
  }

  async function toggleCharacterSelection(id: string) {
    let newId: string | null = null;
    if (selectedCharacterIds.has(id)) {
      selectedCharacterIds.delete(id);
    } else {
      selectedCharacterIds.clear();
      selectedCharacterIds.add(id);
      newId = id;
    }
    selectedCharacterIds = selectedCharacterIds;
    try {
      await invoke('set_chat_character', { chatId: currentChatId, characterId: newId });
      const c = chatList.find(x => x.id === currentChatId);
      if (c) c.character_id = newId || undefined;
    } catch (e) { console.error(e); }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CONTEXT MENU
  // ═══════════════════════════════════════════════════════════════════════════

  function handleContextMenu(e: CustomEvent<{ event: MouseEvent; chatId: number }>) {
    const { event, chatId } = e.detail;
    if (!selectionState.isSelecting) {
      selectionState.selectedIds.clear();
      selectionState.isSelecting = true;
    }
    selectionState.selectedIds.add(chatId);
    selectionState = selectionState;
    contextMenu = { show: true, x: event.clientX, y: event.clientY, chatId };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STORY VIEW EVENTS
  // ═══════════════════════════════════════════════════════════════════════════

  function handleStoryCharacterClick(e: CustomEvent<any>) {
    const detail = e.detail;
    if (detail?.db_id || detail?.char?.db_id) {
      const dbId = detail.db_id ?? detail.char?.db_id;
      const profile = detail.profile ?? characters.find(
        (c: CharacterProfile) => String(c.id) === String(dbId)
      );
      if (profile) { characterToEdit = profile; showCharModal = true; }
    }
  }

  function handleShowCompressedHistory(e: CustomEvent<string>) {
    compressedHistoryPreview = e.detail || 'No compressed history available.';
    showCompressedHistoryModal = true;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // LIFECYCLE
  // ═══════════════════════════════════════════════════════════════════════════

  onMount(() => {
    applyTheme($currentTheme);
    document.addEventListener('click', (e) => {
      if (contextMenu.show && !(e.target as HTMLElement).closest('.context-menu'))
        contextMenu.show = false;
    });
    fetchChatList();
    fetchStoryList();
    fetchCharacterList();
    refreshStoryStore();
  });
</script>

<main>
  <div class="app-layout">
    <Sidebar
      {chatList} {currentChatId} {isLoading} {selectionState}
      on:newChat={startNewChat}
      on:deleteSelected={deleteSelectedChats}
      on:openSettings={() => showSettingsModal = true}
      on:selectChat={(e) => {
        if (selectionState.isSelecting) {
          const id = e.detail;
          if (selectionState.selectedIds.has(id)) selectionState.selectedIds.delete(id);
          else selectionState.selectedIds.add(id);
          if (selectionState.selectedIds.size === 0) selectionState.isSelecting = false;
          selectionState = selectionState;
        } else { loadSelectedChat(e.detail); }
      }}
      on:contextMenu={handleContextMenu}
      on:selectAll={() => {
        selectionState = { isSelecting: true, selectedIds: new Set(chatList.map(c => c.id)) };
        contextMenu.show = false;
      }}
      on:clearSelection={() => {
        selectionState = { isSelecting: false, selectedIds: new Set() };
        contextMenu.show = false;
      }}
    />

    {#if viewMode === 'story'}
      <StoryView
        storyId={$storeStoryId}
        chatId={$storeChatId}
        storyTitle={activeStoryTitle}
        storyDescription={activeStoryDescription}
        characterProfiles={activeStoryCharacters}
        on:openSettings={() => showSettingsModal = true}
        on:characterClick={handleStoryCharacterClick}
        on:showCompressedHistory={handleShowCompressedHistory}
      />
    {:else if viewMode === 'chat'}
      <ChatArea
        {messages} {isLoading} {currentChatId}
        activeCharacterId={Array.from(selectedCharacterIds)[0] || null}
        on:sendMessage={sendMessage}
        on:clearChat={clearChat}
      />
    {:else}
      <StoryHome
        availableCharacters={characters}
        on:loadStory={handleHomeLoadStory}
        on:newStory={handleHomeNewStory}
        on:openSettings={() => showSettingsModal = true}
      />
    {/if}

    <ConfigPanel
      {stories} {characters} {selectedStoryId} {selectedCharacterIds}
      collapsed={configPanelCollapsed}
      on:toggleCollapse={(e) => configPanelCollapsed = e.detail}
      on:selectStory={(e) => handleSelectStory(e.detail)}
      on:toggleCharacter={(e) => toggleCharacterSelection(e.detail)}
      on:createStory={() => { storyToEdit = null; showStoryModal = true; }}
      on:editStory={(e) => { storyToEdit = e.detail; showStoryModal = true; }}
      on:deleteStory={(e) => deleteStory(e.detail)}
      on:createCharacter={() => { characterToEdit = null; showCharModal = true; }}
      on:editCharacter={(e) => { characterToEdit = e.detail; showCharModal = true; }}
      on:deleteCharacter={(e) => deleteCharacter(e.detail)}
    />

    {#if contextMenu.show}
      <ContextMenu
        x={contextMenu.x} y={contextMenu.y} chatId={contextMenu.chatId ?? 0} {selectionState}
        on:close={() => contextMenu.show = false}
        on:delete={(e) => { selectionState.selectedIds = new Set([e.detail]); deleteSelectedChats(); }}
        on:selectAll={() => { selectionState = { isSelecting: true, selectedIds: new Set(chatList.map(c => c.id)) }; contextMenu.show = false; }}
        on:cancelSelection={() => { selectionState = { isSelecting: false, selectedIds: new Set() }; contextMenu.show = false; }}
        on:startSelection={() => {
          selectionState = { isSelecting: true, selectedIds: new Set([contextMenu.chatId!]) };
          contextMenu.show = false;
        }}
      />
    {/if}

    <CharacterModal
      show={showCharModal} character={characterToEdit}
      onclose={() => showCharModal = false}
      onsave={(form) => handleSaveCharacter(form)}
    />
    <StoryModal
      show={showStoryModal} story={storyToEdit}
      on:close={() => showStoryModal = false}
      on:save={handleSaveStory}
    />
    <SettingsModal
      show={showSettingsModal}
      on:close={() => showSettingsModal = false}
    />

    {#if showCompressedHistoryModal}
      <div class="modal-backdrop"
        on:click={() => showCompressedHistoryModal = false}
        on:keydown={(e) => e.key === 'Escape' && (showCompressedHistoryModal = false)}
        role="button" tabindex="0"
      >
        <div class="compressed-history-modal" on:click|stopPropagation role="dialog" tabindex="-1">
          <div class="ch-modal-header">
            <h2>📜 Story So Far</h2>
            <button class="ch-close-btn" on:click={() => showCompressedHistoryModal = false}>✕</button>
          </div>
          <div class="ch-modal-body">
            <p class="ch-description">Compressed summary of earlier turns, condensed to save context space.</p>
            <div class="ch-preview-text">{compressedHistoryPreview}</div>
          </div>
        </div>
      </div>
    {/if}
  </div>
</main>

<style>
  :global(:root) {
    --bg-primary: #ffffff;
    --bg-secondary: #f4f4f9;
    --bg-tertiary: #f9f9fc;
    --bg-chat: #fafafa;
    --bg-message: #ffffff;
    --bg-message-user: #e3f2fd;
    --bg-message-ai: #ffffff;
    --bg-hover: #e0e0e5;
    --bg-active: #4a9eff;
    --text-primary: #333333;
    --text-secondary: #666666;
    --text-muted: #999999;
    --text-inverse: #ffffff;
    --border-primary: #dddddd;
    --border-secondary: #eeeeee;
    --border-active: #2196f3;
    --accent-primary: #007bff;
    --accent-secondary: #6c757d;
    --accent-success: #28a745;
    --accent-danger: #dc3545;
    --accent-warning: #ffc107;
    --shadow: rgba(0, 0, 0, 0.1);
  }

  :global(body) {
    margin: 0;
    padding: 0;
    background: var(--bg-primary);
    color: var(--text-primary);
    transition: background 0.3s, color 0.3s;
  }

  main {
    margin: 0; padding: 0; height: 100vh;
    display: flex; flex-direction: column;
    font-family: 'Segoe UI', Arial, sans-serif;
    background: var(--bg-primary);
  }

  .app-layout { flex: 1; display: flex; overflow: hidden; }

  .modal-backdrop {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000; backdrop-filter: blur(4px);
  }

  .compressed-history-modal {
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 12px; width: 520px; max-height: 70vh;
    overflow: hidden; box-shadow: 0 20px 40px var(--shadow);
    display: flex; flex-direction: column;
  }

  .ch-modal-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 18px 22px; border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
  }
  .ch-modal-header h2 { margin: 0; font-size: 1.15em; color: var(--text-primary); }

  .ch-close-btn {
    background: none; border: none; font-size: 1.4em;
    cursor: pointer; color: var(--text-muted);
    padding: 4px; line-height: 1; transition: color 0.15s;
  }
  .ch-close-btn:hover { color: var(--text-primary); }

  .ch-modal-body { padding: 20px 22px; overflow-y: auto; flex: 1; }

  .ch-description {
    margin: 0 0 14px; font-size: 0.85em;
    color: var(--text-muted); line-height: 1.5;
  }

  .ch-preview-text {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 8px; padding: 16px;
    font-size: 0.9em; line-height: 1.65;
    color: var(--text-primary); white-space: pre-wrap;
    max-height: 320px; overflow-y: auto;
  }
</style>