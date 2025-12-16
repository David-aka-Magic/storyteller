<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { StoryPremise } from '../lib/types';

  export let show = false;
  export let story: StoryPremise | null = null; 

  const dispatch = createEventDispatcher();

  const getEmptyForm = () => ({
    id: crypto.randomUUID() as any,
    title: '',
    description: ''
  });

  let form = getEmptyForm();

  $: if (show) {
      if (story) {
          form = { ...story };
      } else {
          form = getEmptyForm();
      }
  }

  function save() {
      if (!form.title.trim() || !form.description.trim()) {
          alert("Please fill in both title and description.");
          return;
      }
      dispatch('save', form);
  }

  function close() {
      dispatch('close');
  }

  function handleKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') close();
  }
</script>

{#if show}
<div 
  class="modal-backdrop" 
  on:click={close}
  on:keydown={handleKeydown}
  role="button"
  tabindex="0"
>
  <div 
    class="modal" 
    on:click|stopPropagation 
    role="document" 
    tabindex="-1"
  >
    <h2>{story ? 'Edit Story Premise' : 'Create New Story Premise'}</h2>
    
    <div class="group">
      <label for="story-title">Title</label>
      <input id="story-title" type="text" bind:value={form.title} placeholder="e.g., The Lost Starship" />
    </div>

    <div class="group">
      <label for="story-desc">Description (Key Details for AI)</label>
      <textarea 
        id="story-desc" 
        bind:value={form.description} 
        placeholder="e.g., A hard sci-fi setting where resources are scarce..." 
        rows="5"></textarea>
      <small>This description will be sent to the AI to set the scene for your story.</small>
    </div>

    <div class="actions">
        <button on:click={close} class="cancel">Cancel</button>
        <button on:click={save} class="save">Save Story</button>
    </div>
  </div>
</div>
{/if}

<style>
  .modal-backdrop {
    position: fixed; top: 0; left: 0; width: 100%; height: 100%;
    background: rgba(0,0,0,0.5); z-index: 2000;
    display: flex; justify-content: center; align-items: center;
    cursor: pointer;
  }
  .modal {
    background: white; padding: 25px; border-radius: 8px;
    width: 500px; max-height: 90vh; overflow-y: auto;
    box-shadow: 0 5px 15px rgba(0,0,0,0.3);
    cursor: default;
    outline: none;
  }
  h2 { margin-top: 0; border-bottom: 1px solid #eee; padding-bottom: 10px; }

  .group { display: flex; flex-direction: column; gap: 5px; margin-bottom: 20px; }
  
  label { font-size: 0.95em; font-weight: bold; color: #333; }
  input, textarea { padding: 10px; border: 1px solid #ccc; border-radius: 4px; font-family: inherit; font-size: 14px;}
  textarea { resize: vertical; }
  small { color: #666; font-size: 0.85em; }

  .actions { margin-top: 10px; display: flex; justify-content: flex-end; gap: 10px; }
  
  .cancel { background: #6c757d; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; }
  .save { background: #4a9eff; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; font-weight: bold; }
  
  .save:hover { background: #357abd; }
  .cancel:hover { background: #5a6268; }
</style>