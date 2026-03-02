<!-- src/components/layout/TopBar.svelte — App header bar -->
<script lang="ts">
  let {
    title = 'AI Story Writer',
    isLoading = false,
    hasMessages = false,
    configPanelCollapsed = false,
    onClearChat,
    onGenerateImage,
    onToggleConfigPanel,
  }: {
    title?: string;
    isLoading?: boolean;
    hasMessages?: boolean;
    configPanelCollapsed?: boolean;
    onClearChat?: () => void;
    onGenerateImage?: () => void;
    onToggleConfigPanel?: () => void;
  } = $props();
</script>

<div class="topbar">
  <div class="topbar-left">
    <h1>{title}</h1>
  </div>
  <div class="topbar-right">
    <button
      class="img-btn"
      onclick={onGenerateImage}
      disabled={isLoading || !hasMessages}
    >
      🎨 Generate Image
    </button>
    <button
      class="panel-toggle-btn"
      onclick={onToggleConfigPanel}
    >
      {configPanelCollapsed ? '◀ Show Panel' : '▶ Hide Panel'}
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
  }

  h1 {
    margin: 0;
    font-size: 1.1em;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 400px;
  }

  .topbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .img-btn {
    background: var(--bg-tertiary);
    color: var(--accent-primary);
    border: 1px solid var(--border-active);
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85em;
    font-weight: bold;
    transition: all 0.2s;
  }
  .img-btn:hover:not(:disabled) { background: var(--bg-hover); }
  .img-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .panel-toggle-btn {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border-secondary);
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85em;
    transition: all 0.2s;
  }
  .panel-toggle-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

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
