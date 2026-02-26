<script lang="ts">
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
  import {
    processStoryTurn,
    type StoryTurnResult
  } from '$lib/orchestrator-types';

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
  let selectedCharacterIds: Set<number> = new Set();
  
  let showStoryModal = false;
  let storyToEdit: StoryPremise | null = null;
  let stories: StoryPremise[] = [];
  let selectedStoryId: string = '';

  let showSettingsModal = false;
  let configPanelCollapsed = false;

  // Orchestrator state
  let lastTurnResult: StoryTurnResult | null = null;

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
          selectedCharacterIds.add(Number(chat.character_id));
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
  
      console.log('[DEBUG] Calling processStoryTurn with chatId:', currentChatId, 'text:', text); // ADD THIS
  
      try {
        const storyId = selectedStoryId && selectedStoryId !== '1'
        ? parseInt(selectedStoryId, 10)
        : undefined;

      const result = await processStoryTurn(currentChatId, text, storyId);
      lastTurnResult = result;

      const sdPrompt = result.scene
        ? [result.scene.location, result.scene.lighting, result.scene.mood]
            .filter(Boolean).join(', ')
        : undefined;

      const firstChar = result.characters[0];
      const sdDetails: SdDetails | undefined = result.scene
        ? {
            name: firstChar?.name ?? '',
            view: firstChar?.view ?? '',
            features: firstChar?.expression ?? '',
            action_context: firstChar?.action ?? '',
            clothing: firstChar?.clothing ?? '',
            look: [
              result.scene.location,
              result.scene.time_of_day,
              result.scene.lighting,
              result.scene.mood,
            ].filter(Boolean).join(', '),
          }
        : undefined;

      let newMsg: ChatMessage = {
        id: Date.now() + 1,
        text: '',
        sender: 'ai',
        data: {
          story: result.story_text,
          sd_prompt: sdPrompt,
          sd_details: sdDetails,
        },
      };

      if (result.generated_image_path) {
        try {
          const base64 = await invoke<string>('read_file_base64', { path: result.generated_image_path });
          newMsg.image = `data:image/png;base64,${base64}`;
        } catch (e) {
          console.warn('[Orchestrator] Failed to load image:', e);
        }
      }

      messages = [...messages, newMsg];

      if (result.parse_status !== 'ok') {
        console.warn('[Orchestrator] Parse warnings:', result.parse_warnings);
      }
      if (result.image_generation_error) {
        console.warn('[Orchestrator] Image gen error:', result.image_generation_error);
      }

      if (isNew) await fetchChatList();
      } catch (err) {
          console.error('[DEBUG] processStoryTurn error:', JSON.stringify(err));
          messages = [...messages, { id: Date.now(), text: `Error: ${err}`, sender: 'ai' }];
      } finally {
      isLoading = false;
    }
  }

  async function fetchStoryList() {
    try {
        const list = await invoke('get_story_list') as StoryPremise[];
        stories = [{ id: '1', title: 'Free Write', description: 'No constraints.' }, ...list];
        if (!selectedStoryId) selectedStoryId = stories[0].id;
    } catch (e) { console.error(e); }
  }

  async function handleSaveStory(form: StoryPremise) {
      try {
          await invoke('save_story_premise', {
              title: form.title,
              description: form.description,
              id: form.id ? Number(form.id) : null,
          });
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
      try {
        const storyId = selectedStoryId && selectedStoryId !== '1'
          ? parseInt(selectedStoryId, 10)
          : null;
        characters = await invoke('list_characters_for_story', { storyId });
      } catch (e) { console.error(e); }
    }

  // CharacterModal saves directly to backend and returns the form with real ID.
  // Parent just refreshes the list.
  async function handleSaveCharacter(form: CharacterProfile) {
      try {
          // Attach the current story_id if one is selected
          const storyId = selectedStoryId && selectedStoryId !== '1'
              ? parseInt(selectedStoryId, 10)
              : undefined;
  
          const characterToSave = {
              ...form,
              story_id: storyId ?? form.story_id,
          };
  
          if (form.id && form.id > 0) {
              await invoke('update_character', { character: characterToSave });
          } else {
              const newId = await invoke<number>('add_character', { character: { ...characterToSave, id: 0 } });
              
              // If we have an active story, link them
              if (storyId && newId) {
                  await invoke('link_character_to_story', { 
                      characterId: newId, 
                      storyId: storyId 
                  });
              }
          }
  
          showCharModal = false;
          await fetchCharacterList();
      } catch (e) {
          console.error('Failed to save character:', e);
      }
  }
  
  async function handleLinkToStory(characterId: number) {
      if (!selectedStoryId || selectedStoryId === '1') return;
      try {
        await invoke('link_character_to_story', {
          characterId,
          storyId: parseInt(selectedStoryId, 10),
        });
        await fetchCharacterList();
      } catch (e) { console.error('[LinkToStory] Failed:', e); }
    }
  
  async function deleteCharacter(id: number) {
    try {
        await invoke('delete_character', { id }); 
        await fetchCharacterList();
        selectedCharacterIds.delete(id);
        selectedCharacterIds = selectedCharacterIds;
    } catch (e) { console.error(e); }
  }

  async function toggleCharacterSelection(id: number) {
    let newId: number | null = null;
    
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
        if (c) c.character_id = newId != null ? String(newId) : undefined;
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
      storyId={selectedStoryId && selectedStoryId !== '1' ? parseInt(selectedStoryId) : null}
      activeCharacterId={(() => { const first = Array.from(selectedCharacterIds)[0]; return first != null ? String(first) : null; })()}
      on:sendMessage={sendMessage}
      on:clearChat={clearChat}
    />

    <ConfigPanel 
        {stories} 
        {characters} 
        {selectedStoryId} 
        {selectedCharacterIds}
        collapsed={configPanelCollapsed}
        onToggleCollapse={(val) => configPanelCollapsed = val}
        onSelectStory={(id) => { 
            selectedStoryId = id; 
            fetchCharacterList();
        }}
        onToggleCharacter={(id) => toggleCharacterSelection(id)}
        onCreateStory={() => { storyToEdit = null; showStoryModal = true; }}
        onEditStory={(story) => { storyToEdit = story; showStoryModal = true; }}
        onDeleteStory={(id) => deleteStory(id)}
        onCreateCharacter={() => { characterToEdit = null; showCharModal = true; }}
        onEditCharacter={(char) => { characterToEdit = char; showCharModal = true; }}
        onDeleteCharacter={(id) => deleteCharacter(id)}
        onLinkToStory={async (charId) => {
                const storyId = parseInt(selectedStoryId, 10);
                await invoke('link_character_to_story', { characterId: charId, storyId });
                await fetchCharacterList();
            }}
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
        show={showCharModal}
        character={characterToEdit}
        onsave={handleSaveCharacter}
        onclose={() => showCharModal = false}
    />
    
    <StoryModal 
        show={showStoryModal} story={storyToEdit}
        onClose={() => showStoryModal = false}
        onSave={(form) => handleSaveStory(form)}
    />

    <SettingsModal
        show={showSettingsModal}
        on:close={() => showSettingsModal = false}
    />
  </div>
</main>

<style>
  main { 
    margin: 0; 
    padding: 0; 
    height: 100vh; 
    width: 100vw;
    display: flex; 
    flex-direction: column; 
    font-family: 'Segoe UI', Arial, sans-serif; 
    background: var(--bg-primary);
  }

  .app-layout { 
    flex: 1; 
    display: flex; 
    overflow: hidden; 
    width: 100%;
  }
</style>