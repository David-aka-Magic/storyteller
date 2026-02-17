<!-- src/components/story/StoryHome.svelte -->
<!--
  Story Home — the landing page / story selection screen.
  Shown in the center panel when no story is currently loaded.

  Features:
    - Grid of saved stories (StoryCard)
    - "New Story" button
    - Empty state with "Create Your First Story" CTA
    - Delete confirmation dialog
    - Loading state while fetching

  Dispatches:
    'loadStory'   → story_id (number)
    'newStory'    → { title, description, characterIds }
    'openSettings'
-->
<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import StoryCard from './StoryCard.svelte';
  import NewStoryModal from './NewStoryModal.svelte';
  import type { StorySummary } from '$lib/story-manager-types';
  import type { CharacterProfile } from '../../lib/types';

  import {
    stories,
    isLoading,
    lastError,
    refreshStoryList,
    deleteStory as deleteStoryFromStore,
  } from '$lib/stores/story-store';

  export let availableCharacters: CharacterProfile[] = [];

  const dispatch = createEventDispatcher();

  // ── Local State ──
  let showNewStoryModal = false;
  let deleteConfirmId: number | null = null;
  let deleteConfirmTitle = '';
  let isDeleting = false;

  // ── Lifecycle ──
  onMount(() => {
    refreshStoryList();
  });

  // ── Handlers ──
  function handleSelectStory(e: CustomEvent<number>) {
    dispatch('loadStory', e.detail);
  }

  function handleRequestDelete(e: CustomEvent<number>) {
    const storyId = e.detail;
    const story = $stories.find(s => s.story_id === storyId);
    deleteConfirmTitle = story?.title ?? 'this story';
    deleteConfirmId = storyId;
  }

  async function confirmDelete() {
    if (deleteConfirmId === null) return;
    isDeleting = true;
    await deleteStoryFromStore(deleteConfirmId);
    isDeleting = false;
    deleteConfirmId = null;
  }

  function cancelDelete() {
    deleteConfirmId = null;
  }

  function handleCreate(e: CustomEvent<{ title: string; description: string; characterIds: number[] }>) {
    showNewStoryModal = false;
    dispatch('newStory', e.detail);
  }
</script>

<div class="story-home">
  <!-- Header -->
  <header class="home-header">
    <div class="header-title-area">
      <h1 class="home-title">Your Stories</h1>
      <span class="story-count">
        {#if $stories.length > 0}
          {$stories.length} {$stories.length === 1 ? 'story' : 'stories'}
        {/if}
      </span>
    </div>

    <button class="new-story-btn" on:click={() => showNewStoryModal = true}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
        <line x1="12" y1="5" x2="12" y2="19"></line>
        <line x1="5" y1="12" x2="19" y2="12"></line>
      </svg>
      New Story
    </button>
  </header>

  <!-- Content Area -->
  <div class="home-content">
    <!-- Loading -->
    {#if $isLoading && $stories.length === 0}
      <div class="loading-state">
        <div class="loading-spinner"></div>
        <span>Loading stories…</span>
      </div>
    {/if}

    <!-- Error -->
    {#if $lastError}
      <div class="error-banner">
        <span>⚠ {$lastError}</span>
        <button class="retry-btn" on:click={() => refreshStoryList()}>Retry</button>
      </div>
    {/if}

    <!-- Empty State -->
    {#if !$isLoading && $stories.length === 0}
      <div class="empty-state">
        <div class="empty-illustration">
          <svg width="96" height="96" viewBox="0 0 96 96" fill="none">
            <!-- Book -->
            <rect x="16" y="10" width="64" height="76" rx="6" stroke="currentColor" stroke-width="2" opacity="0.18"/>
            <rect x="22" y="16" width="52" height="64" rx="4" stroke="currentColor" stroke-width="1.5" opacity="0.12"/>
            <!-- Page lines -->
            <line x1="30" y1="30" x2="62" y2="30" stroke="currentColor" stroke-width="1.2" opacity="0.12" stroke-linecap="round"/>
            <line x1="30" y1="38" x2="55" y2="38" stroke="currentColor" stroke-width="1.2" opacity="0.12" stroke-linecap="round"/>
            <line x1="30" y1="46" x2="58" y2="46" stroke="currentColor" stroke-width="1.2" opacity="0.12" stroke-linecap="round"/>
            <line x1="30" y1="54" x2="48" y2="54" stroke="currentColor" stroke-width="1.2" opacity="0.12" stroke-linecap="round"/>
            <!-- Sparkle -->
            <circle cx="72" cy="20" r="12" fill="var(--accent-primary, #58a6ff)" opacity="0.12"/>
            <path d="M72 14v12M66 20h12" stroke="var(--accent-primary, #58a6ff)" stroke-width="2" stroke-linecap="round" opacity="0.6"/>
          </svg>
        </div>

        <h2 class="empty-title">No stories yet</h2>
        <p class="empty-desc">
          Create your first interactive story — build a world, add characters,
          and let the AI bring your adventure to life.
        </p>

        <button class="cta-btn" on:click={() => showNewStoryModal = true}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
          Create Your First Story
        </button>
      </div>
    {/if}

    <!-- Story Grid -->
    {#if $stories.length > 0}
      <div class="story-grid">
        {#each $stories as story (story.story_id)}
          <StoryCard
            {story}
            on:select={handleSelectStory}
            on:delete={handleRequestDelete}
          />
        {/each}
      </div>
    {/if}
  </div>

  <!-- Delete Confirmation Dialog -->
  {#if deleteConfirmId !== null}
    <div
      class="confirm-backdrop"
      on:click={cancelDelete}
      on:keydown={(e) => e.key === 'Escape' && cancelDelete()}
      role="button"
      tabindex="0"
    >
      <div class="confirm-dialog" on:click|stopPropagation role="alertdialog" tabindex="-1">
        <div class="confirm-icon">🗑️</div>
        <h3 class="confirm-title">Delete Story?</h3>
        <p class="confirm-desc">
          "<strong>{deleteConfirmTitle}</strong>" and all its characters, turns, and images will be permanently deleted.
        </p>
        <div class="confirm-actions">
          <button class="btn btn-cancel" on:click={cancelDelete} disabled={isDeleting}>Cancel</button>
          <button class="btn btn-danger" on:click={confirmDelete} disabled={isDeleting}>
            {#if isDeleting}
              Deleting…
            {:else}
              Delete
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- New Story Modal -->
  <NewStoryModal
    show={showNewStoryModal}
    {availableCharacters}
    on:create={handleCreate}
    on:close={() => showNewStoryModal = false}
  />
</div>

<style>
  .story-home {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-chat, var(--bg-primary, #0d1117));
    overflow: hidden;
  }

  /* ── Header ── */
  .home-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 18px 28px;
    border-bottom: 1px solid var(--border-primary, #30363d);
    background: var(--bg-secondary, #161b22);
    flex-shrink: 0;
  }

  .header-title-area {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .home-title {
    margin: 0;
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
    letter-spacing: -0.01em;
  }

  .story-count {
    font-size: 0.78rem;
    color: var(--text-muted, #6e7681);
    font-weight: 500;
  }

  .new-story-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 16px;
    border-radius: 8px;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    border: none;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.1s;
    font-family: inherit;
  }

  .new-story-btn:hover {
    opacity: 0.9;
    transform: translateY(-1px);
  }

  .new-story-btn:active {
    transform: translateY(0);
  }

  /* ── Content ── */
  .home-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px 28px 40px;
  }

  .home-content::-webkit-scrollbar {
    width: 6px;
  }
  .home-content::-webkit-scrollbar-track { background: transparent; }
  .home-content::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.08);
    border-radius: 3px;
  }

  /* ── Story Grid ── */
  .story-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 18px;
  }

  /* ── Loading ── */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px 20px;
    color: var(--text-muted, #6e7681);
    font-size: 0.9rem;
  }

  .loading-spinner {
    width: 28px;
    height: 28px;
    border: 3px solid var(--border-secondary, #21262d);
    border-top-color: var(--accent-primary, #58a6ff);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  /* ── Error ── */
  .error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    margin-bottom: 16px;
    background: rgba(248, 81, 73, 0.08);
    border: 1px solid rgba(248, 81, 73, 0.2);
    border-radius: 8px;
    color: var(--accent-danger, #f85149);
    font-size: 0.85rem;
  }

  .retry-btn {
    padding: 4px 12px;
    border-radius: 5px;
    background: rgba(248, 81, 73, 0.12);
    border: 1px solid rgba(248, 81, 73, 0.3);
    color: var(--accent-danger, #f85149);
    cursor: pointer;
    font-size: 0.78rem;
    font-weight: 600;
    transition: background 0.15s;
  }

  .retry-btn:hover { background: rgba(248, 81, 73, 0.2); }

  /* ── Empty State ── */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 60px 32px;
    min-height: 50%;
  }

  .empty-illustration {
    color: var(--text-muted, #6e7681);
    margin-bottom: 20px;
  }

  .empty-title {
    margin: 0 0 10px;
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
    letter-spacing: -0.02em;
  }

  .empty-desc {
    margin: 0 0 28px;
    font-size: 0.92rem;
    line-height: 1.6;
    color: var(--text-secondary, #8b949e);
    max-width: 400px;
  }

  .cta-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 24px;
    border-radius: 10px;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    border: none;
    font-size: 0.95rem;
    font-weight: 700;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.12s;
    font-family: inherit;
  }

  .cta-btn:hover {
    opacity: 0.9;
    transform: translateY(-2px);
  }

  .cta-btn:active { transform: translateY(0); }

  /* ── Delete Confirmation ── */
  .confirm-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
    backdrop-filter: blur(3px);
  }

  .confirm-dialog {
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-primary, #30363d);
    border-radius: 14px;
    padding: 28px 24px 22px;
    width: 380px;
    text-align: center;
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.4);
  }

  .confirm-icon {
    font-size: 2.2rem;
    margin-bottom: 10px;
  }

  .confirm-title {
    margin: 0 0 8px;
    font-size: 1.1rem;
    color: var(--text-primary, #c9d1d9);
  }

  .confirm-desc {
    margin: 0 0 20px;
    font-size: 0.85rem;
    color: var(--text-secondary, #8b949e);
    line-height: 1.5;
  }

  .confirm-actions {
    display: flex;
    gap: 10px;
    justify-content: center;
  }

  .btn {
    padding: 8px 18px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: background 0.15s, opacity 0.15s;
    font-family: inherit;
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-cancel {
    background: var(--bg-tertiary, #21262d);
    color: var(--text-secondary, #8b949e);
    border: 1px solid var(--border-secondary, #30363d);
  }

  .btn-cancel:hover:not(:disabled) {
    background: var(--bg-hover, #30363d);
  }

  .btn-danger {
    background: var(--accent-danger, #f85149);
    color: #fff;
  }

  .btn-danger:hover:not(:disabled) {
    opacity: 0.9;
  }
</style>