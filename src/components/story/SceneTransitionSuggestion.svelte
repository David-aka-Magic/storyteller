<!-- src/components/story/SceneTransitionSuggestion.svelte
     Compact bar shown above the input when the user switches scenes manually.
     Pre-fills a transition action the user can send or dismiss. -->
<script lang="ts">
  let {
    sceneName,
    characterNames = [] as string[],
    onSend,
    onDismiss,
  }: {
    sceneName: string;
    characterNames?: string[];
    onSend?: (text: string) => void;
    onDismiss?: () => void;
  } = $props();

  // Generate a simple pre-fill suggestion
  function defaultSuggestion(name: string): string {
    return `Head to the ${name}.`;
  }

  let suggestion = $state(defaultSuggestion(sceneName));

  // Reset suggestion when scene name changes
  $effect(() => {
    suggestion = defaultSuggestion(sceneName);
  });

  function handleSend() {
    if (suggestion.trim()) {
      onSend?.(suggestion.trim());
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
    if (e.key === 'Escape') {
      onDismiss?.();
    }
  }
</script>

<div class="suggestion-bar">
  <div class="suggestion-header">
    <span class="icon">📍</span>
    <span class="label">Scene changed to: <strong>{sceneName}</strong></span>
    {#if characterNames.length > 0}
      <span class="chars">({characterNames.slice(0, 3).join(', ')}{characterNames.length > 3 ? '…' : ''})</span>
    {/if}
  </div>
  <div class="suggestion-input-row">
    <input
      class="suggestion-input"
      bind:value={suggestion}
      onkeydown={handleKeydown}
      placeholder="Describe the transition..."
    />
    <button class="send-btn" onclick={handleSend} disabled={!suggestion.trim()}>Send</button>
    <button class="dismiss-btn" onclick={onDismiss} title="Dismiss">✕</button>
  </div>
</div>

<style>
  .suggestion-bar {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 16px 10px;
    background: color-mix(in srgb, var(--accent-primary, #58a6ff) 6%, var(--bg-secondary));
    border-top: 1px solid color-mix(in srgb, var(--accent-primary, #58a6ff) 20%, transparent);
    animation: slideDown 0.2s ease-out;
  }

  @keyframes slideDown {
    from { opacity: 0; transform: translateY(-8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .suggestion-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.78em;
    color: var(--text-secondary, #8b949e);
  }

  .icon { font-size: 0.9em; }

  .label { color: var(--text-secondary); }
  .label strong { color: var(--text-primary); }

  .chars {
    color: var(--text-muted, #6e7681);
    font-style: italic;
  }

  .suggestion-input-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .suggestion-input {
    flex: 1;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.85em;
    padding: 5px 10px;
    outline: none;
    font-family: inherit;
  }
  .suggestion-input:focus { border-color: var(--border-active); }

  .send-btn {
    background: var(--accent-primary, #58a6ff);
    border: none;
    color: white;
    border-radius: 4px;
    padding: 5px 14px;
    font-size: 0.82em;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .send-btn:hover:not(:disabled) { opacity: 0.9; }
  .send-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .dismiss-btn {
    background: none;
    border: 1px solid var(--border-secondary);
    color: var(--text-secondary);
    border-radius: 4px;
    padding: 5px 8px;
    font-size: 0.82em;
    cursor: pointer;
    line-height: 1;
  }
  .dismiss-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
</style>
