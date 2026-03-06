<script lang="ts">
  import { onMount } from 'svelte';

  import TitleBar from '../components/layout/TitleBar.svelte';
  import Sidebar from '../components/layout/Sidebar.svelte';
  import TopBar from '../components/layout/TopBar.svelte';
  import MainView from '../components/layout/MainView.svelte';
  import ConfigPanel from '../components/ConfigPanel.svelte';
  import ContextMenu from '../components/ContextMenu.svelte';
  import CharacterModal from '../components/CharacterModal.svelte';
  import StoryModal from '../components/StoryModal.svelte';
  import SettingsModal from '../components/settings/SettingsModal.svelte';
  import StoryGallery from '../components/story/StoryGallery.svelte';

  import { currentTheme, applyTheme } from '$lib/stores/theme';
  import { processStoryTurn, generateSceneImageForTurn } from '$lib/api/text-gen';
  import { readFileBase64 } from '$lib/api/image-gen';
  import { getChatList, newChat, loadChat, deleteChats, clearHistory, setChatCharacter, saveImageForMessage } from '$lib/api/chat';
  import {
    listCharactersForStory,
    addCharacter, updateCharacter,
    deleteCharacter as apiDeleteCharacter,
    linkCharacterToStory,
  } from '$lib/api/character';
  import { getStoryList, saveStoryPremise, deleteStories } from '$lib/api/story';

  import type {
    ChatSummary, ChatMessage, StoryResponse, SdDetails,
    Phase1Response, SelectionState, ContextMenuData,
    CharacterProfile, StoryPremise, StoryTurnResult
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
  let showGallery = false;

  // Orchestrator state
  let lastTurnResult: StoryTurnResult | null = null;

  async function mapMessagesToFrontend(backendMessages: any[]): Promise<ChatMessage[]> {
    let idCounter = Date.now();
    const results: ChatMessage[] = [];
    for (let index = 0; index < backendMessages.length; index++) {
      const msg = backendMessages[index];
      let chatMsg: ChatMessage = {
          id: idCounter + index,
          text: msg.content,
          sender: msg.role === 'user' ? 'user' : 'ai',
          data: undefined,
          image: undefined,
          dbMessageId: msg.db_id ?? undefined,
      };

      if (msg.images && msg.images.length > 0) {
        const imgPath: string = msg.images[0];
        if (imgPath.startsWith('data:')) {
          chatMsg.image = imgPath;
        } else {
          try {
            const base64 = await readFileBase64(imgPath);
            chatMsg.image = `data:image/png;base64,${base64}`;
          } catch (e) {
            console.warn('[mapMessages] Failed to load image:', imgPath, e);
          }
        }
      }

      if (chatMsg.sender === 'ai') {
          const extracted = extractStoryFromContent(msg.content);
          if (extracted) {
              chatMsg.text = '';
              chatMsg.data = extracted;
          } else if (msg.content.trim().startsWith('{')) {
              chatMsg.text = cleanJsonFallback(msg.content);
          }
      }
      results.push(chatMsg);
    }
    return results;
  }

  function extractStoryFromContent(content: string): { story: string; sd_prompt?: string; sd_details?: any } | null {
    const trimmed = content.trim();
    if (!trimmed.startsWith('{')) return null;

    let parsed: any;
    try {
      parsed = JSON.parse(trimmed);
    } catch {
      try {
        parsed = JSON.parse(trimmed.replace(/\/\/[^\n]*/g, ''));
      } catch {
        const fenceMatch = trimmed.match(/```(?:json)?\s*(\{[\s\S]*\})\s*```/);
        if (fenceMatch) {
          try { parsed = JSON.parse(fenceMatch[1]); } catch { return null; }
        } else {
          return null;
        }
      }
    }

    let storyText: string | undefined =
      parsed?.story_json?.response ??
      (typeof parsed?.story_json === 'string' ? parsed.story_json : undefined) ??
      (typeof parsed?.response === 'string' ? parsed.response : undefined) ??
      (typeof parsed?.story === 'string' ? parsed.story : undefined) ??
      (typeof parsed?.story_json?.summary_hint === 'string' ? parsed.story_json.summary_hint : undefined);

    if (!storyText || typeof storyText !== 'string') return null;

    let sdPrompt: string | undefined;
    let sdDetails: any | undefined;
    if (parsed.sd_json) {
      sdPrompt = parsed.sd_json.look;
      sdDetails = parsed.sd_json;
    } else if (parsed.scene_json) {
      const s = parsed.scene_json;
      sdPrompt = [s.location, s.lighting, s.mood].filter(Boolean).join(', ');
    }

    return { story: storyText, sd_prompt: sdPrompt, sd_details: sdDetails };
  }

  function cleanJsonFallback(content: string): string {
    try {
      const parsed = JSON.parse(content.replace(/\/\/[^\n]*/g, ''));
      const candidates = [
        parsed?.story_json?.response,
        parsed?.story_json,
        parsed?.response,
        parsed?.story,
        parsed?.story_json?.summary_hint,
      ];
      for (const c of candidates) {
        if (typeof c === 'string' && c.trim().length > 10) return c;
      }
    } catch { /* fall through */ }

    return content
      .replace(/[{}"[\]]/g, '')
      .replace(/story_json|scene_json|characters_in_scene|generation_flags|turn_id|response|summary_hint|sd_json/g, '')
      .replace(/:\s*/g, ' ')
      .replace(/,\s*/g, ' ')
      .replace(/\s+/g, ' ')
      .trim() || 'Story data could not be displayed.';
  }

  async function fetchChatList() {
    try {
      chatList = await getChatList();
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
      const msgs = await loadChat(id) as any[];
      messages = await mapMessagesToFrontend(msgs);

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
    if (isLoading) return;
    try {
      currentChatId = await newChat();
      messages = [];
      selectedCharacterIds.clear();
      selectedCharacterIds = selectedCharacterIds;
      await fetchChatList();
    } catch (e) { console.error(e); }
  }

  async function clearChat() {
    if (isLoading) return;
    try {
        await clearHistory(currentChatId);
        messages = [];
        await fetchChatList();
    } catch (e) { console.error(e); }
  }

  async function deleteSelectedChats() {
    if (selectionState.selectedIds.size === 0) return;
    const ids = Array.from(selectionState.selectedIds);
    try {
        await deleteChats(ids);
        selectionState = { isSelecting: false, selectedIds: new Set() };
        await fetchChatList();
        if (ids.includes(currentChatId) && chatList.length > 0) {
            currentChatId = chatList[0].id;
            await loadSelectedChat(currentChatId);
        }
    } catch (e) { console.error(e); }
  }

  async function sendMessage(text: string) {
      const isNew = messages.length === 0;
      isLoading = true;
      messages = [...messages, { id: Date.now(), text, sender: 'user' }];

      console.log('[DEBUG] Calling processStoryTurn with chatId:', currentChatId, 'text:', text);

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
          dbMessageId: result.assistant_message_id ?? undefined,
          sceneCharacterNames: result.characters.map(c => c.name),
        };

        if (result.generated_image_path) {
          try {
            const base64 = await readFileBase64(result.generated_image_path);
            newMsg.image = `data:image/png;base64,${base64}`;
          } catch (e) {
            console.warn('[Orchestrator] Failed to load image:', e);
          }
        }

        messages = [...messages, newMsg];

        if (result.parse_status !== 'ok') console.warn('[Orchestrator] Parse warnings:', result.parse_warnings);
        if (result.image_generation_error) console.warn('[Orchestrator] Image gen error:', result.image_generation_error);

        if (isNew) await fetchChatList();
      } catch (err) {
          console.error('[DEBUG] processStoryTurn error:', JSON.stringify(err));
          messages = [...messages, { id: Date.now(), text: `Error: ${err}`, sender: 'ai' }];
      } finally {
        isLoading = false;
      }
  }

  // Called by ChatArea when a per-message image is generated
  function handleImageGenerated(msgId: number, src: string, filePath: string) {
    const idx = messages.findIndex(m => m.id === msgId);
    if (idx !== -1) {
      messages[idx] = { ...messages[idx], image: src };
      messages = messages;

      const dbMsgId = messages[idx].dbMessageId;
      if (dbMsgId) {
        saveImageForMessage(dbMsgId, currentChatId, filePath).catch(e =>
          console.warn('[handleImageGenerated] Failed to persist image:', e)
        );
      }
    }
  }

  // Called by ChatArea after regenerating the last AI message
  function handleRegenerated(newMsg: ChatMessage) {
    messages = [...messages.slice(0, -1), newMsg];
  }

  // TopBar "Generate Image" — generates image for the last AI message with a prompt
  async function generateCurrentImage() {
    const lastAiMsg = [...messages].reverse().find(m => m.sender === 'ai' && m.data?.sd_prompt);
    if (!lastAiMsg || isLoading) return;
    const storyId = selectedStoryId && selectedStoryId !== '1' ? parseInt(selectedStoryId) : undefined;
    try {
      const path = await generateSceneImageForTurn(
        lastAiMsg.data!.sd_prompt!,
        storyId,
        lastAiMsg.sceneCharacterNames
      );
      const base64 = await readFileBase64(path);
      handleImageGenerated(lastAiMsg.id, `data:image/png;base64,${base64}`, path);
    } catch (e) { console.error('[TopBar] Image gen failed:', e); }
  }

  async function fetchStoryList() {
    try {
        const list = await getStoryList();
        stories = [{ id: '1', title: 'Free Write', description: 'No constraints.' }, ...list];
        if (!selectedStoryId) selectedStoryId = stories[0].id;
    } catch (e) { console.error(e); }
  }

  async function handleSaveStory(form: StoryPremise) {
      try {
          await saveStoryPremise(form.title, form.description, form.id ? Number(form.id) : null);
          await fetchStoryList();
          showStoryModal = false;
      } catch (e) { console.error(e); }
  }

  async function deleteStory(id: string) {
    try {
        await deleteStories([Number(id)]);
        await fetchStoryList();
        if (selectedStoryId === id) selectedStoryId = stories[0].id;
    } catch (e) { console.error(e); }
  }

  async function fetchCharacterList() {
      try {
        const storyId = selectedStoryId && selectedStoryId !== '1'
          ? parseInt(selectedStoryId, 10)
          : null;
        characters = await listCharactersForStory(storyId ?? undefined);
      } catch (e) { console.error(e); }
    }

  async function handleSaveCharacter(form: CharacterProfile) {
      try {
          const storyId = selectedStoryId && selectedStoryId !== '1'
              ? parseInt(selectedStoryId, 10)
              : undefined;

          const characterToSave = { ...form, story_id: storyId ?? form.story_id };

          if (form.id && form.id > 0) {
              await updateCharacter(characterToSave);
          } else {
              const newId = await addCharacter({ ...characterToSave, id: 0 });
              if (storyId && newId) await linkCharacterToStory(newId, storyId);
          }

          showCharModal = false;
          await fetchCharacterList();
      } catch (e) { console.error('Failed to save character:', e); }
  }

  async function handleLinkToStory(characterId: number) {
      if (!selectedStoryId || selectedStoryId === '1') return;
      try {
        await linkCharacterToStory(characterId, parseInt(selectedStoryId, 10));
        await fetchCharacterList();
      } catch (e) { console.error('[LinkToStory] Failed:', e); }
    }

  async function deleteCharacter(id: number) {
    try {
        await apiDeleteCharacter(id);
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
        await setChatCharacter(currentChatId, newId);
        const c = chatList.find(x => x.id === currentChatId);
        if (c) c.character_id = newId != null ? String(newId) : undefined;
    } catch (e) { console.error(e); }
  }

  function handleContextMenu(data: { event: MouseEvent; chatId: number }) {
    const { event, chatId } = data;
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
  <TitleBar title={chatList.find(c => c.id === currentChatId)?.title ?? 'AI Story Writer'} />
  <div class="app-layout">
    <Sidebar
      {chatList} {currentChatId} {isLoading} {selectionState}
      onnewchat={startNewChat}
      ondeleteselected={deleteSelectedChats}
      onopensettings={() => showSettingsModal = true}
      onselectchat={(id) => {
          if (selectionState.isSelecting) {
              if (selectionState.selectedIds.has(id)) selectionState.selectedIds.delete(id);
              else selectionState.selectedIds.add(id);
              if (selectionState.selectedIds.size === 0) selectionState.isSelecting = false;
              selectionState = selectionState;
          } else {
              loadSelectedChat(id);
          }
      }}
      oncontextmenu={handleContextMenu}
      onselectall={() => {
          selectionState = { isSelecting: true, selectedIds: new Set(chatList.map(c => c.id)) };
          contextMenu.show = false;
      }}
      onclearselection={() => {
          selectionState = { isSelecting: false, selectedIds: new Set() };
          contextMenu.show = false;
      }}
    />

    <div class="main-column">
      <TopBar
        title={chatList.find(c => c.id === currentChatId)?.title ?? 'AI Story Writer'}
        {isLoading}
        hasMessages={messages.length > 0}
        {configPanelCollapsed}
        onClearChat={clearChat}
        onGenerateImage={generateCurrentImage}
        onToggleConfigPanel={() => configPanelCollapsed = !configPanelCollapsed}
        onOpenGallery={() => showGallery = !showGallery}
      />
      {#if showGallery && selectedStoryId && selectedStoryId !== '1'}
        <StoryGallery
          storyId={parseInt(selectedStoryId, 10)}
          chatId={currentChatId}
          storyTitle={stories.find(s => s.id === selectedStoryId)?.title ?? 'Story'}
          onclose={() => showGallery = false}
        />
      {:else}
      <MainView
        {messages}
        {isLoading}
        {currentChatId}
        storyId={selectedStoryId && selectedStoryId !== '1' ? parseInt(selectedStoryId) : null}
        activeCharacterId={(() => { const first = Array.from(selectedCharacterIds)[0]; return first != null ? String(first) : null; })()}
        onsendmessage={sendMessage}
        onimagegenerated={handleImageGenerated}
        onregenerated={handleRegenerated}
      />
      {/if}
    </div>

    <ConfigPanel
        {stories}
        {characters}
        {selectedStoryId}
        {selectedCharacterIds}
        collapsed={configPanelCollapsed}
        onToggleCollapse={(val) => configPanelCollapsed = val}
        onSelectStory={(id) => {
            selectedStoryId = id;
            showGallery = false;
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
                await linkCharacterToStory(charId, storyId);
                await fetchCharacterList();
            }}
    />

    {#if contextMenu.show}
      <ContextMenu
        x={contextMenu.x} y={contextMenu.y} chatId={contextMenu.chatId ?? 0} {selectionState}
        onclose={() => contextMenu.show = false}
        ondelete={(id) => { selectionState.selectedIds = new Set([id]); deleteSelectedChats(); }}
        onselectall={() => { selectionState = { isSelecting: true, selectedIds: new Set(chatList.map(c => c.id)) }; contextMenu.show = false; }}
        oncancelselection={() => { selectionState = { isSelecting: false, selectedIds: new Set() }; contextMenu.show = false; }}
        onstartselection={() => {
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
        onclose={() => showSettingsModal = false}
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
    overflow: hidden;
  }

  .app-layout {
    flex: 1;
    display: flex;
    overflow: hidden;
    min-height: 0;
  }

  .main-column {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
