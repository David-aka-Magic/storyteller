<!-- src/components/layout/TitleBar.svelte — Custom draggable title bar -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  let {
    title = 'AI Story Writer'
  }: {
    title?: string;
  } = $props();

  let isMaximized = $state(false);
  const appWindow = getCurrentWindow();
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    isMaximized = await appWindow.isMaximized();
    unlisten = await appWindow.onResized(async () => {
      isMaximized = await appWindow.isMaximized();
    });
  });

  onDestroy(() => {
    unlisten?.();
  });

  async function minimize() {
    await appWindow.minimize();
  }

  async function toggleMaximize() {
    await appWindow.toggleMaximize();
  }

  async function close() {
    await appWindow.close();
  }
</script>

<div class="titlebar">
  <div class="titlebar-drag" data-tauri-drag-region>
    <span class="titlebar-title" data-tauri-drag-region>{title}</span>
  </div>
  <div class="titlebar-controls">
    <button class="titlebar-btn" onclick={minimize} title="Minimize">
      <svg width="10" height="1" viewBox="0 0 10 1">
        <rect width="10" height="1" fill="currentColor"/>
      </svg>
    </button>
    <button class="titlebar-btn" onclick={toggleMaximize} title={isMaximized ? 'Restore' : 'Maximize'}>
      {#if isMaximized}
        <svg width="10" height="10" viewBox="0 0 10 10">
          <path d="M2 0v2H0v8h8V8h2V0H2zm6 8H1V3h7v5zM9 7V1H3v1h5v5h1z" fill="currentColor"/>
        </svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10">
          <rect width="10" height="10" rx="0" fill="none" stroke="currentColor" stroke-width="1"/>
        </svg>
      {/if}
    </button>
    <button class="titlebar-btn close-btn" onclick={close} title="Close">
      <svg width="10" height="10" viewBox="0 0 10 10">
        <path d="M1 0L0 1l4 4-4 4 1 1 4-4 4 4 1-1-4-4 4-4-1-1-4 4z" fill="currentColor"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    height: 34px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-secondary);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
    position: relative;
    z-index: 1000;
  }

  .titlebar-drag {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: center;
    padding-left: 14px;
    cursor: default;
  }

  .titlebar-title {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .titlebar-controls {
    display: flex;
    height: 100%;
  }

  .titlebar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 46px;
    height: 100%;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .titlebar-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .close-btn:hover {
    background: var(--accent-danger);
    color: white;
  }
</style>
