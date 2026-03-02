<!-- src/components/shared/Modal.svelte — Generic reusable modal shell -->
<!--
  Usage:
    {#if show}
      <Modal title="My Title" {onclose}>
        (slot content here)
      /Modal
    {/if}

  The parent is responsible for the {#if show} guard so slot content
  mounts/unmounts cleanly with the modal. Modal itself is a pure layout shell.
-->
<script lang="ts">
  let {
    title = '',
    onclose,
    width = '550px',
  }: {
    title?: string;
    onclose?: () => void;
    width?: string;
  } = $props();

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

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="modal-backdrop"
  onclick={handleBackdropClick}
  onkeydown={handleKeydown}
  role="button"
  tabindex="0"
>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="modal"
    style="width: {width}"
    onclick={(e) => e.stopPropagation()}
    onkeydown={() => {}}
    role="dialog"
    tabindex="-1"
  >
    <div class="modal-header">
      <h2>{title}</h2>
      <button class="close-btn" onclick={close}>✕</button>
    </div>
    <div class="modal-body">
      <slot />
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0; left: 0;
    width: 100%; height: 100%;
    background: rgba(0, 0, 0, 0.6);
    z-index: 3000;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .modal {
    background: var(--bg-primary);
    color: var(--text-primary);
    border-radius: 12px;
    max-height: 80vh;
    overflow: hidden;
    box-shadow: 0 20px 40px var(--shadow);
    display: flex;
    flex-direction: column;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 25px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.3em;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.5em;
    cursor: pointer;
    color: var(--text-muted);
    padding: 5px;
    line-height: 1;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .modal-body {
    overflow-y: auto;
    flex: 1;
  }
</style>
