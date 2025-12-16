<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { SelectionState } from '../lib/types';
  export let x: number;
  export let y: number;
  export let chatId: number;
  export let selectionState: SelectionState;

  const dispatch = createEventDispatcher();
  function handleKeydown(e: KeyboardEvent, action: () => void) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      action();
    }
  }
  
  function handleContainerKey(e: KeyboardEvent) {
    if (e.key === 'Escape') dispatch('close');
  }
</script>

<div 
  class="context-menu"
  style="left: {x}px; top: {y}px;"
  on:click|stopPropagation
  on:keydown={handleContainerKey}
  role="menu"
  aria-label="Chat options"
  tabindex="-1"
>
  <div 
    class="context-menu-item" 
    on:click={() => dispatch('delete', chatId)}
    on:keydown={(e) => handleKeydown(e, () => dispatch('delete', chatId))}
    tabindex="0" role="menuitem"
  >
    <span class="icon">🗑️</span> Delete Chat
  </div>

  <div 
    class="context-menu-item" 
    on:click={() => dispatch('selectAll')}
    on:keydown={(e) => handleKeydown(e, () => dispatch('selectAll'))}
    tabindex="0" role="menuitem"
  >
    <span class="icon">✓</span> Select All Chats
  </div>

  {#if selectionState.isSelecting}
    <div 
      class="context-menu-item" 
      on:click={() => dispatch('cancelSelection')}
      on:keydown={(e) => handleKeydown(e, () => dispatch('cancelSelection'))}
      tabindex="0" role="menuitem"
    >
      <span class="icon">✕</span> Cancel Selection
    </div>
  {:else}
    <div 
      class="context-menu-item" 
      on:click={() => {
        dispatch('startSelection', chatId);
      }}
      on:keydown={(e) => handleKeydown(e, () => dispatch('startSelection', chatId))}
      tabindex="0" role="menuitem"
    >
      <span class="icon">📁</span> Select Multiple
    </div>
  {/if}

  <div class="context-menu-divider" role="separator"></div>
  
  <div 
    class="context-menu-item" 
    on:click={() => dispatch('close')}
    on:keydown={(e) => handleKeydown(e, () => dispatch('close'))}
    tabindex="0" role="menuitem"
  >
    <span class="icon">✕</span> Close
  </div>
</div>

<style>
  .context-menu {
    position: fixed;
    background: white;
    border: 1px solid #ddd;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 1000;
    min-width: 180px;
    padding: 6px 0;
    outline: none;
  }
  
  .context-menu-item {
    padding: 8px 16px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    outline: none;
  }
  
  .context-menu-item:focus, .context-menu-item:hover {
    background: #f8f9fa;
    outline: none;
  }
  
  .context-menu-item:focus {
    outline: 2px solid #4a9eff;
    outline-offset: -2px;
  }
  
  .context-menu-divider {
    height: 1px;
    background: #dee2e6;
    margin: 6px 0;
  }
  
  .icon {
    font-size: 12px;
    width: 20px;
    text-align: center;
  }
</style>