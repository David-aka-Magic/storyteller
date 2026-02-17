<!-- src/components/story/SceneHeader.svelte -->
<!--
  Horizontal header bar showing current scene context:
    - Location & type
    - Time of day / weather / mood
    - Active characters (clickable for profile)
    - Token meter (compact)
    - Settings access
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { SceneJson } from '$lib/llm-parser-types';
  import type { CharacterInScene, OrchestratorCompressionInfo } from '$lib/orchestrator-types';
  import type { CharacterProfile } from '$lib/character-types';
  import TokenMeter from './TokenMeter.svelte';

  export let scene: SceneJson | null = null;
  export let characters: CharacterInScene[] = [];
  export let characterProfiles: CharacterProfile[] = [];
  export let compressionInfo: OrchestratorCompressionInfo | null = null;
  export let storyTitle: string = '';
  export let totalTurns: number = 0;

  const dispatch = createEventDispatcher();

  /** Find a full profile for a scene character by name or DB ID */
  function getProfile(char: CharacterInScene): CharacterProfile | undefined {
    if (char.db_id) {
      return characterProfiles.find(p => p.id === char.db_id);
    }
    return characterProfiles.find(p => p.name.toLowerCase() === char.name.toLowerCase());
  }

  function profileImage(profile: CharacterProfile | undefined): string | null {
    if (!profile) return null;
    if (profile.image) return `data:image/png;base64,${profile.image}`;
    if (profile.master_image_path) {
      try { return convertFileSrc(profile.master_image_path); }
      catch { return null; }
    }
    return null;
  }

  function moodEmoji(mood: string): string {
    const m = mood.toLowerCase();
    if (m.includes('tense') || m.includes('danger')) return '⚡';
    if (m.includes('romantic') || m.includes('intimate')) return '💕';
    if (m.includes('mysterious') || m.includes('eerie')) return '🌀';
    if (m.includes('calm') || m.includes('peaceful')) return '🕊';
    if (m.includes('happy') || m.includes('joy')) return '✨';
    if (m.includes('sad') || m.includes('melan')) return '🌧';
    if (m.includes('dark') || m.includes('ominous')) return '🌑';
    return '◈';
  }

  function timeIcon(time: string): string {
    const t = time.toLowerCase();
    if (t.includes('night') || t.includes('midnight')) return '🌙';
    if (t.includes('dawn') || t.includes('sunrise')) return '🌅';
    if (t.includes('dusk') || t.includes('sunset') || t.includes('evening')) return '🌇';
    if (t.includes('morning')) return '🌤';
    return '☀️';
  }

  let showCharacterPanel = false;
</script>

<header class="scene-header">
  <!-- Left: Story Title & Scene Info -->
  <div class="header-left">
    <div class="story-identity">
      {#if storyTitle}
        <h1 class="story-title">{storyTitle}</h1>
      {/if}
      {#if totalTurns > 0}
        <span class="turn-counter">Turn {totalTurns}</span>
      {/if}
    </div>

    {#if scene}
      <div class="scene-pills">
        {#if scene.location}
          <span class="pill location-pill" title="Location: {scene.location}">
            📍 {scene.location}
            {#if scene.location_type}
              <span class="pill-sub">({scene.location_type})</span>
            {/if}
          </span>
        {/if}
        {#if scene.time_of_day}
          <span class="pill time-pill" title="Time: {scene.time_of_day}">
            {timeIcon(scene.time_of_day)} {scene.time_of_day}
          </span>
        {/if}
        {#if scene.weather && scene.weather !== 'clear' && scene.weather !== 'none'}
          <span class="pill weather-pill" title="Weather: {scene.weather}">
            🌦 {scene.weather}
          </span>
        {/if}
        {#if scene.mood}
          <span class="pill mood-pill" title="Mood: {scene.mood}">
            {moodEmoji(scene.mood)} {scene.mood}
          </span>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Right: Characters & Token Meter -->
  <div class="header-right">
    <!-- Character Avatars -->
    {#if characters.length > 0}
      <button
        class="characters-toggle"
        on:click={() => showCharacterPanel = !showCharacterPanel}
        title="Characters in scene"
      >
        <div class="avatar-stack">
          {#each characters.slice(0, 4) as char}
            {@const profile = getProfile(char)}
            {@const img = profileImage(profile)}
            <div class="mini-avatar" class:has-image={!!img}>
              {#if img}
                <img src={img} alt={char.name} />
              {:else}
                <span>{char.name.charAt(0)}</span>
              {/if}
            </div>
          {/each}
          {#if characters.length > 4}
            <div class="mini-avatar overflow">+{characters.length - 4}</div>
          {/if}
        </div>
        <span class="char-count">{characters.length}</span>
      </button>
    {/if}

    <!-- Token Meter (compact) -->
    <TokenMeter info={compressionInfo} compact={true} />

    <!-- Menu -->
    <button class="header-menu-btn" on:click={() => dispatch('openSettings')} title="Settings">
      ⚙
    </button>
  </div>
</header>

<!-- Character Detail Dropdown -->
{#if showCharacterPanel && characters.length > 0}
  <div class="character-panel-backdrop" on:click={() => showCharacterPanel = false} on:keydown={(e) => e.key === 'Escape' && (showCharacterPanel = false)} role="button" tabindex="-1">
    <!-- empty, just catches clicks -->
  </div>
  <div class="character-panel">
    <div class="panel-title">Characters in Scene</div>
    {#each characters as char}
      {@const profile = getProfile(char)}
      {@const img = profileImage(profile)}
      <button
        class="character-row"
        on:click={() => { dispatch('characterClick', { char, profile }); showCharacterPanel = false; }}
      >
        <div class="row-avatar" class:has-image={!!img}>
          {#if img}
            <img src={img} alt={char.name} />
          {:else}
            <span>{char.name.charAt(0)}</span>
          {/if}
        </div>
        <div class="row-info">
          <span class="row-name">{char.name}</span>
          <span class="row-detail">
            {char.expression || ''}{char.action ? (char.expression ? ' · ' : '') + char.action : ''}
          </span>
        </div>
        <div class="row-region">{char.region}</div>
      </button>
    {/each}
  </div>
{/if}

<style>
  .scene-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 20px;
    background: var(--story-header-bg, rgba(0, 0, 0, 0.25));
    border-bottom: 1px solid var(--story-border, rgba(255, 255, 255, 0.06));
    gap: 16px;
    min-height: 48px;
    position: relative;
    z-index: 10;
    backdrop-filter: blur(12px);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 16px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .story-identity {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-shrink: 0;
  }

  .story-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary, #eaeaea);
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
  }

  .turn-counter {
    font-size: 0.7rem;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    color: var(--text-muted, #6e7681);
    white-space: nowrap;
  }

  /* ── Scene Pills ── */
  .scene-pills {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .pill {
    padding: 3px 10px;
    border-radius: 12px;
    font-size: 0.7rem;
    font-weight: 500;
    background: var(--story-pill-bg, rgba(255, 255, 255, 0.05));
    color: var(--story-pill-text, rgba(255, 255, 255, 0.6));
    white-space: nowrap;
    text-transform: capitalize;
    border: 1px solid var(--story-pill-border, rgba(255, 255, 255, 0.06));
  }

  .pill-sub {
    opacity: 0.55;
    font-weight: 400;
  }

  /* ── Right Section ── */
  .header-right {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-shrink: 0;
  }

  .characters-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px 4px 4px;
    border-radius: 20px;
    background: var(--story-chip-bg, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--story-chip-border, rgba(255, 255, 255, 0.08));
    cursor: pointer;
    transition: background 0.15s;
    color: var(--text-secondary, #8b949e);
  }

  .characters-toggle:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .avatar-stack {
    display: flex;
  }

  .mini-avatar {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    font-size: 0.65rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px solid var(--story-header-bg, rgba(0, 0, 0, 0.6));
    margin-left: -6px;
    overflow: hidden;
  }

  .mini-avatar:first-child {
    margin-left: 0;
  }

  .mini-avatar.has-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .mini-avatar.overflow {
    background: var(--story-chip-bg, rgba(255, 255, 255, 0.15));
    color: var(--text-muted, #6e7681);
    font-size: 0.6rem;
  }

  .char-count {
    font-size: 0.7rem;
    font-weight: 600;
  }

  .header-menu-btn {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    background: transparent;
    border: 1px solid var(--story-pill-border, rgba(255, 255, 255, 0.08));
    color: var(--text-muted, #6e7681);
    cursor: pointer;
    font-size: 0.9rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, color 0.15s;
  }

  .header-menu-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary, #eaeaea);
  }

  /* ── Character Panel Dropdown ── */
  .character-panel-backdrop {
    position: fixed;
    inset: 0;
    z-index: 99;
  }

  .character-panel {
    position: absolute;
    top: 100%;
    right: 60px;
    width: 280px;
    max-height: 320px;
    overflow-y: auto;
    background: var(--bg-secondary, #161b22);
    border: 1px solid var(--border-primary, #30363d);
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    z-index: 100;
    padding: 6px;
  }

  .panel-title {
    padding: 8px 10px 6px;
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #6e7681);
  }

  .character-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    background: transparent;
    border: none;
    width: 100%;
    text-align: left;
    color: var(--text-primary, #c9d1d9);
    transition: background 0.12s;
  }

  .character-row:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .row-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    color: var(--text-inverse, #0d1117);
    font-size: 0.8rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    flex-shrink: 0;
  }

  .row-avatar.has-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .row-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .row-name {
    font-size: 0.85rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row-detail {
    font-size: 0.72rem;
    color: var(--text-muted, #6e7681);
    font-style: italic;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row-region {
    font-size: 0.65rem;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    color: var(--text-muted, #6e7681);
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.04);
    text-transform: uppercase;
    flex-shrink: 0;
  }
</style>