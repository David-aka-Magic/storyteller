<!-- src/components/MasterPortraitGenerator.svelte -->
<!--
  Master Portrait Generator
  ==========================
  UI for creating the initial "master" reference image for a character.
  This image is used by IP-Adapter FaceID for consistent rendering.

  Flow:
    1. User fills in character details (or they come pre-filled from registration)
    2. User can preview/edit the SD prompt
    3. Click "Generate 4 Options" → calls ComfyUI via backend
    4. Gallery shows 4 options → user clicks to select the best
    5. "Save as Master" → saves to disk + updates character DB

  Usage:
    <MasterPortraitGenerator
      character={existingCharacter}
      show={true}
      on:saved={(e) => handleSaved(e.detail)}
      on:close={() => showGenerator = false}
    />
-->
<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { CharacterProfile } from '../lib/character-types';

  // ── Props ──
  export let show = false;
  export let character: Partial<CharacterProfile> | null = null;
  export let storyId: number | undefined = undefined;

  const dispatch = createEventDispatcher<{
    saved: { characterId: number; masterImagePath: string };
    close: void;
  }>();

  // ── State ──
  let form = getEmptyForm();
  let generatedImages: string[] = [];      // base64 images
  let generatedPaths: string[] = [];       // file paths on disk
  let selectedIndex: number = -1;
  let isGenerating = false;
  let isSaving = false;
  let promptPreview = '';
  let showPromptEditor = false;
  let generationError = '';
  let promptId = '';
  let seedUsed: number = -1;

  // ── Form defaults ──
  function getEmptyForm() {
    return {
      name: '',
      age: undefined as number | undefined,
      gender: 'Male',
      skin_tone: '',
      hair_color: '',
      hair_style: '',
      body_type: 'Average',
      default_clothing: '',
      physical_features: '',
      art_style: 'Realistic',
      custom_prompt: '',
    };
  }

  const styles = ['Realistic', 'Anime', '3D', 'Painting', 'Sketch'];
  const bodyTypes = ['Slim', 'Athletic', 'Average', 'Curvy', 'Muscular', 'Heavyset'];
  const genders = ['Male', 'Female', 'Non-Binary', 'Other'];

  // ── Reactive: populate form from character prop ──
  $: if (show) {
    if (character) {
      form = {
        name: character.name || '',
        age: character.age,
        gender: character.gender || 'Male',
        skin_tone: character.skin_tone || '',
        hair_color: character.hair_color || '',
        hair_style: character.hair_style || '',
        body_type: character.body_type || 'Average',
        default_clothing: character.default_clothing || '',
        physical_features: character.additional_notes || '',
        art_style: character.art_style || 'Realistic',
        custom_prompt: '',
      };
    } else {
      form = getEmptyForm();
    }
    // Reset generation state
    generatedImages = [];
    generatedPaths = [];
    selectedIndex = -1;
    generationError = '';
    updatePromptPreview();
  }

  // ── Prompt preview (calls backend for consistent prompt building) ──
  async function updatePromptPreview() {
    try {
      promptPreview = await invoke<string>('preview_portrait_prompt', {
        request: {
          name: form.name,
          age: form.age || null,
          gender: form.gender || null,
          skin_tone: form.skin_tone || null,
          hair_color: form.hair_color || null,
          hair_style: form.hair_style || null,
          body_type: form.body_type || null,
          default_clothing: form.default_clothing || null,
          physical_features: form.physical_features || null,
          art_style: form.art_style || null,
          custom_prompt: form.custom_prompt || null,
        }
      });
    } catch {
      // Fallback: build prompt locally if backend not available
      promptPreview = buildLocalPromptPreview();
    }
  }

  function buildLocalPromptPreview(): string {
    if (form.custom_prompt) return form.custom_prompt;

    const parts = ['(masterpiece, best quality)'];
    const gTag = form.gender === 'Female' ? '1girl' : form.gender === 'Male' ? '1boy' : '1person';
    parts.push(`solo, ${gTag}`);
    parts.push('portrait, upper body, looking at viewer');
    if (form.age) parts.push(`${form.age} year old`);
    if (form.skin_tone) parts.push(`${form.skin_tone.toLowerCase()} skin`);
    if (form.hair_color && form.hair_style) {
      parts.push(`${form.hair_color.toLowerCase()} ${form.hair_style.toLowerCase()} hair`);
    } else if (form.hair_color) {
      parts.push(`${form.hair_color.toLowerCase()} hair`);
    }
    if (form.body_type && form.body_type !== 'Average') {
      parts.push(`${form.body_type.toLowerCase()} body`);
    }
    if (form.physical_features) parts.push(form.physical_features.toLowerCase());
    if (form.default_clothing) parts.push(`wearing ${form.default_clothing}`);
    parts.push('neutral gray background, detailed face, sharp focus, soft studio lighting');
    return parts.join(', ');
  }

  // Debounced prompt update
  let promptTimeout: ReturnType<typeof setTimeout>;
  function onFormChange() {
    clearTimeout(promptTimeout);
    promptTimeout = setTimeout(updatePromptPreview, 300);
  }

  // ── Generate portraits ──
  async function generatePortraits() {
    if (isGenerating) return;

    if (!form.name.trim()) {
      generationError = 'Please enter a character name.';
      return;
    }

    isGenerating = true;
    generationError = '';
    generatedImages = [];
    generatedPaths = [];
    selectedIndex = -1;

    try {
      const result = await invoke<{
        images_base64: string[];
        image_paths: string[];
        prompt_used: string;
        seed: number;
        prompt_id: string;
      }>('generate_master_portrait', {
        request: {
          name: form.name,
          age: form.age || null,
          gender: form.gender || null,
          skin_tone: form.skin_tone || null,
          hair_color: form.hair_color || null,
          hair_style: form.hair_style || null,
          body_type: form.body_type || null,
          default_clothing: form.default_clothing || null,
          physical_features: form.physical_features || null,
          art_style: form.art_style || null,
          custom_prompt: form.custom_prompt || null,
          seed: null,
        }
      });

      generatedImages = result.images_base64;
      generatedPaths = result.image_paths;
      promptPreview = result.prompt_used;
      seedUsed = result.seed;
      promptId = result.prompt_id;

    } catch (e) {
      generationError = `Generation failed: ${e}`;
      console.error('[MasterPortrait]', e);
    } finally {
      isGenerating = false;
    }
  }

  // ── Save selected master ──
  async function saveMasterPortrait() {
    if (selectedIndex < 0 || isSaving) return;
    if (!character?.id) {
      generationError = 'Character must be saved first before setting a master image.';
      return;
    }

    isSaving = true;
    generationError = '';

    try {
      const masterPath = await invoke<string>('save_master_portrait', {
        request: {
          character_id: character.id,
          selected_index: selectedIndex,
          image_paths: generatedPaths,
          character_name: form.name,
        }
      });

      dispatch('saved', {
        characterId: character.id,
        masterImagePath: masterPath,
      });

    } catch (e) {
      generationError = `Failed to save master image: ${e}`;
      console.error('[MasterPortrait]', e);
    } finally {
      isSaving = false;
    }
  }

  // ── UI helpers ──
  function selectImage(index: number) {
    selectedIndex = index;
  }

  function close() {
    dispatch('close');
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
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
  <div class="modal" on:click|stopPropagation role="document" tabindex="-1">

    <!-- Header -->
    <div class="header">
      <h2>🎨 Master Portrait Generator</h2>
      <span class="subtitle">Create the reference image for IP-Adapter consistency</span>
    </div>

    <div class="content-layout">
      <!-- LEFT: Character Details Form -->
      <div class="form-panel">
        <h3>Character Details</h3>

        <div class="form-grid">
          <div class="field">
            <label for="mp-name">Name</label>
            <input id="mp-name" type="text" bind:value={form.name} on:input={onFormChange} placeholder="Character name" />
          </div>

          <div class="field">
            <label for="mp-age">Age</label>
            <input id="mp-age" type="number" bind:value={form.age} on:input={onFormChange} placeholder="28" min="1" max="120" />
          </div>

          <div class="field">
            <label for="mp-gender">Gender</label>
            <select id="mp-gender" bind:value={form.gender} on:change={onFormChange}>
              {#each genders as g}
                <option value={g}>{g}</option>
              {/each}
            </select>
          </div>

          <div class="field">
            <label for="mp-skin">Skin Tone</label>
            <input id="mp-skin" type="text" bind:value={form.skin_tone} on:input={onFormChange} placeholder="e.g., warm brown, fair, olive" />
          </div>

          <div class="field">
            <label for="mp-hair-color">Hair Color</label>
            <input id="mp-hair-color" type="text" bind:value={form.hair_color} on:input={onFormChange} placeholder="e.g., black, blonde" />
          </div>

          <div class="field">
            <label for="mp-hair-style">Hair Style</label>
            <input id="mp-hair-style" type="text" bind:value={form.hair_style} on:input={onFormChange} placeholder="e.g., short, long wavy" />
          </div>

          <div class="field">
            <label for="mp-body">Body Type</label>
            <select id="mp-body" bind:value={form.body_type} on:change={onFormChange}>
              {#each bodyTypes as bt}
                <option value={bt}>{bt}</option>
              {/each}
            </select>
          </div>

          <div class="field">
            <label for="mp-style">Art Style</label>
            <select id="mp-style" bind:value={form.art_style} on:change={onFormChange}>
              {#each styles as s}
                <option value={s}>{s}</option>
              {/each}
            </select>
          </div>
        </div>

        <!-- Full-width fields -->
        <div class="field full">
          <label for="mp-features">Physical Features</label>
          <input id="mp-features" type="text" bind:value={form.physical_features} on:input={onFormChange}
            placeholder="e.g., warm brown eyes, short beard, scar on left cheek" />
        </div>

        <div class="field full">
          <label for="mp-clothing">Default Clothing</label>
          <input id="mp-clothing" type="text" bind:value={form.default_clothing} on:input={onFormChange}
            placeholder="e.g., fitted black t-shirt, dark jeans, silver watch" />
        </div>

        <!-- Prompt Preview -->
        <div class="prompt-section">
          <div class="prompt-header">
            <label>SD Prompt</label>
            <button class="link-btn" on:click={() => showPromptEditor = !showPromptEditor}>
              {showPromptEditor ? 'Hide Editor' : 'Edit Manually'}
            </button>
          </div>

          {#if showPromptEditor}
            <textarea 
              bind:value={form.custom_prompt} 
              on:input={onFormChange}
              rows="4" 
              placeholder="Leave empty to auto-generate, or type your own prompt..."
            ></textarea>
            <small>Custom prompt overrides all fields above. Leave empty to auto-generate.</small>
          {/if}

          <div class="prompt-preview">
            <code>{promptPreview || 'Fill in details above to see prompt...'}</code>
          </div>
        </div>

        <!-- Generate Button -->
        <button 
          class="generate-btn" 
          on:click={generatePortraits} 
          disabled={isGenerating || !form.name.trim()}
        >
          {#if isGenerating}
            <span class="spinner"></span> Generating 4 Options...
          {:else}
            🎲 Generate 4 Portrait Options
          {/if}
        </button>
      </div>

      <!-- RIGHT: Gallery Panel -->
      <div class="gallery-panel">
        <h3>Portrait Gallery</h3>

        {#if generationError}
          <div class="error-msg">{generationError}</div>
        {/if}

        {#if isGenerating}
          <div class="loading-state">
            <div class="loading-spinner"></div>
            <p>ComfyUI is generating portraits...</p>
            <small>This may take 30–90 seconds for a batch of 4</small>
          </div>

        {:else if generatedImages.length > 0}
          <div class="gallery-grid">
            {#each generatedImages as img, i}
              <button
                class="gallery-item"
                class:selected={selectedIndex === i}
                on:click={() => selectImage(i)}
              >
                <img src="data:image/png;base64,{img}" alt="Portrait option {i + 1}" />
                <span class="option-label">Option {i + 1}</span>
                {#if selectedIndex === i}
                  <span class="check-badge">✓</span>
                {/if}
              </button>
            {/each}
          </div>

          {#if selectedIndex >= 0}
            <div class="selection-info">
              <span>Selected: <strong>Option {selectedIndex + 1}</strong></span>
              {#if seedUsed !== -1}
                <span class="seed-tag">Seed: {seedUsed}</span>
              {/if}
            </div>
          {:else}
            <p class="hint">Click a portrait to select it as the master reference.</p>
          {/if}

        {:else}
          <!-- Empty state with existing master preview -->
          <div class="empty-state">
            {#if character?.master_image_path}
              <div class="current-master">
                <small>Current Master Image</small>
                {#if character.image}
                  <img src="data:image/png;base64,{character.image}" alt="Current master" />
                {:else}
                  <div class="path-display">{character.master_image_path}</div>
                {/if}
              </div>
            {:else}
              <div class="no-portrait">
                <span class="icon">🖼️</span>
                <p>No master portrait yet</p>
                <small>Fill in details and click Generate to create one</small>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Footer Actions -->
    <div class="actions">
      <button class="cancel-btn" on:click={close}>Cancel</button>

      {#if generatedImages.length > 0}
        <button class="regen-btn" on:click={generatePortraits} disabled={isGenerating}>
          🔄 Regenerate
        </button>
      {/if}

      <button 
        class="save-btn" 
        on:click={saveMasterPortrait}
        disabled={selectedIndex < 0 || isSaving || !character?.id}
      >
        {#if isSaving}
          Saving...
        {:else}
          💾 Save as Master Reference
        {/if}
      </button>
    </div>

  </div>
</div>
{/if}

<style>
  /* ── Modal Shell ── */
  .modal-backdrop {
    position: fixed;
    top: 0; left: 0; width: 100%; height: 100%;
    background: rgba(0, 0, 0, 0.65);
    z-index: 2000;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .modal {
    background: var(--bg-primary, #1a1a2e);
    color: var(--text-primary, #e0e0e0);
    padding: 0;
    border-radius: 12px;
    width: 1050px;
    max-height: 92vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
  }

  /* ── Header ── */
  .header {
    padding: 20px 25px 15px;
    border-bottom: 1px solid var(--border-secondary, #2a2a4a);
  }

  .header h2 {
    margin: 0;
    font-size: 1.3em;
  }

  .subtitle {
    font-size: 0.85em;
    color: var(--text-secondary, #888);
  }

  /* ── Content Layout ── */
  .content-layout {
    display: flex;
    gap: 0;
    min-height: 500px;
  }

  .form-panel {
    flex: 1;
    padding: 20px 25px;
    border-right: 1px solid var(--border-secondary, #2a2a4a);
    overflow-y: auto;
    max-height: 65vh;
  }

  .gallery-panel {
    flex: 1;
    padding: 20px 25px;
    display: flex;
    flex-direction: column;
  }

  h3 {
    margin: 0 0 15px;
    font-size: 1em;
    color: var(--text-secondary, #aaa);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  /* ── Form Grid ── */
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field.full {
    margin-top: 12px;
  }

  .field label {
    font-size: 0.8em;
    font-weight: 600;
    color: var(--text-secondary, #aaa);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  input, select, textarea {
    padding: 8px 10px;
    border: 1px solid var(--border-secondary, #333);
    border-radius: 6px;
    background: var(--bg-secondary, #16213e);
    color: var(--text-primary, #e0e0e0);
    font-family: inherit;
    font-size: 0.9em;
    transition: border-color 0.2s;
  }

  input:focus, select:focus, textarea:focus {
    outline: none;
    border-color: var(--accent, #4a9eff);
  }

  textarea {
    resize: vertical;
  }

  small {
    font-size: 0.8em;
    color: var(--text-secondary, #666);
  }

  /* ── Prompt Section ── */
  .prompt-section {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border-secondary, #2a2a4a);
  }

  .prompt-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .prompt-header label {
    font-size: 0.8em;
    font-weight: 600;
    color: var(--text-secondary, #aaa);
    text-transform: uppercase;
  }

  .link-btn {
    background: none;
    border: none;
    color: var(--accent, #4a9eff);
    cursor: pointer;
    font-size: 0.85em;
    padding: 0;
  }

  .link-btn:hover {
    text-decoration: underline;
  }

  .prompt-preview {
    background: var(--bg-secondary, #0f1729);
    border: 1px solid var(--border-secondary, #2a2a4a);
    border-radius: 6px;
    padding: 10px 12px;
    margin-top: 8px;
    max-height: 80px;
    overflow-y: auto;
  }

  .prompt-preview code {
    font-size: 0.8em;
    color: var(--text-secondary, #888);
    word-break: break-word;
    line-height: 1.4;
  }

  /* ── Generate Button ── */
  .generate-btn {
    width: 100%;
    margin-top: 16px;
    padding: 12px;
    background: linear-gradient(135deg, #4a9eff, #6c5ce7);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1em;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.2s, transform 0.1s;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .generate-btn:hover:not(:disabled) {
    opacity: 0.9;
    transform: translateY(-1px);
  }

  .generate-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ── Gallery ── */
  .gallery-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    flex: 1;
  }

  .gallery-item {
    position: relative;
    border: 3px solid transparent;
    border-radius: 8px;
    overflow: hidden;
    cursor: pointer;
    background: var(--bg-secondary, #16213e);
    padding: 0;
    transition: border-color 0.2s, transform 0.15s;
  }

  .gallery-item:hover {
    border-color: var(--border-secondary, #555);
    transform: scale(1.02);
  }

  .gallery-item.selected {
    border-color: #4a9eff;
    box-shadow: 0 0 12px rgba(74, 158, 255, 0.3);
  }

  .gallery-item img {
    width: 100%;
    height: auto;
    display: block;
  }

  .option-label {
    position: absolute;
    bottom: 6px;
    left: 6px;
    background: rgba(0, 0, 0, 0.7);
    color: white;
    font-size: 0.75em;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .check-badge {
    position: absolute;
    top: 8px;
    right: 8px;
    background: #4a9eff;
    color: white;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.8em;
    font-weight: bold;
  }

  .selection-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 10px;
    padding: 8px 12px;
    background: rgba(74, 158, 255, 0.1);
    border-radius: 6px;
    font-size: 0.9em;
  }

  .seed-tag {
    font-family: monospace;
    font-size: 0.85em;
    color: var(--text-secondary, #888);
  }

  .hint {
    text-align: center;
    color: var(--text-secondary, #888);
    font-size: 0.9em;
    margin-top: 10px;
  }

  /* ── Empty / Loading States ── */
  .empty-state, .loading-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    color: var(--text-secondary, #888);
  }

  .no-portrait .icon {
    font-size: 3em;
    display: block;
    margin-bottom: 10px;
  }

  .current-master img {
    max-width: 200px;
    border-radius: 8px;
    margin-top: 8px;
  }

  .path-display {
    font-family: monospace;
    font-size: 0.8em;
    background: var(--bg-secondary, #16213e);
    padding: 6px 10px;
    border-radius: 4px;
    margin-top: 6px;
    word-break: break-all;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--border-secondary, #333);
    border-top-color: #4a9eff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 15px;
  }

  .spinner {
    display: inline-block;
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-msg {
    background: rgba(255, 71, 87, 0.15);
    color: #ff4757;
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.9em;
    margin-bottom: 10px;
  }

  /* ── Footer Actions ── */
  .actions {
    padding: 15px 25px;
    border-top: 1px solid var(--border-secondary, #2a2a4a);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .cancel-btn {
    padding: 10px 20px;
    background: var(--bg-secondary, #2a2a4a);
    color: var(--text-primary, #ccc);
    border: 1px solid var(--border-secondary, #444);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9em;
  }

  .cancel-btn:hover {
    background: #3a3a5a;
  }

  .regen-btn {
    padding: 10px 20px;
    background: var(--bg-secondary, #2a2a4a);
    color: var(--accent, #4a9eff);
    border: 1px solid var(--accent, #4a9eff);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9em;
  }

  .regen-btn:hover:not(:disabled) {
    background: rgba(74, 158, 255, 0.1);
  }

  .regen-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save-btn {
    padding: 10px 24px;
    background: linear-gradient(135deg, #00b894, #00cec9);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9em;
    font-weight: 600;
    transition: opacity 0.2s;
  }

  .save-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .save-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>