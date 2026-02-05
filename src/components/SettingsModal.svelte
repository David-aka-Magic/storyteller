<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import { currentTheme, themes, type ThemeName } from '$lib/stores/theme';
    import { invoke } from '@tauri-apps/api/core';

    export let show = false;

    const dispatch = createEventDispatcher();

    let selectedTheme: ThemeName = 'light';
    let sdPath = '';
    let autoStartServices = true;
    let ollamaStatus = false;
    let sdStatus = false;
    let checkingStatus = false;
    let savingConfig = false;

    // Subscribe to theme changes
    currentTheme.subscribe(value => {
        selectedTheme = value;
    });

    // Reload data when modal opens
    $: if (show) {
        loadConfig();
        checkStatus();
    }

    async function loadConfig() {
        try {
            const config = await invoke('get_config') as any;
            sdPath = config.sd_webui_path || '';
            autoStartServices = config.auto_start_services ?? true;
        } catch (e) {
            console.error('Failed to load config:', e);
        }
    }

    async function saveConfig() {
        savingConfig = true;
        try {
            await invoke('update_config', {
                newConfig: {
                    sd_webui_path: sdPath,
                    ollama_url: 'http://localhost:11434',
                    sd_api_url: 'http://127.0.0.1:7860',
                    auto_start_services: autoStartServices
                }
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
            const status = await invoke('check_services_status') as any;
            ollamaStatus = status.ollama_running;
            sdStatus = status.sd_running;
        } catch (e) {
            console.error('Failed to check status:', e);
        }
        checkingStatus = false;
    }

    async function startServices() {
        checkingStatus = true;
        try {
            const status = await invoke('start_services') as any;
            ollamaStatus = status.ollama_running;
            sdStatus = status.sd_running;
            
            if (status.ollama_error) {
                console.error('Ollama error:', status.ollama_error);
            }
            if (status.sd_error) {
                console.error('SD error:', status.sd_error);
            }
        } catch (e) {
            console.error('Failed to start services:', e);
            alert('Failed to start services: ' + e);
        }
        checkingStatus = false;
    }

    function selectTheme(theme: ThemeName) {
        selectedTheme = theme;
        currentTheme.set(theme);
    }

    function close() {
        dispatch('close');
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') close();
    }

    function handleBackdropClick(e: MouseEvent) {
        if (e.target === e.currentTarget) {
            close();
        }
    }
</script>

{#if show}
<div 
    class="modal-backdrop" 
    on:click={handleBackdropClick}
    on:keydown={handleKeydown}
    role="button"
    tabindex="0"
>
    <div 
        class="modal" 
        on:click|stopPropagation
        role="dialog"
        tabindex="-1"
    >
        <div class="modal-header">
            <h2>⚙️ Settings</h2>
            <button class="close-btn" on:click={close}>✕</button>
        </div>

        <div class="modal-content">
            <!-- Theme Section -->
            <section class="settings-section">
                <h3>🎨 Theme</h3>
                <div class="theme-grid">
                    {#each Object.values(themes) as theme}
                        <button
                            class="theme-option"
                            class:selected={selectedTheme === theme.name}
                            on:click={() => selectTheme(theme.name)}
                            style="
                                --preview-bg: {theme.colors.bgPrimary};
                                --preview-secondary: {theme.colors.bgSecondary};
                                --preview-accent: {theme.colors.accentPrimary};
                                --preview-text: {theme.colors.textPrimary};
                            "
                        >
                            <div class="theme-preview">
                                <div class="preview-sidebar"></div>
                                <div class="preview-main">
                                    <div class="preview-header"></div>
                                    <div class="preview-content"></div>
                                </div>
                            </div>
                            <span class="theme-label">{theme.label}</span>
                        </button>
                    {/each}
                </div>
            </section>

            <!-- Services Section -->
            <section class="settings-section">
                <h3>🔧 Services</h3>
                
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
                </div>

                <div class="service-actions">
                    <button class="action-btn" on:click={checkStatus} disabled={checkingStatus}>
                        {checkingStatus ? '⏳ Checking...' : '🔄 Refresh Status'}
                    </button>
                    <button class="action-btn primary" on:click={startServices} disabled={checkingStatus}>
                        {checkingStatus ? '⏳ Starting...' : '▶️ Start Services'}
                    </button>
                </div>

                <div class="config-field">
                    <label for="sd-path">Stable Diffusion Path</label>
                    <input 
                        id="sd-path" 
                        type="text" 
                        bind:value={sdPath}
                        on:blur={saveConfig}
                        placeholder="C:\path\to\stable-diffusion-webui"
                    />
                </div>

                <div class="config-field checkbox-field">
                    <label>
                        <input 
                            type="checkbox" 
                            bind:checked={autoStartServices}
                            on:change={saveConfig}
                        />
                        Auto-start services on app launch
                    </label>
                </div>
            </section>

            <!-- About Section -->
            <section class="settings-section">
                <h3>ℹ️ About</h3>
                <p class="about-text">
                    AI Story Writer v0.1.0<br/>
                    Built with Tauri, Svelte, and Rust
                </p>
            </section>
        </div>
    </div>
</div>
{/if}

<style>
    .modal-backdrop {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.6);
        z-index: 3000;
        display: flex;
        justify-content: center;
        align-items: center;
    }

    .modal {
        background: var(--bg-primary);
        color: var(--text-primary);
        padding: 0;
        border-radius: 12px;
        width: 550px;
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

    .modal-content {
        padding: 20px 25px;
        overflow-y: auto;
        flex: 1;
    }

    .settings-section {
        margin-bottom: 30px;
    }

    .settings-section:last-child {
        margin-bottom: 0;
    }

    .settings-section h3 {
        margin: 0 0 15px 0;
        font-size: 1em;
        color: var(--text-secondary);
        border-bottom: 1px solid var(--border-secondary);
        padding-bottom: 8px;
    }

    /* Theme Grid */
    .theme-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 12px;
    }

    .theme-option {
        background: var(--bg-secondary);
        border: 2px solid var(--border-secondary);
        border-radius: 8px;
        padding: 10px;
        cursor: pointer;
        transition: all 0.2s;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
    }

    .theme-option:hover {
        border-color: var(--border-primary);
    }

    .theme-option.selected {
        border-color: var(--accent-primary);
        background: var(--bg-tertiary);
    }

    .theme-preview {
        width: 100%;
        height: 50px;
        border-radius: 4px;
        overflow: hidden;
        display: flex;
        background: var(--preview-bg);
    }

    .preview-sidebar {
        width: 25%;
        background: var(--preview-secondary);
    }

    .preview-main {
        flex: 1;
        display: flex;
        flex-direction: column;
        padding: 4px;
    }

    .preview-header {
        height: 8px;
        background: var(--preview-accent);
        border-radius: 2px;
        margin-bottom: 4px;
    }

    .preview-content {
        flex: 1;
        background: var(--preview-secondary);
        border-radius: 2px;
        opacity: 0.5;
    }

    .theme-label {
        font-size: 0.8em;
        color: var(--text-secondary);
    }

    /* Service Status */
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

    /* Config Fields */
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

    .about-text {
        color: var(--text-muted);
        font-size: 0.9em;
        line-height: 1.6;
    }
</style>