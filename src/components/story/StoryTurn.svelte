<!-- src/components/story/StoryTurn.svelte -->
<!--
  Displays a single story turn:
    - User action (what the player typed)
    - Story response (narrative text from LLM)
    - Generated scene image (if any)
    - Turn number badge
    - Parse warnings (if any)
-->
<script lang="ts">
  import type { CharacterEmotionalState, CharacterInScene, SceneJson } from '$lib/types';
  import { filePathToDataUrl } from '$lib/utils/image-url';
  import ImageLightbox from '../shared/ImageLightbox.svelte';

  export let turnNumber: number = 0;
  export let userAction: string = '';
  export let storyText: string = '';
  export let imagePath: string | null = null;
  export let scene: SceneJson | null = null;
  export let characters: CharacterInScene[] = [];
  export let parseStatus: 'ok' | 'partial' | 'fallback' = 'ok';
  export let parseWarnings: string[] = [];
  export let imageError: string | null = null;
  export let isLatestTurn: boolean = false;
  /** DB message_id for this turn's assistant message (needed to persist generated images). */
  export let messageId: number | null = null;
  /** Whether an image is currently being generated for this specific turn. */
  export let isGeneratingImage: boolean = false;

  /** Short scene description used as the collapsed prompt preview. */
  export let scenePrompt: string = '';
  /** Full enriched SDXL positive prompt (fetched lazily on expand). */
  export let enrichedPrompt: string | null = null;
  /** Full SDXL negative prompt (fetched lazily on expand). */
  export let negativePrompt: string | null = null;
  /** True while the parent is fetching the preview prompts. */
  export let isLoadingPrompt: boolean = false;

  export let emotionalStates: CharacterEmotionalState[] = [];

  export let oncharacterclick: ((char: CharacterInScene) => void) | undefined = undefined;
  export let onillustratescene: ((data: { storyText: string; messageId: number | null; turnNumber: number; positivePrompt?: string; negativePrompt?: string }) => void) | undefined = undefined;
  /** Fired when the user expands the prompt editor — parent should fetch enrichedPrompt/negativePrompt. */
  export let onexpandprompt: ((turnNumber: number) => void) | undefined = undefined;
  export let onrewrite: ((data: { turnNumber: number; editedInput: string }) => void) | undefined = undefined;
  export let onregenerate: ((turnNumber: number) => void) | undefined = undefined;
  /** If set, shows a scene-change marker above this turn. */
  export let sceneTransition: { location: string; timeOfDay?: string; mood?: string } | null = null;

  /** Format narrative text: handle paragraph breaks and dialogue */
  function formatNarrative(text: string): string[] {
    if (!text) return [];
    return text
      .split(/\n\n+/)
      .map(p => p.trim())
      .filter(p => p.length > 0);
  }

  /** Check if a paragraph looks like dialogue */
  function isDialogue(paragraph: string): boolean {
    return /^[""\u201C]/.test(paragraph.trim()) || /^[A-Z][a-z]+\s*:/.test(paragraph.trim());
  }

  let resolvedImageUrl: string | null = null;
  let imageLoaded = false;
  let imageLoadError = false;
  let showLightbox = false;

  $: if (imagePath) {
    filePathToDataUrl(imagePath).then(url => {
      resolvedImageUrl = url;
      imageLoaded = false;
      imageLoadError = false;
    });
  } else {
    resolvedImageUrl = null;
  }

  function handleImageLoad() {
    imageLoaded = true;
    imageLoadError = false;
  }

  function handleImageError() {
    imageLoadError = true;
    imageLoaded = false;
  }

  function handleCharacterClick(char: CharacterInScene) {
    if (char.db_id) {
      oncharacterclick?.(char);
    }
  }

  // ── Rewrite editor state ──
  let rewriteExpanded = false;
  let editedUserInput = '';

  // ── Illustrate editor state ──
  let illustrateExpanded = false;
  let editedPositive = '';
  let editedNegative = '';
  let promptDirty = false;

  // Sync from parent when prompts arrive (don't overwrite user edits)
  $: if (enrichedPrompt !== null && !promptDirty) editedPositive = enrichedPrompt;
  $: if (negativePrompt !== null && !promptDirty) editedNegative = negativePrompt;
  // Also seed with scenePrompt as a placeholder before server data arrives
  $: if (!editedPositive && scenePrompt) editedPositive = scenePrompt;

  $: isEdited = promptDirty && (
    editedPositive !== (enrichedPrompt ?? scenePrompt) ||
    editedNegative !== (negativePrompt ?? '')
  );

  function resetPrompts() {
    editedPositive = enrichedPrompt ?? scenePrompt;
    editedNegative = negativePrompt ?? '';
    promptDirty = false;
  }

  function handleIllustrate() {
    if (isEdited) {
      onillustratescene?.({ storyText, messageId, turnNumber, positivePrompt: editedPositive, negativePrompt: editedNegative });
    } else {
      onillustratescene?.({ storyText, messageId, turnNumber });
    }
  }

  $: paragraphs = formatNarrative(storyText);
</script>

<div class="story-turn" class:latest={isLatestTurn}>
  <!-- Scene Transition Banner -->
  {#if sceneTransition}
    <div class="scene-transition-banner">
      <span class="transition-icon">📍</span>
      <span class="transition-location">{sceneTransition.location}</span>
      {#if sceneTransition.timeOfDay}
        <span class="transition-detail">· {sceneTransition.timeOfDay}</span>
      {/if}
      {#if sceneTransition.mood}
        <span class="transition-detail">· {sceneTransition.mood}</span>
      {/if}
    </div>
  {/if}

  <!-- Turn Number Badge -->
  <div class="turn-gutter">
    <span class="turn-badge">{turnNumber}</span>
    {#if parseStatus !== 'ok'}
      <span class="parse-indicator" class:partial={parseStatus === 'partial'} class:fallback={parseStatus === 'fallback'}
        title={parseWarnings.join('; ') || parseStatus}>
        {parseStatus === 'partial' ? '⚠' : '⚡'}
      </span>
    {/if}
  </div>

  <div class="turn-content">
    <!-- User Action -->
    {#if userAction}
      <div class="user-action">
        <span class="action-prefix">▸</span>
        <span class="action-text">{userAction}</span>
      </div>
    {/if}

    <!-- Scene Image -->
    {#if imagePath}
      <div class="scene-image-container" class:loaded={imageLoaded}>
        {#if !imageLoaded && !imageLoadError}
          <div class="image-placeholder">
            <div class="image-loading-shimmer"></div>
          </div>
        {/if}
        {#if imageLoadError}
          <div class="image-error">
            <span class="error-icon">🖼</span>
            <span class="error-text">Image could not be loaded</span>
          </div>
        {:else}
          <img
            src={resolvedImageUrl ?? ''}
            alt="Scene: {scene?.location || 'Story scene'}"
            class="scene-image"
            class:visible={imageLoaded}
            on:load={handleImageLoad}
            on:error={handleImageError}
            on:click={() => { if (imageLoaded) showLightbox = true; }}
            style="cursor: {imageLoaded ? 'pointer' : 'default'};"
          />
        {/if}

        <!-- Scene Overlay Badges -->
        {#if scene && imageLoaded}
          <div class="scene-overlay">
            {#if scene.location}
              <span class="scene-badge location-badge">📍 {scene.location}</span>
            {/if}
            {#if scene.time_of_day}
              <span class="scene-badge time-badge">
                {scene.time_of_day === 'night' ? '🌙' : scene.time_of_day === 'dawn' || scene.time_of_day === 'dusk' ? '🌅' : '☀️'}
                {scene.time_of_day}
              </span>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Character Indicators (inline below image or action) -->
    {#if characters.length > 0}
      <div class="character-strip">
        {#each characters as char}
          <button
            class="character-chip"
            class:has-reference={char.has_reference_image}
            class:clickable={char.db_id !== null}
            on:click={() => handleCharacterClick(char)}
            title="{char.name} — {char.expression}{char.action ? ', ' + char.action : ''}"
          >
            <span class="chip-avatar" class:no-ref={!char.has_reference_image}>
              {char.name.charAt(0).toUpperCase()}
            </span>
            <span class="chip-name">{char.name}</span>
            {#if char.expression}
              <span class="chip-expression">{char.expression}</span>
            {/if}
          </button>
        {/each}
      </div>
    {/if}

    <!-- Narrative Text -->
    <div class="narrative">
      {#each paragraphs as paragraph, i}
        <p class="narrative-paragraph"
           class:dialogue={isDialogue(paragraph)}
           class:fade-in={isLatestTurn}
           style={isLatestTurn ? `animation-delay: ${i * 60}ms` : ''}>
          {paragraph}
        </p>
      {/each}
    </div>

    <!-- Emotional State Indicators -->
    {#if emotionalStates && emotionalStates.length > 0}
      <div class="emotional-states">
        {#each emotionalStates as es}
          <span class="emotional-state-entry">
            {es.name}: <em>{es.current_emotion}</em>{es.emotion_intensity === 'high' || es.emotion_intensity === 'overwhelming' ? ' ⚡' : ''}
          </span>
        {/each}
      </div>
    {/if}

    <!-- Illustrate / Inline Prompt Editor -->
    {#if illustrateExpanded}
      <div class="illustrate-editor">
        <label class="illustrate-label">Edit visual prompt before generating:</label>
        {#if isLoadingPrompt}
          <div class="prompt-loading">
            <span class="img-gen-pulse"></span>
            <span>Building prompt...</span>
          </div>
        {:else}
          <div class="prompt-field">
            <div class="field-header">
              <span class="field-label">Positive</span>
              {#if isEdited}
                <button class="reset-btn" on:click={resetPrompts} title="Reset to auto-generated">↺ Reset</button>
              {/if}
            </div>
            <textarea
              class="prompt-textarea"
              bind:value={editedPositive}
              on:input={() => { promptDirty = true; }}
              rows="4"
              placeholder="Quality tags, scene, character descriptions..."
            ></textarea>
          </div>
          <div class="prompt-field">
            <div class="field-header">
              <span class="field-label">Negative</span>
            </div>
            <textarea
              class="prompt-textarea negative"
              bind:value={editedNegative}
              on:input={() => { promptDirty = true; }}
              rows="2"
              placeholder="Things to avoid..."
            ></textarea>
          </div>
          <div class="illustrate-actions">
            <button
              class="action-btn illustrate-confirm"
              on:click={() => { handleIllustrate(); illustrateExpanded = false; }}
              disabled={isGeneratingImage}
            >
              {#if imagePath}↺ Redraw Image{:else}🎨 Generate Image{/if}{#if isEdited}<span class="edited-badge">edited</span>{/if}
            </button>
            <button
              class="action-btn illustrate-cancel"
              on:click={() => { illustrateExpanded = false; }}
            >
              Cancel
            </button>
          </div>
        {/if}
      </div>
    {:else}
      <button
        class="action-btn img-btn"
        on:click={() => {
          illustrateExpanded = true;
          if (enrichedPrompt === null && !isLoadingPrompt) {
            onexpandprompt?.(turnNumber);
          }
        }}
        disabled={isGeneratingImage}
        title={imagePath ? 'Edit prompt and regenerate image' : 'Edit prompt and generate an image'}
      >
        {#if isGeneratingImage}
          <span class="img-gen-pulse"></span> Generating...
        {:else}
          {#if imagePath}↺ Redraw Image{:else}🎨 Illustrate Scene{/if}
        {/if}
      </button>
    {/if}

    <!-- Parse Warnings -->
    {#if parseWarnings.length > 0 && isLatestTurn}
      <div class="parse-warnings">
        {#each parseWarnings as warning}
          <span class="warning-chip">⚠ {warning}</span>
        {/each}
      </div>
    {/if}

    <!-- Image Generation Error -->
    {#if imageError}
      <div class="image-gen-error">
        <span class="error-icon-small">🖼</span>
        <span>Image generation failed: {imageError}</span>
      </div>
    {/if}

    <!-- Rewrite Section (latest turn only) -->
    {#if isLatestTurn}
      <div class="rewrite-section">
        {#if rewriteExpanded}
          <div class="rewrite-editor">
            <label class="rewrite-label">Edit your action before rewriting:</label>
            <textarea
              class="rewrite-textarea"
              bind:value={editedUserInput}
              on:input={(e) => { editedUserInput = (e.target as HTMLTextAreaElement).value; }}
              rows="3"
              placeholder="Edit your action..."
            ></textarea>
            <div class="rewrite-actions">
              <button
                class="action-btn rewrite-submit"
                on:click={() => {
                  if (editedUserInput.trim()) {
                    onrewrite?.({ turnNumber, editedInput: editedUserInput });
                    rewriteExpanded = false;
                  }
                }}
                disabled={!editedUserInput.trim()}
              >
                ↻ Rewrite with changes
              </button>
              <button
                class="action-btn rewrite-same"
                on:click={() => {
                  onregenerate?.(turnNumber);
                  rewriteExpanded = false;
                }}
              >
                ↻ Rewrite (same input)
              </button>
              <button
                class="action-btn rewrite-cancel"
                on:click={() => { rewriteExpanded = false; }}
              >
                Cancel
              </button>
            </div>
          </div>
        {:else}
          <button
            class="action-btn regen-btn"
            on:click={() => {
              editedUserInput = userAction;
              rewriteExpanded = true;
            }}
          >
            ↻ Rewrite
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if showLightbox && resolvedImageUrl}
  <ImageLightbox
    src={resolvedImageUrl}
    alt="Scene: {scene?.location || 'Story scene'}"
    onclose={() => showLightbox = false}
  />
{/if}

<style>
  .story-turn {
    display: flex;
    gap: 16px;
    padding: 20px 0;
    border-bottom: 1px solid var(--story-border, rgba(255, 255, 255, 0.06));
    position: relative;
  }

  .story-turn.latest {
    border-bottom: none;
  }

  /* ── Turn Gutter ── */
  .turn-gutter {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    min-width: 36px;
    padding-top: 2px;
  }

  .turn-badge {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: var(--story-badge-bg, rgba(255, 255, 255, 0.06));
    color: var(--story-badge-text, rgba(255, 255, 255, 0.35));
    font-size: 0.75rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
  }

  .parse-indicator {
    font-size: 0.7rem;
    cursor: help;
  }
  .parse-indicator.partial { color: #e9a84c; }
  .parse-indicator.fallback { color: #e06060; }

  /* ── Turn Content ── */
  .turn-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  /* ── User Action ── */
  .user-action {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 14px;
    background: var(--story-action-bg, rgba(100, 180, 255, 0.08));
    border-left: 3px solid var(--story-action-border, rgba(100, 180, 255, 0.4));
    border-radius: 0 6px 6px 0;
    font-style: italic;
    color: var(--story-action-text, rgba(180, 210, 255, 0.85));
    font-size: 0.92rem;
    line-height: 1.5;
  }

  .action-prefix {
    color: var(--story-action-border, rgba(100, 180, 255, 0.4));
    font-style: normal;
    font-weight: bold;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .action-text {
    flex: 1;
  }

  /* ── Scene Image ── */
  .scene-image-container {
    position: relative;
    border-radius: 10px;
    overflow: hidden;
    background: var(--story-image-bg, rgba(0, 0, 0, 0.3));
    max-width: 100%;
    aspect-ratio: 2 / 3;
    max-height: 480px;
    align-self: center;
  }

  .scene-image-container.loaded {
    aspect-ratio: auto;
    max-height: none;
  }

  .image-placeholder {
    width: 100%;
    height: 100%;
    min-height: 200px;
    position: relative;
    overflow: hidden;
  }

  .image-loading-shimmer {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      110deg,
      transparent 30%,
      rgba(255, 255, 255, 0.04) 50%,
      transparent 70%
    );
    animation: shimmer 1.8s ease-in-out infinite;
  }

  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  .scene-image {
    width: 100%;
    display: block;
    border-radius: 10px;
    opacity: 0;
    transition: opacity 0.4s ease;
  }

  .scene-image.visible {
    opacity: 1;
  }

  .image-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 40px;
    color: var(--text-muted, #888);
    font-size: 0.85rem;
  }

  .error-icon {
    font-size: 2rem;
    opacity: 0.4;
  }

  /* Scene overlay badges on image */
  .scene-overlay {
    position: absolute;
    bottom: 8px;
    left: 8px;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .scene-badge {
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 500;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(6px);
    color: rgba(255, 255, 255, 0.85);
    text-transform: capitalize;
  }

  /* ── Character Strip ── */
  .character-strip {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .character-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px 4px 4px;
    border-radius: 20px;
    background: var(--story-chip-bg, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--story-chip-border, rgba(255, 255, 255, 0.08));
    font-size: 0.78rem;
    color: var(--story-chip-text, rgba(255, 255, 255, 0.7));
    cursor: default;
    transition: background 0.15s, border-color 0.15s;
  }

  .character-chip.clickable {
    cursor: pointer;
  }

  .character-chip.clickable:hover {
    background: var(--story-chip-hover, rgba(255, 255, 255, 0.1));
    border-color: var(--story-chip-border-hover, rgba(255, 255, 255, 0.2));
  }

  .chip-avatar {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    font-size: 0.7rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .chip-avatar.no-ref {
    background: var(--story-chip-noref, rgba(255, 255, 255, 0.15));
    color: var(--story-chip-text, rgba(255, 255, 255, 0.5));
  }

  .chip-name {
    font-weight: 600;
  }

  .chip-expression {
    color: var(--story-chip-expr, rgba(255, 255, 255, 0.4));
    font-style: italic;
    font-size: 0.72rem;
  }

  /* ── Narrative Text ── */
  .narrative {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .narrative-paragraph {
    margin: 0;
    line-height: 1.72;
    font-size: 0.96rem;
    color: var(--story-text, rgba(255, 255, 255, 0.88));
    letter-spacing: 0.01em;
  }

  .narrative-paragraph.dialogue {
    padding-left: 16px;
    border-left: 2px solid var(--story-dialogue-border, rgba(255, 200, 100, 0.25));
    color: var(--story-dialogue-text, rgba(255, 230, 180, 0.92));
  }

  .narrative-paragraph.fade-in {
    animation: fadeSlideIn 0.35s ease both;
  }

  @keyframes fadeSlideIn {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* ── Warnings / Errors ── */
  .parse-warnings {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .warning-chip {
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 0.72rem;
    background: rgba(233, 168, 76, 0.12);
    color: #e9a84c;
    border: 1px solid rgba(233, 168, 76, 0.2);
  }

  .image-gen-error {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(224, 96, 96, 0.1);
    color: #e06060;
    font-size: 0.8rem;
    border: 1px solid rgba(224, 96, 96, 0.15);
  }

  .error-icon-small {
    font-size: 1rem;
    opacity: 0.7;
  }

  /* ── Illustrate Editor ── */
  .img-btn {
    color: var(--text-muted, #6e7681);
  }

  .illustrate-editor {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.5rem;
    padding: 0.6rem;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-secondary, rgba(255, 255, 255, 0.08));
  }

  .illustrate-label {
    font-size: 0.78rem;
    color: var(--text-muted, #6e7681);
    font-weight: 600;
  }

  .illustrate-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-top: 0.25rem;
  }

  .illustrate-confirm {
    background: var(--accent-primary, #58a6ff) !important;
    color: var(--text-inverse, #0d1117) !important;
    font-weight: 600;
  }

  .illustrate-cancel {
    background: transparent !important;
    opacity: 0.7;
  }

  .prompt-loading {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.78rem;
    color: var(--text-muted, #6e7681);
    font-style: italic;
    padding: 4px 0;
  }

  .prompt-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .field-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #6e7681);
  }

  .reset-btn {
    background: none;
    border: none;
    color: var(--accent-primary, #58a6ff);
    font-size: 0.72rem;
    cursor: pointer;
    padding: 1px 6px;
    border-radius: 4px;
  }

  .reset-btn:hover {
    background: color-mix(in srgb, var(--accent-primary, #58a6ff) 10%, transparent);
  }

  .prompt-textarea {
    width: 100%;
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-secondary, #21262d);
    border-radius: 6px;
    color: var(--text-secondary, #8b949e);
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.75rem;
    line-height: 1.5;
    padding: 6px 8px;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }

  .prompt-textarea:focus {
    border-color: var(--accent-primary, #58a6ff);
    color: var(--text-primary, #c9d1d9);
  }

  .prompt-textarea.negative {
    color: color-mix(in srgb, var(--accent-danger, #f85149) 70%, var(--text-muted, #6e7681));
  }

  .illustrate-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 11px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--text-muted, #6e7681);
    cursor: pointer;
    font-size: 0.78rem;
    font-weight: 500;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .illustrate-btn:hover {
    background: rgba(88, 166, 255, 0.12);
    border-color: rgba(88, 166, 255, 0.3);
    color: var(--accent-primary, #58a6ff);
  }

  .illustrate-btn.edited {
    border-color: rgba(88, 166, 255, 0.35);
    color: var(--accent-primary, #58a6ff);
  }

  .edited-badge {
    font-size: 0.68rem;
    font-weight: 400;
    opacity: 0.75;
    padding: 1px 4px;
    background: rgba(88, 166, 255, 0.15);
    border-radius: 3px;
  }

  .img-generating {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.78rem;
    color: var(--text-muted, #6e7681);
    font-style: italic;
    flex-shrink: 0;
  }

  .img-gen-pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    animation: pulse 1.2s ease-in-out infinite;
    flex-shrink: 0;
  }

  /* ── Scene Transition Banner ── */
  .scene-transition-banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px 16px;
    margin: 12px 0 4px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    font-size: 0.8rem;
    color: var(--text-muted, #6e7681);
    grid-column: 1 / -1;
  }

  .transition-icon {
    font-size: 0.9rem;
    flex-shrink: 0;
  }

  .transition-location {
    font-weight: 600;
    color: var(--text-secondary, #8b949e);
    text-transform: capitalize;
  }

  .transition-detail {
    color: var(--text-muted, #6e7681);
    font-style: italic;
  }

  /* ── Emotional State Indicator ── */
  .emotional-states {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.25rem;
    padding: 0.35rem 0.6rem;
    font-size: 0.74rem;
    color: var(--text-muted, #6e7681);
    border-left: 2px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
    opacity: 0.65;
  }

  .emotional-state-entry {
    white-space: nowrap;
  }

  .emotional-state-entry em {
    font-style: italic;
    color: var(--text-secondary, #8b949e);
  }

  /* ── Rewrite Section ── */
  .rewrite-section {
    margin-top: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-subtle, rgba(255,255,255,0.06));
  }

  .rewrite-editor {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .rewrite-label {
    font-size: 0.78rem;
    color: var(--text-muted, #8b949e);
    font-weight: 600;
  }

  .rewrite-textarea {
    width: 100%;
    padding: 0.5rem 0.6rem;
    border-radius: 6px;
    border: 1px solid var(--border-secondary, #333);
    background: var(--bg-primary, #0d1117);
    color: var(--text-primary, #c9d1d9);
    font-family: inherit;
    font-size: 0.88rem;
    resize: vertical;
    min-height: 60px;
    box-sizing: border-box;
  }

  .rewrite-textarea:focus {
    outline: none;
    border-color: var(--accent-primary, #58a6ff);
  }

  .rewrite-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .action-btn {
    padding: 5px 11px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-muted, #6e7681);
    cursor: pointer;
    font-size: 0.78rem;
    font-weight: 500;
    font-family: inherit;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
    white-space: nowrap;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.09);
    color: var(--text-secondary, #8b949e);
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .regen-btn {
    color: var(--text-muted, #6e7681);
  }

  .rewrite-submit {
    background: var(--accent-primary, #58a6ff) !important;
    border-color: var(--accent-primary, #58a6ff) !important;
    color: var(--text-inverse, #0d1117) !important;
    font-weight: 600;
  }

  .rewrite-submit:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent-primary, #58a6ff) 85%, white) !important;
  }

  .rewrite-same {
    background: rgba(255,255,255,0.06) !important;
  }

  .rewrite-cancel {
    background: transparent !important;
    border-color: transparent !important;
    opacity: 0.7;
  }

  .rewrite-cancel:hover {
    opacity: 1;
    background: rgba(255,255,255,0.04) !important;
  }
</style>