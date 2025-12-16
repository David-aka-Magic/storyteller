<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  
  import Sidebar from '../components/Sidebar.svelte';
  import ChatArea from '../components/ChatArea.svelte';
  import ContextMenu from '../components/ContextMenu.svelte';
  import CharacterModal from '../components/CharacterModal.svelte';
  import StoryModal from '../components/StoryModal.svelte'; 
  
  import type { 
    ChatSummary, ChatMessage, StoryResponse, SdDetails, 
    Phase1Response, SelectionState, ContextMenuData,
    CharacterProfile, StoryPremise
  } from '../lib/types';

  let chatList: ChatSummary[] = [];
  let currentChatId: number = 1;
  let messages: ChatMessage[] = [];
  let isLoading = false;
  
  let selectionState: SelectionState = {
    selectedIds: new Set<number>(),
    isSelecting: false
  };

  let contextMenu: ContextMenuData = {
    show: false,
    x: 0,
    y: 0,
    chatId: null
  };

  let showCharModal = false;
  let characterToEdit: CharacterProfile | null = null;
  let characters: CharacterProfile[] = [];
  let selectedCharacterIds: Set<string> = new Set();
  
  let showStoryModal = false;
  let storyToEdit: StoryPremise | null = null;
  let stories: StoryPremise[] = [];
  let selectedStoryId: string = '';

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

        if (msg.images && msg.images.length > 0) {
            chatMsg.image = msg.images[0];
        }

        if (chatMsg.sender === 'ai') {
            try {
                if (msg.content.trim().startsWith('{')) {
                    const parsedJson = JSON.parse(msg.content);
                    
                    if (parsedJson.story_json || parsedJson.sd_json) {
                        let storyText = parsedJson.story_json?.response || parsedJson.story_json || parsedJson.response || "Story loaded from history.";
                        if (typeof storyText !== 'string') storyText = "Story data found, but text extraction failed.";
                        
                        let sdDetails: SdDetails | undefined = undefined;
                        if (parsedJson.sd_json) sdDetails = parsedJson.sd_json;

                        chatMsg.data = {
                            story: storyText,
                            sd_prompt: sdDetails?.look,
                            sd_details: sdDetails,
                        };
                        chatMsg.text = ""; 
                    }
                }
            } catch (e) {
                console.warn("Failed to parse AI message", e);
                chatMsg.text = msg.content;
            }
        }
        if (chatMsg.sender === 'user') chatMsg.text = msg.content;
        return chatMsg;
    });
  }

  async function fetchChatList() {
    try {
      const list = await invoke('get_chat_list') as ChatSummary[];
      chatList = list;
      if (chatList.length > 0 && !chatList.some(c => c.id === currentChatId)) {
        currentChatId = chatList[chatList.length - 1].id;
      }
    } catch (e) {
      console.error("Failed to fetch chat list:", e);
    }
  }

  async function loadSelectedChat(id: number) {
    if (isLoading || id === currentChatId) return;
    isLoading = true;
    currentChatId = id;
    messages = []; 

    try {
      const msgs = await invoke('load_chat', { id }) as any[];
      messages = mapMessagesToFrontend(msgs);
    } catch (e) {
      console.error(`Failed to load chat ${id}:`, e);
      messages = [{ id: Date.now(), text: `Error loading chat: ${e}`, sender: 'ai' }];
    } finally {
      isLoading = false;
    }
  }

  async function startNewChat() {
    if(isLoading) return;
    try {
      const newId = await invoke('new_chat') as number;
      currentChatId = newId;
      messages = [];
      clearSelection();
      await fetchChatList(); 
    } catch (e) {
      console.error("Failed to create new chat:", e);
    }
  }

  async function clearChat() {
    if (isLoading) return;
    try {
        await invoke('clear_history');
        messages = [];
        clearSelection();
        await fetchChatList();
    } catch (e) {
        console.error("Failed to clear history:", e);
    }
  }

  async function sendMessage(e: CustomEvent<string>) {
    const userMsgText = e.detail;
    const isNewChat = messages.length === 0;
    isLoading = true;

    messages = [...messages, { 
        id: Date.now(),
        text: userMsgText, 
        sender: 'user' 
    }];

    try {
      const response = await invoke('generate_story', { prompt: userMsgText });
      
      let newAiMsg: ChatMessage = {
          id: Date.now() + 1, 
          text: '',
          sender: 'ai',
          data: undefined
      };

      if (response && typeof response === 'object') {
        if ('type' in response && (response as any).type === 'phase1') {
          newAiMsg.text = (response as Phase1Response).text;
        } else if ('story' in response) {
          newAiMsg.text = ""; 
          newAiMsg.data = response as StoryResponse;
        } else {
            newAiMsg.text = JSON.stringify(response, null, 2);
        }
      } else if (typeof response === 'string') {
          newAiMsg.text = response;
      }

      messages = [...messages, newAiMsg];
      if (isNewChat) await fetchChatList();

    } catch (error) {
        let errorMessage = error instanceof Error ? error.message : String(error);
        messages = [...messages, { 
            id: Date.now(),
            text: `Error: ${errorMessage}`, 
            sender: 'ai' 
        }];
    } finally {
      isLoading = false;
    }
  }

  async function fetchStoryList() {
    try {
        const list = await invoke('get_story_list') as StoryPremise[];
        stories = [{ id: '1', title: 'Free Write', description: 'No specific plot constraints.' }, ...list];
        
        if (!selectedStoryId || !stories.some(s => s.id === selectedStoryId)) {
            selectedStoryId = stories[0].id;
        }
    } catch (e) {
        console.error("Failed to fetch story list:", e);
    }
  }

  function openStoryCreator() {
    storyToEdit = null;
    showStoryModal = true;
  }

  function editStory(story: StoryPremise) {
    storyToEdit = story;
    showStoryModal = true;
  }

  async function handleSaveStory(e: CustomEvent<StoryPremise>) {
    const newStory = e.detail;
    try {
        await invoke('save_story_premise', { story: newStory });
        await fetchStoryList();
        showStoryModal = false;
    } catch (error) {
        console.error("Failed to save story:", error);
    }
  }

  async function deleteStory(id: string) {
    try {
        await invoke('delete_stories', { ids: [id] });
        await fetchStoryList();
        if (selectedStoryId === id) {
             selectedStoryId = stories[0].id;
        }
    } catch (error) {
        console.error("Failed to delete story:", error);
    }
  }

  async function fetchCharacterList() {
      try {
          const list = await invoke('get_character_list') as CharacterProfile[];
          characters = list;
      } catch (e) {
          console.error("Failed to fetch characters:", e);
      }
  }

  function openCreator() {
    characterToEdit = null;
    showCharModal = true;
  }

  function editCharacter(char: CharacterProfile) {
    characterToEdit = char;
    showCharModal = true;
  }

  async function handleSaveCharacter(e: CustomEvent<CharacterProfile>) {
    const newChar = e.detail;
    try {
        await invoke('save_character', { character: newChar });
        await fetchCharacterList();
        if (!characters.some(c => c.id === newChar.id)) {
             selectedCharacterIds.add(newChar.id);
             selectedCharacterIds = selectedCharacterIds;
        }
        showCharModal = false;
    } catch (error) {
        console.error("Failed to save character:", error);
    }
  }
  
  async function deleteCharacter(id: string) {
    try {
        await invoke('delete_character', { id }); 
        await fetchCharacterList();
        selectedCharacterIds.delete(id);
        selectedCharacterIds = selectedCharacterIds; 
    } catch (error) {
        console.error("Failed to delete character:", error);
    }
  }

  function toggleCharacterSelection(id: string) {
    if (selectedCharacterIds.has(id)) {
        selectedCharacterIds.delete(id);
    } else {
        selectedCharacterIds.add(id);
    }
    selectedCharacterIds = selectedCharacterIds; 
  }

  function handleContextMenu(e: CustomEvent<{event: MouseEvent, chatId: number}>) {
    const { event, chatId } = e.detail;
    if (!selectionState.isSelecting) {
      selectionState.selectedIds.clear();
      selectionState.isSelecting = true;
    }
    
    if (selectionState.selectedIds.has(chatId)) {
      selectionState.selectedIds.delete(chatId);
      if (selectionState.selectedIds.size === 0) selectionState.isSelecting = false;
    } else {
      selectionState.selectedIds.add(chatId);
    }
    selectionState.selectedIds = selectionState.selectedIds; 
    contextMenu = {
      show: true,
      x: event.clientX,
      y: event.clientY,
      chatId
    };
  }

  function handleChatSelect(e: CustomEvent<number>) {
    const chatId = e.detail;
    if (selectionState.isSelecting) {
      if (selectionState.selectedIds.has(chatId)) {
        selectionState.selectedIds.delete(chatId);
        if (selectionState.selectedIds.size === 0) selectionState.isSelecting = false;
      } else {
        selectionState.selectedIds.add(chatId);
      }
      selectionState.selectedIds = selectionState.selectedIds;
    } else {
      loadSelectedChat(chatId);
    }
  }

  function selectAllChats() {
    selectionState.isSelecting = true;
    selectionState.selectedIds = new Set(chatList.map(chat => chat.id));
    closeContextMenu();
  }

  function clearSelection() {
    selectionState.selectedIds = new Set();
    selectionState.isSelecting = false;
    closeContextMenu();
  }

  async function deleteSelectedChats() {
    if (isLoading || selectionState.selectedIds.size === 0) return;
    isLoading = true;
    try {
      const deleted = await invoke('delete_chats', {ids: Array.from(selectionState.selectedIds)}) as boolean;
      if (deleted) {
        await fetchChatList();
        if (selectionState.selectedIds.has(currentChatId)) {
          if (chatList.length > 0) await loadSelectedChat(chatList[0].id);
          else await startNewChat();
        }
        clearSelection();
      }
    } catch (error) {
      console.error('Failed to delete chats:', error);
    } finally {
      isLoading = false;
    }
  }

  function closeContextMenu() {
    contextMenu.show = false;
  }

  async function deleteSingleChat(chatId: number) {
    selectionState.selectedIds = new Set([chatId]);
    await deleteSelectedChats();
    closeContextMenu();
  }

  onMount(() => {
    document.addEventListener('click', (e) => {
        const target = e.target as HTMLElement;
        if (!target.closest('.context-menu') && !target.closest('.chat-item')) {
            closeContextMenu();
        }
    });
    document.addEventListener('contextmenu', closeContextMenu);
    
    fetchChatList();
    fetchStoryList();
    fetchCharacterList(); 
  });
</script>

<main>
  <div class="app-layout">
    <Sidebar 
      {chatList}
      {currentChatId}
      {isLoading}
      {selectionState}
      on:newChat={startNewChat}
      on:deleteSelected={deleteSelectedChats}
      on:selectChat={handleChatSelect}
      on:contextMenu={handleContextMenu}
      on:selectAll={selectAllChats}
      on:clearSelection={clearSelection}
    />
    
    <ChatArea 
      {messages}
      {isLoading}
      {currentChatId}
      on:sendMessage={sendMessage}
      on:clearChat={clearChat}
    />

    <div class="config-panel">
        <div class="config-section">
            <div class="section-header">
                <h2>Story Setting</h2>
                <button class="create-btn" on:click={openStoryCreator}>+ Create Story</button>
            </div>
            
            <div class="story-list">
                {#each stories as story (story.id)}
                    <label class="radio-item" class:selected-radio={selectedStoryId === story.id}>
                        <input 
                            type="radio" 
                            bind:group={selectedStoryId} 
                            value={story.id} 
                        />
                        <div class="radio-content">
                            <span class="radio-title">{story.title}</span>
                            <span class="radio-desc">{story.description}</span>
                        </div>
                        {#if story.id !== '1'}
                            <div class="story-controls">
                                <button class="edit-btn" on:click|preventDefault={() => editStory(story)}>✎</button>
                                <button class="delete-btn" on:click|preventDefault={() => deleteStory(story.id)}>🗑️</button>
                            </div>
                        {/if}
                    </label>
                {/each}
            </div>
        </div>

        <div class="config-section">
            <div class="section-header">
                <h2>Characters</h2>
                <button class="create-btn" on:click={openCreator}>+ Create</button>
            </div>
            
            {#if characters.length === 0}
                <p class="empty-state">No characters yet. Create one to add them to your story!</p>
            {:else}
                <div class="char-list">
                    {#each characters as char (char.id)}
                        <div 
                            class="char-item" 
                            class:active={selectedCharacterIds.has(char.id)}
                        >
                            <label class="char-checkbox-area">
                                <input 
                                    type="checkbox" 
                                    checked={selectedCharacterIds.has(char.id)}
                                    on:change={() => toggleCharacterSelection(char.id)}
                                />
                                <div class="char-info">
                                    <span class="char-name">{char.name}</span>
                                    <span class="char-meta">{char.gender}, {char.age}</span>
                                </div>
                            </label>
                            <div class="char-controls">
                                <button class="edit-icon" on:click={() => editCharacter(char)}>✎</button>
                                <button class="delete-icon" on:click={() => deleteCharacter(char.id)}>🗑️</button>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    </div>

    {#if contextMenu.show}
      <ContextMenu 
        x={contextMenu.x}
        y={contextMenu.y}
        chatId={contextMenu.chatId ?? 0}
        {selectionState}
        on:close={closeContextMenu}
        on:delete={(e) => deleteSingleChat(e.detail)}
        on:selectAll={selectAllChats}
        on:cancelSelection={clearSelection}
        on:startSelection={() => {
            selectionState.isSelecting = true;
            selectionState.selectedIds.add(contextMenu.chatId!);
            selectionState.selectedIds = selectionState.selectedIds;
            closeContextMenu();
        }}
      />
    {/if}

    <CharacterModal 
        show={showCharModal}
        character={characterToEdit}
        on:close={() => showCharModal = false}
        on:save={handleSaveCharacter}
    />

    <StoryModal 
        show={showStoryModal}
        story={storyToEdit}
        on:close={() => showStoryModal = false}
        on:save={handleSaveStory}
    />
  </div>
</main>

<style>
  main {
      margin: 0;
      padding: 0;
      height: 100vh;
      display: flex;
      flex-direction: column;
      box-sizing: border-box;
      font-family: Arial, sans-serif;
  }

  .app-layout {
      flex: 1;
      display: flex;
      overflow: hidden; 
  }

  .config-panel {
      width: 300px;
      background: #f9f9fc;
      border-left: 1px solid #ddd;
      display: flex;
      flex-direction: column;
      overflow-y: auto;
      flex-shrink: 0;
      padding: 20px;
      gap: 30px;
  }

  .config-section h2 {
      font-size: 1.2em;
      color: #333;
      margin-bottom: 15px;
      padding-bottom: 5px;
      border-bottom: 2px solid #e0e0e0;
  }

  .section-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 15px;
  }
  
  .section-header h2 { margin-bottom: 0; border-bottom: none; }

  .create-btn {
      background: #28a745;
      color: white;
      border: none;
      padding: 4px 10px;
      border-radius: 4px;
      font-size: 0.8em;
      cursor: pointer;
  }
  .create-btn:hover { background: #218838; }

  .empty-state { font-size: 0.9em; color: #999; font-style: italic; }

  .story-list { display: flex; flex-direction: column; gap: 10px; }
  
  .radio-item {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 10px;
      padding: 10px;
      background: white;
      border: 1px solid #eee;
      border-radius: 6px;
      cursor: pointer;
      transition: all 0.2s;
  }
  
  .radio-item:hover { background: #f0f4ff; border-color: #bbdefb; }
  .radio-item.selected-radio { 
      background: #e3f2fd; 
      border-color: #2196f3;
  }

  .radio-content { display: flex; flex-direction: column; flex: 1; }
  .radio-title { font-weight: bold; font-size: 0.95em; color: #333; }
  .radio-desc { font-size: 0.8em; color: #666; margin-top: 2px; }
  
  .story-controls { display: flex; gap: 5px; }
  .story-controls button {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 0.9em;
    padding: 3px;
  }
  .story-controls .edit-btn { color: #007bff; }
  .story-controls .delete-btn { color: #dc3545; }

  .char-list { display: flex; flex-direction: column; gap: 8px; }

  .char-item {
      display: flex;
      justify-content: space-between;
      align-items: center;
      background: white;
      border: 1px solid #eee;
      border-radius: 6px;
      padding: 8px;
      transition: all 0.2s;
  }

  .char-item.active { border-color: #2196f3; background: #f5faff; }

  .char-checkbox-area {
      display: flex;
      align-items: center;
      gap: 10px;
      flex: 1;
      cursor: pointer;
  }

  .char-info { display: flex; flex-direction: column; }
  .char-name { font-weight: bold; font-size: 0.9em; }
  .char-meta { font-size: 0.75em; color: #777; }

  .char-controls { display: flex; gap: 5px; }
  
  .char-controls button {
      background: transparent;
      border: none;
      cursor: pointer;
      padding: 4px;
      font-size: 1em;
  }
  .char-controls .edit-icon { color: #007bff; }
  .char-controls .delete-icon { color: #dc3545; }

</style>