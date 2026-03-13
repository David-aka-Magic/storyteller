<!-- src/components/settings/PoseLoraManager.svelte — Manage pose LoRAs for scene generation -->
<script lang="ts">
  import {
    listPoseLoras,
    createPoseLora,
    updatePoseLora,
    deletePoseLora,
    seedDefaultPoseLoras,
  } from '$lib/api/pose-loras';
  import type { PoseLora } from '$lib/types';

  // ─── State ────────────────────────────────────────────────────────────────

  let loras = $state<PoseLora[]>([]);
  let loading = $state(false);
  let error = $state('');

  // Form visibility: null = hidden, 'add' = new entry, number = editing id
  let formMode = $state<null | 'add' | number>(null);

  // Form fields
  let fName = $state('');
  let fKeywords = $state('');
  let fLoraFilename = $state('');
  let fTriggerWords = $state('');
  let fStrength = $state(0.7);
  let fEnabled = $state(true);
  let saving = $state(false);

  // ─── Lifecycle ────────────────────────────────────────────────────────────

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      loras = await listPoseLoras();
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  // ─── Form helpers ─────────────────────────────────────────────────────────

  function openAdd() {
    fName = '';
    fKeywords = '';
    fLoraFilename = '';
    fTriggerWords = '';
    fStrength = 0.7;
    fEnabled = true;
    formMode = 'add';
  }

  function openEdit(lora: PoseLora) {
    fName = lora.name;
    fKeywords = lora.keywords;
    fLoraFilename = lora.lora_filename;
    fTriggerWords = lora.trigger_words;
    fStrength = lora.strength;
    fEnabled = lora.enabled;
    formMode = lora.id;
  }

  function closeForm() {
    formMode = null;
  }

  async function saveForm() {
    if (!fName.trim() || !fKeywords.trim() || !fLoraFilename.trim()) {
      error = 'Name, Keywords, and LoRA Filename are required.';
      return;
    }
    saving = true;
    error = '';
    try {
      if (formMode === 'add') {
        await createPoseLora(
          fName.trim(),
          fKeywords.trim(),
          fLoraFilename.trim(),
          fTriggerWords.trim(),
          fStrength,
        );
      } else if (typeof formMode === 'number') {
        await updatePoseLora(
          formMode,
          fName.trim(),
          fKeywords.trim(),
          fLoraFilename.trim(),
          fTriggerWords.trim(),
          fStrength,
          fEnabled,
        );
      }
      formMode = null;
      await load();
    } catch (e) {
      error = String(e);
    }
    saving = false;
  }

  // ─── Inline toggle ────────────────────────────────────────────────────────

  async function toggleEnabled(lora: PoseLora) {
    try {
      await updatePoseLora(
        lora.id,
        lora.name,
        lora.keywords,
        lora.lora_filename,
        lora.trigger_words,
        lora.strength,
        !lora.enabled,
      );
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  // ─── Delete ───────────────────────────────────────────────────────────────

  async function handleDelete(lora: PoseLora) {
    if (!confirm(`Delete pose LoRA "${lora.name}"? This cannot be undone.`)) return;
    try {
      await deletePoseLora(lora.id);
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  // ─── Seed defaults ────────────────────────────────────────────────────────

  async function handleSeedDefaults() {
    saving = true;
    error = '';
    try {
      await seedDefaultPoseLoras();
      await load();
    } catch (e) {
      error = String(e);
    }
    saving = false;
  }
</script>

<div class="pose-lora-manager">
  <div class="section-header">
    <div>
      <div class="section-title">Pose LoRAs</div>
      <div class="section-desc">Configure LoRAs injected into scene generation based on character pose.</div>
    </div>
    <div class="header-actions">
      {#if loras.length === 0 && !loading}
        <button class="btn btn-secondary" onclick={handleSeedDefaults} disabled={saving}>
          Seed Defaults
        </button>
      {/if}
      <button class="btn btn-primary" onclick={openAdd} disabled={formMode !== null}>
        + Add Pose LoRA
      </button>
    </div>
  </div>

  {#if error}
    <div class="error-msg">{error}</div>
  {/if}

  <!-- Add/Edit Form -->
  {#if formMode !== null}
    <div class="lora-form">
      <div class="form-title">{formMode === 'add' ? 'Add Pose LoRA' : 'Edit Pose LoRA'}</div>

      <div class="form-grid">
        <div class="field">
          <label for="f-name">Name <span class="req">*</span></label>
          <input id="f-name" type="text" bind:value={fName} placeholder="e.g. Sitting" />
        </div>

        <div class="field">
          <label for="f-lora">LoRA Filename <span class="req">*</span></label>
          <input id="f-lora" type="text" bind:value={fLoraFilename} placeholder="e.g. sitting_pose.safetensors" />
          <div class="field-hint">Exact filename in your ComfyUI loras folder</div>
        </div>

        <div class="field full-width">
          <label for="f-keywords">Keywords <span class="req">*</span></label>
          <input id="f-keywords" type="text" bind:value={fKeywords} placeholder="e.g. sitting, seated, sat" />
          <div class="field-hint">Comma-separated words matched against scene prompts</div>
        </div>

        <div class="field full-width">
          <label for="f-trigger">Trigger Words</label>
          <input id="f-trigger" type="text" bind:value={fTriggerWords} placeholder="e.g. person sitting down" />
          <div class="field-hint">SD prompt words injected when this LoRA is active</div>
        </div>

        <div class="field">
          <label for="f-strength">Strength: <strong>{fStrength.toFixed(2)}</strong></label>
          <div class="strength-row">
            <input
              id="f-strength"
              type="range"
              min="0"
              max="1"
              step="0.05"
              bind:value={fStrength}
            />
            <input
              type="number"
              min="0"
              max="1"
              step="0.05"
              bind:value={fStrength}
              class="strength-number"
            />
          </div>
        </div>

        <div class="field enabled-field">
          <label class="toggle-label">
            <input type="checkbox" bind:checked={fEnabled} />
            <span>Enabled</span>
          </label>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn btn-secondary" onclick={closeForm} disabled={saving}>Cancel</button>
        <button class="btn btn-primary" onclick={saveForm} disabled={saving}>
          {saving ? 'Saving…' : 'Save'}
        </button>
      </div>
    </div>
  {/if}

  <!-- LoRA List -->
  {#if loading}
    <div class="loading-msg">Loading…</div>
  {:else if loras.length === 0}
    <div class="empty-msg">
      No pose LoRAs configured. Click "Seed Defaults" to add built-in poses, or "Add Pose LoRA" to create one.
    </div>
  {:else}
    <div class="lora-table">
      <div class="table-header">
        <span class="col-enabled">On</span>
        <span class="col-name">Name</span>
        <span class="col-keywords">Keywords</span>
        <span class="col-file">LoRA File</span>
        <span class="col-strength">Strength</span>
        <span class="col-actions"></span>
      </div>

      {#each loras as lora (lora.id)}
        <div class="table-row" class:disabled={!lora.enabled}>
          <span class="col-enabled">
            <button
              class="toggle-btn"
              class:on={lora.enabled}
              onclick={() => toggleEnabled(lora)}
              title={lora.enabled ? 'Disable' : 'Enable'}
            >
              {lora.enabled ? 'ON' : 'OFF'}
            </button>
          </span>
          <span class="col-name">{lora.name}</span>
          <span class="col-keywords" title={lora.keywords}>{lora.keywords}</span>
          <span class="col-file" title={lora.lora_filename}>{lora.lora_filename}</span>
          <span class="col-strength">{lora.strength.toFixed(2)}</span>
          <span class="col-actions">
            <button class="icon-btn" onclick={() => openEdit(lora)} title="Edit" disabled={formMode !== null}>
              ✏️
            </button>
            <button class="icon-btn danger" onclick={() => handleDelete(lora)} title="Delete">
              🗑️
            </button>
          </span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .pose-lora-manager {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .section-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .section-title {
    font-size: 0.95em;
    font-weight: 600;
    color: var(--text-primary);
  }

  .section-desc {
    font-size: 0.8em;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .header-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .error-msg {
    background: color-mix(in srgb, var(--accent-danger, #e55) 12%, var(--bg-secondary));
    border: 1px solid var(--accent-danger, #e55);
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 0.82em;
    color: var(--text-primary);
  }

  .loading-msg,
  .empty-msg {
    font-size: 0.85em;
    color: var(--text-muted);
    padding: 16px 0;
    text-align: center;
  }

  /* ── Form ── */

  .lora-form {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .form-title {
    font-size: 0.9em;
    font-weight: 600;
    color: var(--text-primary);
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field.full-width {
    grid-column: 1 / -1;
  }

  .field.enabled-field {
    justify-content: center;
  }

  .field label {
    font-size: 0.8em;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .field input[type="text"],
  .field input[type="number"].strength-number {
    background: var(--bg-primary);
    border: 1px solid var(--border-secondary);
    border-radius: 6px;
    padding: 6px 10px;
    color: var(--text-primary);
    font-size: 0.85em;
  }

  .field input[type="text"]:focus,
  .field input[type="number"]:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .field-hint {
    font-size: 0.72em;
    color: var(--text-muted);
  }

  .req {
    color: var(--accent-danger, #e55);
  }

  .strength-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .strength-row input[type="range"] {
    flex: 1;
    accent-color: var(--accent-primary);
  }

  .strength-number {
    width: 60px;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 0.85em;
    color: var(--text-secondary);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  /* ── Table ── */

  .lora-table {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    overflow: hidden;
    font-size: 0.82em;
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 52px 120px 1fr 1fr 64px 68px;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
  }

  .table-header {
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-primary);
    font-size: 0.78em;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .table-row {
    border-bottom: 1px solid var(--border-secondary);
    color: var(--text-primary);
    transition: background 0.1s;
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background: var(--bg-hover, var(--bg-secondary));
  }

  .table-row.disabled {
    opacity: 0.5;
  }

  .col-keywords,
  .col-file {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-strength {
    text-align: center;
  }

  .col-actions {
    display: flex;
    gap: 4px;
    justify-content: flex-end;
  }

  /* ── Buttons ── */

  .btn {
    padding: 6px 14px;
    border: none;
    border-radius: 6px;
    font-size: 0.83em;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s, background 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--accent-primary);
    color: #fff;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.85;
  }

  .btn-secondary {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover, var(--bg-tertiary, var(--bg-secondary)));
  }

  .toggle-btn {
    padding: 2px 7px;
    border-radius: 4px;
    border: 1px solid var(--border-secondary);
    font-size: 0.72em;
    font-weight: 700;
    cursor: pointer;
    background: var(--bg-primary);
    color: var(--text-muted);
    transition: all 0.15s;
  }

  .toggle-btn.on {
    background: color-mix(in srgb, var(--accent-primary) 15%, var(--bg-primary));
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .icon-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    font-size: 0.95em;
    line-height: 1;
    transition: background 0.1s;
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--bg-hover, var(--bg-secondary));
  }

  .icon-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon-btn.danger:hover {
    background: color-mix(in srgb, var(--accent-danger, #e55) 15%, transparent);
  }
</style>
