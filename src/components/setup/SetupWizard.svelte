<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { checkSetupStatus, installDependency, checkOllamaGpuUsage } from '$lib/api/setup';
  import { getConfig, updateConfig } from '$lib/api/config';
  import type { SetupStatus, SetupProgress, DependencyStatus, GpuInfo, OllamaGpuStatus } from '$lib/types/setup';
  import DependencyItem from './DependencyItem.svelte';

  let { oncomplete }: { oncomplete?: () => void } = $props();

  // ── Static metadata ──────────────────────────────────────────────────────
  const DEP_LABELS: Record<string, string> = {
    ollama:                 'Ollama Runtime',
    ollama_model:           'Story Model (Story_v27)',
    comfyui:                'ComfyUI',
    comfyui_torch:          'PyTorch GPU (ComfyUI)',
    checkpoint_juggernaut:  'Checkpoint: JuggernautXL Ragnarok',
    checkpoint_animagine:   'Checkpoint: Animagine XL 3.1',
    custom_node_ipadapter:  'Custom Node: IP-Adapter Plus',
    ipadapter_faceid_model: 'IP-Adapter FaceID Model',
    ipadapter_faceid_lora:  'IP-Adapter FaceID LoRA',
    insightface_buffalo_l:  'InsightFace Buffalo Model',
    clip_vision:            'CLIP Vision Model',
    custom_node_controlnet_aux: 'Custom Node: ControlNet Aux',
    controlnet_openpose_model:  'ControlNet OpenPose SDXL Model',
  };

  const DEP_SIZES: Record<string, string> = {
    ollama:                 '~270 MB',
    ollama_model:           '~6.6 GB',
    comfyui:                '~100 MB',
    comfyui_torch:          '~2.5 GB',
    checkpoint_juggernaut:  '~6.8 GB',
    checkpoint_animagine:   '~6.9 GB',
    custom_node_ipadapter:  '~5 MB',
    ipadapter_faceid_model: '~860 MB',
    ipadapter_faceid_lora:  '~150 MB',
    insightface_buffalo_l:  '~700 MB',
    clip_vision:            '~3.9 GB',
    custom_node_controlnet_aux: '~50 MB',
    controlnet_openpose_model:  '~2.5 GB',
  };

  // Install order matches backend's expected dependency order
  const DEP_ORDER = [
    'ollama', 'ollama_model', 'comfyui', 'comfyui_torch',
    'checkpoint_juggernaut', 'checkpoint_animagine',
    'custom_node_ipadapter', 'ipadapter_faceid_model',
    'ipadapter_faceid_lora', 'insightface_buffalo_l', 'clip_vision',
    'custom_node_controlnet_aux', 'controlnet_openpose_model',
  ];

  // ── State ─────────────────────────────────────────────────────────────────
  let setupStatus     = $state<SetupStatus | null>(null);
  let ollamaGpuStatus = $state<OllamaGpuStatus | null>(null);
  let isChecking   = $state(true);
  let isInstalling = $state(false);
  let currentStep  = $state('');      // dep name being installed right now
  let progressPct  = $state(0);       // 0-1 for the current dep
  let progressMsg  = $state('');
  let queueIndex   = $state(0);       // which item in the selected queue
  let queueTotal   = $state(0);       // how many items to install this run
  let errors       = $state<string[]>([]);
  let itemErrors   = $state<Map<string, string>>(new Map());
  let selected     = $state<Set<string>>(new Set());

  // ── GPU-aware label override ──────────────────────────────────────────────
  let torchLabel = $derived(() => {
    if (!setupStatus) return DEP_LABELS.comfyui_torch;
    const v = setupStatus.gpu_info.vendor;
    if (v === 'amd') return 'PyTorch DirectML (AMD GPU)';
    if (v === 'nvidia') return 'PyTorch CUDA (NVIDIA GPU)';
    return DEP_LABELS.comfyui_torch;
  });

  function getLabel(name: string): string {
    if (name === 'comfyui_torch') return torchLabel();
    return DEP_LABELS[name] ?? name;
  }

  // ── Derived ───────────────────────────────────────────────────────────────
  let allDeps = $derived<DependencyStatus[]>(
    setupStatus
      ? [
          setupStatus.ollama,
          setupStatus.ollama_model,
          setupStatus.comfyui,
          setupStatus.comfyui_torch,
          ...setupStatus.checkpoints,
          ...setupStatus.custom_nodes,
        ]
      : []
  );

  // Items that are selected AND still missing (what will actually be installed)
  let toInstall = $derived(
    allDeps.filter(d => selected.has(d.name) && !d.installed)
  );

  let selectedCount = $derived(toInstall.length);
  let allInstalled  = $derived(setupStatus?.all_ready ?? false);

  // Overall progress: each queued item accounts for 1/queueTotal of the bar
  let overallPct = $derived(
    queueTotal > 0 ? (queueIndex + progressPct) / queueTotal : 0
  );

  // ── Helpers ───────────────────────────────────────────────────────────────
  function getInstallState(dep: DependencyStatus): 'idle' | 'queued' | 'installing' | 'done' | 'error' {
    if (dep.installed) return 'done';
    if (itemErrors.has(dep.name)) return 'error';
    if (isInstalling && currentStep === dep.name) return 'installing';
    if (isInstalling && selected.has(dep.name)) return 'queued';
    return 'idle';
  }

  async function markDone() {
    try {
      const cfg = await getConfig();
      await updateConfig({ ...cfg, setup_completed: true });
    } catch (e) {
      console.error('Failed to save setup_completed:', e);
    }
  }

  async function doCheck() {
    isChecking = true;
    try {
      [setupStatus, ollamaGpuStatus] = await Promise.all([
        checkSetupStatus(),
        checkOllamaGpuUsage().catch(() => null),
      ]);
      // Auto-select all missing on first load; preserve selection on rechecks
      if (selected.size === 0 && setupStatus) {
        const missing = [
          setupStatus.ollama,
          setupStatus.ollama_model,
          setupStatus.comfyui,
          setupStatus.comfyui_torch,
          ...setupStatus.checkpoints,
          ...setupStatus.custom_nodes,
        ].filter(d => !d.installed).map(d => d.name);
        selected = new Set(missing);
      }
    } catch (e) {
      errors = [...errors, `Status check failed: ${e}`];
    } finally {
      isChecking = false;
    }
  }

  // ── Selection ─────────────────────────────────────────────────────────────
  function toggleSelection(name: string) {
    const next = new Set(selected);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    selected = next;
  }

  function selectAllMissing() {
    selected = new Set(allDeps.filter(d => !d.installed).map(d => d.name));
  }

  function selectNone() {
    selected = new Set();
  }

  // ── Install ───────────────────────────────────────────────────────────────
  async function installSelected() {
    // Install in canonical order, filtered to selection
    const queue = DEP_ORDER.filter(name =>
      selected.has(name) && allDeps.some(d => d.name === name && !d.installed)
    );
    if (queue.length === 0) return;

    queueTotal   = queue.length;
    queueIndex   = 0;
    isInstalling = true;
    errors       = [];
    itemErrors   = new Map();
    progressPct  = 0;

    for (let i = 0; i < queue.length; i++) {
      const name = queue[i];
      queueIndex  = i;
      currentStep = name;
      progressPct = 0;
      progressMsg = `Starting ${DEP_LABELS[name] ?? name}…`;

      try {
        await installDependency(name);
      } catch (e) {
        const msg = String(e);
        itemErrors = new Map([...itemErrors, [name, msg]]);
        errors = [...errors, `${DEP_LABELS[name] ?? name}: ${msg}`];
        break; // stop on first failure to avoid cascading
      }
    }

    isInstalling = false;
    currentStep  = '';
    queueIndex   = 0;
    queueTotal   = 0;

    await doCheck();

    if (setupStatus?.all_ready) {
      await markDone();
      oncomplete?.();
    }
  }

  async function skip() {
    await markDone();
    oncomplete?.();
  }

  // ── Event listener + initial check ────────────────────────────────────────
  $effect(() => {
    doCheck();

    let unlisten: (() => void) | undefined;

    listen<SetupProgress>('setup-progress', (event) => {
      const p = event.payload;
      // Only update step/progress for the dep currently being installed
      if (p.step === currentStep || p.step === 'all') {
        progressPct = p.progress_pct;
        progressMsg = p.message;
      }
      if (p.is_error) {
        errors = [...errors, p.message];
      }
    }).then(fn => { unlisten = fn; });

    return () => { unlisten?.(); };
  });
</script>

<div class="wizard">
  <!-- Header -->
  <div class="wizard-header">
    <div class="wizard-icon">⚙</div>
    <h1>First-time Setup</h1>
    <p>Check the status of each dependency below. Select what you want to install and click <strong>Install Selected</strong>.</p>
  </div>

  <!-- GPU badge -->
  {#if setupStatus}
    {@const gpu = setupStatus.gpu_info}
    <div class="gpu-badge gpu-{gpu.vendor}">
      <div class="gpu-badge-row">
        {#if gpu.vendor === 'nvidia'}🟢{:else if gpu.vendor === 'amd'}🔴{:else}⚪{/if}
        GPU: {gpu.name}
        {#if gpu.vendor === 'amd'}<span class="gpu-note">→ DirectML mode</span>{/if}
        {#if gpu.vendor === 'nvidia'}<span class="gpu-note">→ CUDA mode</span>{/if}
        {#if ollamaGpuStatus}
          <span class="gpu-note gpu-ollama-status">
            · Ollama: {ollamaGpuStatus.using_gpu ? '⚡ GPU' : '🐌 CPU'} ({ollamaGpuStatus.processor_info})
          </span>
        {/if}
      </div>
      {#if gpu.vendor === 'amd' && ollamaGpuStatus && !ollamaGpuStatus.using_gpu && ollamaGpuStatus.processor_info !== 'No model currently loaded' && ollamaGpuStatus.processor_info !== 'Ollama not found'}
        <div class="gpu-rocm-warning">{gpu.notes}</div>
      {/if}
    </div>
  {/if}

  <!-- Dependency list -->
  <div class="dep-list">
    {#if isChecking}
      <div class="checking">
        <span class="spinner-lg"></span>
        <span>Checking dependencies…</span>
      </div>
    {:else if allDeps.length === 0}
      <div class="checking">Could not retrieve status. Check your installation.</div>
    {:else}
      {#each allDeps as dep (dep.name)}
        <div class="dep-item-wrap">
          <DependencyItem
            status={dep}
            label={getLabel(dep.name)}
            sizeBadge={DEP_SIZES[dep.name]}
            installState={getInstallState(dep)}
            isSelected={selected.has(dep.name)}
            disabled={isInstalling}
            progressPct={currentStep === dep.name ? progressPct : 0}
            errorMessage={itemErrors.get(dep.name)}
            onToggle={() => toggleSelection(dep.name)}
          />
        </div>
      {/each}
    {/if}
  </div>

  <!-- Selection toolbar -->
  {#if !isChecking && allDeps.length > 0 && !allInstalled}
    <div class="selection-toolbar">
      <button class="toolbar-btn" onclick={selectAllMissing} disabled={isInstalling}>
        Select All Missing
      </button>
      <button class="toolbar-btn" onclick={selectNone} disabled={isInstalling}>
        Deselect All
      </button>
      <span class="selection-summary">
        {selectedCount} item{selectedCount !== 1 ? 's' : ''} selected
      </span>
    </div>
  {/if}

  <!-- Overall progress (during install) -->
  {#if isInstalling}
    <div class="overall-progress">
      <div class="progress-header">
        <span>Installing {queueIndex + 1} of {queueTotal}</span>
        <span>{Math.round(overallPct * 100)}%</span>
      </div>
      <div class="progress-track">
        <div class="progress-fill" style="width: {Math.round(overallPct * 100)}%"></div>
      </div>
      <p class="progress-msg">{progressMsg}</p>
    </div>
  {/if}

  <!-- Error log -->
  {#if errors.length > 0}
    <div class="error-log">
      <div class="error-title">Errors</div>
      {#each errors as err}
        <div class="error-item">⚠ {err}</div>
      {/each}
    </div>
  {/if}

  <!-- Footer -->
  <div class="wizard-footer">
    {#if allInstalled}
      <div class="all-good">✓ All dependencies are installed</div>
      <button class="btn-primary" onclick={() => { markDone(); oncomplete?.(); }}>
        Launch App
      </button>
    {:else}
      <button
        class="btn-primary"
        onclick={installSelected}
        disabled={isInstalling || isChecking || selectedCount === 0}
      >
        {#if isInstalling}
          Installing…
        {:else}
          Install Selected ({selectedCount})
        {/if}
      </button>

      <button
        class="btn-secondary"
        onclick={skip}
        disabled={isInstalling}
        title="Skip if you already have these tools installed"
      >
        Skip Setup
      </button>
    {/if}

    {#if !isInstalling && !isChecking}
      <button class="btn-ghost" onclick={doCheck} title="Re-check all dependency statuses">
        Recheck
      </button>
    {/if}
  </div>
</div>

<style>
  .wizard {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  /* ── Header ── */
  .wizard-header {
    padding: 28px 40px 20px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-primary);
    text-align: center;
  }

  .wizard-icon {
    font-size: 1.8rem;
    color: var(--accent-primary);
    margin-bottom: 6px;
  }

  .wizard-header h1 {
    margin: 0 0 6px;
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .wizard-header p {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-secondary);
    max-width: 500px;
    margin-inline: auto;
    line-height: 1.5;
  }

  /* ── GPU badge ── */
  .gpu-badge {
    padding: 6px 40px;
    font-size: 0.78rem;
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-primary);
    color: var(--text-secondary);
  }

  .gpu-badge-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .gpu-note {
    font-size: 0.72rem;
    color: var(--text-muted);
    margin-left: 4px;
  }

  .gpu-ollama-status {
    margin-left: 8px;
  }

  .gpu-rocm-warning {
    font-size: 0.72rem;
    color: var(--accent-danger);
    padding: 4px 0 2px;
    line-height: 1.4;
  }

  /* ── Dep list ── */
  .dep-list {
    flex: 1;
    overflow-y: auto;
    padding: 14px 40px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .dep-item-wrap {
    display: flex;
    flex-direction: column;
  }

  .checking {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--text-muted);
    font-size: 0.9rem;
    padding: 24px 0;
    justify-content: center;
  }

  .spinner-lg {
    display: inline-block;
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-primary);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  /* ── Selection toolbar ── */
  .selection-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 40px;
    border-top: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    flex-wrap: wrap;
  }

  .toolbar-btn {
    padding: 4px 12px;
    font-size: 0.78rem;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }

  .toolbar-btn:hover:not(:disabled) {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }

  .toolbar-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .selection-summary {
    margin-left: auto;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  /* ── Overall progress ── */
  .overall-progress {
    padding: 10px 40px;
    border-top: 1px solid var(--border-primary);
    background: var(--bg-secondary);
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    font-size: 0.78rem;
    color: var(--text-secondary);
    margin-bottom: 5px;
  }

  .progress-track {
    height: 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent-primary);
    border-radius: 3px;
    transition: width 0.3s ease;
    min-width: 2%;
  }

  .progress-msg {
    margin: 5px 0 0;
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Error log ── */
  .error-log {
    max-height: 90px;
    overflow-y: auto;
    padding: 6px 40px;
    background: color-mix(in srgb, var(--bg-secondary) 80%, var(--accent-danger));
    border-top: 1px solid var(--accent-danger);
  }

  .error-title {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--accent-danger);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 3px;
  }

  .error-item {
    font-size: 0.76rem;
    color: var(--text-primary);
    padding: 1px 0;
  }

  /* ── Footer ── */
  .wizard-footer {
    padding: 14px 40px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-primary);
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .all-good {
    flex: 1;
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--accent-success);
  }

  .btn-primary {
    padding: 9px 22px;
    background: var(--accent-primary);
    color: var(--text-inverse);
    border: none;
    border-radius: 5px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .btn-primary:hover:not(:disabled) { opacity: 0.88; }
  .btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-secondary {
    padding: 9px 18px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 5px;
    font-size: 0.875rem;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .btn-secondary:hover:not(:disabled) {
    border-color: var(--text-secondary);
    color: var(--text-primary);
  }

  .btn-secondary:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-ghost {
    margin-left: auto;
    padding: 8px 12px;
    background: transparent;
    color: var(--text-muted);
    border: none;
    border-radius: 4px;
    font-size: 0.78rem;
    cursor: pointer;
    transition: color 0.15s;
  }

  .btn-ghost:hover { color: var(--text-primary); }
</style>
