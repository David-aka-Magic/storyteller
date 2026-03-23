<!-- src/components/story/StoryView.svelte -->
<!--
  Main Story Display View — the primary interface for reading and interacting
  with the story. Composes:
    - SceneHeader (location, characters, token meter)
    - StoryTurn (scrollable history of turns)
    - StoryInput (user action input)

  Integration:
    - Calls processStoryTurn() from orchestrator-types
    - Reads from story-store (currentStory, etc.)
    - Manages loading/error/empty states

  Usage in parent:
    <StoryView
      storyId={123}
      chatId={456}
      onopenesettings={() => ...}
      oncharacterclick={(data) => ...}
    />
-->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';

  import StoryTurn from './StoryTurn.svelte';
  import SceneHeader from './SceneHeader.svelte';
  import StoryInput from './StoryInput.svelte';
  import TokenMeter from './TokenMeter.svelte';
  import SceneTransitionSuggestion from './SceneTransitionSuggestion.svelte';

  import {
    hasGeneratedImage,
    imageGenStatus,
    type CharacterEmotionalState,
    type StoryTurnResult,
    type CharacterInScene,
    type OrchestratorCompressionInfo,
    type SceneJson,
    type CharacterProfile,
  } from '$lib/types';
  import { processStoryTurn, generateSceneImageForTurn, previewScenePrompt, illustrateSceneCustom, regenerateStory, regenerateStoryWithInput } from '$lib/api/text-gen';
  import { saveImageForMessage } from '$lib/api/chat';
  import { clearImageCache } from '$lib/utils/image-url';

  import {
    currentStory,
    updateLocation,
    recordTurnPlayed,
    updateThumbnail,
  } from '$lib/stores/story-store';

  // ── Props ──
  export let storyId: number | null = null;
  export let chatId: number | null = null;
  export let storyTitle: string = '';
  export let storyDescription: string = '';
  export let characterProfiles: CharacterProfile[] = [];

  export let oncharacterclick: ((data: CharacterInScene | { char: CharacterInScene; profile?: CharacterProfile }) => void) | undefined = undefined;
  export let onshowcompressedhistory: ((summary: string | undefined) => void) | undefined = undefined;
  export let onopenesettings: (() => void) | undefined = undefined;
  /** Called when a scene is auto-created or matched after a turn. */
  export let onscenechanged: ((sceneId: number, isNewScene: boolean) => void) | undefined = undefined;
  /** Set this from the parent to trigger a scene-swap suggestion bar. Cleared automatically on send/dismiss. */
  export let incomingSceneTransition: { sceneName: string; characterNames: string[] } | null = null;

  // ── Internal State ──

  /** All accumulated turn results */
  interface DisplayTurn {
    turnNumber: number;
    userAction: string;
    storyText: string;
    imagePath: string | null;
    scene: SceneJson | null;
    characters: CharacterInScene[];
    parseStatus: 'ok' | 'partial' | 'fallback';
    parseWarnings: string[];
    imageError: string | null;
    /** DB message_id of the assistant message (needed to persist images). */
    messageId: number | null;
    /** If the scene location changed from the previous turn, show a banner. */
    sceneTransition: { location: string; timeOfDay?: string; mood?: string } | null;
    /** Raw scene prompt string used to preview enriched SDXL prompts. */
    scenePrompt: string;
    /** Enriched positive SDXL prompt (loaded lazily on expand). */
    enrichedPrompt: string | null;
    /** Enriched negative SDXL prompt (loaded lazily on expand). */
    negativePrompt: string | null;
    /** Emotional states for each character at the end of this turn. */
    emotionalStates: CharacterEmotionalState[];
  }

  /** Pending scene-swap suggestion shown above the input (user-initiated). */
  let pendingSceneTransition: { sceneName: string; characterNames: string[] } | null = null;

  let turns: DisplayTurn[] = [];
  let currentScene: SceneJson | null = null;
  let currentCharacters: CharacterInScene[] = [];
  let compressionInfo: OrchestratorCompressionInfo | null = null;
  let totalTurns = 0;

  // Loading/error
  let isGenerating = false;
  let isGeneratingImage = false;
  let lastError: string | null = null;

  /** turnNumber of the turn currently having an image generated (null = none) */
  let generatingImageForTurn: number | null = null;

  /** turnNumber of the turn currently loading its prompt preview (null = none) */
  let loadingPromptForTurn: number | null = null;

  // Scroll
  let scrollContainer: HTMLDivElement;
  let storyInputRef: StoryInput;

  // ── Derived ──
  $: isEmpty = turns.length === 0;

  // Mirror incoming scene transition from parent to local state
  $: if (incomingSceneTransition !== null) {
    pendingSceneTransition = incomingSceneTransition;
  }

  // ── Lifecycle ──
  onMount(() => {
    // If resuming a story with existing turns, populate from store
    const session = $currentStory;
    if (session) {
      storyTitle = storyTitle || session.title;
      storyDescription = storyDescription || session.description;
      storyId = storyId ?? session.story_id;
      chatId = chatId ?? session.chat_id;
      characterProfiles = characterProfiles.length > 0 ? characterProfiles : session.characters;
      totalTurns = session.total_turns;

      // Rebuild display turns from recent_turns in the session
      if (session.recent_turns && session.recent_turns.length > 0) {
        turns = session.recent_turns.map((rt) => {
          let storyText = rt.assistant_response;
          // Try to parse story text from JSON
          try {
            const parsed = JSON.parse(rt.assistant_response);
            storyText =
              parsed?.story_json?.response ||
              parsed?.response ||
              parsed?.story ||
              rt.assistant_response;
          } catch {
            // Use raw text
          }

          return {
            turnNumber: rt.turn_number,
            userAction: rt.user_input,
            storyText: typeof storyText === 'string' ? storyText : 'Story data loaded.',
            imagePath: rt.image_path ?? null,
            scene: null,
            characters: [],
            parseStatus: 'ok' as const,
            parseWarnings: [],
            imageError: null,
            messageId: rt.message_id ?? null,
            sceneTransition: null,
            scenePrompt: typeof storyText === 'string' ? storyText : 'Story data loaded.',
            enrichedPrompt: null,
            negativePrompt: null,
            emotionalStates: [],
          };
        });
      }

      // Set initial location from session
      if (session.current_location) {
        currentScene = { location: session.current_location, location_type: '', time_of_day: '', weather: '', lighting: '', mood: '' };
      }
    }

    // Focus input after mount
    tick().then(() => storyInputRef?.focus());
  });

  // ── Auto-scroll on new turns ──
  async function scrollToBottom() {
    await tick();
    if (scrollContainer) {
      scrollContainer.scrollTo({
        top: scrollContainer.scrollHeight,
        behavior: 'smooth',
      });
    }
  }

  // ── Process a turn ──
  async function handleSubmit(userInput: string) {
    if (!chatId) {
      lastError = 'No active chat. Please load or create a story first.';
      return;
    }

    isGenerating = true;
    isGeneratingImage = false;
    lastError = null;

    // Optimistically add user action to display
    const pendingTurnNumber = totalTurns + 1;

    try {
      // Kick off a timer to switch to "image generating" state after ~3s
      // (LLM usually responds in 2-5s, then image gen starts)
      const imageTimer = setTimeout(() => {
        if (isGenerating) isGeneratingImage = true;
      }, 4000);

      const result: StoryTurnResult = await processStoryTurn(chatId, userInput, storyId ?? undefined);

      clearTimeout(imageTimer);

      // Detect scene location change for the transition banner
      const prevLocation = currentScene?.location ?? null;
      const newLocation = result.scene?.location ?? null;
      const sceneTransition =
        newLocation && newLocation !== prevLocation
          ? {
              location: newLocation,
              timeOfDay: result.scene?.time_of_day || undefined,
              mood: result.scene?.mood || undefined,
            }
          : null;

      // Build the display turn
      const builtScenePrompt = buildScenePrompt(result.scene, result.story_text);
      const displayTurn: DisplayTurn = {
        turnNumber: result.turn_id || pendingTurnNumber,
        userAction: userInput,
        storyText: result.story_text,
        imagePath: result.generated_image_path,
        scene: result.scene,
        characters: result.characters,
        parseStatus: result.parse_status as 'ok' | 'partial' | 'fallback',
        parseWarnings: result.parse_warnings,
        imageError: result.image_generation_error,
        messageId: result.assistant_message_id ?? null,
        sceneTransition,
        scenePrompt: builtScenePrompt,
        enrichedPrompt: result.enriched_prompt ?? null,
        negativePrompt: result.negative_prompt ?? null,
        emotionalStates: result.emotional_states ?? [],
      };

      turns = [...turns, displayTurn];
      totalTurns = pendingTurnNumber;

      // Clear any pending suggestion (user sent a message, suggestion resolved)
      pendingSceneTransition = null;

      // Update scene state
      if (result.scene) {
        currentScene = result.scene;
        if (result.scene.location) {
          updateLocation(result.scene.location);
        }
      }

      // Update character state
      if (result.characters.length > 0) {
        currentCharacters = result.characters;
      }

      // Update compression info
      compressionInfo = result.compression_info;

      // Notify parent of scene change (for ScenePanel refresh)
      if (result.active_scene_id != null) {
        const isNew = sceneTransition !== null;
        onscenechanged?.(result.active_scene_id, isNew);
      }

      // Update story store
      recordTurnPlayed();
      if (result.generated_image_path) {
        updateThumbnail(result.generated_image_path);
      }

      // Scroll & focus
      await scrollToBottom();
      storyInputRef?.focus();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
      console.error('[StoryView] Turn failed:', lastError);
    } finally {
      isGenerating = false;
      isGeneratingImage = false;
    }
  }

  function handleCharacterClick(char: CharacterInScene) {
    oncharacterclick?.(char);
  }

  function handleHeaderCharacterClick(data: { char: CharacterInScene; profile?: CharacterProfile }) {
    oncharacterclick?.(data);
  }

  /**
   * Build a concise visual scene description from structured scene data.
   * Falls back to the raw story text if scene data is unavailable.
   */
  function buildScenePrompt(scene: SceneJson | null, fallback: string): string {
    if (!scene) return fallback;
    const parts: string[] = [];
    if (scene.location) parts.push(scene.location);
    if (scene.location_type) parts.push(scene.location_type);
    if (scene.time_of_day) parts.push(scene.time_of_day);
    if (scene.lighting) parts.push(scene.lighting + ' lighting');
    if (scene.weather && scene.weather !== 'clear' && scene.weather !== 'none') parts.push(scene.weather);
    if (scene.mood) parts.push(scene.mood + ' atmosphere');
    return parts.length > 0 ? parts.join(', ') : fallback;
  }

  async function handleExpandPrompt(turnNumber: number) {
    const turn = turns.find(t => t.turnNumber === turnNumber);
    if (!turn || turn.enrichedPrompt !== null) return; // already loaded

    loadingPromptForTurn = turnNumber;
    try {
      const preview = await previewScenePrompt(
        turn.scenePrompt,
        storyId ?? undefined,
        turn.characters.map(c => c.name),
        turn.characters.map(c => c.view ?? 'UPPER-BODY'),
      );
      turns = turns.map(t =>
        t.turnNumber === turnNumber
          ? { ...t, enrichedPrompt: preview.positive, negativePrompt: preview.negative }
          : t
      );
    } catch (e) {
      console.warn('[StoryView] Failed to load prompt preview:', e);
    } finally {
      loadingPromptForTurn = null;
    }
  }

  async function handleRewrite(data: { turnNumber: number; editedInput: string }) {
    if (!chatId || isGenerating) return;

    isGenerating = true;
    isGeneratingImage = false;
    lastError = null;

    try {
      const imageTimer = setTimeout(() => {
        if (isGenerating) isGeneratingImage = true;
      }, 4000);

      const result: StoryTurnResult = await regenerateStoryWithInput(
        chatId,
        data.editedInput,
        storyId ?? undefined
      );

      clearTimeout(imageTimer);

      const prevLocation = currentScene?.location ?? null;
      const newLocation = result.scene?.location ?? null;
      const sceneTransition =
        newLocation && newLocation !== prevLocation
          ? { location: newLocation, timeOfDay: result.scene?.time_of_day || undefined, mood: result.scene?.mood || undefined }
          : null;

      const builtScenePrompt = buildScenePrompt(result.scene, result.story_text);
      const displayTurn: DisplayTurn = {
        turnNumber: result.turn_id || data.turnNumber,
        userAction: data.editedInput,
        storyText: result.story_text,
        imagePath: result.generated_image_path,
        scene: result.scene,
        characters: result.characters,
        parseStatus: result.parse_status as 'ok' | 'partial' | 'fallback',
        parseWarnings: result.parse_warnings,
        imageError: result.image_generation_error,
        messageId: result.assistant_message_id ?? null,
        sceneTransition,
        scenePrompt: builtScenePrompt,
        enrichedPrompt: result.enriched_prompt ?? null,
        negativePrompt: result.negative_prompt ?? null,
        emotionalStates: result.emotional_states ?? [],
      };

      turns = [...turns.filter(t => t.turnNumber !== data.turnNumber), displayTurn];

      if (result.scene) {
        currentScene = result.scene;
        if (result.scene.location) updateLocation(result.scene.location);
      }
      if (result.characters.length > 0) currentCharacters = result.characters;
      compressionInfo = result.compression_info;
      if (result.active_scene_id != null) {
        onscenechanged?.(result.active_scene_id, sceneTransition !== null);
      }
      if (result.generated_image_path) updateThumbnail(result.generated_image_path);

      await scrollToBottom();
      storyInputRef?.focus();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
      console.error('[StoryView] Rewrite failed:', lastError);
    } finally {
      isGenerating = false;
      isGeneratingImage = false;
    }
  }

  async function handleRegenerate(turnNumber: number) {
    if (!chatId || isGenerating) return;

    isGenerating = true;
    isGeneratingImage = false;
    lastError = null;

    try {
      const imageTimer = setTimeout(() => {
        if (isGenerating) isGeneratingImage = true;
      }, 4000);

      const result: StoryTurnResult = await regenerateStory(chatId, storyId ?? undefined);

      clearTimeout(imageTimer);

      const oldTurn = turns.find(t => t.turnNumber === turnNumber);
      const originalAction = oldTurn?.userAction ?? '';

      const prevLocation = currentScene?.location ?? null;
      const newLocation = result.scene?.location ?? null;
      const sceneTransition =
        newLocation && newLocation !== prevLocation
          ? { location: newLocation, timeOfDay: result.scene?.time_of_day || undefined, mood: result.scene?.mood || undefined }
          : null;

      const builtScenePrompt = buildScenePrompt(result.scene, result.story_text);
      const displayTurn: DisplayTurn = {
        turnNumber: result.turn_id || turnNumber,
        userAction: originalAction,
        storyText: result.story_text,
        imagePath: result.generated_image_path,
        scene: result.scene,
        characters: result.characters,
        parseStatus: result.parse_status as 'ok' | 'partial' | 'fallback',
        parseWarnings: result.parse_warnings,
        imageError: result.image_generation_error,
        messageId: result.assistant_message_id ?? null,
        sceneTransition,
        scenePrompt: builtScenePrompt,
        enrichedPrompt: result.enriched_prompt ?? null,
        negativePrompt: result.negative_prompt ?? null,
        emotionalStates: result.emotional_states ?? [],
      };

      turns = [...turns.filter(t => t.turnNumber !== turnNumber), displayTurn];

      if (result.scene) {
        currentScene = result.scene;
        if (result.scene.location) updateLocation(result.scene.location);
      }
      if (result.characters.length > 0) currentCharacters = result.characters;
      compressionInfo = result.compression_info;
      if (result.active_scene_id != null) {
        onscenechanged?.(result.active_scene_id, sceneTransition !== null);
      }
      if (result.generated_image_path) updateThumbnail(result.generated_image_path);

      await scrollToBottom();
      storyInputRef?.focus();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
      console.error('[StoryView] Regenerate failed:', lastError);
    } finally {
      isGenerating = false;
      isGeneratingImage = false;
    }
  }

  async function handleIllustrate(data: { storyText: string; messageId: number | null; turnNumber: number; positivePrompt?: string; negativePrompt?: string }) {
    const { storyText, messageId, turnNumber, positivePrompt, negativePrompt } = data;
    if (generatingImageForTurn !== null) return; // already generating

    generatingImageForTurn = turnNumber;
    lastError = null;

    try {
      const turn = turns.find(t => t.turnNumber === turnNumber);
      // Clear stale cache entry so the new image isn't served from cache
      if (turn?.imagePath) clearImageCache(turn.imagePath);

      let imagePath: string;

      if (positivePrompt && messageId !== null && storyId !== null && chatId !== null) {
        // User provided edited prompts — backend saves to DB itself
        imagePath = await illustrateSceneCustom(
          storyId,
          chatId,
          messageId,
          positivePrompt,
          negativePrompt ?? '',
        );
      } else {
        // No edits — use the standard enrichment pipeline
        const scenePrompt = turn?.scenePrompt ?? buildScenePrompt(turn?.scene ?? null, storyText);
        imagePath = await generateSceneImageForTurn(
          scenePrompt,
          storyId ?? undefined,
          turn?.characters.map(c => c.name),
          turn?.characters.map(c => c.view ?? 'UPPER-BODY'),
        );
        // Persist to DB separately
        if (messageId !== null && chatId !== null) {
          await saveImageForMessage(messageId, chatId, imagePath).catch(e =>
            console.warn('[StoryView] Failed to persist image to DB:', e)
          );
        }
      }

      // Update the turn in the array with the new image path
      turns = turns.map(t =>
        t.turnNumber === turnNumber ? { ...t, imagePath, imageError: null } : t
      );

      // Update story thumbnail with the latest generated image
      updateThumbnail(imagePath);
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e);
      console.error('[StoryView] Illustrate scene failed:', errMsg);
      turns = turns.map(t =>
        t.turnNumber === turnNumber ? { ...t, imageError: errMsg } : t
      );
    } finally {
      generatingImageForTurn = null;
    }
  }
</script>

<div class="story-view">
  <!-- Scene Header -->
  <SceneHeader
    scene={currentScene}
    characters={currentCharacters}
    {characterProfiles}
    {compressionInfo}
    {storyTitle}
    {totalTurns}
    oncharacterclick={handleHeaderCharacterClick}
    onopenesettings={onopenesettings}
  />

  <!-- Scrollable Story Area -->
  <div class="story-scroll-area" bind:this={scrollContainer}>
    <!-- Empty State -->
    {#if isEmpty && !isGenerating}
      <div class="empty-state">
        <div class="empty-emblem">
          <svg width="64" height="64" viewBox="0 0 64 64" fill="none">
            <rect x="8" y="4" width="48" height="56" rx="4" stroke="currentColor" stroke-width="2" opacity="0.25"/>
            <path d="M18 18h28M18 26h20M18 34h24M18 42h16" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" opacity="0.2"/>
            <circle cx="48" cy="48" r="14" fill="var(--accent-primary, #58a6ff)" opacity="0.15"/>
            <path d="M44 48l3 3 6-6" stroke="var(--accent-primary, #58a6ff)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>

        <h2 class="empty-title">{storyTitle || 'Your Story Awaits'}</h2>

        {#if storyDescription}
          <p class="empty-description">{storyDescription}</p>
        {/if}

        {#if characterProfiles.length > 0}
          <div class="empty-characters">
            <span class="empty-chars-label">Characters</span>
            <div class="empty-chars-list">
              {#each characterProfiles as char}
                <div class="empty-char-chip">
                  <span class="ecc-initial">{char.name.charAt(0)}</span>
                  <span class="ecc-name">{char.name}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <div class="empty-prompt">
          <span class="empty-prompt-text">Type your first action below to begin the adventure</span>
          <span class="empty-prompt-arrow">↓</span>
        </div>
      </div>
    {/if}

    <!-- Story Turns -->
    {#if turns.length > 0}
      <div class="turns-container">
        <!-- Compressed History Indicator -->
        {#if compressionInfo && compressionInfo.compressed_turns > 0}
          <div class="compressed-indicator">
            <span class="compress-icon">📜</span>
            <span class="compress-text">
              {compressionInfo.compressed_turns} earlier turns summarized
            </span>
            <button
              class="compress-preview-btn"
              on:click={() => onshowcompressedhistory?.(compressionInfo?.compressed_summary_preview)}
              title="View summary"
            >
              View
            </button>
          </div>
        {/if}

        {#each turns as turn, i (turn.turnNumber)}
          <StoryTurn
            turnNumber={turn.turnNumber}
            userAction={turn.userAction}
            storyText={turn.storyText}
            imagePath={turn.imagePath}
            scene={turn.scene}
            characters={turn.characters}
            parseStatus={turn.parseStatus}
            parseWarnings={turn.parseWarnings}
            imageError={turn.imageError}
            isLatestTurn={i === turns.length - 1}
            messageId={turn.messageId}
            isGeneratingImage={generatingImageForTurn === turn.turnNumber}
            sceneTransition={turn.sceneTransition}
            scenePrompt={turn.scenePrompt}
            enrichedPrompt={turn.enrichedPrompt}
            negativePrompt={turn.negativePrompt}
            isLoadingPrompt={loadingPromptForTurn === turn.turnNumber}
            emotionalStates={turn.emotionalStates}
            oncharacterclick={handleCharacterClick}
            onillustratescene={handleIllustrate}
            onexpandprompt={handleExpandPrompt}
            onrewrite={handleRewrite}
            onregenerate={handleRegenerate}
          />
        {/each}
      </div>
    {/if}

    <!-- Generating Indicator (in scroll area) -->
    {#if isGenerating}
      <div class="generating-placeholder">
        <div class="gen-pulse"></div>
        <span class="gen-text">
          {isGeneratingImage ? 'Painting the scene...' : 'Crafting the next chapter...'}
        </span>
      </div>
    {/if}
  </div>

  <!-- Error Banner -->
  {#if lastError}
    <div class="error-banner">
      <span class="error-icon">⚠</span>
      <span class="error-message">{lastError}</span>
      <button class="error-dismiss" on:click={() => lastError = null}>✕</button>
    </div>
  {/if}

  <!-- Scene Transition Suggestion Bar (user-initiated scene swap) -->
  {#if pendingSceneTransition}
    <SceneTransitionSuggestion
      sceneName={pendingSceneTransition.sceneName}
      characterNames={pendingSceneTransition.characterNames}
      onSend={(text) => { pendingSceneTransition = null; handleSubmit(text); }}
      onDismiss={() => pendingSceneTransition = null}
    />
  {/if}

  <!-- Input Area -->
  <StoryInput
    bind:this={storyInputRef}
    {isGenerating}
    {isGeneratingImage}
    disabled={!chatId}
    placeholder={isEmpty ? 'Begin your adventure...' : 'What do you do next?'}
    onsubmit={handleSubmit}
  />

  <!-- Bottom Token Meter (full, only when data available) -->
  {#if compressionInfo && turns.length > 3}
    <div class="bottom-meter">
      <TokenMeter info={compressionInfo} compact={false} />
    </div>
  {/if}
</div>

<style>
  .story-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--story-bg, var(--bg-chat, #0d1117));
    color: var(--text-primary, #c9d1d9);
    position: relative;
    overflow: hidden;
  }

  /* ── Scroll Area ── */
  .story-scroll-area {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0 24px;
    scroll-behavior: smooth;
  }

  /* Subtle scrollbar */
  .story-scroll-area::-webkit-scrollbar {
    width: 6px;
  }
  .story-scroll-area::-webkit-scrollbar-track {
    background: transparent;
  }
  .story-scroll-area::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.08);
    border-radius: 3px;
  }
  .story-scroll-area::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  /* ── Turns Container ── */
  .turns-container {
    max-width: 720px;
    margin: 0 auto;
    padding: 20px 0 40px;
  }

  /* ── Compressed History Indicator ── */
  .compressed-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    margin-bottom: 16px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px dashed rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    font-size: 0.78rem;
    color: var(--text-muted, #6e7681);
  }

  .compress-icon {
    font-size: 1rem;
  }

  .compress-text {
    flex: 1;
    font-style: italic;
  }

  .compress-preview-btn {
    padding: 3px 10px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--accent-primary, #58a6ff);
    cursor: pointer;
    font-size: 0.72rem;
    font-weight: 600;
    transition: background 0.15s;
  }

  .compress-preview-btn:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  /* ── Empty State ── */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 80px 32px 60px;
    max-width: 500px;
    margin: 0 auto;
    min-height: 60%;
  }

  .empty-emblem {
    color: var(--text-muted, #6e7681);
    margin-bottom: 24px;
    opacity: 0.7;
  }

  .empty-title {
    margin: 0 0 12px;
    font-size: 1.6rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
    letter-spacing: -0.02em;
  }

  .empty-description {
    margin: 0 0 28px;
    font-size: 0.95rem;
    line-height: 1.6;
    color: var(--text-secondary, #8b949e);
    max-width: 420px;
  }

  .empty-characters {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    margin-bottom: 32px;
  }

  .empty-chars-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted, #6e7681);
  }

  .empty-chars-list {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .empty-char-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px 5px 5px;
    border-radius: 20px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .ecc-initial {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    font-size: 0.7rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .ecc-name {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-primary, #c9d1d9);
  }

  .empty-prompt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    animation: pulseGently 2.5s ease-in-out infinite;
  }

  .empty-prompt-text {
    font-size: 0.85rem;
    color: var(--text-muted, #6e7681);
    font-style: italic;
  }

  .empty-prompt-arrow {
    font-size: 1.4rem;
    color: var(--accent-primary, #58a6ff);
    opacity: 0.5;
  }

  @keyframes pulseGently {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  /* ── Generating Placeholder ── */
  .generating-placeholder {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 24px;
    max-width: 720px;
    margin: 0 auto;
  }

  .gen-pulse {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.3; transform: scale(0.8); }
    50% { opacity: 1; transform: scale(1.1); }
  }

  .gen-text {
    font-size: 0.88rem;
    color: var(--text-muted, #6e7681);
    font-style: italic;
  }

  /* ── Error Banner ── */
  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    background: rgba(224, 96, 96, 0.1);
    border-top: 1px solid rgba(224, 96, 96, 0.2);
    color: #e06060;
    font-size: 0.85rem;
  }

  .error-icon {
    flex-shrink: 0;
  }

  .error-message {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error-dismiss {
    background: none;
    border: none;
    color: #e06060;
    cursor: pointer;
    padding: 2px 6px;
    font-size: 0.9rem;
    opacity: 0.6;
    transition: opacity 0.15s;
  }

  .error-dismiss:hover {
    opacity: 1;
  }

  /* ── Bottom Meter ── */
  .bottom-meter {
    padding: 6px 24px 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
    background: rgba(0, 0, 0, 0.15);
  }
</style>