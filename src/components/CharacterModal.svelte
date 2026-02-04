<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { CharacterProfile } from '../lib/types';

  export let show = false;
  export let character: CharacterProfile | null = null; 

  const dispatch = createEventDispatcher();
  let isGenerating = false;

  const styles = ["Realistic", "Anime", "3D", "Painting", "Sketch"];

  const getEmptyForm = () => ({
    id: crypto.randomUUID(),
    name: '',
    age: 25,
    gender: 'Female',
    skin_tone: 'Fair',
    hair_style: 'Long, straight',
    hair_color: 'Black',
    body_type: 'Average',
    personality: 'Brave and curious',
    additional_notes: '',
    sd_prompt: '',
    image: undefined as string | undefined, 
    seed: undefined as number | undefined,
    art_style: "Realistic"
  });

  let form = getEmptyForm();

  // FIX: Track the previous state of 'show' to ensure we only run this ONCE when opening
  let wasShown = false;
  
  $: if (show && !wasShown) {
      wasShown = true; // Mark as opened so typing doesn't re-trigger this
      
      if (character) {
          form = { ...character };
          // Ensure legacy characters have a style
          if (!form.art_style) form.art_style = "Realistic";
      } else {
          form = getEmptyForm();
          generateSdPrompt(); 
      }
  } else if (!show && wasShown) {
      wasShown = false; // Reset when closed
  }

  function generateSdPrompt() {
    const base = `(masterpiece, best quality), solo, ${form.gender}, ${form.age} years old`;
    const features = `skin: ${form.skin_tone}, hair: ${form.hair_color} ${form.hair_style}, body: ${form.body_type}`;
    const details = `${form.personality} expression, detailed face, looking at viewer, portrait`;
    
    let clothing = "casual clothing";
    if (form.gender === 'Female') clothing = "detailed dress";
    else if (form.gender === 'Male') clothing = "shirt and pants";

    form.sd_prompt = `${base}, ${features}, ${clothing}, ${details}`;
  }

  async function generatePortrait() {
      if (isGenerating) return;
      isGenerating = true;
      
      if (!form.sd_prompt) generateSdPrompt();

      try {
          const [base64, seedStr] = await invoke('generate_character_portrait', { 
              prompt: form.sd_prompt,
              style: form.art_style 
          }) as [string, string];

          form.image = base64;
          form.seed = parseInt(seedStr); 
      } catch (e) {
          console.error(e);
          alert("Failed to generate portrait.");
      } finally {
          isGenerating = false;
      }
  }

  function save() {
      if (!form.sd_prompt) generateSdPrompt();
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
    <h2>{character ? 'Edit Character' : 'Create New Character'}</h2>
    
    <div class="split-layout">
        <div class="inputs-column">
            <div class="form-grid">
                <div class="group">
                    <label for="char-name">Name</label>
                    <input id="char-name" type="text" bind:value={form.name} placeholder="Name" />
                </div>
                <div class="group">
                    <label for="char-age">Age</label>
                    <input id="char-age" type="number" bind:value={form.age} on:input={generateSdPrompt}/>
                </div>
                <div class="group">
                    <label for="char-gender">Gender</label>
                    <select id="char-gender" bind:value={form.gender} on:change={generateSdPrompt}>
                        <option>Male</option><option>Female</option><option>Non-Binary</option><option>Other</option>
                    </select>
                </div>
                <div class="group">
                    <label for="char-skin">Skin Tone</label>
                    <input id="char-skin" type="text" bind:value={form.skin_tone} on:input={generateSdPrompt} />
                </div>
                <div class="group">
                    <label for="char-haircolor">Hair Color</label>
                    <input id="char-haircolor" type="text" bind:value={form.hair_color} on:input={generateSdPrompt} />
                </div>
                <div class="group">
                    <label for="char-hairstyle">Hair Style</label>
                    <input id="char-hairstyle" type="text" bind:value={form.hair_style} on:input={generateSdPrompt} />
                </div>
                <div class="group">
                    <label for="char-body">Body Type</label>
                    <select id="char-body" bind:value={form.body_type} on:change={generateSdPrompt}>
                        <option>Slim</option><option>Athletic</option><option>Average</option><option>Curvy</option><option>Muscular</option><option>Heavyset</option>
                    </select>
                </div>
                 <div class="group">
                    <label for="char-personality">Vibe</label>
                    <input id="char-personality" type="text" bind:value={form.personality} on:input={generateSdPrompt} />
                </div>
            </div>
            
            <div class="group full">
                <label for="char-prompt">Prompt Preview</label>
                <textarea id="char-prompt" bind:value={form.sd_prompt} rows="3"></textarea>
                <small>Editing this changes the image generator instructions directly.</small>
            </div>
        </div>

        <div class="portrait-column">
            <div class="image-preview">
                {#if form.image}
                    <img src="data:image/png;base64,{form.image}" alt="Character Portrait" />
                {:else}
                    <div class="placeholder">
                        <span>No Portrait</span>
                        <small>Click Generate below</small>
                    </div>
                {/if}
            </div>
            
            <label class="style-label">Art Style</label>
            <select class="style-dropdown" bind:value={form.art_style}>
                {#each styles as style}
                    <option value={style}>{style}</option>
                {/each}
            </select>

            <button class="gen-btn" on:click={generatePortrait} disabled={isGenerating}>
                {isGenerating ? 'Generating...' : '🎲 Generate Portrait'}
            </button>

            {#if form.seed}
                <div class="seed-info">
                    DNA Seed: <code>{form.seed}</code>
                </div>
            {/if}
        </div>
    </div>

    <div class="actions">
        <button on:click={close} class="cancel">Cancel</button>
        <button on:click={save} class="save">Save Character</button>
    </div>
  </div>
</div>
{/if}

<style>
  .modal-backdrop {
    position: fixed; top: 0; left: 0; width: 100%; height: 100%;
    background: rgba(0,0,0,0.6); z-index: 2000;
    display: flex; justify-content: center; align-items: center;
    cursor: pointer;
  }
  .modal {
    background: white; padding: 25px; border-radius: 8px;
    width: 900px;
    max-height: 90vh; overflow-y: auto;
    box-shadow: 0 10px 25px rgba(0,0,0,0.5);
    cursor: default; outline: none;
  }
  h2 { margin-top: 0; border-bottom: 1px solid #eee; padding-bottom: 10px; }

  .split-layout { display: flex; gap: 20px; }

  .inputs-column { flex: 1; }
  
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .group { display: flex; flex-direction: column; gap: 4px; }
  .full { width: 100%; margin-top: 10px; }
  
  label { font-size: 0.85em; font-weight: bold; color: #555; }
  input, select, textarea { padding: 8px; border: 1px solid #ccc; border-radius: 4px; font-size: 14px; }
  small { font-size: 0.75em; color: #888; }

  .portrait-column {
      width: 300px;
      display: flex;
      flex-direction: column;
      gap: 10px;
      background: #f8f9fa;
      padding: 15px;
      border-radius: 8px;
      border: 1px solid #eee;
  }

  .image-preview {
      width: 100%;
      aspect-ratio: 2/3;
      background: #e9ecef;
      border-radius: 4px;
      overflow: hidden;
      display: flex;
      align-items: center;
      justify-content: center;
      border: 1px solid #dee2e6;
  }
  .image-preview img { width: 100%; height: 100%; object-fit: cover; }
  .placeholder { text-align: center; color: #adb5bd; display: flex; flex-direction: column; }

  .style-label { margin-bottom: -5px; margin-top: 5px; }

  .style-dropdown {
      width: 100%;
      padding: 8px;
      margin-bottom: 5px;
      border: 1px solid #ccc;
      border-radius: 4px;
  }

  .gen-btn {
      background: #6f42c1; color: white; border: none; padding: 10px;
      border-radius: 4px; cursor: pointer; font-weight: bold;
      transition: background 0.2s;
  }
  .gen-btn:hover { background: #59359a; }
  .gen-btn:disabled { opacity: 0.7; cursor: wait; }

  .seed-info { font-size: 0.75em; color: #666; text-align: center; }
  code { background: #eee; padding: 2px 4px; border-radius: 3px; font-family: monospace; }

  .actions { margin-top: 20px; display: flex; justify-content: flex-end; gap: 10px; pt: 10px; border-top: 1px solid #eee; }
  .cancel { background: #6c757d; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; }
  .save { background: #28a745; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; font-weight: bold;}
  .save:hover { background: #218838; }
</style>