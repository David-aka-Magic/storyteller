<!-- src/components/chat/MessageBubble.svelte — Renders a single chat message -->
<script lang="ts">
  import type { ChatMessage } from '$lib/types';
  import ImageLightbox from '../shared/ImageLightbox.svelte';

  let {
    message,
    isLast = false,
    isLoading = false,
    isGeneratingImage = false,
    imageError = undefined,
    onGenerateImage,
    onRegenerate,
    onRewriteWithInput,
    lastUserInput = '',
  }: {
    message: ChatMessage;
    isLast?: boolean;
    isLoading?: boolean;
    isGeneratingImage?: boolean;
    imageError?: string;
    onGenerateImage?: (positivePrompt?: string, negativePrompt?: string) => void;
    onRegenerate?: () => void;
    onRewriteWithInput?: (input: string) => void;
    lastUserInput?: string;
  } = $props();

  // ── Rewrite editor state ──
  let rewriteExpanded = $state(false);
  let editedUserInput = $state('');

  let showLightbox = $state(false);

  // ── Illustrate editor ──
  let illustrateExpanded = $state(false);
  let editedPositive = $state('');
  let editedNegative = $state('');
  let promptDirty = $state(false);

  // Seed editor from enriched prompt when it arrives; don't overwrite user edits
  $effect(() => {
    if (!promptDirty) {
      editedPositive = message.enrichedPrompt ?? message.data?.sd_prompt ?? '';
      editedNegative = message.negativePrompt ?? '';
    }
  });

  const isEdited = $derived(
    promptDirty && (
      editedPositive !== (message.enrichedPrompt ?? message.data?.sd_prompt ?? '') ||
      editedNegative !== (message.negativePrompt ?? '')
    )
  );

  function resetPrompts() {
    editedPositive = message.enrichedPrompt ?? message.data?.sd_prompt ?? '';
    editedNegative = message.negativePrompt ?? '';
    promptDirty = false;
  }

  function handleIllustrate() {
    if (isEdited) {
      onGenerateImage?.(editedPositive, editedNegative);
    } else {
      onGenerateImage?.();
    }
  }
</script>

<div class="message-wrapper {message.sender}">
  {#if message.text}
    <div><strong>{message.sender.toUpperCase()}:</strong> {message.text}</div>
  {/if}

  {#if message.data}
    <div class="story-block">
      <div class="story-text">{message.data.story}</div>

      {#if illustrateExpanded}
        <div class="illustrate-editor">
          <span class="illustrate-label">Edit visual prompt before generating:</span>
          <div class="prompt-field">
            <div class="field-header">
              <span class="field-label">Positive</span>
              {#if isEdited}
                <button class="reset-btn" onclick={resetPrompts} title="Reset to auto-generated">↺ Reset</button>
              {/if}
            </div>
            <textarea
              class="prompt-textarea"
              bind:value={editedPositive}
              oninput={() => { promptDirty = true; }}
              rows="4"
              placeholder="Quality tags, scene, character descriptions..."
            ></textarea>
          </div>
          <div class="prompt-field">
            <div class="field-header">
              <span class="field-label">Negative</span>
            </div>
            <textarea
              class="prompt-textarea negative"
              bind:value={editedNegative}
              oninput={() => { promptDirty = true; }}
              rows="2"
              placeholder="Things to avoid..."
            ></textarea>
          </div>
          <div class="illustrate-actions">
            <button
              class="action-btn illustrate-confirm"
              onclick={() => { handleIllustrate(); illustrateExpanded = false; }}
              disabled={isLoading || isGeneratingImage}
            >
              {message.image ? '↺ Redraw Image' : '🎨 Generate Image'}{#if isEdited}<span class="edited-badge">edited</span>{/if}
            </button>
            <button
              class="action-btn illustrate-cancel"
              onclick={() => { illustrateExpanded = false; }}
            >
              Cancel
            </button>
          </div>
        </div>
      {/if}

      <div class="actions-row">
        {#if !illustrateExpanded}
          {#if isGeneratingImage}
            <span class="action-btn img-btn" style="cursor:default; opacity:0.7;">
              <span class="spinner">↻</span> Generating...
            </span>
          {:else}
            <button
              class="action-btn img-btn"
              onclick={() => { illustrateExpanded = true; }}
              disabled={isLoading}
            >
              {message.image ? '↺ Redraw Image' : '🎨 Illustrate Scene'}
            </button>
          {/if}
        {/if}

        {#if isLast && message.sender === 'ai'}
          {#if rewriteExpanded}
            <div class="rewrite-editor">
              <label class="rewrite-label">Edit your action before rewriting:</label>
              <textarea
                class="rewrite-textarea"
                bind:value={editedUserInput}
                oninput={(e) => { editedUserInput = e.currentTarget.value; }}
                rows="3"
                placeholder="Edit your action..."
              ></textarea>
              <div class="rewrite-actions">
                <button
                  class="action-btn rewrite-submit"
                  onclick={() => { if (editedUserInput.trim()) { onRewriteWithInput?.(editedUserInput); rewriteExpanded = false; } }}
                  disabled={!editedUserInput.trim() || isLoading}
                >↻ Rewrite with changes</button>
                <button
                  class="action-btn regen-btn"
                  onclick={() => { onRegenerate?.(); rewriteExpanded = false; }}
                  disabled={isLoading}
                >↻ Rewrite (same input)</button>
                <button
                  class="action-btn rewrite-cancel"
                  onclick={() => { rewriteExpanded = false; }}
                >Cancel</button>
              </div>
            </div>
          {:else}
            <button
              class="action-btn regen-btn"
              onclick={() => { editedUserInput = lastUserInput ?? ''; rewriteExpanded = true; }}
              disabled={isLoading}
            >
              ↻ Rewrite Story
            </button>
          {/if}
        {/if}
      </div>

      {#if imageError}
        <div class="image-error">⚠ {imageError}</div>
      {/if}
    </div>
  {/if}

  {#if message.image}
    <div class="img-container">
      <img
        src={message.image}
        alt="Generated Scene"
        class:dimmed={isGeneratingImage}
        onclick={() => { if (!isGeneratingImage) showLightbox = true; }}
        role="button"
        tabindex="0"
        onkeydown={(e) => { if (e.key === 'Enter') showLightbox = true; }}
      />
    </div>
  {/if}
</div>

{#if showLightbox && message.image}
  <ImageLightbox
    src={message.image}
    alt="Generated Scene"
    onclose={() => showLightbox = false}
  />
{/if}

<style>
  .message-wrapper {
    max-width: 620px;
    padding: 10px 15px;
    border-radius: 8px;
    font-size: 0.95em;
  }

  .message-wrapper.user {
    align-self: flex-end;
    text-align: left;
    background: var(--bg-message-user);
    color: var(--text-primary);
    border-bottom-right-radius: 0;
  }

  .message-wrapper.ai {
    align-self: flex-start;
    background: var(--bg-message-ai);
    border: 1px solid var(--border-secondary);
    color: var(--text-primary);
    border-bottom-left-radius: 0;
  }

  .story-block {
    margin-top: 15px;
    background: var(--bg-secondary);
    padding: 15px;
    border-radius: 8px;
    border-left: 4px solid var(--accent-primary);
    text-align: left;
  }

  .story-text {
    white-space: pre-wrap;
    line-height: 1.6;
    color: var(--text-primary);
  }

  .sd-details {
    margin-top: 12px;
    border-top: 1px solid var(--border-secondary);
    padding-top: 8px;
  }

  /* ── Illustrate Editor ── */
  .illustrate-editor {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.5rem;
    padding: 0.75rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    animation: slideDown 0.15s ease-out;
  }

  @keyframes slideDown {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .illustrate-label {
    font-size: 0.78rem;
    color: var(--text-muted);
    font-weight: 600;
  }

  .illustrate-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-top: 0.25rem;
  }

  .illustrate-confirm {
    background: var(--accent-primary) !important;
    color: var(--text-inverse) !important;
    border-color: var(--accent-primary) !important;
    font-weight: 600;
  }

  .illustrate-cancel {
    background: transparent !important;
    border: 1px solid var(--border-primary) !important;
    color: var(--text-muted) !important;
    opacity: 0.8;
  }

  .prompt-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .field-label {
    font-size: 0.72em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .reset-btn {
    background: none;
    border: none;
    color: var(--accent-primary);
    font-size: 0.72em;
    cursor: pointer;
    padding: 1px 6px;
    border-radius: 4px;
  }

  .reset-btn:hover {
    background: color-mix(in srgb, var(--accent-primary) 10%, transparent);
  }

  .prompt-textarea {
    width: 100%;
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-secondary);
    border-radius: 4px;
    color: var(--text-secondary);
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.78em;
    line-height: 1.5;
    padding: 6px 8px;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }

  .prompt-textarea:focus {
    border-color: var(--accent-primary);
    color: var(--text-primary);
  }

  .prompt-textarea.negative {
    color: color-mix(in srgb, var(--accent-danger, #f85149) 70%, var(--text-muted, #6e7681));
  }

  /* ── Actions Row ── */
  .actions-row {
    margin-top: 12px;
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }

  .action-btn {
    border: none;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85em;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-weight: bold;
    transition: all 0.2s;
  }

  .img-btn {
    background: var(--bg-tertiary);
    color: var(--accent-primary);
    border: 1px solid var(--border-active);
  }
  .img-btn:hover:not(:disabled) { background: var(--bg-hover); }
  .img-btn:disabled { opacity: 0.7; cursor: wait; }

  .img-btn.edited {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-primary) 30%, transparent);
  }

  .edited-badge {
    font-size: 0.72em;
    font-weight: 400;
    opacity: 0.8;
    padding: 1px 5px;
    background: color-mix(in srgb, var(--accent-primary) 20%, transparent);
    border-radius: 3px;
  }

  .regen-btn {
    background: var(--bg-tertiary);
    color: var(--accent-warning);
    border: 1px solid var(--accent-warning);
  }
  .regen-btn:hover:not(:disabled) { background: var(--bg-hover); }

  /* ── Rewrite editor ── */
  .rewrite-editor {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.5rem;
    padding: 0.75rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
  }

  .rewrite-label {
    font-size: 0.78rem;
    color: var(--text-muted);
    font-weight: 600;
  }

  .rewrite-textarea {
    width: 100%;
    padding: 0.5rem 0.6rem;
    border-radius: 6px;
    border: 1px solid var(--border-secondary);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.88rem;
    resize: vertical;
    min-height: 60px;
    box-sizing: border-box;
  }

  .rewrite-textarea:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .rewrite-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .rewrite-submit {
    background: var(--accent-primary) !important;
    color: var(--text-inverse) !important;
    border-color: var(--accent-primary) !important;
    font-weight: 600;
  }

  .rewrite-cancel {
    background: transparent !important;
    border: 1px solid var(--border-primary) !important;
    color: var(--text-muted) !important;
    opacity: 0.8;
  }

  .img-container {
    margin-top: 15px;
    text-align: center;
  }

  .img-container img {
    max-width: 100%;
    border-radius: 8px;
    box-shadow: 0 4px 8px var(--shadow);
    transition: opacity 0.3s;
    cursor: pointer;
  }

  .img-container img.dimmed {
    opacity: 0.5;
    cursor: wait;
  }

  .image-error {
    margin-top: 8px;
    padding: 6px 10px;
    border-radius: 4px;
    background: rgba(224, 96, 96, 0.12);
    border: 1px solid rgba(224, 96, 96, 0.3);
    color: #e06060;
    font-size: 0.82em;
    line-height: 1.4;
  }

  .spinner {
    animation: spin 1s infinite linear;
    display: inline-block;
    font-weight: normal;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>
