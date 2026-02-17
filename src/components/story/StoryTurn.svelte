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
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { createEventDispatcher } from 'svelte';
  import type { CharacterInScene } from '$lib/orchestrator-types';
  import type { SceneJson } from '$lib/llm-parser-types';

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

  const dispatch = createEventDispatcher();

  /** Convert an absolute file path to a Tauri asset URL */
  function assetSrc(path: string): string {
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }

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

  let imageLoaded = false;
  let imageLoadError = false;

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
      dispatch('characterClick', char);
    }
  }

  $: paragraphs = formatNarrative(storyText);
</script>

<div class="story-turn" class:latest={isLatestTurn}>
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
            src={assetSrc(imagePath)}
            alt="Scene: {scene?.location || 'Story scene'}"
            class="scene-image"
            class:visible={imageLoaded}
            on:load={handleImageLoad}
            on:error={handleImageError}
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

    <!-- Parse Warnings -->
    {#if parseWarnings.length > 0 && isLatestTurn}
      <div class="parse-warnings">
        {#each parseWarnings as warning}
          <span class="warning-chip">⚠ {warning}</span>
        {/each}
      </div>
    {/if}

    <!-- Image Generation Error -->
    {#if imageError && isLatestTurn}
      <div class="image-gen-error">
        <span class="error-icon-small">🖼</span>
        <span>Image generation failed: {imageError}</span>
      </div>
    {/if}
  </div>
</div>

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
</style>