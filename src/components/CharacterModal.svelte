<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { CharacterProfile } from '../lib/types';

  export let show = false;
  export let character: CharacterProfile | null = null; 

  const dispatch = createEventDispatcher();

  const getEmptyForm = () => ({
    id: crypto.randomUUID() as any,
    name: '',
    age: 25,
    gender: 'Female',
    skin_tone: 'Fair',
    hair_style: 'Long, straight',
    hair_color: 'Black',
    body_type: 'Average',
    personality: 'Brave and curious',
    additional_notes: '',
    sd_prompt: ''
  });

  let form = getEmptyForm();

  $: if (show) {
      if (character) {
          form = { ...character };
      } else {
          form = getEmptyForm();
          generateSdPrompt(); 
      }
  }

  function generateSdPrompt() {
    const base = `(masterpiece, best quality), solo, ${form.gender}, ${form.age} years old`;
    const features = `skin: ${form.skin_tone}, hair: ${form.hair_color} ${form.hair_style}, body: ${form.body_type}`;
    const details = `${form.personality} expression, detailed face`;
    
    let clothing = "casual clothing";
    if (form.gender === 'Female') clothing = "detailed dress";
    else if (form.gender === 'Male') clothing = "shirt and pants";

    form.sd_prompt = `${base}, ${features}, ${clothing}, ${details}`;
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
    
    <div class="form-grid">
      <div class="group">
        <label for="char-name">Name</label>
        <input id="char-name" type="text" bind:value={form.name} placeholder="Character Name" />
      </div>

      <div class="group">
        <label for="char-age">Age</label>
        <input id="char-age" type="number" bind:value={form.age} on:input={generateSdPrompt}/>
      </div>

      <div class="group">
        <label for="char-gender">Gender</label>
        <select id="char-gender" bind:value={form.gender} on:change={generateSdPrompt}>
          <option value="Male">Male</option>
          <option value="Female">Female</option>
          <option value="Non-Binary">Non-Binary</option>
          <option value="Other">Other</option>
        </select>
      </div>

      <div class="group">
        <label for="char-skin">Skin Tone</label>
        <input id="char-skin" type="text" bind:value={form.skin_tone} on:input={generateSdPrompt} />
      </div>

      <div class="group">
        <label for="char-body">Body Type</label>
        <select id="char-body" bind:value={form.body_type} on:change={generateSdPrompt}>
            <option value="Slim">Slim</option>
            <option value="Athletic">Athletic</option>
            <option value="Average">Average</option>
            <option value="Curvy">Curvy/Muscular</option>
            <option value="Heavyset">Heavyset</option>
        </select>
      </div>

      <div class="group">
          <label for="char-haircolor">Hair Color</label>
          <input id="char-haircolor" type="text" bind:value={form.hair_color} on:input={generateSdPrompt} />
      </div>

      <div class="group">
          <label for="char-hairstyle">Hair Style</label>
          <input id="char-hairstyle" type="text" bind:value={form.hair_style} on:input={generateSdPrompt} />
      </div>
    </div>

    <div class="group full">
        <label for="char-personality">Personality / Vibe</label>
        <input id="char-personality" type="text" bind:value={form.personality} on:input={generateSdPrompt} placeholder="e.g. Cheerful, Mysterious, Angry"/>
    </div>

    <div class="sd-preview">
        <label for="char-prompt">Generated SD Prompt (Editable)</label>
        <textarea id="char-prompt" bind:value={form.sd_prompt} rows="3"></textarea>
        <button class="regen-btn" on:click={generateSdPrompt}>Regenerate Prompt from Fields</button>
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
    background: rgba(0,0,0,0.5); z-index: 2000;
    display: flex; justify-content: center; align-items: center;
    cursor: pointer;
  }
  .modal {
    background: white; padding: 25px; border-radius: 8px;
    width: 600px; max-height: 90vh; overflow-y: auto;
    box-shadow: 0 5px 15px rgba(0,0,0,0.3);
    cursor: default;
    outline: none;
  }
  h2 { margin-top: 0; border-bottom: 1px solid #eee; padding-bottom: 10px; }
  
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 15px; }
  .group { display: flex; flex-direction: column; gap: 5px; }
  .full { width: 100%; margin-top: 15px; }
  
  label { font-size: 0.9em; font-weight: bold; color: #555; }
  input, select, textarea { padding: 8px; border: 1px solid #ccc; border-radius: 4px; font-size: 14px; }
  
  .sd-preview { margin-top: 20px; background: #f0f8ff; padding: 15px; border-radius: 5px; border: 1px solid #d0e8ff; }
  .sd-preview textarea { width: 100%; box-sizing: border-box; resize: vertical; margin-top: 5px; }
  
  .regen-btn { 
      margin-top: 8px; font-size: 0.8em; cursor: pointer; 
      background: #e1f5fe; border: 1px solid #81d4fa; padding: 4px 8px; border-radius: 4px; color: #0277bd;
  }
  .regen-btn:hover { background: #b3e5fc; }

  .actions { margin-top: 20px; display: flex; justify-content: flex-end; gap: 10px; pt: 10px; border-top: 1px solid #eee; }
  .cancel { background: #6c757d; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; }
  .save { background: #28a745; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; font-weight: bold;}
  .save:hover { background: #218838; }
  .cancel:hover { background: #5a6268; }
</style>