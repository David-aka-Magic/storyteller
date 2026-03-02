<!-- src/components/settings/ContentSettings.svelte — Content rating toggle -->
<script lang="ts">
  import { getConfig, updateConfig } from '$lib/api/config';

  let contentRating = $state<'sfw' | 'nsfw'>('sfw');
  let saving = $state(false);

  $effect(() => {
    loadSetting();
  });

  async function loadSetting() {
    try {
      const config = await getConfig();
      contentRating = config.content_rating ?? 'sfw';
    } catch (e) {
      console.error('Failed to load content setting:', e);
    }
  }

  async function toggle() {
    const newRating = contentRating === 'sfw' ? 'nsfw' : 'sfw';
    saving = true;
    try {
      const config = await getConfig();
      await updateConfig({ ...config, content_rating: newRating });
      contentRating = newRating;
    } catch (e) {
      console.error('Failed to save content setting:', e);
    }
    saving = false;
  }
</script>

<div class="content-setting">
  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Content Rating</span>
      <span class="setting-desc">
        {#if contentRating === 'sfw'}
          Safe for work — story text and images will avoid explicit content.
        {:else}
          Unrestricted — no content filters applied to story or image generation.
        {/if}
      </span>
    </div>
    <button
      class="toggle-btn"
      class:nsfw={contentRating === 'nsfw'}
      onclick={toggle}
      disabled={saving}
    >
      <span class="toggle-track">
        <span class="toggle-thumb"></span>
      </span>
      <span class="toggle-label">{contentRating.toUpperCase()}</span>
    </button>
  </div>
</div>

<style>
  .content-setting {
    margin-bottom: 12px;
  }

  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: 8px;
  }

  .setting-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .setting-label {
    font-size: 0.95em;
    font-weight: 600;
    color: var(--text-primary);
  }

  .setting-desc {
    font-size: 0.8em;
    color: var(--text-muted);
    max-width: 300px;
  }

  .toggle-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
  }

  .toggle-btn:disabled {
    opacity: 0.5;
    cursor: wait;
  }

  .toggle-track {
    position: relative;
    width: 44px;
    height: 24px;
    background: var(--accent-success);
    border-radius: 12px;
    transition: background 0.2s;
  }

  .toggle-btn.nsfw .toggle-track {
    background: var(--accent-danger);
  }

  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    background: white;
    border-radius: 50%;
    transition: transform 0.2s;
  }

  .toggle-btn.nsfw .toggle-thumb {
    transform: translateX(20px);
  }

  .toggle-label {
    font-size: 0.8em;
    font-weight: 700;
    color: var(--text-secondary);
    min-width: 36px;
  }
</style>
