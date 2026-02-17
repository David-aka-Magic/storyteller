<!-- src/components/story/NewStoryModal.svelte -->
<!--
  New Story creation modal with a streamlined wizard flow:
    Step 1: Title & Description (premise/setting)
    Step 2: (Optional) Link existing characters or note to create later
    Step 3: Review & Start

  Dispatches:
    'create' → { title, description, characterIds }
    'close'  → dismiss modal
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { CharacterProfile } from '../../lib/types';

  export let show: boolean = false;
  /** All available characters the user can pick from */
  export let availableCharacters: CharacterProfile[] = [];

  const dispatch = createEventDispatcher();

  // ── Wizard State ──
  let step = 1;
  const TOTAL_STEPS = 3;

  // ── Form Data ──
  let title = '';
  let description = '';
  let selectedCharIds: Set<string> = new Set();

  // ── Validation ──
  $: titleValid = title.trim().length >= 2;
  $: descValid = description.trim().length >= 10;
  $: step1Valid = titleValid && descValid;

  // ── Reset on show ──
  $: if (show) {
    step = 1;
    title = '';
    description = '';
    selectedCharIds = new Set();
  }

  function nextStep() {
    if (step < TOTAL_STEPS) step++;
  }

  function prevStep() {
    if (step > 1) step--;
  }

  function toggleCharacter(id: string) {
    if (selectedCharIds.has(id)) {
      selectedCharIds.delete(id);
    } else {
      selectedCharIds.add(id);
    }
    selectedCharIds = selectedCharIds; // trigger reactivity
  }

  function handleCreate() {
    if (!step1Valid) return;
    dispatch('create', {
      title: title.trim(),
      description: description.trim(),
      characterIds: Array.from(selectedCharIds).map(id => parseInt(id, 10)).filter(n => !isNaN(n)),
    });
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

  // ── Genre / Setting Presets ──
  const presets = [
    { label: '⚔️ Fantasy', desc: 'A medieval fantasy world with magic, kingdoms, and ancient prophecies.' },
    { label: '🚀 Sci-Fi', desc: 'A futuristic setting with space travel, advanced technology, and alien encounters.' },
    { label: '🔍 Mystery', desc: 'A noir detective story with hidden clues, suspects, and a crime to solve.' },
    { label: '🧟 Horror', desc: 'A dark, unsettling world where unknown terrors lurk in every shadow.' },
    { label: '💕 Romance', desc: 'A story of connection and relationships in a charming, everyday setting.' },
    { label: '🏴‍☠️ Adventure', desc: 'A globe-trotting adventure with treasure, danger, and unexpected allies.' },
  ];

  function applyPreset(preset: { label: string; desc: string }) {
    description = preset.desc;
    if (!title.trim()) {
      title = 'Untitled ' + preset.label.slice(3); // strip emoji + space
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
  <div class="modal" on:click|stopPropagation role="dialog" tabindex="-1">

    <!-- Header -->
    <div class="modal-header">
      <div class="header-left">
        <h2>✨ New Story</h2>
        <div class="step-indicator">
          {#each Array(TOTAL_STEPS) as _, i}
            <div
              class="step-dot"
              class:active={step === i + 1}
              class:completed={step > i + 1}
            ></div>
          {/each}
        </div>
      </div>
      <button class="close-btn" on:click={close}>✕</button>
    </div>

    <!-- Body -->
    <div class="modal-body">

      <!-- ═══ Step 1: Premise ═══ -->
      {#if step === 1}
        <div class="step-content" >
          <div class="step-title">Story Premise</div>
          <p class="step-subtitle">Give your story a name and describe the world it takes place in.</p>

          <div class="form-group">
            <label for="story-title">Title</label>
            <input
              id="story-title"
              type="text"
              bind:value={title}
              placeholder="The Lost Kingdom"
              maxlength="80"
              class:invalid={title.length > 0 && !titleValid}
            />
            {#if title.length > 0 && !titleValid}
              <span class="field-hint error">At least 2 characters</span>
            {/if}
          </div>

          <div class="form-group">
            <label for="story-desc">Description / Setting</label>
            <textarea
              id="story-desc"
              bind:value={description}
              placeholder="Describe the world, era, genre, and any important context for the story..."
              rows="4"
              maxlength="1000"
              class:invalid={description.length > 0 && !descValid}
            ></textarea>
            <div class="textarea-footer">
              {#if description.length > 0 && !descValid}
                <span class="field-hint error">At least 10 characters</span>
              {:else}
                <span class="field-hint"></span>
              {/if}
              <span class="char-counter">{description.length}/1000</span>
            </div>
          </div>

          <!-- Quick Presets -->
          <div class="presets-section">
            <span class="presets-label">Quick start</span>
            <div class="presets-grid">
              {#each presets as preset}
                <button
                  class="preset-btn"
                  on:click={() => applyPreset(preset)}
                  title={preset.desc}
                >
                  {preset.label}
                </button>
              {/each}
            </div>
          </div>
        </div>
      {/if}

      <!-- ═══ Step 2: Characters ═══ -->
      {#if step === 2}
        <div class="step-content">
          <div class="step-title">Characters</div>
          <p class="step-subtitle">
            Select existing characters to include, or skip this — you can add characters later.
          </p>

          {#if availableCharacters.length === 0}
            <div class="no-chars-notice">
              <span class="notice-icon">👤</span>
              <span>No characters created yet. You can add them after starting the story.</span>
            </div>
          {:else}
            <div class="character-picker">
              {#each availableCharacters as char (char.id)}
                <button
                  class="char-pick-card"
                  class:selected={selectedCharIds.has(char.id)}
                  on:click={() => toggleCharacter(char.id)}
                >
                  <div class="cpc-avatar" class:has-image={!!char.image}>
                    {#if char.image}
                      <img src="data:image/png;base64,{char.image}" alt={char.name} />
                    {:else}
                      <span>{char.name.charAt(0).toUpperCase()}</span>
                    {/if}
                  </div>
                  <div class="cpc-info">
                    <span class="cpc-name">{char.name}</span>
                    <span class="cpc-detail">
                      {[char.gender, char.personality].filter(Boolean).join(' · ') || 'No details'}
                    </span>
                  </div>
                  <div class="cpc-check">
                    {#if selectedCharIds.has(char.id)}
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--accent-primary, #58a6ff)" stroke="none">
                        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
                      </svg>
                    {:else}
                      <div class="cpc-unchecked"></div>
                    {/if}
                  </div>
                </button>
              {/each}
            </div>

            {#if selectedCharIds.size > 0}
              <div class="selected-count">
                {selectedCharIds.size} character{selectedCharIds.size !== 1 ? 's' : ''} selected
              </div>
            {/if}
          {/if}
        </div>
      {/if}

      <!-- ═══ Step 3: Review ═══ -->
      {#if step === 3}
        <div class="step-content">
          <div class="step-title">Review & Start</div>
          <p class="step-subtitle">Everything look good? Hit "Create Story" to begin your adventure.</p>

          <div class="review-card">
            <div class="review-row">
              <span class="review-label">Title</span>
              <span class="review-value">{title}</span>
            </div>
            <div class="review-row">
              <span class="review-label">Setting</span>
              <span class="review-value desc-preview">{description}</span>
            </div>
            <div class="review-row">
              <span class="review-label">Characters</span>
              <span class="review-value">
                {#if selectedCharIds.size === 0}
                  <em class="none-text">None selected — you can add later</em>
                {:else}
                  {availableCharacters
                    .filter(c => selectedCharIds.has(c.id))
                    .map(c => c.name)
                    .join(', ')}
                {/if}
              </span>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="modal-footer">
      {#if step > 1}
        <button class="btn btn-secondary" on:click={prevStep}>← Back</button>
      {:else}
        <div></div>
      {/if}

      <div class="footer-right">
        {#if step < TOTAL_STEPS}
          <button
            class="btn btn-primary"
            on:click={nextStep}
            disabled={step === 1 && !step1Valid}
          >
            Next →
          </button>
        {:else}
          <button
            class="btn btn-create"
            on:click={handleCreate}
            disabled={!step1Valid}
          >
            🚀 Create Story
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>
{/if}

<style>
  /* ── Backdrop ── */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }

  .modal {
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-primary, #30363d);
    border-radius: 14px;
    width: 520px;
    max-height: 82vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }

  /* ── Header ── */
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 18px 22px;
    border-bottom: 1px solid var(--border-primary, #30363d);
    background: var(--bg-secondary, #161b22);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.15rem;
    color: var(--text-primary, #c9d1d9);
  }

  .step-indicator {
    display: flex;
    gap: 6px;
  }

  .step-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border-secondary, #21262d);
    transition: background 0.2s, transform 0.2s;
  }

  .step-dot.active {
    background: var(--accent-primary, #58a6ff);
    transform: scale(1.25);
  }

  .step-dot.completed {
    background: var(--accent-success, #238636);
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.4rem;
    cursor: pointer;
    color: var(--text-muted, #6e7681);
    padding: 4px;
    line-height: 1;
    transition: color 0.15s;
  }

  .close-btn:hover { color: var(--text-primary, #c9d1d9); }

  /* ── Body ── */
  .modal-body {
    padding: 22px;
    overflow-y: auto;
    flex: 1;
  }

  .step-content {
    animation: fadeIn 0.2s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateX(8px); }
    to { opacity: 1; transform: translateX(0); }
  }

  .step-title {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
    margin-bottom: 4px;
  }

  .step-subtitle {
    font-size: 0.82rem;
    color: var(--text-muted, #6e7681);
    margin: 0 0 18px;
    line-height: 1.5;
  }

  /* ── Form Fields ── */
  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--text-secondary, #8b949e);
    margin-bottom: 6px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .form-group input,
  .form-group textarea {
    width: 100%;
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-secondary, #30363d);
    border-radius: 8px;
    padding: 10px 12px;
    color: var(--text-primary, #c9d1d9);
    font-size: 0.9rem;
    font-family: inherit;
    outline: none;
    transition: border-color 0.2s;
    box-sizing: border-box;
  }

  .form-group input::placeholder,
  .form-group textarea::placeholder {
    color: var(--text-muted, #6e7681);
  }

  .form-group input:focus,
  .form-group textarea:focus {
    border-color: var(--accent-primary, #58a6ff);
  }

  .form-group input.invalid,
  .form-group textarea.invalid {
    border-color: var(--accent-danger, #f85149);
  }

  .form-group textarea {
    resize: vertical;
    min-height: 80px;
  }

  .textarea-footer {
    display: flex;
    justify-content: space-between;
    margin-top: 4px;
  }

  .field-hint {
    font-size: 0.7rem;
    color: var(--text-muted, #6e7681);
  }

  .field-hint.error {
    color: var(--accent-danger, #f85149);
  }

  .char-counter {
    font-size: 0.68rem;
    color: var(--text-muted, #6e7681);
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
  }

  /* ── Presets ── */
  .presets-section {
    margin-top: 8px;
  }

  .presets-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-muted, #6e7681);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .presets-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }

  .preset-btn {
    padding: 5px 10px;
    border-radius: 6px;
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-secondary, #30363d);
    color: var(--text-secondary, #8b949e);
    font-size: 0.78rem;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .preset-btn:hover {
    background: var(--bg-hover, #30363d);
    border-color: var(--accent-primary, #58a6ff);
    color: var(--text-primary, #c9d1d9);
  }

  /* ── Character Picker ── */
  .no-chars-notice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 16px;
    background: var(--bg-tertiary, #21262d);
    border: 1px dashed var(--border-secondary, #30363d);
    border-radius: 8px;
    font-size: 0.85rem;
    color: var(--text-muted, #6e7681);
  }

  .notice-icon {
    font-size: 1.4rem;
    opacity: 0.5;
  }

  .character-picker {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 260px;
    overflow-y: auto;
  }

  .char-pick-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-secondary, #30363d);
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
    text-align: left;
    color: inherit;
    font-family: inherit;
    width: 100%;
  }

  .char-pick-card:hover {
    background: var(--bg-hover, #30363d);
  }

  .char-pick-card.selected {
    border-color: var(--accent-primary, #58a6ff);
    background: rgba(88, 166, 255, 0.06);
  }

  .cpc-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    font-size: 0.85rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    flex-shrink: 0;
  }

  .cpc-avatar.has-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .cpc-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .cpc-name {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary, #c9d1d9);
  }

  .cpc-detail {
    font-size: 0.72rem;
    color: var(--text-muted, #6e7681);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cpc-check {
    flex-shrink: 0;
  }

  .cpc-unchecked {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--border-secondary, #30363d);
  }

  .selected-count {
    margin-top: 8px;
    font-size: 0.75rem;
    color: var(--accent-primary, #58a6ff);
    font-weight: 600;
    text-align: center;
  }

  /* ── Review ── */
  .review-card {
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-secondary, #30363d);
    border-radius: 10px;
    overflow: hidden;
  }

  .review-row {
    display: flex;
    gap: 12px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-secondary, #30363d);
  }

  .review-row:last-child {
    border-bottom: none;
  }

  .review-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-muted, #6e7681);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    min-width: 80px;
    padding-top: 2px;
    flex-shrink: 0;
  }

  .review-value {
    font-size: 0.88rem;
    color: var(--text-primary, #c9d1d9);
    line-height: 1.5;
    flex: 1;
    min-width: 0;
  }

  .desc-preview {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 0.82rem;
  }

  .none-text {
    color: var(--text-muted, #6e7681);
  }

  /* ── Footer ── */
  .modal-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 22px;
    border-top: 1px solid var(--border-primary, #30363d);
    background: var(--bg-secondary, #161b22);
  }

  .footer-right {
    display: flex;
    gap: 8px;
  }

  .btn {
    padding: 8px 18px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: background 0.15s, opacity 0.15s;
    font-family: inherit;
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-tertiary, #21262d);
    color: var(--text-secondary, #8b949e);
    border: 1px solid var(--border-secondary, #30363d);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover, #30363d);
  }

  .btn-primary {
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-create {
    background: var(--accent-success, #238636);
    color: #fff;
    padding: 8px 22px;
  }

  .btn-create:hover:not(:disabled) {
    opacity: 0.9;
  }
</style>