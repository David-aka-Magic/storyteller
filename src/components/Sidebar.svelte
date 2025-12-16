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

<style>
  .sidebar {
    width: 250px;
    padding: 20px 10px;
    background: #f4f4f9;
    border-right: 1px solid #ddd;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    flex-shrink: 0;
  }

  .sidebar h2 {
    margin-top: 0;
    margin-bottom: 15px;
    font-size: 1.5em;
    border-bottom: 1px solid #e0e0e0;
    padding-bottom: 10px;
    text-align: center;
  }
  
  .sidebar-controls {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 15px;
  }
  
  .new-chat-btn {
      width: 100%;
      background: #007bff;
      color: white;
      padding: 10px;
      font-weight: bold;
      border: none;
      border-radius: 4px;
      cursor: pointer;
  }
  .new-chat-btn:hover:not(:disabled) { background: #0056b3; }
  
  .delete-selected-btn {
    background: #dc3545;
    color: white;
    padding: 10px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    font-weight: bold;
  }
  .delete-selected-btn:hover:not(:disabled) { background: #c82333; }
  
  .selection-info {
    background: #fff3cd;
    border: 1px solid #ffeaa7;
    border-radius: 4px;
    padding: 10px;
    margin-bottom: 15px;
    font-size: 13px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  
  .selection-action-buttons {
    display: flex;
    gap: 8px;
  }
  
  .select-all-btn, .clear-select-btn {
    background: transparent;
    border: 1px solid #ffc107;
    color: #856404;
    padding: 4px 10px;
    border-radius: 3px;
    font-size: 12px;
    cursor: pointer;
    flex: 1;
  }
  
  .select-all-btn:hover, .clear-select-btn:hover { background: #ffc107; }
  
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
      background: #fff;
      border: 1px solid #eee;
      transition: all 0.2s;
      font-size: 0.9em;
      outline: none;
  }
  
  .chat-item:focus {
    outline: 2px solid #4a9eff;
    outline-offset: 2px;
  }
  
  .chat-item:hover { background: #e0e0e5; }
  
  .chat-item.active {
      background: #4a9eff;
      color: white;
      font-weight: bold;
  }
  
  .chat-item.selected {
    background: #e3f2fd !important;
    border: 2px solid #2196f3 !important;
  }
  
  .chat-item.active.selected {
    background: #2d8aff !important;
    border: 2px solid #1565c0 !important;
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
</style>