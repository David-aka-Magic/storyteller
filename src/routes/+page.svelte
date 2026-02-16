

l<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  
  import Sidebar from '../components/Sidebar.svelte';
  import ChatArea from '../components/ChatArea.svelte';
  import ConfigPanel from '../components/ConfigPanel.svelte';
  import ContextMenu from '../components/ContextMenu.svelte';
  import CharacterModal from '../components/CharacterModal.svelte';
  import StoryModal from '../components/StoryModal.svelte';
  import SettingsModal from '../components/SettingsModal.svelte';

  import { currentTheme, applyTheme } from '$lib/stores/theme';

  import type { 
    ChatSummary, ChatMessage, StoryResponse, SdDetails, 
    Phase1Response, SelectionState, ContextMenuData,
    CharacterProfile, StoryPremise
  } from '../lib/types';

  let chatList: ChatSummary[] = [];
  let currentChatId: number = 1;
  let messages: ChatMessage[] = [];
  let isLoading = false;
  
  let selectionState: SelectionState = { selectedIds: new Set<number>(), isSelecting: false };
  let contextMenu: ContextMenuData = { show: false, x: 0, y: 0, chatId: null };

  let showCharModal = false;
  let characterToEdit: CharacterProfile | null = null;
  let characters: CharacterProfile[] = [];
  let selectedCharacterIds: Set<string> = new Set();
  
  let showStoryModal = false;
  let storyToEdit: StoryPremise | null = null;
  let stories: StoryPremise[] = [];
  let selectedStoryId: string = '';

  let showSettingsModal = false;
  let configPanelCollapsed = false;

  function mapMessagesToFrontend(backendMessages: any[]): ChatMessage[] {
    let idCounter = Date.now();
    return backendMessages.map((msg, index) => {
        let chatMsg: ChatMessage = {
            id: idCounter + index,
            text: msg.content,
            sender: msg.role === 'user' ? 'user' : 'ai',
            data: undefined,
            image: undefined
        };

        if (msg.images && msg.images.length > 0) chatMsg.image = msg.images[0];

        if (chatMsg.sender === 'ai' && msg.content.trim().startsWith('{')) {
            try {
                const parsed = JSON.parse(msg.content);
                if (parsed.story_json || parsed.sd_json) {
                    let storyText = parsed.story_json?.response || parsed.story_json || parsed.response || "Story loaded.";
                    if (typeof storyText !== 'string') storyText = "Story data error.";
                    
                    chatMsg.data = {
                        story: storyText,
                        sd_prompt: parsed.sd_json?.look,
                        sd_details: parsed.sd_json,
                    };
                    chatMsg.text = ""; 
                }
            } catch (e) { chatMsg.text = msg.content; }
        }
        return chatMsg;
    });
  }

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

    try {
      const msgs = await invoke('load_chat', { id }) as any[];
      messages = mapMessagesToFrontend(msgs);
      
      const chat = chatList.find(c => c.id === id);
      selectedCharacterIds.clear();
      if (chat && chat.character_id) {
          selectedCharacterIds.add(chat.character_id);
      }
      selectedCharacterIds = selectedCharacterIds;
    } catch (e) {
      messages = [{ id: Date.now(), text: `Error: ${e}`, sender: 'ai' }];
    } finally {
      isLoading = false;
    }
  }

  async function startNewChat() {
    if(isLoading) return;
    try {
      currentChatId = await invoke('new_chat');
      messages = [];
      selectedCharacterIds.clear();
      selectedCharacterIds = selectedCharacterIds;
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

  async function sendMessage(e: CustomEvent<string>) {
    const text = e.detail;
    const isNew = messages.length === 0;
    isLoading = true;
    messages = [...messages, { id: Date.now(), text, sender: 'user' }];

    try {
      const res: any = await invoke('generate_story', { prompt: text, chatId: currentChatId });
      let newMsg: ChatMessage = { id: Date.now() + 1, text: '', sender: 'ai' };

      if (res && typeof res === 'object' && 'story' in res) {
          newMsg.data = res as StoryResponse;
      } else if (res && typeof res === 'object' && res.type === 'phase1') {
          newMsg.text = (res as Phase1Response).text;
      } else {
          newMsg.text = typeof res === 'string' ? res : JSON.stringify(res);
      }
      messages = [...messages, newMsg];
      if (isNew) await fetchChatList();
    } catch (err) {
        messages = [...messages, { id: Date.now(), text: `Error: ${err}`, sender: 'ai' }];
    } finally { isLoading = false; }
  }

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
        showStoryModal = false;
    } catch (e) { console.error(e); }
  }

  async function deleteStory(id: string) {
    try {
        await invoke('delete_stories', { ids: [id] });
        await fetchStoryList();
        if (selectedStoryId === id) selectedStoryId = stories[0].id;
    } catch (e) { console.error(e); }
  }

  async function fetchCharacterList() {
      try { characters = await invoke('get_character_list'); } catch (e) { console.error(e); }
  }

  async function handleSaveCharacter(e: CustomEvent<CharacterProfile>) {
    try {
        await invoke('save_character', { character: e.detail });
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
  
  function handleContextMenu(e: CustomEvent<{event: MouseEvent, chatId: number}>) {
    const { event, chatId } = e.detail;
    if (!selectionState.isSelecting) {
      selectionState.selectedIds.clear();
      selectionState.isSelecting = true;
    }
    selectionState.selectedIds.add(chatId);
    selectionState = selectionState;
    contextMenu = { show: true, x: event.clientX, y: event.clientY, chatId };
  }

  onMount(() => {
    // Apply theme on mount
    applyTheme($currentTheme);
    
    document.addEventListener('click', (e) => {
        if (contextMenu.show && !(e.target as HTMLElement).closest('.context-menu')) contextMenu.show = false;
    });
    fetchChatList();
    fetchStoryList();
    fetchCharacterList(); 
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
          } else {
              loadSelectedChat(e.detail);
          }
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
    
    <ChatArea 
      {messages} {isLoading} {currentChatId}
      activeCharacterId={Array.from(selectedCharacterIds)[0] || null}
      on:sendMessage={sendMessage}
      on:clearChat={clearChat}
    />

    <ConfigPanel 
        {stories} 
        {characters} 
        {selectedStoryId} 
        {selectedCharacterIds}
        collapsed={configPanelCollapsed}
        on:toggleCollapse={(e) => configPanelCollapsed = e.detail}
        on:selectStory={(e) => selectedStoryId = e.detail}
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
        on:close={() => showCharModal = false}
        on:save={handleSaveCharacter}
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
  </div>
</main>

<style>
  :global(:root) {
    /* Default light theme - will be overridden by JS */
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
    margin: 0; 
    padding: 0; 
    height: 100vh; 
    display: flex; 
    flex-direction: column; 
    font-family: 'Segoe UI', Arial, sans-serif; 
    background: var(--bg-primary);
  }

  .app-layout { 
    flex: 1; 
    display: flex; 
    overflow: hidden; 
  }
</style>