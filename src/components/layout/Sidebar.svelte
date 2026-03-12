<!-- src/components/layout/Sidebar.svelte — Story list sidebar -->
<script lang="ts">
  import type { StoryPremise } from '$lib/types';

  let {
    stories = [] as StoryPremise[],
    selectedStoryId = '',
    isLoading,
    onnewstory,
    onselectstory,
    onopensettings,
    ondeletestory,
    oneditstory,
  }: {
    stories?: StoryPremise[];
    selectedStoryId?: string;
    isLoading: boolean;
    onnewstory?: () => void;
    onselectstory?: (id: string) => void;
    onopensettings?: () => void;
    ondeletestory?: (id: string) => void;
    oneditstory?: (story: StoryPremise) => void;
  } = $props();

  let contextMenu: { show: boolean; x: number; y: number; story: StoryPremise | null } = $state({
    show: false, x: 0, y: 0, story: null,
  });

  function handleRightClick(e: MouseEvent, story: StoryPremise) {
    e.preventDefault();
    contextMenu = { show: true, x: e.clientX, y: e.clientY, story };
  }

  function closeContextMenu() {
    contextMenu = { ...contextMenu, show: false };
  }
</script>

<svelte:window onclick={(e) => {
  if (contextMenu.show && !(e.target as HTMLElement).closest('.story-context-menu')) closeContextMenu();
}} />

<div class="sidebar">
  <div class="sidebar-top">
    <h2>Stories</h2>
    <div class="sidebar-controls">
      <button
        onclick={() => onnewstory?.()}
        class="new-story-btn"
        disabled={isLoading}
      >
        + New Story
      </button>
    </div>

    <div class="story-list">
      <!-- Free Write always at the top -->
      <!-- svelte-ignore a11y_interactive_supports_focus -->
      <div
        class="story-item free-write"
        class:active={selectedStoryId === '1'}
        onclick={() => onselectstory?.('1')}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselectstory?.('1'); } }}
        tabindex="0"
        role="button"
      >
        <div class="story-icon">✍</div>
        <div class="story-info">
          <span class="story-title">Free Write</span>
          <span class="story-desc">No story constraints</span>
        </div>
      </div>

      {#if stories.filter(s => s.id !== '1').length > 0}
        <div class="section-label">Your Stories</div>
      {/if}

      {#each stories.filter(s => s.id !== '1') as story (story.id)}
        <!-- svelte-ignore a11y_interactive_supports_focus -->
        <div
          class="story-item"
          class:active={story.id === selectedStoryId}
          onclick={() => onselectstory?.(story.id)}
          oncontextmenu={(e) => handleRightClick(e, story)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselectstory?.(story.id); } }}
          tabindex="0"
          role="button"
        >
          <div class="story-icon">📖</div>
          <div class="story-info">
            <span class="story-title">{story.title}</span>
            {#if story.description}
              <span class="story-desc">{story.description}</span>
            {/if}
          </div>
          <div class="story-item-actions">
            <button
              class="icon-action"
              onclick={(e) => { e.stopPropagation(); oneditstory?.(story); }}
              title="Edit story"
            >✏</button>
          </div>
        </div>
      {/each}

      {#if stories.filter(s => s.id !== '1').length === 0}
        <p class="empty-hint">No stories yet. Create one to get started.</p>
      {/if}
    </div>
  </div>

  <div class="sidebar-bottom">
    <button class="settings-btn" onclick={() => onopensettings?.()}>
      <span class="settings-icon">⚙️</span>
      <span class="settings-text">Settings</span>
    </button>
  </div>
</div>

<!-- Context menu for right-click on story items -->
{#if contextMenu.show && contextMenu.story}
  <div
    class="story-context-menu"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
    role="menu"
  >
    <button onclick={() => { oneditstory?.(contextMenu.story!); closeContextMenu(); }}>
      ✏ Edit Story
    </button>
    <button class="danger" onclick={() => { ondeletestory?.(contextMenu.story!.id); closeContextMenu(); }}>
      🗑 Delete Story
    </button>
  </div>
{/if}

<style>
  .sidebar {
    width: 250px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-primary);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-top {
    flex: 1;
    padding: 20px 10px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar h2 {
    margin-top: 0;
    margin-bottom: 12px;
    font-size: 1.2em;
    border-bottom: 1px solid var(--border-secondary);
    padding-bottom: 10px;
    text-align: center;
    color: var(--text-primary);
  }

  .sidebar-controls {
    margin-bottom: 12px;
  }

  .new-story-btn {
    width: 100%;
    background: var(--accent-primary);
    color: var(--text-inverse);
    padding: 8px 10px;
    font-weight: bold;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.88em;
    transition: opacity 0.2s;
  }
  .new-story-btn:hover:not(:disabled) { opacity: 0.9; }
  .new-story-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .section-label {
    font-size: 0.72em;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 8px 6px 4px;
    margin-top: 4px;
  }

  .story-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .story-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 8px;
    cursor: pointer;
    border-radius: 6px;
    background: transparent;
    border: 1px solid transparent;
    transition: all 0.15s;
    outline: none;
    color: var(--text-primary);
    position: relative;
  }

  .story-item:hover { background: var(--bg-hover); border-color: var(--border-secondary); }
  .story-item:focus { outline: 2px solid var(--accent-primary); outline-offset: 1px; }

  .story-item.active {
    background: var(--bg-active);
    border-color: var(--border-active);
    color: var(--text-inverse);
  }

  .story-item.free-write .story-icon { color: var(--accent-warning, #f0a500); }

  .story-icon {
    font-size: 1.1em;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .story-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .story-title {
    font-size: 0.88em;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .story-desc {
    font-size: 0.72em;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0.8;
  }

  .story-item.active .story-desc { color: inherit; opacity: 0.7; }

  .story-item-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s;
    flex-shrink: 0;
  }

  .story-item:hover .story-item-actions { opacity: 1; }
  .story-item.active .story-item-actions { opacity: 1; }

  .icon-action {
    background: none;
    border: none;
    color: inherit;
    padding: 2px 4px;
    font-size: 0.8em;
    cursor: pointer;
    border-radius: 3px;
    opacity: 0.7;
  }
  .icon-action:hover { opacity: 1; background: rgba(255,255,255,0.1); }

  .empty-hint {
    font-size: 0.8em;
    color: var(--text-secondary);
    text-align: center;
    padding: 12px;
    opacity: 0.7;
    font-style: italic;
  }

  .sidebar-bottom {
    padding: 10px;
    border-top: 1px solid var(--border-primary);
  }

  .settings-btn {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 15px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 6px;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 0.95em;
    transition: all 0.2s;
  }

  .settings-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .settings-icon { font-size: 1.2em; }
  .settings-text { flex: 1; text-align: left; }

  /* Context menu */
  .story-context-menu {
    position: fixed;
    z-index: 9999;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 4px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.4);
    min-width: 150px;
  }

  .story-context-menu button {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    color: var(--text-primary);
    padding: 8px 12px;
    font-size: 0.88em;
    cursor: pointer;
    border-radius: 4px;
    transition: background 0.1s;
  }

  .story-context-menu button:hover { background: var(--bg-hover); }
  .story-context-menu button.danger:hover { color: var(--accent-danger); }
</style>
