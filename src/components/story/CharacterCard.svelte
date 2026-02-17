<!-- src/components/story/CharacterCard.svelte -->
<!--
  Character card for the CharacterPanel grid.
  Shows portrait, name, age, personality, master-image warning.
  Click to edit, dedicated delete button, optional quick-view on hover.

  Dispatches: 'edit', 'delete', 'generatePortrait'
  Uses types.ts CharacterProfile (id: string) for compatibility with CharacterModal.
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { CharacterProfile } from '../../lib/types';

  export let character: CharacterProfile;
  /** Whether this card is in "selected" state (for batch operations) */
  export let selected: boolean = false;
  /** Whether to show the selection checkbox */
  export let selectable: boolean = false;

  const dispatch = createEventDispatcher();

  // ── Image source resolution ──
  // Characters can have: base64 `image`, file-path `master_image_path`, or neither
  let imgLoaded = false;
  let imgError = false;

  $: portraitSrc = resolvePortrait(character);
  $: hasMasterImage = !!(character as any).master_image_path;
  $: hasAnyImage = !!character.image || hasMasterImage;

  function resolvePortrait(c: CharacterProfile): string | null {
    // Prefer master_image_path (file on disk) if available
    const masterPath = (c as any).master_image_path;
    if (masterPath) {
      try { return convertFileSrc(masterPath); }
      catch { /* fall through */ }
    }
    // Fall back to base64 inline image
    if (c.image) {
      return `data:image/png;base64,${c.image}`;
    }
    return null;
  }

  // ── Placeholder color from name ──
  function avatarGradient(name: string): string {
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    const h = Math.abs(hash % 360);
    return `linear-gradient(135deg, hsl(${h}, 50%, 32%) 0%, hsl(${(h + 35) % 360}, 60%, 22%) 100%)`;
  }

  // ── Quick View state ──
  let showQuickView = false;
  let quickViewTimer: ReturnType<typeof setTimeout> | null = null;

  function startQuickView() {
    quickViewTimer = setTimeout(() => { showQuickView = true; }, 500);
  }

  function cancelQuickView() {
    if (quickViewTimer) clearTimeout(quickViewTimer);
    showQuickView = false;
  }

  function truncate(text: string | undefined, max: number): string {
    if (!text) return '';
    if (text.length <= max) return text;
    return text.slice(0, max).trimEnd() + '…';
  }
</script>

<div
  class="char-card"
  class:selected
  on:click={() => dispatch('edit', character)}
  on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && dispatch('edit', character)}
  on:mouseenter={startQuickView}
  on:mouseleave={cancelQuickView}
  role="button"
  tabindex="0"
  title="Edit {character.name}"
>
  <!-- Selection checkbox -->
  {#if selectable}
    <div
      class="select-checkbox"
      on:click|stopPropagation={() => dispatch('toggleSelect', character.id)}
      on:keydown|stopPropagation={(e) => (e.key === 'Enter' || e.key === ' ') && dispatch('toggleSelect', character.id)}
      role="checkbox"
      aria-checked={selected}
      tabindex="0"
    >
      {#if selected}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="var(--accent-primary, #58a6ff)" stroke="none">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
        </svg>
      {:else}
        <div class="unchecked-box"></div>
      {/if}
    </div>
  {/if}

  <!-- Portrait -->
  <div class="card-portrait" style={!portraitSrc || imgError ? `background: ${avatarGradient(character.name)}` : ''}>
    {#if portraitSrc && !imgError}
      <img
        src={portraitSrc}
        alt={character.name}
        class="portrait-img"
        class:loaded={imgLoaded}
        on:load={() => imgLoaded = true}
        on:error={() => imgError = true}
      />
    {:else}
      <span class="portrait-initial">{character.name.charAt(0).toUpperCase()}</span>
    {/if}

    <!-- Missing master image warning -->
    {#if !hasMasterImage}
      <div
        class="missing-master"
        title="No master reference image — portraits won't use IP-Adapter"
        on:click|stopPropagation={() => dispatch('generatePortrait', character)}
        on:keydown|stopPropagation={(e) => (e.key === 'Enter') && dispatch('generatePortrait', character)}
        role="button"
        tabindex="0"
      >
        ⚠️
      </div>
    {/if}
  </div>

  <!-- Info -->
  <div class="card-info">
    <div class="card-name-row">
      <span class="card-name">{character.name}</span>
      {#if character.age}
        <span class="card-age">{character.age}</span>
      {/if}
    </div>

    {#if character.personality}
      <span class="card-personality">{truncate(character.personality, 40)}</span>
    {/if}

    {#if character.gender}
      <span class="card-gender">{character.gender}</span>
    {/if}
  </div>

  <!-- Action buttons (visible on hover) -->
  <div class="card-actions">
    <div
      class="action-btn edit-btn"
      on:click|stopPropagation={() => dispatch('edit', character)}
      on:keydown|stopPropagation={(e) => (e.key === 'Enter') && dispatch('edit', character)}
      role="button"
      tabindex="0"
      title="Edit character"
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"></path>
        <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"></path>
      </svg>
    </div>
    <div
      class="action-btn delete-btn"
      on:click|stopPropagation={() => dispatch('delete', character.id)}
      on:keydown|stopPropagation={(e) => (e.key === 'Enter') && dispatch('delete', character.id)}
      role="button"
      tabindex="0"
      title="Delete character"
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <polyline points="3 6 5 6 21 6"></polyline>
        <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"></path>
      </svg>
    </div>
  </div>

  <!-- Quick View Popover -->
  {#if showQuickView}
    <div class="quick-view" on:mouseenter={() => showQuickView = true} on:mouseleave={cancelQuickView}>
      <div class="qv-header">
        {#if portraitSrc && !imgError}
          <img src={portraitSrc} alt={character.name} class="qv-portrait" />
        {:else}
          <div class="qv-portrait-placeholder" style="background: {avatarGradient(character.name)}">
            {character.name.charAt(0).toUpperCase()}
          </div>
        {/if}
        <div class="qv-title">
          <strong>{character.name}</strong>
          {#if character.age}<span class="qv-age">{character.age} yrs</span>{/if}
          {#if character.gender}<span class="qv-meta">{character.gender}</span>{/if}
        </div>
      </div>

      <div class="qv-attrs">
        {#if character.personality}
          <div class="qv-row"><span class="qv-label">Vibe</span><span>{character.personality}</span></div>
        {/if}
        {#if character.body_type}
          <div class="qv-row"><span class="qv-label">Build</span><span>{character.body_type}</span></div>
        {/if}
        {#if character.hair_color || character.hair_style}
          <div class="qv-row"><span class="qv-label">Hair</span><span>{[character.hair_color, character.hair_style].filter(Boolean).join(', ')}</span></div>
        {/if}
        {#if character.skin_tone}
          <div class="qv-row"><span class="qv-label">Skin</span><span>{character.skin_tone}</span></div>
        {/if}
        {#if character.additional_notes}
          <div class="qv-row"><span class="qv-label">Features</span><span>{truncate(character.additional_notes, 60)}</span></div>
        {/if}
      </div>

      {#if character.sd_prompt}
        <div class="qv-prompt">
          <span class="qv-label">SD Prompt</span>
          <code>{truncate(character.sd_prompt, 120)}</code>
        </div>
      {/if}

      {#if !hasMasterImage}
        <div class="qv-warning">⚠️ No master reference image</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .char-card {
    position: relative;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary, #161b22);
    border: 1px solid var(--border-secondary, #21262d);
    border-radius: 10px;
    overflow: visible;
    cursor: pointer;
    transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s;
  }

  .char-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.2);
    border-color: var(--accent-primary, #58a6ff);
  }

  .char-card:focus {
    outline: 2px solid var(--accent-primary, #58a6ff);
    outline-offset: 2px;
  }

  .char-card.selected {
    border-color: var(--accent-primary, #58a6ff);
    box-shadow: 0 0 0 2px rgba(88, 166, 255, 0.15);
  }

  /* ── Selection Checkbox ── */
  .select-checkbox {
    position: absolute;
    top: 6px;
    left: 6px;
    z-index: 3;
    cursor: pointer;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .unchecked-box {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.35);
    background: rgba(0, 0, 0, 0.3);
  }

  /* ── Portrait ── */
  .card-portrait {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: 10px 10px 0 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-tertiary, #21262d);
  }

  .portrait-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.3s;
  }

  .portrait-img.loaded {
    opacity: 1;
  }

  .portrait-initial {
    font-size: 2rem;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.3);
    user-select: none;
  }

  .missing-master {
    position: absolute;
    bottom: 6px;
    right: 6px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    cursor: pointer;
    transition: transform 0.15s;
  }

  .missing-master:hover {
    transform: scale(1.2);
  }

  /* ── Info ── */
  .card-info {
    padding: 10px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .card-name-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .card-name {
    font-size: 0.88rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-age {
    font-size: 0.7rem;
    color: var(--text-muted, #6e7681);
    flex-shrink: 0;
  }

  .card-personality {
    font-size: 0.72rem;
    color: var(--text-secondary, #8b949e);
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-gender {
    font-size: 0.65rem;
    color: var(--text-muted, #6e7681);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  /* ── Action Buttons ── */
  .card-actions {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.15s;
    z-index: 2;
  }

  .char-card:hover .card-actions {
    opacity: 1;
  }

  .action-btn {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(4px);
    color: rgba(255, 255, 255, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .edit-btn:hover {
    background: var(--accent-primary, #58a6ff);
    color: #fff;
  }

  .delete-btn:hover {
    background: var(--accent-danger, #f85149);
    color: #fff;
  }

  /* ── Quick View Popover ── */
  .quick-view {
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    margin-top: 8px;
    width: 280px;
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-primary, #30363d);
    border-radius: 10px;
    padding: 14px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.4);
    z-index: 100;
    cursor: default;
    animation: qvFadeIn 0.15s ease;
  }

  @keyframes qvFadeIn {
    from { opacity: 0; transform: translateX(-50%) translateY(4px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
  }

  .qv-header {
    display: flex;
    gap: 10px;
    margin-bottom: 10px;
  }

  .qv-portrait {
    width: 48px;
    height: 48px;
    border-radius: 8px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .qv-portrait-placeholder {
    width: 48px;
    height: 48px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.35);
    flex-shrink: 0;
  }

  .qv-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .qv-title strong {
    font-size: 0.92rem;
    color: var(--text-primary, #c9d1d9);
  }

  .qv-age, .qv-meta {
    font-size: 0.72rem;
    color: var(--text-muted, #6e7681);
  }

  .qv-attrs {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .qv-row {
    display: flex;
    gap: 8px;
    font-size: 0.75rem;
    color: var(--text-secondary, #8b949e);
    line-height: 1.4;
  }

  .qv-label {
    font-weight: 600;
    color: var(--text-muted, #6e7681);
    min-width: 50px;
    text-transform: uppercase;
    font-size: 0.65rem;
    letter-spacing: 0.03em;
    padding-top: 1px;
    flex-shrink: 0;
  }

  .qv-prompt {
    padding-top: 8px;
    border-top: 1px solid var(--border-secondary, #21262d);
  }

  .qv-prompt .qv-label {
    display: block;
    margin-bottom: 4px;
  }

  .qv-prompt code {
    font-size: 0.68rem;
    color: var(--text-muted, #6e7681);
    line-height: 1.4;
    word-break: break-all;
    display: block;
  }

  .qv-warning {
    margin-top: 8px;
    font-size: 0.72rem;
    color: var(--accent-warning, #e3b341);
    padding: 4px 8px;
    background: rgba(227, 179, 65, 0.08);
    border-radius: 4px;
    text-align: center;
  }
</style>