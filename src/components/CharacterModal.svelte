<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { CharacterProfile } from '../lib/types';

  export let show = false;
  export let character: CharacterProfile | null = null; 

  const dispatch = createEventDispatcher();
  let isGenerating = false;

  const styles = ["Realistic", "Anime", "3D", "Painting", "Sketch"];

  const getEmptyForm = (): CharacterProfile => ({
    id: crypto.randomUUID() as `${string}-${string}-${string}-${string}-${string}`,
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
    image: undefined,
    seed: undefined,
    art_style: "Realistic"
  });

  let form: CharacterProfile = getEmptyForm();

  // Track the previous state of 'show' to ensure we only run this ONCE when opening
  let wasShown = false;
  
  $: if (show && !wasShown) {
      wasShown = true;
      
      if (character) {
          form = { ...character };
          if (!form.art_style) form.art_style = "Realistic";
      } else {
          form = getEmptyForm();
          generateSdPrompt(); 
      }
  } else if (!show && wasShown) {
      wasShown = false;
  }

  function generateSdPrompt() {
    // Build prompt in SD-friendly format without colons
    const parts: string[] = [];
    
    // Quality tags first (these get highest weight)
    parts.push('(masterpiece, best quality, high resolution)');
    
    // Subject - use 1girl/1boy for better SD understanding
    const genderTag = form.gender === 'Female' ? '1girl' : 
                      form.gender === 'Male' ? '1boy' : 
                      '1person';
    parts.push(`solo, ${genderTag}`);
    
    // Age
    if (form.age) {
      parts.push(`${form.age} years old`);
    }
    
    // Hair - put color BEFORE style for emphasis
    if (form.hair_color && form.hair_style) {
      // Clean up the hair style to be SD-friendly
      const hairStyle = form.hair_style.toLowerCase().replace(',', '');
      parts.push(`${form.hair_color.toLowerCase()} hair`);
      parts.push(`${hairStyle} hair`);
    } else if (form.hair_color) {
      parts.push(`${form.hair_color.toLowerCase()} hair`);
    }
    
    // Skin tone
    if (form.skin_tone) {
      parts.push(`${form.skin_tone.toLowerCase()} skin`);
    }
    
    // Body type
    if (form.body_type) {
      const bodyMap: Record<string, string> = {
        'Slim': 'slim body',
        'Athletic': 'athletic body, fit',
        'Average': 'normal body',
        'Curvy': 'curvy body',
        'Muscular': 'muscular body',
        'Heavyset': 'large body'
      };
      parts.push(bodyMap[form.body_type] || `${form.body_type.toLowerCase()} body`);
    }
    
    // Clothing based on gender
    if (form.gender === 'Female') {
      parts.push('wearing elegant dress');
    } else if (form.gender === 'Male') {
      parts.push('wearing casual shirt');
    } else {
      parts.push('wearing casual clothing');
    }
    
    // Expression/personality
    if (form.personality) {
      parts.push(`${form.personality.toLowerCase()} expression`);
    }
    
    // Portrait framing tags
    parts.push('detailed face, looking at viewer, portrait, upper body');
    
    // Lighting and quality
    parts.push('soft lighting, studio lighting');

    form.sd_prompt = parts.join(', ');
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
          alert("Failed to generate portrait: " + e);
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

  function handleBackdropClick(e: MouseEvent) {
      if (e.target === e.currentTarget) {
          close();
      }
  }
</script>

{#if show}
<div 
  class="modal-backdrop" 
  on:click={handleBackdropClick}
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
                <textarea id="char-prompt" bind:value={form.sd_prompt} rows="4"></textarea>
                <small>Editing this changes the image generator instructions directly. The prompt is auto-generated from the fields above.</small>
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
  }
  .modal {
    background: var(--bg-primary, white); 
    color: var(--text-primary, #333);
    padding: 25px; 
    border-radius: 8px;
    width: 900px;
    max-height: 90vh; 
    overflow-y: auto;
    box-shadow: 0 10px 25px var(--shadow, rgba(0,0,0,0.5));
  }
  h2 { 
    margin-top: 0; 
    border-bottom: 1px solid var(--border-secondary, #eee); 
    padding-bottom: 10px; 
  }

  .split-layout { display: flex; gap: 20px; }

  .inputs-column { flex: 1; }
  
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .group { display: flex; flex-direction: column; gap: 4px; }
  .full { width: 100%; margin-top: 10px; grid-column: 1 / -1; }
  
  label { 
    font-size: 0.85em; 
    font-weight: bold; 
    color: var(--text-secondary, #555); 
  }
  
  input, select, textarea { 
    padding: 8px; 
    border: 1px solid var(--border-primary, #ccc); 
    border-radius: 4px; 
    font-size: 14px; 
    background: var(--bg-secondary, #fff);
    color: var(--text-primary, #333);
  }
  
  input:focus, select:focus, textarea:focus {
    outline: none;
    border-color: var(--accent-primary, #007bff);
  }
  
  small { 
    font-size: 0.75em; 
    color: var(--text-muted, #888); 
  }

  .portrait-column {
      width: 300px;
      display: flex;
      flex-direction: column;
      gap: 10px;
      background: var(--bg-tertiary, #f8f9fa);
      padding: 15px;
      border-radius: 8px;
      border: 1px solid var(--border-secondary, #eee);
  }

  .image-preview {
      width: 100%;
      aspect-ratio: 2/3;
      background: var(--bg-secondary, #e9ecef);
      border-radius: 4px;
      overflow: hidden;
      display: flex;
      align-items: center;
      justify-content: center;
      border: 1px solid var(--border-primary, #dee2e6);
  }
  .image-preview img { width: 100%; height: 100%; object-fit: cover; }
  .placeholder { 
    text-align: center; 
    color: var(--text-muted, #adb5bd); 
    display: flex; 
    flex-direction: column; 
  }

  .style-label { margin-bottom: -5px; margin-top: 5px; }

  .style-dropdown {
      width: 100%;
      padding: 8px;
      margin-bottom: 5px;
      border: 1px solid var(--border-primary, #ccc);
      border-radius: 4px;
      background: var(--bg-secondary, #fff);
      color: var(--text-primary, #333);
  }

  .gen-btn {
      background: var(--accent-primary, #6f42c1); 
      color: var(--text-inverse, white); 
      border: none; 
      padding: 10px;
      border-radius: 4px; 
      cursor: pointer; 
      font-weight: bold;
      transition: opacity 0.2s;
  }
  .gen-btn:hover:not(:disabled) { opacity: 0.9; }
  .gen-btn:disabled { opacity: 0.7; cursor: wait; }

  .seed-info { 
    font-size: 0.75em; 
    color: var(--text-muted, #666); 
    text-align: center; 
  }
  code { 
    background: var(--bg-secondary, #eee); 
    padding: 2px 4px; 
    border-radius: 3px; 
    font-family: monospace; 
  }

  .actions { 
    margin-top: 20px; 
    display: flex; 
    justify-content: flex-end; 
    gap: 10px; 
    padding-top: 15px; 
    border-top: 1px solid var(--border-secondary, #eee); 
  }
  
  .cancel { 
    background: var(--accent-secondary, #6c757d); 
    color: white; 
    padding: 10px 20px; 
    border: none; 
    border-radius: 4px; 
    cursor: pointer; 
  }
  .cancel:hover { opacity: 0.9; }
  
  .save { 
    background: var(--accent-success, #28a745); 
    color: white; 
    padding: 10px 20px; 
    border: none; 
    border-radius: 4px; 
    cursor: pointer; 
    font-weight: bold;
  }
  .save:hover { opacity: 0.9; }
</style>