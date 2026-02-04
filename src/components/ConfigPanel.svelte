<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { StoryPremise, CharacterProfile } from '../lib/types';

  export let stories: StoryPremise[] = [];
  export let characters: CharacterProfile[] = [];
  export let selectedStoryId: string = '';
  export let selectedCharacterIds: Set<string> = new Set();

  const dispatch = createEventDispatcher();

  function toggleCharacter(id: string) {
    dispatch('toggleCharacter', id);
  }
</script>

<div class="config-panel">
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

<style>
  .config-panel { width: 300px; background: #f9f9fc; border-left: 1px solid #ddd; display: flex; flex-direction: column; overflow-y: auto; flex-shrink: 0; padding: 20px; gap: 30px; }
  .config-section h2 { font-size: 1.2em; color: #333; margin-bottom: 15px; padding-bottom: 5px; border-bottom: 2px solid #e0e0e0; }
  .section-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px; }
  .section-header h2 { margin-bottom: 0; border-bottom: none; }

  .create-btn { background: #28a745; color: white; border: none; padding: 4px 10px; border-radius: 4px; font-size: 0.8em; cursor: pointer; }
  .create-btn:hover { background: #218838; }

  .empty-state { font-size: 0.9em; color: #999; font-style: italic; }

  .story-list { display: flex; flex-direction: column; gap: 10px; }
  .radio-item { display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 10px; background: white; border: 1px solid #eee; border-radius: 6px; cursor: pointer; transition: all 0.2s; }
  .radio-item:hover { background: #f0f4ff; border-color: #bbdefb; }
  .radio-item.selected-radio { background: #e3f2fd; border-color: #2196f3; }
  .radio-content { display: flex; flex-direction: column; flex: 1; }
  .radio-title { font-weight: bold; font-size: 0.95em; color: #333; }
  .radio-desc { font-size: 0.8em; color: #666; margin-top: 2px; }
  .story-controls { display: flex; gap: 5px; }
  .story-controls button { background: transparent; border: none; cursor: pointer; font-size: 0.9em; padding: 3px; }
  .story-controls .edit-btn { color: #007bff; }
  .story-controls .delete-btn { color: #dc3545; }

  .char-list { display: flex; flex-direction: column; gap: 8px; }
  .char-item { display: flex; justify-content: space-between; align-items: center; background: white; border: 1px solid #eee; border-radius: 6px; padding: 8px; transition: all 0.2s; }
  .char-item.active { border-color: #2196f3; background: #f5faff; }
  .char-checkbox-area { display: flex; align-items: center; gap: 10px; flex: 1; cursor: pointer; }
  .char-info { display: flex; flex-direction: column; }
  .char-name { font-weight: bold; font-size: 0.9em; }
  .char-meta { font-size: 0.75em; color: #777; }
  .char-controls { display: flex; gap: 5px; }
  .char-controls button { background: transparent; border: none; cursor: pointer; padding: 4px; font-size: 1em; }
  .char-controls .edit-icon { color: #007bff; }
  .char-controls .delete-icon { color: #dc3545; }
</style>