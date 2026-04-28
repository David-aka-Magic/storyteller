<!-- src/components/settings/WritingSettings.svelte — AI response length setting -->
<script lang="ts">
  import { getConfig, updateConfig } from '$lib/api/config';
  import type { AppConfig } from '$lib/api/config';

  type ResponseLength = AppConfig['response_length'];

  let responseLength = $state<ResponseLength>('medium');
  let keepAlive = $state('30m');
  let saving = $state(false);

  $effect(() => {
    loadSettings();
  });

  async function loadSettings() {
    try {
      const config = await getConfig();
      responseLength = config.response_length ?? 'medium';
      keepAlive = config.keep_alive ?? '30m';
    } catch (e) {
      console.error('[WritingSettings] Failed to load:', e);
    }
  }

  async function setLength(val: ResponseLength) {
    if (val === responseLength || saving) return;
    saving = true;
    try {
      const config = await getConfig();
      await updateConfig({ ...config, response_length: val });
      responseLength = val;
    } catch (e) {
      console.error('[WritingSettings] Failed to save:', e);
    }
    saving = false;
  }

  async function setKeepAlive(val: string) {
    if (val === keepAlive || saving) return;
    saving = true;
    try {
      const config = await getConfig();
      await updateConfig({ ...config, keep_alive: val });
      keepAlive = val;
    } catch (e) {
      console.error('[WritingSettings] Failed to save keep_alive:', e);
    }
    saving = false;
  }

  const options = [
    {
      value: 'short' as ResponseLength,
      label: 'Short',
      detail: '~1 paragraph',
      speed: 'Fastest  (~5–10s)',
    },
    {
      value: 'medium' as ResponseLength,
      label: 'Medium',
      detail: '2–3 paragraphs',
      speed: 'Balanced — default  (~10–20s)',
    },
    {
      value: 'long' as ResponseLength,
      label: 'Long',
      detail: '3–4 paragraphs',
      speed: 'Most detailed · Slowest  (~20–40s)',
    },
  ];

  const keepAliveOptions = [
    { value: '-1', label: 'Always loaded' },
    { value: '1h', label: '1 hour' },
    { value: '30m', label: '30 minutes (default)' },
    { value: '5m', label: '5 minutes' },
    { value: '0', label: 'Unload after each turn' },
  ];
</script>

<div class="writing-settings">
  <div class="setting-group">
    <div class="group-label">Response Length</div>
    <div class="group-desc">Controls how much prose the AI writes per turn.</div>

    <div class="length-cards" class:disabled={saving}>
      {#each options as opt}
        <button
          class="length-card"
          class:active={responseLength === opt.value}
          onclick={() => setLength(opt.value)}
          disabled={saving}
        >
          <span class="card-label">{opt.label}</span>
          <span class="card-detail">{opt.detail}</span>
          <span class="card-speed">{opt.speed}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="setting-group">
    <div class="group-label">Story Model — VRAM Residence</div>
    <div class="group-desc">Controls how long the story model stays loaded in VRAM between turns.</div>

    <select
      class="keep-alive-select"
      value={keepAlive}
      onchange={(e) => setKeepAlive((e.target as HTMLSelectElement).value)}
      disabled={saving}
    >
      {#each keepAliveOptions as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>

    <div class="help-text">
      Higher values keep the story model in VRAM longer, making turns feel instant. Lower values free VRAM faster for image generation. If image generation feels slow, try a lower value.
    </div>
  </div>
</div>

<style>
  .writing-settings {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group-label {
    font-size: 0.95em;
    font-weight: 600;
    color: var(--text-primary);
  }

  .group-desc {
    font-size: 0.8em;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .length-cards {
    display: flex;
    gap: 10px;
  }

  .length-cards.disabled {
    opacity: 0.6;
    pointer-events: none;
  }

  .length-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px 14px;
    background: var(--bg-secondary);
    border: 2px solid var(--border-secondary);
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.15s, background 0.15s;
  }

  .length-card:hover {
    border-color: var(--border-active, var(--accent-primary));
    background: var(--bg-hover);
  }

  .length-card.active {
    border-color: var(--accent-primary);
    background: color-mix(in srgb, var(--accent-primary) 8%, var(--bg-secondary));
  }

  .card-label {
    font-size: 0.95em;
    font-weight: 700;
    color: var(--text-primary);
  }

  .length-card.active .card-label {
    color: var(--accent-primary);
  }

  .card-detail {
    font-size: 0.8em;
    color: var(--text-secondary);
  }

  .card-speed {
    font-size: 0.72em;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .keep-alive-select {
    width: 100%;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border: 2px solid var(--border-secondary);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 0.9em;
    cursor: pointer;
    transition: border-color 0.15s;
  }

  .keep-alive-select:hover:not(:disabled) {
    border-color: var(--border-active, var(--accent-primary));
  }

  .keep-alive-select:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .keep-alive-select:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .help-text {
    font-size: 0.75em;
    color: var(--text-muted);
    line-height: 1.5;
    margin-top: 4px;
  }
</style>
