<!-- src/components/settings/ServiceSettings.svelte — Service status + app config -->
<script lang="ts">
  import { getConfig, updateConfig } from '$lib/api/config';
  import { checkServicesStatus, startServices as apiStartServices } from '$lib/api/services';

  let sdPath = $state('');
  let comfyuiPath = $state('');
  let autoStartServices = $state(true);
  let ollamaStatus = $state(false);
  let sdStatus = $state(false);
  let comfyuiStatus = $state(false);
  let checkingStatus = $state(false);
  let savingConfig = $state(false);

  $effect(() => {
    loadConfig();
    checkStatus();
  });

  async function loadConfig() {
    try {
      const config = await getConfig();
      sdPath = config.sd_webui_path || '';
      comfyuiPath = config.comfyui_path || '';
      autoStartServices = config.auto_start_services ?? true;
    } catch (e) {
      console.error('Failed to load config:', e);
    }
  }

  async function saveConfig() {
    savingConfig = true;
    try {
      const config = await getConfig();
      await updateConfig({
        ...config,
        sd_webui_path: sdPath,
        comfyui_path: comfyuiPath,
        auto_start_services: autoStartServices,
      });
    } catch (e) {
      console.error('Failed to save config:', e);
      alert('Failed to save settings: ' + e);
    }
    savingConfig = false;
  }

  async function checkStatus() {
    checkingStatus = true;
    try {
      const status = await checkServicesStatus();
      ollamaStatus = status.ollama_running;
      sdStatus = status.sd_running;
      comfyuiStatus = status.comfyui_running;
    } catch (e) {
      console.error('Failed to check status:', e);
    }
    checkingStatus = false;
  }

  async function startServices() {
    checkingStatus = true;
    try {
      const status = await apiStartServices();
      ollamaStatus = status.ollama_running;
      sdStatus = status.sd_running;
      comfyuiStatus = status.comfyui_running;
      if (status.ollama_error) console.error('Ollama error:', status.ollama_error);
      if (status.sd_error) console.error('SD error:', status.sd_error);
      if (status.comfyui_error) console.error('ComfyUI error:', status.comfyui_error);
    } catch (e) {
      console.error('Failed to start services:', e);
      alert('Failed to start services: ' + e);
    }
    checkingStatus = false;
  }
</script>

<div class="service-status">
  <div class="status-item">
    <span class="status-label">Ollama</span>
    <span class="status-indicator" class:online={ollamaStatus} class:offline={!ollamaStatus}>
      {ollamaStatus ? '● Online' : '○ Offline'}
    </span>
  </div>
  <div class="status-item">
    <span class="status-label">Stable Diffusion</span>
    <span class="status-indicator" class:online={sdStatus} class:offline={!sdStatus}>
      {sdStatus ? '● Online' : '○ Offline'}
    </span>
  </div>
  <div class="status-item">
    <span class="status-label">ComfyUI</span>
    <span class="status-indicator" class:online={comfyuiStatus} class:offline={!comfyuiStatus}>
      {comfyuiStatus ? '● Online' : '○ Offline'}
    </span>
  </div>
</div>

<div class="service-actions">
  <button class="action-btn" onclick={checkStatus} disabled={checkingStatus}>
    {checkingStatus ? '⏳ Checking...' : '🔄 Refresh Status'}
  </button>
  <button class="action-btn primary" onclick={startServices} disabled={checkingStatus}>
    {checkingStatus ? '⏳ Starting...' : '▶️ Start Services'}
  </button>
</div>

<div class="config-field">
  <label for="sd-path">Stable Diffusion Path</label>
  <input
    id="sd-path"
    type="text"
    bind:value={sdPath}
    onblur={saveConfig}
    placeholder="C:\path\to\stable-diffusion-webui"
  />
</div>

<div class="config-field">
  <label for="comfyui-path">ComfyUI Path</label>
  <input
    id="comfyui-path"
    type="text"
    bind:value={comfyuiPath}
    onblur={saveConfig}
    placeholder="C:\path\to\ComfyUI"
  />
</div>

<div class="config-field checkbox-field">
  <!-- svelte-ignore a11y_label_has_associated_control -->
  <label>
    <input
      type="checkbox"
      bind:checked={autoStartServices}
      onchange={saveConfig}
    />
    Auto-start services on app launch
  </label>
</div>

<style>
  .service-status {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 15px;
  }

  .status-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 15px;
    background: var(--bg-secondary);
    border-radius: 6px;
  }

  .status-label {
    font-weight: 500;
  }

  .status-indicator {
    font-size: 0.9em;
  }

  .status-indicator.online {
    color: var(--accent-success);
  }

  .status-indicator.offline {
    color: var(--text-muted);
  }

  .service-actions {
    display: flex;
    gap: 10px;
    margin-bottom: 20px;
  }

  .action-btn {
    flex: 1;
    padding: 10px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9em;
    transition: all 0.2s;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--bg-tertiary);
  }

  .action-btn.primary {
    background: var(--accent-primary);
    color: var(--text-inverse);
    border-color: var(--accent-primary);
  }

  .action-btn.primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .action-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .config-field {
    margin-bottom: 15px;
  }

  .config-field label {
    display: block;
    margin-bottom: 5px;
    font-size: 0.9em;
    color: var(--text-secondary);
  }

  .config-field input[type="text"] {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-radius: 6px;
    font-size: 0.9em;
    box-sizing: border-box;
  }

  .config-field input[type="text"]:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .checkbox-field label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }

  .checkbox-field input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }
</style>
