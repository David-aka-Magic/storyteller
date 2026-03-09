<script lang="ts">
  import type { DependencyStatus } from '$lib/types/setup';

  type InstallState = 'idle' | 'queued' | 'installing' | 'done' | 'error';

  let {
    status,
    label,
    sizeBadge,
    installState = 'idle',
    isSelected = false,
    disabled = false,
    progressPct = 0,
    errorMessage,
    onToggle,
  }: {
    status: DependencyStatus;
    label: string;
    sizeBadge?: string;
    installState?: InstallState;
    isSelected?: boolean;
    disabled?: boolean;
    progressPct?: number;
    errorMessage?: string;
    onToggle?: () => void;
  } = $props();

  let showCheckbox = $derived(installState !== 'done');

  let icon = $derived(
    installState === 'done'      ? '✓'
    : installState === 'error'   ? '✗'
    : installState === 'queued'  ? '○'
    : '' // 'idle' shows no icon (checkbox handles it), 'installing' uses spinner
  );

  let statusText = $derived(
    installState === 'done'      ? (status.version ? `Installed (${status.version})` : 'Installed')
    : installState === 'installing' ? 'Installing…'
    : installState === 'queued'  ? 'Queued'
    : installState === 'error'   ? 'Error'
    : 'Not installed'
  );
</script>

<div
  class="dep-row"
  class:done={installState === 'done'}
  class:installing={installState === 'installing'}
  class:queued={installState === 'queued'}
  class:has-error={installState === 'error'}
>
  <!-- Checkbox (hidden for already-installed items) -->
  <div class="checkbox-col">
    {#if showCheckbox}
      <input
        type="checkbox"
        checked={isSelected}
        {disabled}
        onchange={onToggle}
        aria-label="Select {label}"
      />
    {:else}
      <!-- Spacer so columns stay aligned -->
      <span class="done-placeholder" aria-hidden="true"></span>
    {/if}
  </div>

  <!-- Status icon -->
  <div class="icon-col" aria-hidden="true">
    {#if installState === 'installing'}
      <span class="spinner"></span>
    {:else}
      <span class="icon icon-{installState}">{icon}</span>
    {/if}
  </div>

  <!-- Name + status text -->
  <div class="info-col">
    <span class="dep-name">{label}</span>
    <span class="dep-status status-{installState}">{statusText}</span>
  </div>

  <!-- Size badge -->
  {#if sizeBadge}
    <span class="size-badge">{sizeBadge}</span>
  {/if}
</div>

<!-- Per-item progress bar (only while installing) -->
{#if installState === 'installing'}
  <div class="item-progress-track">
    <div class="item-progress-fill" style="width: {Math.round(progressPct * 100)}%"></div>
  </div>
{/if}

<!-- Per-item error -->
{#if errorMessage && installState === 'error'}
  <div class="item-error">{errorMessage}</div>
{/if}

<style>
  .dep-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border-radius: 6px 6px 0 0;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    transition: border-color 0.15s, background 0.15s;
  }

  /* Remove bottom-radius when a progress bar or error follows */
  .dep-row.installing {
    border-radius: 6px 6px 0 0;
    border-color: var(--accent-primary);
    background: color-mix(in srgb, var(--bg-secondary) 93%, var(--accent-primary));
  }

  .dep-row.done {
    border-color: var(--accent-success);
    background: color-mix(in srgb, var(--bg-secondary) 94%, var(--accent-success));
    border-radius: 6px;
  }

  .dep-row.queued {
    border-color: var(--accent-warning);
    background: color-mix(in srgb, var(--bg-secondary) 95%, var(--accent-warning));
  }

  .dep-row.has-error {
    border-radius: 6px 6px 0 0;
    border-color: var(--accent-danger);
    background: color-mix(in srgb, var(--bg-secondary) 95%, var(--accent-danger));
  }

  /* ── Checkbox ── */
  .checkbox-col {
    width: 18px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  input[type='checkbox'] {
    width: 15px;
    height: 15px;
    accent-color: var(--accent-primary);
    cursor: pointer;
    flex-shrink: 0;
  }

  input[type='checkbox']:disabled { cursor: not-allowed; opacity: 0.5; }

  .done-placeholder { width: 15px; height: 15px; }

  /* ── Status icon ── */
  .icon-col {
    width: 20px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.9rem;
    font-weight: 700;
  }

  .icon-done    { color: var(--accent-success); }
  .icon-error   { color: var(--accent-danger); }
  .icon-queued  { color: var(--accent-warning); }
  .icon-idle    { color: var(--text-muted); }

  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid var(--border-primary);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  /* ── Info ── */
  .info-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .dep-name {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dep-status {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .status-done      { color: var(--accent-success); }
  .status-error     { color: var(--accent-danger); }
  .status-queued    { color: var(--accent-warning); }
  .status-installing { color: var(--accent-primary); }

  /* ── Size badge ── */
  .size-badge {
    font-size: 0.72rem;
    padding: 2px 7px;
    border-radius: 10px;
    background: var(--bg-tertiary);
    color: var(--text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* ── Per-item progress bar ── */
  .item-progress-track {
    height: 4px;
    background: var(--bg-tertiary);
    border-radius: 0 0 4px 4px;
    border: 1px solid var(--accent-primary);
    border-top: none;
    overflow: hidden;
  }

  .item-progress-fill {
    height: 100%;
    background: var(--accent-primary);
    transition: width 0.25s ease;
    min-width: 2%;
  }

  /* ── Per-item error text ── */
  .item-error {
    font-size: 0.75rem;
    color: var(--accent-danger);
    padding: 3px 12px 5px;
    border: 1px solid var(--accent-danger);
    border-top: none;
    border-radius: 0 0 6px 6px;
    background: color-mix(in srgb, var(--bg-secondary) 90%, var(--accent-danger));
  }
</style>
