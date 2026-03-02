<!-- src/components/image_gen/PortraitForm.svelte — Character detail form + SD prompt preview -->
<script lang="ts">
  const styles    = ['Realistic', 'Anime', '3D', 'Painting', 'Sketch'];
  const bodyTypes = ['Slim', 'Athletic', 'Average', 'Curvy', 'Muscular', 'Heavyset'];
  const genders   = ['Male', 'Female', 'Non-Binary', 'Other'];

  let {
    form,
    promptPreview = '',
    isGenerating = false,
    onchange,
    ongenerate,
  }: {
    form: {
      name: string; age: number | undefined; gender: string; skin_tone: string;
      hair_color: string; hair_style: string; body_type: string; default_clothing: string;
      physical_features: string; art_style: string; custom_prompt: string;
    };
    promptPreview?: string;
    isGenerating?: boolean;
    onchange?: () => void;
    ongenerate?: () => void;
  } = $props();

  let showPromptEditor = $state(false);
</script>

<div class="form-panel">
  <h3>Character Details</h3>

  <div class="form-grid">
    <div class="field">
      <label for="mp-name">Name</label>
      <input id="mp-name" type="text" bind:value={form.name} oninput={onchange} placeholder="Character name" />
    </div>

    <div class="field">
      <label for="mp-age">Age</label>
      <input id="mp-age" type="number" bind:value={form.age} oninput={onchange} placeholder="28" min="1" max="120" />
    </div>

    <div class="field">
      <label for="mp-gender">Gender</label>
      <select id="mp-gender" bind:value={form.gender} onchange={onchange}>
        {#each genders as g}
          <option value={g}>{g}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <label for="mp-skin">Skin Tone</label>
      <input id="mp-skin" type="text" bind:value={form.skin_tone} oninput={onchange} placeholder="e.g., warm brown, fair, olive" />
    </div>

    <div class="field">
      <label for="mp-hair-color">Hair Color</label>
      <input id="mp-hair-color" type="text" bind:value={form.hair_color} oninput={onchange} placeholder="e.g., black, blonde" />
    </div>

    <div class="field">
      <label for="mp-hair-style">Hair Style</label>
      <input id="mp-hair-style" type="text" bind:value={form.hair_style} oninput={onchange} placeholder="e.g., short, long wavy" />
    </div>

    <div class="field">
      <label for="mp-body">Body Type</label>
      <select id="mp-body" bind:value={form.body_type} onchange={onchange}>
        {#each bodyTypes as bt}
          <option value={bt}>{bt}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label for="mp-style">Art Style</label>
      <select id="mp-style" bind:value={form.art_style} onchange={onchange}>
        {#each styles as s}
          <option value={s}>{s}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="field full">
    <label for="mp-features">Physical Features</label>
    <input id="mp-features" type="text" bind:value={form.physical_features} oninput={onchange}
      placeholder="e.g., warm brown eyes, short beard, scar on left cheek" />
  </div>

  <div class="field full">
    <label for="mp-clothing">Default Clothing</label>
    <input id="mp-clothing" type="text" bind:value={form.default_clothing} oninput={onchange}
      placeholder="e.g., fitted black t-shirt, dark jeans, silver watch" />
  </div>

  <div class="prompt-section">
    <div class="prompt-header">
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label>SD Prompt</label>
      <button class="link-btn" onclick={() => (showPromptEditor = !showPromptEditor)}>
        {showPromptEditor ? 'Hide Editor' : 'Edit Manually'}
      </button>
    </div>

    {#if showPromptEditor}
      <textarea
        bind:value={form.custom_prompt}
        oninput={onchange}
        rows="4"
        placeholder="Leave empty to auto-generate, or type your own prompt..."
      ></textarea>
      <small>Custom prompt overrides all fields above. Leave empty to auto-generate.</small>
    {/if}

    <div class="prompt-preview">
      <code>{promptPreview || 'Fill in details above to see prompt...'}</code>
    </div>
  </div>

  <button
    class="generate-btn"
    onclick={ongenerate}
    disabled={isGenerating || !form.name.trim()}
  >
    {#if isGenerating}
      <span class="spinner"></span> Generating 4 Options...
    {:else}
      🎲 Generate 4 Portrait Options
    {/if}
  </button>
</div>

<style>
  .form-panel {
    flex: 1;
    padding: 20px 25px;
    border-right: 1px solid var(--border-secondary, #2a2a4a);
    overflow-y: auto;
    max-height: 65vh;
  }

  h3 {
    margin: 0 0 15px;
    font-size: 1em;
    color: var(--text-secondary, #aaa);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

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

  .spinner {
    display: inline-block;
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
