<!-- src/components/settings/ImageSettings.svelte — Image generation settings -->
<script lang="ts">
  import { getConfig, updateConfig } from '$lib/api/config';

  let enabled = $state(true);
  let strength = $state(0.85);
  let saving = $state(false);

  $effect(() => {
    loadSettings();
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
</style>
