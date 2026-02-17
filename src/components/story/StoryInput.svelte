<!-- src/components/story/StoryInput.svelte -->
<!--
  User action input area:
    - Text input for player's next action
    - Submit button + Enter key support
    - Loading states (LLM generating, image generating)
    - Disabled during generation
    - Character/word count hint
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let isGenerating: boolean = false;
  export let isGeneratingImage: boolean = false;
  export let disabled: boolean = false;
  export let placeholder: string = 'What do you do?';

  const dispatch = createEventDispatcher();

  let inputText = '';
  let textareaEl: HTMLTextAreaElement;

  $: isDisabled = disabled || isGenerating;
  $: statusText = isGeneratingImage
    ? 'Generating scene image...'
    : isGenerating
      ? 'Writing story...'
      : '';

  function handleSubmit() {
    const text = inputText.trim();
    if (!text || isDisabled) return;

    dispatch('submit', text);
    inputText = '';

    // Reset textarea height
    if (textareaEl) {
      textareaEl.style.height = 'auto';
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function autoResize(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    target.style.height = 'auto';
    target.style.height = Math.min(target.scrollHeight, 160) + 'px';
  }

  /** Focus the input (called from parent via bind:this) */
  export function focus() {
    textareaEl?.focus();
  }
</script>

<div class="story-input" class:generating={isGenerating}>
  <!-- Loading Indicator -->
  {#if isGenerating}
    <div class="loading-bar">
      <div class="loading-fill" class:image-phase={isGeneratingImage}></div>
    </div>
    <div class="loading-status">
      <span class="loading-dot-anim">
        <span class="dot"></span>
        <span class="dot"></span>
        <span class="dot"></span>
      </span>
      <span class="loading-text">{statusText}</span>
    </div>
  {/if}

  <!-- Input Area -->
  <div class="input-row">
    <textarea
      bind:this={textareaEl}
      bind:value={inputText}
      on:keydown={handleKeydown}
      on:input={autoResize}
      {placeholder}
      disabled={isDisabled}
      rows="1"
      class="action-input"
    ></textarea>

    <button
      class="submit-btn"
      on:click={handleSubmit}
      disabled={isDisabled || !inputText.trim()}
      title="Send action (Enter)"
    >
      {#if isGenerating}
        <span class="spinner"></span>
      {:else}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <line x1="22" y1="2" x2="11" y2="13"></line>
          <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
        </svg>
      {/if}
    </button>
  </div>

  <!-- Helper text -->
  {#if !isGenerating && inputText.length > 0}
    <div class="input-hint">
      <span class="hint-text">Press <kbd>Enter</kbd> to send · <kbd>Shift+Enter</kbd> for new line</span>
      <span class="char-count">{inputText.length}</span>
    </div>
  {/if}
</div>

<style>
  .story-input {
    padding: 12px 20px 14px;
    background: var(--story-input-bg, rgba(0, 0, 0, 0.2));
    border-top: 1px solid var(--story-border, rgba(255, 255, 255, 0.06));
    display: flex;
    flex-direction: column;
    gap: 0;
    position: relative;
  }

  /* ── Loading Bar ── */
  .loading-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: rgba(255, 255, 255, 0.04);
    overflow: hidden;
  }

  .loading-fill {
    height: 100%;
    width: 40%;
    background: var(--accent-primary, #58a6ff);
    border-radius: 2px;
    animation: loadingSlide 1.6s ease-in-out infinite;
  }

  .loading-fill.image-phase {
    background: linear-gradient(90deg, #58a6ff, #a78bfa, #58a6ff);
    background-size: 200% 100%;
    animation: loadingSlide 1.6s ease-in-out infinite, gradientShift 2s ease infinite;
  }

  @keyframes loadingSlide {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(350%); }
  }

  @keyframes gradientShift {
    0%, 100% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
  }

  .loading-status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0 8px;
  }

  .loading-text {
    font-size: 0.78rem;
    color: var(--text-muted, #6e7681);
    font-style: italic;
  }

  .loading-dot-anim {
    display: flex;
    gap: 3px;
  }

  .dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    animation: dotBounce 1.2s ease-in-out infinite;
  }

  .dot:nth-child(2) { animation-delay: 0.15s; }
  .dot:nth-child(3) { animation-delay: 0.3s; }

  @keyframes dotBounce {
    0%, 60%, 100% { opacity: 0.3; transform: translateY(0); }
    30% { opacity: 1; transform: translateY(-3px); }
  }

  /* ── Input Row ── */
  .input-row {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }

  .action-input {
    flex: 1;
    background: var(--story-input-field-bg, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--story-input-border, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    padding: 10px 14px;
    color: var(--text-primary, #eaeaea);
    font-size: 0.92rem;
    line-height: 1.5;
    resize: none;
    outline: none;
    min-height: 42px;
    max-height: 160px;
    transition: border-color 0.2s, background 0.2s;
    font-family: inherit;
  }

  .action-input::placeholder {
    color: var(--text-muted, #6e7681);
    font-style: italic;
  }

  .action-input:focus {
    border-color: var(--accent-primary, #58a6ff);
    background: var(--story-input-field-focus, rgba(255, 255, 255, 0.07));
  }

  .action-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ── Submit Button ── */
  .submit-btn {
    width: 42px;
    height: 42px;
    border-radius: 10px;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background 0.15s, opacity 0.15s, transform 0.1s;
  }

  .submit-btn:hover:not(:disabled) {
    opacity: 0.9;
    transform: scale(1.03);
  }

  .submit-btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .submit-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid transparent;
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ── Hint ── */
  .input-hint {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 2px 0;
  }

  .hint-text {
    font-size: 0.68rem;
    color: var(--text-muted, #6e7681);
    opacity: 0.6;
  }

  kbd {
    padding: 1px 4px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.06);
    font-size: 0.9em;
    font-family: inherit;
  }

  .char-count {
    font-size: 0.68rem;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    color: var(--text-muted, #6e7681);
    opacity: 0.5;
  }
</style>