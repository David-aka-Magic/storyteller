<script lang="ts">
  import { onMount } from 'svelte';

  import SetupWizard from '../components/setup/SetupWizard.svelte';
  import TitleBar from '../components/layout/TitleBar.svelte';
  import Sidebar from '../components/layout/Sidebar.svelte';
  import TopBar from '../components/layout/TopBar.svelte';
  import MainView from '../components/layout/MainView.svelte';
  import ScenePanel from '../components/layout/ScenePanel.svelte';
  import SceneTransitionSuggestion from '../components/story/SceneTransitionSuggestion.svelte';
  import CharacterModal from '../components/CharacterModal.svelte';
  import StoryModal from '../components/StoryModal.svelte';
  import SettingsModal from '../components/settings/SettingsModal.svelte';
  import StoryGallery from '../components/story/StoryGallery.svelte';

  import { currentTheme, applyTheme } from '$lib/stores/theme';
  import { processStoryTurn, generateSceneImageForTurn } from '$lib/api/text-gen';
  import { readFileBase64 } from '$lib/api/image-gen';
  import { newChat, loadChat, clearHistory, setChatCharacter, saveImageForMessage } from '$lib/api/chat';
  import {
    listCharactersForStory,
    addCharacter, updateCharacter,
    deleteCharacter as apiDeleteCharacter,
    linkCharacterToStory,
  } from '$lib/api/character';
  import { listStories, loadStory, createStory, saveStoryPremise, deleteStory as apiDeleteStory } from '$lib/api/story';
  import { getConfig } from '$lib/api/config';
  import { setSceneHint } from '$lib/api/scene';

  import type {
    ChatMessage, SdDetails,
    CharacterProfile, StoryPremise, StoryTurnResult
  } from '../lib/types';

  let currentChatId: number = 1;
  let messages: ChatMessage[] = [];
  let isLoading = false;

  let showCharModal = false;
  let characterToEdit: CharacterProfile | null = null;
  let characters: CharacterProfile[] = [];
  let selectedCharacterIds: Set<number> = new Set();

  let showStoryModal = false;
  let storyToEdit: StoryPremise | null = null;
  let stories: StoryPremise[] = [];
  let selectedStoryId: string = '';

  let showSettingsModal = false;
  let scenePanelCollapsed = false;
  let showGallery = false;

  // Setup wizard gate
  let setupComplete = false;

  // Orchestrator state
  let lastTurnResult: StoryTurnResult | null = null;

  // Scene sync state
  let scenePanelRefreshKey = 0;
  let pendingSceneTransition: { sceneName: string; characterNames: string[] } | null = null;

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

  async function loadSelectedStory(id: string) {
    if (id === selectedStoryId && messages.length > 0) return;
    selectedStoryId = id;
    showGallery = false;

    if (id === '1') {
      // Free Write — use default chat (id 1) and clear the view
      currentChatId = 1;
      isLoading = true;
      try {
        const msgs = await loadChat(1) as any[];
        messages = await mapMessagesToFrontend(msgs);
      } catch {
        messages = [];
      } finally {
        isLoading = false;
      }
      selectedCharacterIds = new Set();
      await fetchCharacterList();
      return;
    }

    isLoading = true;
    try {
      const session = await loadStory(parseInt(id, 10));
      if (session.chat_id) {
        currentChatId = session.chat_id;
        const msgs = await loadChat(session.chat_id) as any[];
        messages = await mapMessagesToFrontend(msgs);
      } else {
        messages = [];
      }
      selectedCharacterIds = new Set();
      await fetchCharacterList();
    } catch (e) {
      console.error('[loadSelectedStory] failed:', e);
      messages = [];
    } finally {
      isLoading = false;
    }
  }

  async function clearChat() {
    if (isLoading) return;
    try {
      await clearHistory(currentChatId);
      messages = [];
    } catch (e) { console.error(e); }
  }

  async function sendMessage(text: string) {
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

      // Refresh ScenePanel if a scene was auto-created/matched this turn
      if (result.active_scene_id != null) {
        scenePanelRefreshKey += 1;
        pendingSceneTransition = null;
      }
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
      const list = await listStories();
      stories = [
        { id: '1', title: 'Free Write', description: 'No constraints.' },
        ...list.map(s => ({ id: String(s.story_id), title: s.title, description: s.description })),
      ];
      if (!selectedStoryId) selectedStoryId = stories[0].id;
    } catch (e) { console.error(e); }
  }

  async function handleSaveStory(form: StoryPremise) {
    try {
      if (form.id && Number(form.id) > 0) {
        await saveStoryPremise(form.title, form.description, Number(form.id));
        await fetchStoryList();
        showStoryModal = false;
      } else {
        const newId = await createStory(form.title, form.description);
        await fetchStoryList();
        showStoryModal = false;
        await loadSelectedStory(String(newId));
      }
    } catch (e) { console.error(e); }
  }

  async function deleteStory(id: string) {
    try {
      await apiDeleteStory(Number(id));
      await fetchStoryList();
      if (selectedStoryId === id) await loadSelectedStory('1');
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
    } catch (e) { console.error(e); }
  }

  function loadAppData() {
    fetchStoryList();
    fetchCharacterList();
  }

  async function handleSetupComplete() {
    setupComplete = true;
    loadAppData();
  }

  onMount(async () => {
    applyTheme($currentTheme);

    try {
      const cfg = await getConfig();
      setupComplete = cfg.setup_completed;
    } catch {
      setupComplete = false;
    }

    if (setupComplete) {
      loadAppData();
    }
  });
</script>

<main>
  <TitleBar title={setupComplete ? (stories.find(s => s.id === selectedStoryId)?.title ?? 'AI Story Writer') : 'AI Story Writer — Setup'} />

  {#if !setupComplete}
    <SetupWizard oncomplete={handleSetupComplete} />
  {:else}
  <div class="app-layout">
    <Sidebar
      {stories}
      {selectedStoryId}
      {isLoading}
      onnewstory={() => { storyToEdit = null; showStoryModal = true; }}
      onselectstory={(id) => loadSelectedStory(id)}
      onopensettings={() => showSettingsModal = true}
      ondeletestory={(id) => deleteStory(id)}
      oneditstory={(story) => { storyToEdit = story; showStoryModal = true; }}
    />

    <div class="main-column">
      <TopBar
        title={stories.find(s => s.id === selectedStoryId)?.title ?? 'AI Story Writer'}
        {isLoading}
        hasMessages={messages.length > 0}
        onClearChat={clearChat}
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

      {#if pendingSceneTransition}
        <SceneTransitionSuggestion
          sceneName={pendingSceneTransition.sceneName}
          characterNames={pendingSceneTransition.characterNames}
          onSend={(text) => { pendingSceneTransition = null; sendMessage(text); }}
          onDismiss={() => pendingSceneTransition = null}
        />
      {/if}
    </div>

    <ScenePanel
      storyId={selectedStoryId && selectedStoryId !== '1' ? parseInt(selectedStoryId, 10) : null}
      storyCharacters={characters}
      collapsed={scenePanelCollapsed}
      refreshKey={scenePanelRefreshKey}
      onToggleCollapse={(val) => scenePanelCollapsed = val}
      onCreateCharacter={() => { characterToEdit = null; showCharModal = true; }}
      onEditCharacter={(char) => { characterToEdit = char; showCharModal = true; }}
      onDeleteCharacter={(id) => deleteCharacter(id)}
      onsceneselected={(sceneId, sceneName, charNames) => {
        pendingSceneTransition = { sceneName, characterNames: charNames };
        const storyIdNum = selectedStoryId ? parseInt(selectedStoryId, 10) : null;
        if (storyIdNum) setSceneHint(storyIdNum, sceneId).catch(e => console.error('[Page] setSceneHint failed:', e));
      }}
    />

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
  {/if}
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
