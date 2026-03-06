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
    onGenerateImage?: () => void;
    onRegenerate?: () => void;
  } = $props();

  let showLightbox = $state(false);
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
          <div class="sd-prompt">
            <strong>Visual Prompt:</strong> {message.data.sd_prompt}
          </div>
        </div>
      {/if}

      <div class="actions-row">
        <button
          class="action-btn img-btn"
          onclick={onGenerateImage}
          disabled={isLoading || isGeneratingImage}
        >
          {#if isGeneratingImage}
            <span class="spinner">↻</span> Generating...
          {:else}
            {message.image ? '↻ Redraw Image' : '🎨 Illustrate Scene'}
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
    margin-top: 15px;
    border-top: 1px solid var(--border-secondary);
    padding-top: 10px;
  }

  .sd-prompt {
    background: var(--bg-tertiary);
    padding: 8px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 0.85em;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .actions-row {
    margin-top: 15px;
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
