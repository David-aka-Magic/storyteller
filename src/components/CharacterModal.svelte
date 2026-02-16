<!-- src/components/CharacterModal.svelte -->
<!--
  Character Creation / Edit Modal with Master Portrait Generator
  ================================================================
  Svelte 5 — uses callback props instead of createEventDispatcher.
  Types match src/lib/types.ts CharacterProfile (id: string, required fields).

  Flow:
    1. Fill in character details (left panel)
    2. Click "Generate 4 Options" → ComfyUI batch generation
    3. Gallery shows 4 options → click to select the best
    4. Save → character data + selected master image saved to DB

  Props:
    - show: boolean
    - character: CharacterProfile | null
    - onsave: (form: CharacterProfile) => void
    - onclose: () => void
-->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { CharacterProfile } from '../lib/types';

  // ── Svelte 5 props ──
  let {
    show = false,
    character = null,
    onsave,
    onclose,
  }: {
    show?: boolean;
    character?: CharacterProfile | null;
    onsave?: (form: CharacterProfile) => void;
    onclose?: () => void;
  } = $props();

  // ── Generation state ──
  let isGenerating = $state(false);
  let generatedImages = $state<string[]>([]);       // base64 batch from ComfyUI
  let generatedPaths = $state<string[]>([]);         // file paths on disk
  let selectedIndex = $state(-1);                    // which of the 4 the user picked
  let generationError = $state('');
  let showPromptEditor = $state(false);

  // ── Dropdown options ──
  const styles = ['Realistic', 'Anime', '3D', 'Painting', 'Sketch'];
  const bodyTypes = ['Slim', 'Athletic', 'Average', 'Curvy', 'Muscular', 'Heavyset'];
  const genders = ['Male', 'Female', 'Non-Binary', 'Other'];

  // ── Form (matches src/lib/types.ts CharacterProfile) ──
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
    art_style: 'Realistic',
  });

  let form = $state<CharacterProfile>(getEmptyForm());

  // ── Open/close reactivity ──
  let wasShown = $state(false);

  $effect(() => {
    if (show && !wasShown) {
      wasShown = true;
      if (character) {
        form = { ...character };
        if (!form.art_style) form.art_style = 'Realistic';
      } else {
        form = getEmptyForm();
        generateSdPrompt();
      }
      // Reset gallery state on open
      generatedImages = [];
      generatedPaths = [];
      selectedIndex = -1;
      generationError = '';
      showPromptEditor = false;
    } else if (!show && wasShown) {
      wasShown = false;
    }
  });

  // ── Prompt builder (matches master_portrait.rs logic) ──
  function generateSdPrompt() {
    const parts: string[] = [];

    parts.push('(masterpiece, best quality)');

    const genderTag =
      form.gender === 'Female' ? '1girl' :
      form.gender === 'Male' ? '1boy' :
      '1person';
    parts.push(`solo, ${genderTag}`);

    parts.push('portrait, upper body, looking at viewer');

    if (form.age) parts.push(`${form.age} years old`);

    if (form.skin_tone) parts.push(`${form.skin_tone.toLowerCase()} skin`);

    if (form.hair_color && form.hair_style) {
      const hairStyle = form.hair_style.toLowerCase().replace(',', '');
      parts.push(`${form.hair_color.toLowerCase()} ${hairStyle} hair`);
    } else if (form.hair_color) {
      parts.push(`${form.hair_color.toLowerCase()} hair`);
    }

    if (form.body_type) {
      const bodyMap: Record<string, string> = {
        'Slim': 'slim body',
        'Athletic': 'athletic body, fit',
        'Average': 'normal body',
        'Curvy': 'curvy body',
        'Muscular': 'muscular body',
        'Heavyset': 'large body',
      };
      parts.push(bodyMap[form.body_type] || `${form.body_type.toLowerCase()} body`);
    }

    // additional_notes as physical features
    if (form.additional_notes) {
      for (const feat of form.additional_notes.split(',')) {
        const trimmed = feat.trim();
        if (trimmed) parts.push(trimmed.toLowerCase());
      }
    }

    if (form.personality) {
      parts.push(`${form.personality.toLowerCase()} expression`);
    }

    parts.push('detailed face, looking at viewer');
    parts.push('neutral gray background');
    parts.push('soft studio lighting, rim lighting');

    form.sd_prompt = parts.join(', ');
  }

  // ── Generate 4 portrait options via ComfyUI ──
  async function generatePortraitBatch() {
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

    if (!form.sd_prompt) generateSdPrompt();

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
          physical_features: form.additional_notes || null,
          art_style: form.art_style || null,
          custom_prompt: showPromptEditor ? form.sd_prompt : null,
          seed: null,
        },
      });

      generatedImages = result.images_base64;
      generatedPaths = result.image_paths;
      form.sd_prompt = result.prompt_used;
      form.seed = result.seed !== -1 ? result.seed : undefined;
    } catch (e) {
      console.warn('[CharacterModal] ComfyUI failed, trying SD fallback:', e);
      // Fallback to existing SD API single-image generation
      await fallbackSingleGenerate();
    } finally {
      isGenerating = false;
    }
  }

  // ── Fallback to existing single-image SD API ──
  async function fallbackSingleGenerate() {
    try {
      const [base64, seedStr] = (await invoke('generate_character_portrait', {
        prompt: form.sd_prompt,
        style: form.art_style,
      })) as [string, string];

      generatedImages = [base64];
      generatedPaths = [];
      selectedIndex = 0;
      form.image = base64;
      form.seed = parseInt(seedStr);
      generationError = '';
    } catch (fallbackErr) {
      generationError = `Both ComfyUI and SD API failed. Is either running?\n${fallbackErr}`;
    }
  }

  // ── Gallery selection ──
  function selectImage(index: number) {
    selectedIndex = index;
    form.image = generatedImages[index];
  }

  // ── Save: persist character + optionally save master image ──
  async function save() {
    if (!form.sd_prompt) generateSdPrompt();

    // If user selected a ComfyUI image, try to save as master reference
    // (uses numeric character ID from backend — the string ID here is
    //  converted by the backend on insert, so this is best-effort)
    if (selectedIndex >= 0 && generatedPaths.length > 0) {
      try {
        const numericId = parseInt(form.id);
        if (!isNaN(numericId) && numericId > 0) {
          await invoke<string>('save_master_portrait', {
            request: {
              character_id: numericId,
              selected_index: selectedIndex,
              image_paths: generatedPaths,
              character_name: form.name,
            },
          });
        }
      } catch (e) {
        console.warn('[CharacterModal] Could not save master image:', e);
        // Non-fatal — character still saves with inline base64 image
      }
    }

    onsave?.(form);
  }

  function close() {
    onclose?.();
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
  onclick={handleBackdropClick}
  onkeydown={handleKeydown}
  role="button"
  tabindex="0"
>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={() => {}} role="document" tabindex="-1">
    <h2>{character ? 'Edit Character' : 'Create New Character'}</h2>

    <div class="split-layout">
      <!-- ════════ LEFT: Character Details ════════ -->
      <div class="inputs-column">
        <div class="form-grid">
          <div class="group">
            <label for="char-name">Name</label>
            <input id="char-name" type="text" bind:value={form.name} placeholder="Name" />
          </div>
          <div class="group">
            <label for="char-age">Age</label>
            <input id="char-age" type="number" bind:value={form.age} oninput={generateSdPrompt} />
          </div>
          <div class="group">
            <label for="char-gender">Gender</label>
            <select id="char-gender" bind:value={form.gender} onchange={generateSdPrompt}>
              {#each genders as g}
                <option>{g}</option>
              {/each}
            </select>
          </div>
          <div class="group">
            <label for="char-skin">Skin Tone</label>
            <input id="char-skin" type="text" bind:value={form.skin_tone} oninput={generateSdPrompt} />
          </div>
          <div class="group">
            <label for="char-haircolor">Hair Color</label>
            <input id="char-haircolor" type="text" bind:value={form.hair_color} oninput={generateSdPrompt} />
          </div>
          <div class="group">
            <label for="char-hairstyle">Hair Style</label>
            <input id="char-hairstyle" type="text" bind:value={form.hair_style} oninput={generateSdPrompt} />
          </div>
          <div class="group">
            <label for="char-body">Body Type</label>
            <select id="char-body" bind:value={form.body_type} onchange={generateSdPrompt}>
              {#each bodyTypes as bt}
                <option>{bt}</option>
              {/each}
            </select>
          </div>
          <div class="group">
            <label for="char-personality">Vibe</label>
            <input id="char-personality" type="text" bind:value={form.personality} oninput={generateSdPrompt} />
          </div>
        </div>

        <!-- Physical features / additional notes -->
        <div class="group full">
          <label for="char-features">Physical Features</label>
          <input id="char-features" type="text" bind:value={form.additional_notes} oninput={generateSdPrompt}
            placeholder="e.g., warm brown eyes, short beard, scar on left cheek" />
        </div>

        <!-- Prompt preview -->
        <div class="group full prompt-section">
          <div class="prompt-header">
            <label for="char-prompt">SD Prompt</label>
            <button class="link-btn" onclick={() => { showPromptEditor = !showPromptEditor; }}>
              {showPromptEditor ? 'Auto-generate' : 'Edit manually'}
            </button>
          </div>

          {#if showPromptEditor}
            <textarea id="char-prompt" bind:value={form.sd_prompt} rows="4"></textarea>
            <small>Custom prompt overrides auto-generation.</small>
          {:else}
            <div class="prompt-preview">
              <code>{form.sd_prompt || 'Fill in details above...'}</code>
            </div>
            <small>Auto-generated from the fields above.</small>
          {/if}
        </div>
      </div>

      <!-- ════════ RIGHT: Portrait Gallery ════════ -->
      <div class="portrait-column">

        <!-- Art Style -->
        <label class="style-label">Art Style</label>
        <select class="style-dropdown" bind:value={form.art_style} onchange={generateSdPrompt}>
          {#each styles as style}
            <option value={style}>{style}</option>
          {/each}
        </select>

        <!-- Error banner -->
        {#if generationError}
          <div class="error-msg">{generationError}</div>
        {/if}

        <!-- Gallery / Image Display -->
        {#if isGenerating}
          <div class="loading-state">
            <div class="loading-spinner"></div>
            <p>Generating portraits via ComfyUI...</p>
            <small>Batch of 4 — may take 30–90 seconds</small>
          </div>

        {:else if generatedImages.length > 1}
          <!-- Batch gallery: 2×2 grid -->
          <div class="gallery-grid">
            {#each generatedImages as img, i}
              <button
                class="gallery-item"
                class:selected={selectedIndex === i}
                onclick={() => selectImage(i)}
              >
                <img src="data:image/png;base64,{img}" alt="Option {i + 1}" />
                <span class="option-label">Option {i + 1}</span>
                {#if selectedIndex === i}
                  <span class="check-badge">✓</span>
                {/if}
              </button>
            {/each}
          </div>

          {#if selectedIndex >= 0}
            <div class="selection-info">
              Selected: <strong>Option {selectedIndex + 1}</strong>
            </div>
          {:else}
            <p class="hint">Click a portrait to select it.</p>
          {/if}

        {:else if generatedImages.length === 1}
          <!-- Single image (fallback SD API) -->
          <div class="image-preview">
            <img src="data:image/png;base64,{generatedImages[0]}" alt="Character Portrait" />
          </div>

        {:else if form.image}
          <!-- Existing image from DB -->
          <div class="image-preview">
            <img src="data:image/png;base64,{form.image}" alt="Character Portrait" />
          </div>

        {:else}
          <!-- Empty state -->
          <div class="placeholder">
            <span>No Portrait</span>
            <small>Click Generate below</small>
          </div>
        {/if}

        <!-- Generate button -->
        <button class="gen-btn" onclick={generatePortraitBatch} disabled={isGenerating}>
          {#if isGenerating}
            Generating...
          {:else if generatedImages.length > 0}
            🔄 Regenerate 4 Options
          {:else}
            🎲 Generate 4 Portrait Options
          {/if}
        </button>

        {#if form.seed}
          <div class="seed-info">
            DNA Seed: <code>{form.seed}</code>
          </div>
        {/if}
      </div>
    </div>

    <!-- Footer Actions -->
    <div class="actions">
      <button onclick={close} class="cancel">Cancel</button>
      <button onclick={save} class="save">Save Character</button>
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
    width: 960px;
    max-height: 92vh;
    overflow-y: auto;
    box-shadow: 0 10px 25px var(--shadow, rgba(0,0,0,0.5));
  }
  h2 {
    margin-top: 0;
    border-bottom: 1px solid var(--border-secondary, #eee);
    padding-bottom: 10px;
  }

  /* ── Layout ── */
  .split-layout { display: flex; gap: 20px; }
  .inputs-column { flex: 1; }
  .portrait-column {
    width: 340px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* ── Form ── */
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .group { display: flex; flex-direction: column; gap: 4px; }
  .full { width: 100%; margin-top: 10px; grid-column: 1 / -1; }

  label {
    font-size: 0.85em;
    font-weight: bold;
    color: var(--text-secondary, #666);
  }
  input, select, textarea {
    padding: 8px 10px;
    border: 1px solid var(--border-secondary, #ccc);
    border-radius: 6px;
    background: var(--bg-secondary, white);
    color: var(--text-primary, #333);
    font-family: inherit;
    font-size: 0.9em;
  }
  input:focus, select:focus, textarea:focus {
    outline: none;
    border-color: var(--accent, #4a9eff);
  }
  textarea { resize: vertical; }
  small { color: var(--text-secondary, #888); font-size: 0.8em; }

  /* ── Prompt section ── */
  .prompt-section { border-top: 1px solid var(--border-secondary, #eee); padding-top: 10px; }
  .prompt-header { display: flex; justify-content: space-between; align-items: center; }
  .link-btn {
    background: none; border: none;
    color: var(--accent, #4a9eff);
    cursor: pointer; font-size: 0.85em; padding: 0;
  }
  .link-btn:hover { text-decoration: underline; }
  .prompt-preview {
    background: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-secondary, #ddd);
    border-radius: 6px; padding: 8px 10px;
    max-height: 70px; overflow-y: auto; margin-top: 4px;
  }
  .prompt-preview code {
    font-size: 0.8em;
    color: var(--text-secondary, #666);
    word-break: break-word; line-height: 1.4;
  }

  /* ── Portrait column ── */
  .style-label { font-size: 0.85em; font-weight: bold; color: var(--text-secondary, #666); }
  .style-dropdown { width: 100%; }

  .image-preview img {
    width: 100%; border-radius: 8px;
  }
  .placeholder {
    width: 100%; height: 220px;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    background: var(--bg-secondary, #f0f0f0);
    border-radius: 8px;
    color: var(--text-secondary, #999);
    gap: 4px;
  }

  /* ── Gallery grid ── */
  .gallery-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .gallery-item {
    position: relative;
    border: 3px solid transparent;
    border-radius: 6px;
    overflow: hidden;
    cursor: pointer;
    background: var(--bg-secondary, #f0f0f0);
    padding: 0;
    transition: border-color 0.2s, transform 0.15s;
  }
  .gallery-item:hover {
    border-color: var(--border-secondary, #999);
    transform: scale(1.02);
  }
  .gallery-item.selected {
    border-color: var(--accent, #4a9eff);
    box-shadow: 0 0 8px rgba(74, 158, 255, 0.3);
  }
  .gallery-item img { width: 100%; height: auto; display: block; }
  .option-label {
    position: absolute; bottom: 4px; left: 4px;
    background: rgba(0,0,0,0.7); color: white;
    font-size: 0.7em; padding: 2px 6px; border-radius: 3px;
  }
  .check-badge {
    position: absolute; top: 4px; right: 4px;
    background: var(--accent, #4a9eff); color: white;
    width: 20px; height: 20px; border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    font-size: 0.75em; font-weight: bold;
  }
  .selection-info {
    display: flex; justify-content: space-between; align-items: center;
    padding: 6px 10px;
    background: rgba(74, 158, 255, 0.1);
    border-radius: 6px; font-size: 0.85em;
  }
  .hint { text-align: center; color: var(--text-secondary, #888); font-size: 0.85em; margin: 4px 0; }

  /* ── Loading / Error ── */
  .loading-state {
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    padding: 40px 0;
    color: var(--text-secondary, #888);
    text-align: center;
  }
  .loading-spinner {
    width: 36px; height: 36px;
    border: 3px solid var(--border-secondary, #ddd);
    border-top-color: var(--accent, #4a9eff);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 12px;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .error-msg {
    background: rgba(255, 71, 87, 0.12);
    color: #ff4757; padding: 8px 12px;
    border-radius: 6px; font-size: 0.85em;
    white-space: pre-wrap;
  }

  /* ── Buttons ── */
  .gen-btn {
    width: 100%;
    padding: 10px;
    background: linear-gradient(135deg, #4a9eff, #6c5ce7);
    color: white; border: none; border-radius: 8px;
    font-size: 0.95em; font-weight: 600;
    cursor: pointer;
    transition: opacity 0.2s;
  }
  .gen-btn:hover:not(:disabled) { opacity: 0.9; }
  .gen-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .seed-info {
    font-size: 0.8em;
    color: var(--text-secondary, #888);
    text-align: center;
  }
  .seed-info code {
    background: var(--bg-secondary, #f0f0f0);
    padding: 2px 6px; border-radius: 3px;
  }

  /* ── Footer ── */
  .actions {
    margin-top: 15px;
    display: flex; justify-content: flex-end; gap: 10px;
    border-top: 1px solid var(--border-secondary, #eee);
    padding-top: 15px;
  }
  .cancel {
    background: #6c757d; color: white;
    padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer;
  }
  .save {
    background: #4a9eff; color: white;
    padding: 10px 20px; border: none; border-radius: 4px;
    cursor: pointer; font-weight: bold;
  }
  .save:hover { background: #357abd; }
  .cancel:hover { background: #5a6268; }
</style>