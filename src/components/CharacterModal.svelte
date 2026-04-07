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
  import { generateMasterPortrait, generateCharacterPortrait, saveMasterPortrait } from '$lib/api/image-gen';
  import { updateCharacter, addCharacter } from '$lib/api/character';
  import { getConfig } from '$lib/api/config';
  import { listCustomCheckpoints, addCustomCheckpoint, scanAvailableCheckpoints } from '$lib/api/custom-assets';
  import type { CharacterProfile, CustomCheckpoint } from '../lib/types';
  import ImageLightbox from './shared/ImageLightbox.svelte';

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
  let lightboxSrc = $state<string | null>(null);
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
    id: 0,
    name: '',
    age: 25,
    gender: 'Female',
    skin_tone: 'Fair',
    hair_style: 'Long, straight',
    hair_color: 'Black',
    eye_color: '',
    body_type: 'Average',
    height_scale: 3,
    weight_scale: 3,
    personality: '',
    additional_notes: '',
    sd_prompt: '',
    image: undefined,
    seed: undefined,
    art_style: 'Realistic',
    content_rating: 'sfw',
  });

  const heightLabels = ['Very Short', 'Short', 'Average', 'Tall', 'Very Tall'];
  const weightLabels = ['Very Slim', 'Slim', 'Average', 'Heavyset', 'Very Heavyset'];

  function getHeightLabel(val: number): string {
    return heightLabels[(val ?? 3) - 1] ?? 'Average';
  }
  function getWeightLabel(val: number): string {
    return weightLabels[(val ?? 3) - 1] ?? 'Average';
  }

  let form = $state<CharacterProfile>(getEmptyForm());

  let customCheckpoints = $state<CustomCheckpoint[]>([]);
  let showAddCheckpoint = $state(false);
  let newCheckpointName = $state('');
  let availableCheckpointFiles = $state<string[]>([]);
  let selectedCheckpointFile = $state('');
  let addingCheckpoint = $state(false);

  // ── Open/close reactivity ──
  let wasShown = $state(false);

  $effect(() => {
    if (show && !wasShown) {
      wasShown = true;
      if (character) {
        form = { ...character };
        if (!form.art_style) form.art_style = 'Realistic';
        if (!form.content_rating) form.content_rating = 'sfw';
      } else {
        form = getEmptyForm();
        // Default new characters to the app's current content rating
        getConfig().then(cfg => { form.content_rating = cfg.content_rating; }).catch(() => {});
        generateSdPrompt();
      }
      // Reset gallery state on open
      generatedImages = [];
      generatedPaths = [];
      selectedIndex = -1;
      generationError = '';
      showPromptEditor = false;
      // Load custom checkpoints
      listCustomCheckpoints().then(cps => { customCheckpoints = cps; }).catch(() => {});
    } else if (!show && wasShown) {
      wasShown = false;
    }
  });

  $effect(() => {
    if (form.art_style === '__add_checkpoint__') {
      form.art_style = 'Realistic';
      openAddCheckpoint();
    }
  });

  // ── Custom checkpoint helpers ──
  function isCustomCheckpoint(style: string): boolean {
    return style.startsWith('custom:');
  }

  function getCheckpointFilename(style: string): string | null {
    if (!isCustomCheckpoint(style)) return null;
    const cp = customCheckpoints.find(c => `custom:${c.display_name}` === style);
    return cp?.filename ?? null;
  }

  async function openAddCheckpoint() {
    showAddCheckpoint = true;
    newCheckpointName = '';
    selectedCheckpointFile = '';
    try {
      availableCheckpointFiles = await scanAvailableCheckpoints();
      const registered = new Set(customCheckpoints.map(c => c.filename));
      availableCheckpointFiles = availableCheckpointFiles.filter(f => !registered.has(f));
    } catch {
      availableCheckpointFiles = [];
    }
  }

  async function confirmAddCheckpoint() {
    if (!newCheckpointName.trim() || !selectedCheckpointFile) return;
    addingCheckpoint = true;
    try {
      const cp = await addCustomCheckpoint(newCheckpointName.trim(), selectedCheckpointFile);
      customCheckpoints = [...customCheckpoints, cp];
      form.art_style = `custom:${cp.display_name}`;
      showAddCheckpoint = false;
      generateSdPrompt();
    } catch (e) {
      console.error('Failed to add checkpoint:', e);
    }
    addingCheckpoint = false;
  }

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

    if (form.eye_color) {
      parts.push(`${form.eye_color.toLowerCase()} eyes`);
    }

    // Height from slider
    const heightDesc = ['petite, short stature', 'short stature', '', 'tall', 'very tall, imposing stature'][(form.height_scale ?? 3) - 1];
    if (heightDesc) parts.push(heightDesc);

    // Build/Weight from slider
    const weightDesc = ['very slim, thin body', 'slim body', '', 'heavyset, large body', 'very heavyset, large body'][(form.weight_scale ?? 3) - 1];
    if (weightDesc) parts.push(weightDesc);

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

    parts.push('detailed face, looking at viewer');
    parts.push('neutral gray background');
    parts.push('soft studio lighting, rim lighting');

    form.sd_prompt = parts.join(', ');
  }

  // ── Generate portrait(s) — tries ComfyUI first, falls back to SD ──
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
      const checkpointOverride = isCustomCheckpoint(form.art_style ?? '')
        ? getCheckpointFilename(form.art_style ?? '')
        : null;
      const result = await generateMasterPortrait({
        name: form.name,
        age: form.age || null,
        gender: form.gender || null,
        skin_tone: form.skin_tone || null,
        hair_color: form.hair_color || null,
        hair_style: form.hair_style || null,
        eye_color: form.eye_color || null,
        body_type: form.body_type || null,
        height_scale: form.height_scale ?? 3,
        weight_scale: form.weight_scale ?? 3,
        physical_features: form.additional_notes || null,
        art_style: form.art_style || null,
        custom_prompt: showPromptEditor ? form.sd_prompt : null,
        seed: null,
        checkpoint_override: checkpointOverride,
      });

      generatedImages = result.images_base64;
      generatedPaths = result.image_paths;
      form.sd_prompt = result.prompt_used;
      form.seed = result.seed !== -1 ? result.seed : undefined;
    } catch (e) {
      generationError = `Image generation failed.\n\nMake sure ComfyUI is running (check http://127.0.0.1:8188).\n\nDetails: ${e}`;
    } finally {
      isGenerating = false;
    }
  }

  // ── Single-image generation via SD WebUI API ──
  async function fallbackSingleGenerate() {
    try {
      const [base64, seedStr] = (await generateCharacterPortrait(form.sd_prompt, form.art_style) as unknown) as [string, string];

      generatedImages = [base64];
      generatedPaths = [];
      selectedIndex = 0;
      form.image = base64;
      form.seed = parseInt(seedStr);
      generationError = '';
    } catch (fallbackErr) {
      generationError =
        `SD WebUI generation failed.\n\n` +
        `Make sure SD WebUI is fully loaded (check http://127.0.0.1:7860).\n\n` +
        `Details: ${fallbackErr}`;
    }
  }

  // ── Gallery selection ──
  function selectImage(index: number) {
    selectedIndex = index;
    form.image = generatedImages[index];
  }

  // ── Save: persist character + optionally save master image ──
  // ── Save: persist character + optionally save master image ──
    async function save() {
      if (!form.sd_prompt) generateSdPrompt();
  
      try {
        let characterId: number;
  
        if (form.id && form.id > 0) {
          // Existing character — update in place
          await updateCharacter(form);
          characterId = form.id;
        } else {
          // New character — insert and get real ID back
          characterId = await addCharacter({ ...form, id: 0 });
          form.id = characterId;
        }

        // If user selected a ComfyUI image, save it as the master reference
        if (selectedIndex >= 0 && generatedPaths.length > 0 && characterId > 0) {
          try {
            const masterPath = await saveMasterPortrait({
              character_id: characterId,
              selected_index: selectedIndex,
              image_paths: generatedPaths,
              character_name: form.name,
            });
            form.master_image_path = masterPath;
            console.log('[CharacterModal] Master portrait saved:', masterPath);
          } catch (e) {
            console.warn('[CharacterModal] Could not save master image:', e);
            // Non-fatal — character is saved, just without master reference
          }
        }
  
        onsave?.(form);
      } catch (e) {
        generationError = `Failed to save character: ${e}`;
        console.error('[CharacterModal] Save failed:', e);
      }
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
            <label for="char-eyes">Eye Color</label>
            <input id="char-eyes" type="text" bind:value={form.eye_color} oninput={generateSdPrompt}
              placeholder="e.g., Blue, Hazel, Dark brown" />
          </div>
          <div class="group">
            <label for="char-body">Body Type</label>
            <select id="char-body" bind:value={form.body_type} onchange={generateSdPrompt}>
              {#each bodyTypes as bt}
                <option>{bt}</option>
              {/each}
            </select>
          </div>
        </div>

        <!-- Height & Weight sliders -->
        <div class="sliders-section">
          <div class="slider-group">
            <label>Height: <strong>{getHeightLabel(form.height_scale ?? 3)}</strong></label>
            <input type="range" min="1" max="5" step="1"
              bind:value={form.height_scale} oninput={generateSdPrompt} class="slider" />
            <div class="slider-labels">
              <span>Very Short</span>
              <span>Average</span>
              <span>Very Tall</span>
            </div>
          </div>
          <div class="slider-group">
            <label>Build: <strong>{getWeightLabel(form.weight_scale ?? 3)}</strong></label>
            <input type="range" min="1" max="5" step="1"
              bind:value={form.weight_scale} oninput={generateSdPrompt} class="slider" />
            <div class="slider-labels">
              <span>Very Slim</span>
              <span>Average</span>
              <span>Very Heavy</span>
            </div>
          </div>
        </div>

        <!-- Personality / backstory -->
        <div class="group full">
          <label for="char-personality">Personality & Details</label>
          <textarea
            id="char-personality"
            bind:value={form.personality}
            rows="3"
            placeholder="Describe their personality, backstory, quirks, motivations..."
          ></textarea>
          <small>Sent to the AI to shape how the character behaves in the story.</small>
        </div>

        <!-- Physical features / additional notes -->
        <div class="group full">
          <label for="char-features">Physical Features</label>
          <input id="char-features" type="text" bind:value={form.additional_notes} oninput={generateSdPrompt}
            placeholder="e.g., short beard, scar on left cheek, freckles" />
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
          {#if customCheckpoints.length > 0}
            <option disabled>──────────</option>
            {#each customCheckpoints as cp}
              <option value="custom:{cp.display_name}">{cp.display_name}</option>
            {/each}
          {/if}
          <option disabled>──────────</option>
          <option value="__add_checkpoint__">+ Add Checkpoint…</option>
        </select>

        {#if showAddCheckpoint}
          <div class="add-checkpoint-form">
            {#if availableCheckpointFiles.length === 0}
              <small class="add-cp-hint">No new .safetensors files found in ComfyUI/models/checkpoints/. Place your checkpoint file there first, then try again.</small>
              <button class="link-btn" onclick={() => showAddCheckpoint = false}>Cancel</button>
            {:else}
              <div class="add-cp-field">
                <label>Display Name</label>
                <input type="text" bind:value={newCheckpointName} placeholder="e.g. DreamShaper XL" />
              </div>
              <div class="add-cp-field">
                <label>Checkpoint File</label>
                <select bind:value={selectedCheckpointFile}>
                  <option value="">Select a file…</option>
                  {#each availableCheckpointFiles as f}
                    <option value={f}>{f}</option>
                  {/each}
                </select>
              </div>
              <div class="add-cp-actions">
                <button class="link-btn" onclick={() => showAddCheckpoint = false}>Cancel</button>
                <button class="accent-btn" onclick={confirmAddCheckpoint} disabled={addingCheckpoint || !newCheckpointName.trim() || !selectedCheckpointFile}>
                  {addingCheckpoint ? 'Adding…' : 'Add'}
                </button>
              </div>
            {/if}
          </div>
        {/if}

        <!-- Content Rating toggle -->
        <div class="rating-row">
          <label class="style-label">Content Rating</label>
          <div class="rating-toggle">
            <button
              class="rating-btn"
              class:active={form.content_rating === 'sfw'}
              onclick={() => form.content_rating = 'sfw'}
            >SFW</button>
            <button
              class="rating-btn nsfw-btn"
              class:active={form.content_rating === 'nsfw'}
              onclick={() => form.content_rating = 'nsfw'}
            >NSFW</button>
          </div>
        </div>

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
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  class="expand-btn"
                  onclick={(e) => { e.stopPropagation(); lightboxSrc = `data:image/png;base64,${img}`; }}
                  onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); lightboxSrc = `data:image/png;base64,${img}`; } }}
                  role="button"
                  tabindex="0"
                  title="View fullscreen"
                >⛶</span>
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
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
          <div class="image-preview" onclick={() => lightboxSrc = `data:image/png;base64,${generatedImages[0]}`} role="button" tabindex="0">
            <img src="data:image/png;base64,{generatedImages[0]}" alt="Character Portrait" />
          </div>

        {:else if form.image}
          <!-- Existing image from DB -->
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
          <div class="image-preview" onclick={() => lightboxSrc = `data:image/png;base64,${form.image}`} role="button" tabindex="0">
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
            🔄 Regenerate Portrait
          {:else}
            🎲 Generate Portrait
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

{#if lightboxSrc}
  <ImageLightbox
    src={lightboxSrc}
    alt="Character Portrait"
    onclose={() => lightboxSrc = null}
  />
{/if}
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

  /* ── Sliders ── */
  .sliders-section {
    display: flex;
    gap: 20px;
    margin-top: 10px;
  }
  .slider-group {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .slider-group label {
    font-size: 0.85em;
    font-weight: bold;
    color: var(--text-secondary, #666);
  }
  .slider-group label strong {
    color: var(--accent, #4a9eff);
  }
  .slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: var(--bg-tertiary, #21262d);
    outline: none;
    padding: 0;
    border: none;
  }
  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent, #4a9eff);
    cursor: pointer;
    border: 2px solid var(--bg-primary, #0d1117);
  }
  .slider-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.7em;
    color: var(--text-muted, #6e7681);
  }

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

  .rating-row { display: flex; flex-direction: column; gap: 4px; }
  .rating-toggle { display: flex; gap: 0; border-radius: 6px; overflow: hidden; border: 1px solid var(--border-secondary, #ccc); }
  .rating-btn {
    flex: 1; padding: 6px 0; border: none; cursor: pointer;
    font-size: 0.8em; font-weight: 700; letter-spacing: 0.04em;
    background: var(--bg-secondary, #f0f0f0); color: var(--text-secondary, #888);
    transition: background 0.15s, color 0.15s;
  }
  .rating-btn.active { background: rgba(35, 134, 54, 0.2); color: #3fb950; }
  .nsfw-btn.active { background: rgba(248, 81, 73, 0.2); color: #f85149; }

  .image-preview {
    cursor: pointer;
  }
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

  .expand-btn {
    position: absolute;
    top: 4px;
    right: 4px;
    background: rgba(0, 0, 0, 0.6);
    border: none;
    color: white;
    font-size: 0.9em;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
    z-index: 2;
  }
  .gallery-item:hover .expand-btn { opacity: 1; }
  .expand-btn:hover { background: rgba(0, 0, 0, 0.8); }
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

  .add-checkpoint-form {
    margin-top: 8px;
    padding: 10px;
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-secondary, #30363d);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .add-cp-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .add-cp-field label {
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .add-cp-field input,
  .add-cp-field select {
    padding: 6px 8px;
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-secondary, #30363d);
    border-radius: 6px;
    color: var(--text-primary, #c9d1d9);
    font-size: 0.85rem;
  }

  .add-cp-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 4px;
  }

  .accent-btn {
    padding: 5px 14px;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .accent-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .add-cp-hint {
    color: var(--text-muted, #8b949e);
    font-size: 0.78rem;
  }
</style>