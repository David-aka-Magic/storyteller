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
  }: {
    message: ChatMessage;
    isLast?: boolean;
    isLoading?: boolean;
    isGeneratingImage?: boolean;
    imageError?: string;
    onGenerateImage?: (positivePrompt?: string, negativePrompt?: string) => void;
    onRegenerate?: () => void;
  } = $props();

  let showLightbox = $state(false);

  // ── Expandable prompt editor ──
  let promptExpanded = $state(false);
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

      {#if message.data.sd_prompt}
        <div class="sd-details">
          <!-- Collapsible prompt bar -->
          <div class="prompt-bar">
            <button class="prompt-toggle" onclick={() => promptExpanded = !promptExpanded}>
              <span class="toggle-arrow" class:expanded={promptExpanded}>▶</span>
              <span class="prompt-label">Visual Prompt:</span>
              {#if !promptExpanded}
                <span class="prompt-preview">{message.data.sd_prompt}</span>
              {/if}
            </button>

            {#if promptExpanded}
              <div class="prompt-editor">
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
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <div class="actions-row">
        <button
          class="action-btn img-btn"
          class:edited={isEdited}
          onclick={handleIllustrate}
          disabled={isLoading || isGeneratingImage}
        >
          {#if isGeneratingImage}
            <span class="spinner">↻</span> Generating...
          {:else}
            {message.image ? '↻ Redraw Image' : '🎨 Illustrate Scene'}{#if isEdited}<span class="edited-badge">edited</span>{/if}
          {/if}
        </button>

        {#if isLast && message.sender === 'ai'}
          <button
            class="action-btn regen-btn"
            onclick={onRegenerate}
            disabled={isLoading}
          >
            ↻ Rewrite Story
          </button>
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

  /* ── Prompt Bar ── */
  .prompt-bar {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .prompt-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 6px 8px;
    font-size: 0.82em;
    font-family: monospace;
    text-align: left;
    width: 100%;
    overflow: hidden;
    transition: border-color 0.15s, color 0.15s;
  }

  .prompt-toggle:hover {
    color: var(--text-primary);
    border-color: var(--accent-primary);
  }

  .toggle-arrow {
    font-size: 0.65em;
    flex-shrink: 0;
    transition: transform 0.15s;
  }

  .toggle-arrow.expanded {
    transform: rotate(90deg);
  }

  .prompt-label {
    font-weight: 600;
    white-space: nowrap;
    flex-shrink: 0;
    font-family: sans-serif;
  }

  .prompt-preview {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.75;
  }

  .prompt-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 4px;
    animation: slideDown 0.15s ease-out;
  }

  @keyframes slideDown {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
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
