<!-- src/components/layout/TopBar.svelte — App header bar -->
<script lang="ts">
  let {
    title = 'AI Story Writer',
    isLoading = false,
    hasMessages = false,
    onClearChat,
    onOpenGallery,
    onCreateCharacter,
  }: {
    title?: string;
    isLoading?: boolean;
    hasMessages?: boolean;
    onClearChat?: () => void;
    onOpenGallery?: () => void;
    onCreateCharacter?: () => void;
  } = $props();
</script>

<div class="topbar">
  <div class="topbar-left">
    <h1>{title}</h1>
  </div>
  <div class="topbar-right">
    <button
      class="create-char-btn"
      onclick={onCreateCharacter}
      title="Create a new character (not linked to any story)"
    >
      👤+ New Character
    </button>
    <button
      class="gallery-btn"
      onclick={onOpenGallery}
    >
      🖼 Gallery
    </button>
    <button
      class="clear-btn"
      onclick={onClearChat}
      disabled={isLoading || !hasMessages}
    >
      Clear Chat
    </button>
  </div>
</div>

<style>
  .topbar {
    padding: 12px 20px;
    border-bottom: 1px solid var(--border-primary);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .topbar-left {
    display: flex;
    align-items: center;
    gap: 15px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 1.1em;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 400px;
    flex-shrink: 0;
  }

  .topbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .create-char-btn {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border-secondary);
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85em;
    transition: all 0.2s;
  }
  .create-char-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .gallery-btn {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border-secondary);
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85em;
    transition: all 0.2s;
  }
  .gallery-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .clear-btn {
    background: var(--accent-danger);
    color: white;
    border: none;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85em;
    transition: opacity 0.2s;
  }
  .clear-btn:hover:not(:disabled) { opacity: 0.9; }
  .clear-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
