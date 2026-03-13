<!-- src/components/settings/SettingsModal.svelte — Tab container for settings -->
<script lang="ts">
  import Modal from '../shared/Modal.svelte';
  import ThemeSettings from './ThemeSettings.svelte';
  import ContentSettings from './ContentSettings.svelte';
  import WritingSettings from './WritingSettings.svelte';
  import ServiceSettings from './ServiceSettings.svelte';
  import AboutSection from './AboutSection.svelte';
  import PoseLoraManager from './PoseLoraManager.svelte';

  let {
    show = false,
    onclose,
  }: {
    show?: boolean;
    onclose?: () => void;
  } = $props();

  type Tab = 'writing' | 'theme' | 'content' | 'services' | 'poseloras' | 'about';
  let activeTab: Tab = $state('writing');
</script>

{#if show}
  <Modal title="⚙️ Settings" {onclose}>
    <div class="tabs">
      <button
        class="tab-btn"
        class:active={activeTab === 'writing'}
        onclick={() => (activeTab = 'writing')}
      >
        ✍️ Writing
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'theme'}
        onclick={() => (activeTab = 'theme')}
      >
        🎨 Theme
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'content'}
        onclick={() => (activeTab = 'content')}
      >
        🔒 Content
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'services'}
        onclick={() => (activeTab = 'services')}
      >
        🔧 Services
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'poseloras'}
        onclick={() => (activeTab = 'poseloras')}
      >
        🎭 Pose LoRAs
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'about'}
        onclick={() => (activeTab = 'about')}
      >
        ℹ️ About
      </button>
    </div>

    <div class="tab-content">
      {#if activeTab === 'writing'}
        <WritingSettings />
      {/if}
      {#if activeTab === 'theme'}
        <ThemeSettings />
      {/if}
      {#if activeTab === 'content'}
        <ContentSettings />
      {/if}
      {#if activeTab === 'services'}
        <ServiceSettings />
      {/if}
      {#if activeTab === 'poseloras'}
        <PoseLoraManager />
      {/if}
      {#if activeTab === 'about'}
        <AboutSection />
      {/if}
    </div>
  </Modal>
{/if}

<style>
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    padding: 0 20px;
  }

  .tab-btn {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 12px 16px;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 0.9em;
    transition: all 0.2s;
    margin-bottom: -1px;
  }

  .tab-btn:hover {
    color: var(--text-primary);
  }

  .tab-btn.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
  }

  .tab-content {
    padding: 20px 25px;
  }
</style>
