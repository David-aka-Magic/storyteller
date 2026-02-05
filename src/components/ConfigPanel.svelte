<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { StoryPremise, CharacterProfile } from '../lib/types';

  export let stories: StoryPremise[] = [];
  export let characters: CharacterProfile[] = [];
  export let selectedStoryId: string = '';
  export let selectedCharacterIds: Set<string> = new Set();
  export let collapsed: boolean = false;

  const dispatch = createEventDispatcher();

  function toggleCharacter(id: string) {
    dispatch('toggleCharacter', id);
  }

  function toggleCollapse() {
    collapsed = !collapsed;
    dispatch('toggleCollapse', collapsed);
  }
</script>

<div class="config-panel" class:collapsed>
  <!-- Collapse Toggle Button -->
  <button class="collapse-toggle" on:click={toggleCollapse} title={collapsed ? 'Expand panel' : 'Collapse panel'}>
    <span class="toggle-icon">{collapsed ? '◀' : '▶'}</span>
  </button>

  {#if !collapsed}
    <div class="panel-content">
      <div class="config-section">
          <div class="section-header">
              <h2>Story Setting</h2>
              <button class="create-btn" on:click={() => dispatch('createStory')}>+ Create Story</button>
          </div>
          
          <div class="story-list">
              {#each stories as story (story.id)}
                  <label class="radio-item" class:selected-radio={selectedStoryId === story.id}>
                      <input 
                          type="radio" 
                          name="storyGroup" 
                          value={story.id} 
                          checked={selectedStoryId === story.id}
                          on:change={() => dispatch('selectStory', story.id)}
                      />
                      <div class="radio-content">
                          <span class="radio-title">{story.title}</span>
                          <span class="radio-desc">{story.description}</span>
                      </div>
                      {#if story.id !== '1'}
                          <div class="story-controls">
                              <button class="edit-btn" on:click|preventDefault={() => dispatch('editStory', story)}>✎</button>
                              <button class="delete-btn" on:click|preventDefault={() => dispatch('deleteStory', story.id)}>🗑️</button>
                          </div>
                      {/if}
                  </label>
              {/each}
          </div>
      </div>

      <div class="config-section">
          <div class="section-header">
              <h2>Characters</h2>
              <button class="create-btn" on:click={() => dispatch('createCharacter')}>+ Create</button>
          </div>
          
          {#if characters.length === 0}
              <p class="empty-state">No characters yet.</p>
          {:else}
              <div class="char-list">
                  {#each characters as char (char.id)}
                      <div class="char-item" class:active={selectedCharacterIds.has(char.id)}>
                          <label class="char-checkbox-area">
                              <input 
                                  type="checkbox" 
                                  checked={selectedCharacterIds.has(char.id)}
                                  on:change={() => toggleCharacter(char.id)}
                              />
                              <div class="char-info">
                                  <span class="char-name">{char.name}</span>
                                  <span class="char-meta">{char.gender}, {char.age}</span>
                              </div>
                          </label>
                          <div class="char-controls">
                              <button class="edit-icon" on:click={() => dispatch('editCharacter', char)}>✎</button>
                              <button class="delete-icon" on:click={() => dispatch('deleteCharacter', char.id)}>🗑️</button>
                          </div>
                      </div>
                  {/each}
              </div>
          {/if}
      </div>
    </div>
  {:else}
    <!-- Collapsed View -->
    <div class="collapsed-content">
      <div class="collapsed-icon" title="Story Settings">📖</div>
      <div class="collapsed-icon" title="Characters">👤</div>
    </div>
  {/if}
</div>

<style>
  .config-panel { 
    width: 300px; 
    background: var(--bg-tertiary); 
    border-left: 1px solid var(--border-primary); 
    display: flex; 
    flex-direction: column; 
    flex-shrink: 0; 
    position: relative;
    transition: width 0.3s ease;
  }

  .config-panel.collapsed {
    width: 50px;
  }

  .collapse-toggle {
    position: absolute;
    left: -12px;
    top: 50%;
    transform: translateY(-50%);
    width: 24px;
    height: 48px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    border-radius: 4px 0 0 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 0.8em;
    z-index: 10;
    transition: all 0.2s;
  }

  .collapse-toggle:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .panel-content {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 30px;
  }

  .collapsed-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 20px;
    gap: 20px;
  }

  .collapsed-icon {
    font-size: 1.5em;
    cursor: default;
    opacity: 0.6;
  }

  .config-section h2 { 
    font-size: 1.2em; 
    color: var(--text-primary); 
    margin-bottom: 15px; 
    padding-bottom: 5px; 
    border-bottom: 2px solid var(--border-secondary); 
  }

  .section-header { 
    display: flex; 
    justify-content: space-between; 
    align-items: center; 
    margin-bottom: 15px; 
  }

  .section-header h2 { 
    margin-bottom: 0; 
    border-bottom: none; 
  }

  .create-btn { 
    background: var(--accent-success); 
    color: white; 
    border: none; 
    padding: 4px 10px; 
    border-radius: 4px; 
    font-size: 0.8em; 
    cursor: pointer; 
    transition: opacity 0.2s;
  }
  .create-btn:hover { opacity: 0.9; }

  .empty-state { 
    font-size: 0.9em; 
    color: var(--text-muted); 
    font-style: italic; 
  }

  .story-list { 
    display: flex; 
    flex-direction: column; 
    gap: 10px; 
  }

  .radio-item { 
    display: flex; 
    justify-content: space-between; 
    align-items: center; 
    gap: 10px; 
    padding: 10px; 
    background: var(--bg-primary); 
    border: 1px solid var(--border-secondary); 
    border-radius: 6px; 
    cursor: pointer; 
    transition: all 0.2s; 
  }
  .radio-item:hover { 
    background: var(--bg-hover); 
    border-color: var(--border-primary); 
  }
  .radio-item.selected-radio { 
    background: var(--bg-secondary); 
    border-color: var(--border-active); 
  }
  
  .radio-content { 
    display: flex; 
    flex-direction: column; 
    flex: 1; 
  }
  .radio-title { 
    font-weight: bold; 
    font-size: 0.95em; 
    color: var(--text-primary); 
  }
  .radio-desc { 
    font-size: 0.8em; 
    color: var(--text-secondary); 
    margin-top: 2px; 
  }

  .story-controls { 
    display: flex; 
    gap: 5px; 
  }
  .story-controls button { 
    background: transparent; 
    border: none; 
    cursor: pointer; 
    font-size: 0.9em; 
    padding: 3px; 
  }
  .story-controls .edit-btn { color: var(--accent-primary); }
  .story-controls .delete-btn { color: var(--accent-danger); }

  .char-list { 
    display: flex; 
    flex-direction: column; 
    gap: 8px; 
  }

  .char-item { 
    display: flex; 
    justify-content: space-between; 
    align-items: center; 
    background: var(--bg-primary); 
    border: 1px solid var(--border-secondary); 
    border-radius: 6px; 
    padding: 8px; 
    transition: all 0.2s; 
  }
  .char-item.active { 
    border-color: var(--border-active); 
    background: var(--bg-secondary); 
  }

  .char-checkbox-area { 
    display: flex; 
    align-items: center; 
    gap: 10px; 
    flex: 1; 
    cursor: pointer; 
  }
  
  .char-info { 
    display: flex; 
    flex-direction: column; 
  }
  .char-name { 
    font-weight: bold; 
    font-size: 0.9em; 
    color: var(--text-primary);
  }
  .char-meta { 
    font-size: 0.75em; 
    color: var(--text-muted); 
  }

  .char-controls { 
    display: flex; 
    gap: 5px; 
  }
  .char-controls button { 
    background: transparent; 
    border: none; 
    cursor: pointer; 
    padding: 4px; 
    font-size: 1em; 
  }
  .char-controls .edit-icon { color: var(--accent-primary); }
  .char-controls .delete-icon { color: var(--accent-danger); }
</style>