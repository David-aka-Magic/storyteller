<!-- src/components/story/CharacterPanel.svelte -->
<!--
  Character Management Panel for a story.
  Can be used as a slide-out side panel during story play,
  or as an inline section on the Story Home.

  Features:
    - Grid of CharacterCards for the current story
    - "Add Character" button → opens CharacterModal in create mode
    - Click card → opens CharacterModal in edit mode
    - Missing-image warning → opens MasterPortraitGenerator
    - Batch select / delete / regenerate portraits
    - Empty state

  Dispatches:
    'openCharacterModal'  → { character: CharacterProfile | null, mode: 'create' | 'edit' }
    'openPortraitGen'     → CharacterProfile (to open MasterPortraitGenerator)
    'deleteCharacter'     → character id (string)
    'deleteCharacters'    → string[] (batch)
    'close'               → dismiss panel (when used as slide-out)

  Props:
    characters: CharacterProfile[]  — all characters for this story
    storyTitle: string              — displayed in header
    isSlideOut: boolean             — if true, shows close button and slide-out styling
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import CharacterCard from './CharacterCard.svelte';
  import type { CharacterProfile } from '../../lib/types';

  export let characters: CharacterProfile[] = [];
  export let storyTitle: string = '';
  export let isSlideOut: boolean = false;

  const dispatch = createEventDispatcher();

  // ── Batch Selection ──
  let isSelecting = false;
  let selectedIds: Set<string> = new Set();

  function toggleSelect(id: string) {
    if (selectedIds.has(id)) {
      selectedIds.delete(id);
    } else {
      selectedIds.add(id);
    }
    selectedIds = selectedIds;
    if (selectedIds.size === 0) isSelecting = false;
  }

  function startBatchMode() {
    isSelecting = true;
    selectedIds = new Set();
  }

  function cancelBatchMode() {
    isSelecting = false;
    selectedIds = new Set();
  }

  function selectAll() {
    selectedIds = new Set(characters.map(c => c.id));
  }

  function batchDelete() {
    if (selectedIds.size === 0) return;
    dispatch('deleteCharacters', Array.from(selectedIds));
    cancelBatchMode();
  }

  function batchRegenerate() {
    // Dispatch each character for portrait regeneration
    const chars = characters.filter(c => selectedIds.has(c.id));
    for (const c of chars) {
      dispatch('openPortraitGen', c);
    }
    cancelBatchMode();
  }

  // ── Handlers ──
  function handleEdit(e: CustomEvent<CharacterProfile>) {
    if (isSelecting) {
      toggleSelect(e.detail.id);
      return;
    }
    dispatch('openCharacterModal', { character: e.detail, mode: 'edit' });
  }

  function handleDelete(e: CustomEvent<string>) {
    dispatch('deleteCharacter', e.detail);
  }

  function handleGeneratePortrait(e: CustomEvent<CharacterProfile>) {
    dispatch('openPortraitGen', e.detail);
  }

  function handleToggleSelect(e: CustomEvent<string>) {
    toggleSelect(e.detail);
  }

  function addCharacter() {
    dispatch('openCharacterModal', { character: null, mode: 'create' });
  }

  // ── Stats ──
  $: missingPortraitCount = characters.filter(c => !(c as any).master_image_path).length;
</script>

<div class="char-panel" class:slide-out={isSlideOut}>
  <!-- Header -->
  <div class="panel-header">
    <div class="header-left">
      <h2 class="panel-title">Characters</h2>
      {#if characters.length > 0}
        <span class="char-count">{characters.length}</span>
      {/if}
      {#if missingPortraitCount > 0}
        <span class="missing-badge" title="{missingPortraitCount} missing master portraits">
          ⚠️ {missingPortraitCount}
        </span>
      {/if}
    </div>

    <div class="header-right">
      {#if characters.length > 1 && !isSelecting}
        <button class="header-btn text-btn" on:click={startBatchMode} title="Select multiple">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <polyline points="9 11 12 14 22 4"></polyline>
            <path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11"></path>
          </svg>
        </button>
      {/if}

      <button class="header-btn add-btn" on:click={addCharacter} title="Add character">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <line x1="12" y1="5" x2="12" y2="19"></line>
          <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
        <span>Add</span>
      </button>

      {#if isSlideOut}
        <button class="header-btn close-btn" on:click={() => dispatch('close')} title="Close">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      {/if}
    </div>
  </div>

  <!-- Batch Selection Bar -->
  {#if isSelecting}
    <div class="batch-bar">
      <div class="batch-info">
        <span>{selectedIds.size} selected</span>
        <button class="batch-link" on:click={selectAll}>Select All</button>
        <button class="batch-link" on:click={cancelBatchMode}>Cancel</button>
      </div>
      <div class="batch-actions">
        {#if selectedIds.size > 0}
          <button class="batch-btn regen-batch" on:click={batchRegenerate} title="Regenerate portraits for selected">
            🎨 Regen
          </button>
          <button class="batch-btn delete-batch" on:click={batchDelete} title="Delete selected characters">
            🗑 Delete
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Content -->
  <div class="panel-content">
    {#if characters.length === 0}
      <!-- Empty State -->
      <div class="empty-state">
        <div class="empty-icon">👤</div>
        <h3 class="empty-title">No characters yet</h3>
        <p class="empty-desc">
          Add characters to your story to give the AI consistent personalities and visual references.
        </p>
        <button class="empty-cta" on:click={addCharacter}>
          + Add First Character
        </button>
      </div>
    {:else}
      <!-- Character Grid -->
      <div class="char-grid">
        {#each characters as char (char.id)}
          <CharacterCard
            character={char}
            selected={selectedIds.has(char.id)}
            selectable={isSelecting}
            on:edit={handleEdit}
            on:delete={handleDelete}
            on:generatePortrait={handleGeneratePortrait}
            on:toggleSelect={handleToggleSelect}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .char-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-chat, var(--bg-primary, #0d1117));
    overflow: hidden;
  }

  .char-panel.slide-out {
    border-left: 1px solid var(--border-primary, #30363d);
    width: 340px;
    min-width: 280px;
    max-width: 400px;
  }

  /* ── Header ── */
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-primary, #30363d);
    background: var(--bg-secondary, #161b22);
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .panel-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
  }

  .char-count {
    font-size: 0.7rem;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: 10px;
    background: var(--bg-tertiary, #21262d);
    color: var(--text-muted, #6e7681);
  }

  .missing-badge {
    font-size: 0.7rem;
    padding: 1px 7px;
    border-radius: 10px;
    background: rgba(227, 179, 65, 0.1);
    color: var(--accent-warning, #e3b341);
    cursor: help;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .header-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: 6px;
    border: none;
    cursor: pointer;
    font-size: 0.78rem;
    font-weight: 600;
    font-family: inherit;
    transition: background 0.15s, color 0.15s;
  }

  .text-btn {
    background: var(--bg-tertiary, #21262d);
    color: var(--text-secondary, #8b949e);
    padding: 5px 8px;
  }

  .text-btn:hover {
    background: var(--bg-hover, #30363d);
    color: var(--text-primary, #c9d1d9);
  }

  .add-btn {
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
  }

  .add-btn:hover {
    opacity: 0.9;
  }

  .close-btn {
    background: none;
    color: var(--text-muted, #6e7681);
    padding: 4px;
  }

  .close-btn:hover {
    color: var(--text-primary, #c9d1d9);
  }

  /* ── Batch Bar ── */
  .batch-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    background: rgba(88, 166, 255, 0.06);
    border-bottom: 1px solid var(--border-secondary, #21262d);
    flex-shrink: 0;
  }

  .batch-info {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.78rem;
    color: var(--text-secondary, #8b949e);
  }

  .batch-link {
    background: none;
    border: none;
    color: var(--accent-primary, #58a6ff);
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
    font-family: inherit;
  }

  .batch-link:hover {
    text-decoration: underline;
  }

  .batch-actions {
    display: flex;
    gap: 6px;
  }

  .batch-btn {
    padding: 4px 10px;
    border-radius: 5px;
    border: none;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
    font-family: inherit;
    transition: background 0.15s;
  }

  .regen-batch {
    background: rgba(88, 166, 255, 0.1);
    color: var(--accent-primary, #58a6ff);
  }

  .regen-batch:hover {
    background: rgba(88, 166, 255, 0.2);
  }

  .delete-batch {
    background: rgba(248, 81, 73, 0.1);
    color: var(--accent-danger, #f85149);
  }

  .delete-batch:hover {
    background: rgba(248, 81, 73, 0.2);
  }

  /* ── Content ── */
  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .panel-content::-webkit-scrollbar { width: 5px; }
  .panel-content::-webkit-scrollbar-track { background: transparent; }
  .panel-content::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.08);
    border-radius: 3px;
  }

  /* ── Character Grid ── */
  .char-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
  }

  /* ── Empty State ── */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 40px 20px;
    min-height: 200px;
  }

  .empty-icon {
    font-size: 2.5rem;
    opacity: 0.2;
    margin-bottom: 12px;
  }

  .empty-title {
    margin: 0 0 8px;
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
  }

  .empty-desc {
    margin: 0 0 20px;
    font-size: 0.82rem;
    color: var(--text-secondary, #8b949e);
    line-height: 1.5;
    max-width: 260px;
  }

  .empty-cta {
    padding: 8px 16px;
    border-radius: 8px;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    border: none;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    font-family: inherit;
    transition: opacity 0.15s;
  }

  .empty-cta:hover {
    opacity: 0.9;
  }
</style>