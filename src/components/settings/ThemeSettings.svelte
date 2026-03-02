<!-- src/components/settings/ThemeSettings.svelte — Theme picker grid -->
<script lang="ts">
  import { currentTheme, themes, type ThemeName } from '$lib/stores/theme';

  function selectTheme(theme: ThemeName) {
    currentTheme.set(theme);
  }
</script>

<div class="theme-grid">
  {#each Object.values(themes) as theme}
    <button
      class="theme-option"
      class:selected={$currentTheme === theme.name}
      onclick={() => selectTheme(theme.name)}
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

<style>
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
</style>
