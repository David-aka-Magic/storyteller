<!-- src/components/chat/ChatInput.svelte — Message input area -->
<script lang="ts">
  let {
    disabled = false,
    onsubmit,
  }: {
    disabled?: boolean;
    onsubmit?: (text: string) => void;
  } = $props();

  let value = $state('');

  function send() {
    if (!value.trim() || disabled) return;
    onsubmit?.(value);
    value = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }
</script>

<div class="input-area">
  <textarea
    bind:value
    onkeydown={handleKeydown}
    placeholder="Type your action..."
    rows="1"
    {disabled}
  ></textarea>
  <button type="button" onclick={send} disabled={disabled || !value.trim()}>
    Send
  </button>
</div>

<style>
  .input-area {
    padding: 20px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-primary);
    display: flex;
    gap: 10px;
    flex-shrink: 0;
  }

  textarea {
    flex: 1;
    padding: 10px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    font-family: inherit;
    resize: none;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  textarea:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  textarea::placeholder {
    color: var(--text-muted);
  }

  button {
    padding: 0 20px;
    background: var(--accent-primary);
    color: var(--text-inverse);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: bold;
    transition: opacity 0.2s;
  }

  button:hover:not(:disabled) { opacity: 0.9; }

  button:disabled {
    background: var(--bg-tertiary);
    color: var(--text-muted);
    cursor: not-allowed;
  }
</style>
