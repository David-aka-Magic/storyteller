<!-- src/components/settings/ImageSettings.svelte — Image generation settings -->
<script lang="ts">
  import { getConfig, updateConfig } from '$lib/api/config';
  import { listCustomPoses, addCustomPose, deleteCustomPose, scanAvailablePoses } from '$lib/api/custom-assets';
  import type { CustomPose } from '$lib/types';

  let enabled = $state(true);
  let strength = $state(0.85);
  let saving = $state(false);
  let customPoses = $state<CustomPose[]>([]);
  let addingPose = $state(false);
  let newPoseName = $state('');
  let showAddPose = $state(false);
  let availablePoseFiles = $state<string[]>([]);
  let selectedPoseFile = $state('');

  $effect(() => {
    loadSettings();
    listCustomPoses().then(p => { customPoses = p; }).catch(() => {});
  });

  async function loadSettings() {
    try {
      const config = await getConfig();
      enabled = config.controlnet_pose_enabled ?? true;
      strength = config.controlnet_pose_strength ?? 0.85;
    } catch (e) {
      console.error('[ImageSettings] Failed to load:', e);
    }
  }

  async function save() {
    if (saving) return;
    saving = true;
    try {
      const config = await getConfig();
      await updateConfig({ ...config, controlnet_pose_enabled: enabled, controlnet_pose_strength: strength });
    } catch (e) {
      console.error('[ImageSettings] Failed to save:', e);
    }
    saving = false;
  }

  function strengthLabel(v: number): string {
    if (v < 0.4) return 'Very subtle';
    if (v < 0.6) return 'Soft';
    if (v < 0.75) return 'Moderate';
    if (v < 0.9) return 'Strong (recommended)';
    return 'Maximum';
  }

  async function openAddPose() {
    showAddPose = true;
    newPoseName = '';
    selectedPoseFile = '';
    try {
      const all = await scanAvailablePoses();
      const registered = new Set(customPoses.map(p => p.filename));
      availablePoseFiles = all.filter(f => !registered.has(f));
    } catch {
      availablePoseFiles = [];
    }
  }

  async function handleAddPose() {
    if (!newPoseName.trim() || !selectedPoseFile) return;
    addingPose = true;
    try {
      const pose = await addCustomPose(newPoseName.trim(), selectedPoseFile);
      customPoses = [...customPoses, pose];
      newPoseName = '';
      selectedPoseFile = '';
      showAddPose = false;
    } catch (e) {
      console.error('[ImageSettings] Failed to add pose:', e);
    }
    addingPose = false;
  }

  async function handleDeletePose(pose: CustomPose) {
    if (!confirm(`Delete pose "${pose.display_name}"?`)) return;
    try {
      await deleteCustomPose(pose.id);
      customPoses = customPoses.filter(p => p.id !== pose.id);
    } catch (e) {
      console.error('[ImageSettings] Failed to delete pose:', e);
    }
  }
</script>

<div class="image-settings">
  <div class="section-header">Pose Control (ControlNet)</div>
  <p class="section-desc">
    When enabled, pre-generated OpenPose skeleton images guide the character's body position.
    The pose is detected from the LLM's output or inferred from scene keywords.
  </p>

  <div class="setting-row">
    <label class="toggle-label" for="cn-toggle">
      <span class="label-text">Enable ControlNet Pose Guidance</span>
      <span class="label-sub">Requires ComfyUI ControlNet Aux nodes and OpenPoseXL2 model</span>
    </label>
    <button
      id="cn-toggle"
      class="toggle"
      class:on={enabled}
      onclick={() => { enabled = !enabled; save(); }}
      disabled={saving}
      aria-pressed={enabled}
    >
      <span class="thumb"></span>
    </button>
  </div>

  <div class="setting-row slider-row" class:dimmed={!enabled}>
    <div class="slider-header">
      <span class="label-text">ControlNet Strength</span>
      <span class="strength-value">{strength.toFixed(2)} — {strengthLabel(strength)}</span>
    </div>
    <input
      type="range"
      min="0"
      max="1"
      step="0.05"
      bind:value={strength}
      onchange={save}
      disabled={!enabled || saving}
      class="slider"
    />
    <div class="slider-ticks">
      <span>0.0</span>
      <span>0.25</span>
      <span>0.5</span>
      <span>0.75</span>
      <span>1.0</span>
    </div>
    <p class="hint">Higher values = stronger pose adherence. Recommended: 0.7–0.9</p>
  </div>

  <!-- Custom Poses -->
  <div class="section-header" style="margin-top: 24px;">Custom Pose Skeletons</div>
  <p class="section-desc">
    Add your own OpenPose skeleton PNGs to use as ControlNet guidance.
    These appear alongside the built-in poses during scene generation.
  </p>

  {#if customPoses.length > 0}
    <div class="pose-list">
      {#each customPoses as pose (pose.id)}
        <div class="pose-item">
          <span class="pose-name">{pose.display_name}</span>
          <span class="pose-file">{pose.filename}</span>
          <button class="pose-delete" onclick={() => handleDeletePose(pose)} title="Remove">✕</button>
        </div>
      {/each}
    </div>
  {:else}
    <div class="no-poses">No custom poses added yet.</div>
  {/if}

  {#if showAddPose}
    <div class="add-pose-form">
      {#if availablePoseFiles.length === 0}
        <small class="add-pose-hint">No unregistered .png files found in the pose_skeletons directory. Place your PNG there first, then try again.</small>
        <button class="pose-cancel-btn" onclick={() => showAddPose = false}>Cancel</button>
      {:else}
        <div class="add-pose-field">
          <label>Display Name</label>
          <input type="text" bind:value={newPoseName} placeholder="e.g. Dancing" />
        </div>
        <div class="add-pose-field">
          <label>Pose File</label>
          <select bind:value={selectedPoseFile}>
            <option value="">Select a file…</option>
            {#each availablePoseFiles as f}
              <option value={f}>{f}</option>
            {/each}
          </select>
        </div>
        <div class="add-pose-actions">
          <button class="pose-cancel-btn" onclick={() => showAddPose = false}>Cancel</button>
          <button class="add-pose-btn" onclick={handleAddPose} disabled={addingPose || !newPoseName.trim() || !selectedPoseFile}>
            {addingPose ? 'Adding…' : 'Add'}
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <button class="add-pose-btn" onclick={openAddPose}>+ Add Pose…</button>
  {/if}
</div>

<style>
  .image-settings {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .section-header {
    font-size: 0.95em;
    font-weight: 600;
    color: var(--text-primary);
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border-secondary);
  }

  .section-desc {
    font-size: 0.8em;
    color: var(--text-muted);
    margin: 0;
    line-height: 1.5;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .slider-row {
    flex-direction: column;
    align-items: stretch;
    transition: opacity 0.2s;
  }

  .slider-row.dimmed {
    opacity: 0.45;
    pointer-events: none;
  }

  .toggle-label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    cursor: pointer;
  }

  .label-text {
    font-size: 0.9em;
    font-weight: 500;
    color: var(--text-primary);
  }

  .label-sub {
    font-size: 0.75em;
    color: var(--text-muted);
  }

  /* Toggle switch */
  .toggle {
    position: relative;
    width: 44px;
    height: 24px;
    flex-shrink: 0;
    border-radius: 12px;
    border: none;
    background: var(--border-secondary);
    cursor: pointer;
    transition: background 0.2s;
    padding: 0;
  }

  .toggle.on {
    background: var(--accent-primary);
  }

  .toggle:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    transition: transform 0.2s;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  .toggle.on .thumb {
    transform: translateX(20px);
  }

  /* Slider */
  .slider-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 6px;
  }

  .strength-value {
    font-size: 0.8em;
    color: var(--accent-primary);
    font-weight: 500;
  }

  .slider {
    width: 100%;
    accent-color: var(--accent-primary);
    cursor: pointer;
  }

  .slider-ticks {
    display: flex;
    justify-content: space-between;
    font-size: 0.7em;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .hint {
    font-size: 0.75em;
    color: var(--text-muted);
    margin: 4px 0 0;
  }

  .pose-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .pose-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--bg-tertiary, #21262d);
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .pose-name {
    font-weight: 600;
    color: var(--text-primary, #c9d1d9);
    flex: 1;
  }

  .pose-file {
    color: var(--text-muted, #8b949e);
    font-size: 0.75rem;
    font-family: monospace;
  }

  .pose-delete {
    background: none;
    border: none;
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    font-size: 0.9rem;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .pose-delete:hover {
    background: rgba(255, 100, 100, 0.15);
    color: #f85149;
  }

  .no-poses {
    font-size: 0.82rem;
    color: var(--text-muted, #8b949e);
    padding: 8px 0;
  }

  .add-pose-btn {
    padding: 6px 14px;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .add-pose-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .add-pose-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-secondary, #30363d);
    border-radius: 8px;
    margin-top: 4px;
  }

  .add-pose-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .add-pose-field label {
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .add-pose-field input,
  .add-pose-field select {
    padding: 6px 8px;
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-secondary, #30363d);
    border-radius: 6px;
    color: var(--text-primary, #c9d1d9);
    font-size: 0.85rem;
  }

  .add-pose-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 4px;
  }

  .pose-cancel-btn {
    background: none;
    border: none;
    color: var(--accent-primary, #58a6ff);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0;
  }

  .pose-cancel-btn:hover {
    text-decoration: underline;
  }

  .add-pose-hint {
    font-size: 0.78rem;
    color: var(--text-muted, #8b949e);
  }
</style>
