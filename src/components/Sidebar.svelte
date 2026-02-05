<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { ChatSummary, SelectionState } from '../lib/types';

  export let chatList: ChatSummary[] = [];
  export let currentChatId: number;
  export let isLoading: boolean;
  export let selectionState: SelectionState;

  const dispatch = createEventDispatcher();

  function handleRightClick(e: MouseEvent, chatId: number) {
    e.preventDefault();
    dispatch('contextMenu', {
        event: e,
        chatId: chatId
    });
  }
</script>

<div class="sidebar">
  <div class="sidebar-top">
    <h2>History</h2>
    <div class="sidebar-controls">
      <button 
          on:click={() => dispatch('newChat')} 
          class="new-chat-btn" 
          disabled={isLoading}
      >
        + New Chat
      </button>

      {#if selectionState.isSelecting && selectionState.selectedIds.size > 0}
        <button 
          on:click={() => dispatch('deleteSelected')} 
          class="delete-selected-btn"
          disabled={isLoading}
        >
          Delete ({selectionState.selectedIds.size})
        </button>
      {/if}
    </div>
    
    {#if selectionState.isSelecting}
      <div class="selection-info">
        <span>Selection Mode: {selectionState.selectedIds.size} selected</span>
        <div class="selection-action-buttons">
          <button on:click={() => dispatch('selectAll')} class="select-all-btn">Select All</button>
          <button on:click={() => dispatch('clearSelection')} class="clear-select-btn">Cancel</button>
        </div>
      </div>
    {/if}
    
    <div class="chat-list">
      {#each chatList as chat (chat.id)}
        <div 
          class="chat-item"
          class:active={chat.id === currentChatId}
          class:selected={selectionState.selectedIds.has(chat.id)}
          on:click={() => dispatch('selectChat', chat.id)}
          on:contextmenu={(e) => handleRightClick(e, chat.id)}
          on:keydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              dispatch('selectChat', chat.id);
            }
          }}
          tabindex="0"
          role="button"
        >
          <div class="chat-item-content">
            {#if selectionState.isSelecting}
              <input
                type="checkbox"
                checked={selectionState.selectedIds.has(chat.id)}
                class="chat-checkbox"
              />
            {/if}
            <span class="chat-title">{chat.title}</span>
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Settings Button at Bottom -->
  <div class="sidebar-bottom">
    <button class="settings-btn" on:click={() => dispatch('openSettings')}>
      <span class="settings-icon">⚙️</span>
      <span class="settings-text">Settings</span>
    </button>
  </div>
</div>

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
    margin-bottom: 15px;
    font-size: 1.5em;
    border-bottom: 1px solid var(--border-secondary);
    padding-bottom: 10px;
    text-align: center;
    color: var(--text-primary);
  }
  
  .sidebar-controls {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 15px;
  }
  
  .new-chat-btn {
      width: 100%;
      background: var(--accent-primary);
      color: var(--text-inverse);
      padding: 10px;
      font-weight: bold;
      border: none;
      border-radius: 4px;
      cursor: pointer;
      transition: opacity 0.2s;
  }
  .new-chat-btn:hover:not(:disabled) { opacity: 0.9; }
  .new-chat-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  
  .delete-selected-btn {
    background: var(--accent-danger);
    color: white;
    padding: 10px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    font-weight: bold;
  }
  .delete-selected-btn:hover:not(:disabled) { opacity: 0.9; }
  
  .selection-info {
    background: var(--accent-warning);
    border: 1px solid var(--accent-warning);
    border-radius: 4px;
    padding: 10px;
    margin-bottom: 15px;
    font-size: 13px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: #333;
  }
  
  .selection-action-buttons {
    display: flex;
    gap: 8px;
  }
  
  .select-all-btn, .clear-select-btn {
    background: transparent;
    border: 1px solid rgba(0,0,0,0.3);
    color: #333;
    padding: 4px 10px;
    border-radius: 3px;
    font-size: 12px;
    cursor: pointer;
    flex: 1;
  }
  
  .select-all-btn:hover, .clear-select-btn:hover { 
    background: rgba(0,0,0,0.1); 
  }
  
  .chat-list {
      flex: 1;
      overflow-y: auto;
  }
  
  .chat-item {
      padding: 10px;
      margin-bottom: 5px;
      cursor: pointer;
      border-radius: 4px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      background: var(--bg-primary);
      border: 1px solid var(--border-secondary);
      transition: all 0.2s;
      font-size: 0.9em;
      outline: none;
      color: var(--text-primary);
  }
  
  .chat-item:focus {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }
  
  .chat-item:hover { 
    background: var(--bg-hover); 
  }
  
  .chat-item.active {
      background: var(--bg-active);
      color: var(--text-inverse);
      font-weight: bold;
  }
  
  .chat-item.selected {
    background: var(--bg-tertiary) !important;
    border: 2px solid var(--border-active) !important;
  }
  
  .chat-item.active.selected {
    background: var(--bg-active) !important;
    border: 2px solid var(--border-active) !important;
  }
  
  .chat-item-content {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  
  .chat-checkbox {
    margin: 0;
    cursor: pointer;
    width: 16px;
    height: 16px;
  }
  
  .chat-title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Settings Button */
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

  .settings-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .settings-icon {
    font-size: 1.2em;
  }

  .settings-text {
    flex: 1;
    text-align: left;
  }
</style>