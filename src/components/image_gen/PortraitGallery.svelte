<!-- src/components/image_gen/PortraitGallery.svelte — 2×2 portrait grid + states + Save button -->
<script lang="ts">
  import type { CharacterProfile } from '$lib/types';

  let {
    images = [],
    selectedIndex = -1,
    isGenerating = false,
    generationError = '',
    seedUsed = -1,
    character = null,
    isSaving = false,
    onselect,
    onsave,
  }: {
    images?: string[];
    selectedIndex?: number;
    isGenerating?: boolean;
    generationError?: string;
    seedUsed?: number;
    character?: Partial<CharacterProfile> | null;
    isSaving?: boolean;
    onselect?: (index: number) => void;
    onsave?: () => void;
  } = $props();
</script>

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

  {:else if images.length > 0}
    <div class="gallery-grid">
      {#each images as img, i}
        <button
          class="gallery-item"
          class:selected={selectedIndex === i}
          onclick={() => onselect?.(i)}
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

    <button
      class="save-btn"
      onclick={onsave}
      disabled={selectedIndex < 0 || isSaving || !character?.id}
    >
      {isSaving ? 'Saving...' : '💾 Save as Master Reference'}
    </button>

  {:else}
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

<style>
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

  .save-btn {
    margin-top: 12px;
    padding: 10px 24px;
    background: linear-gradient(135deg, #00b894, #00cec9);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9em;
    font-weight: 600;
    transition: opacity 0.2s;
    width: 100%;
  }

  .save-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .save-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
