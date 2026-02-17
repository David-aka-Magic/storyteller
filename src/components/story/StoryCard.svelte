<!-- src/components/story/StoryCard.svelte -->
<!--
  Individual story card for the Story Home grid.
  Shows thumbnail, title, description, meta stats, hover "Continue" action.
  Dispatches: 'select' (load & play), 'delete' (with story_id)
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { StorySummary } from '$lib/story-manager-types';

  export let story: StorySummary;

  const dispatch = createEventDispatcher();

  // ── Thumbnail handling ──
  let thumbLoaded = false;
  let thumbError = false;

  function assetSrc(path: string): string {
    try { return convertFileSrc(path); }
    catch { return path; }
  }

  $: hasThumbnail = !!story.thumbnail_path;

  // ── Relative time ──
  function relativeTime(isoDate: string): string {
    if (!isoDate) return 'Never';
    try {
      const date = new Date(isoDate);
      const now = Date.now();
      const diffMs = now - date.getTime();
      const diffMin = Math.floor(diffMs / 60000);
      if (diffMin < 1) return 'Just now';
      if (diffMin < 60) return `${diffMin}m ago`;
      const diffHr = Math.floor(diffMin / 60);
      if (diffHr < 24) return `${diffHr}h ago`;
      const diffDay = Math.floor(diffHr / 24);
      if (diffDay < 7) return `${diffDay}d ago`;
      if (diffDay < 30) return `${Math.floor(diffDay / 7)}w ago`;
      return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
    } catch { return 'Unknown'; }
  }

  // ── Placeholder gradient based on story title ──
  function placeholderGradient(title: string): string {
    let hash = 0;
    for (let i = 0; i < title.length; i++) {
      hash = title.charCodeAt(i) + ((hash << 5) - hash);
    }
    const h1 = Math.abs(hash % 360);
    const h2 = (h1 + 40 + Math.abs((hash >> 8) % 40)) % 360;
    return `linear-gradient(135deg, hsl(${h1}, 45%, 28%) 0%, hsl(${h2}, 55%, 18%) 100%)`;
  }

  // ── Description truncation ──
  function truncate(text: string, max: number): string {
    if (!text || text.length <= max) return text || '';
    return text.slice(0, max).trimEnd() + '…';
  }
</script>

<div
  class="story-card"
  on:click={() => dispatch('select', story.story_id)}
  on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && dispatch('select', story.story_id)}
  role="button"
  tabindex="0"
  title="Continue: {story.title}"
>
  <!-- Thumbnail / Placeholder -->
  <div class="card-thumbnail" style={!hasThumbnail || thumbError ? `background: ${placeholderGradient(story.title)}` : ''}>
    {#if hasThumbnail && !thumbError}
      <img
        src={assetSrc(story.thumbnail_path ?? '')}
        alt={story.title}
        class="thumb-img"
        class:loaded={thumbLoaded}
        on:load={() => thumbLoaded = true}
        on:error={() => thumbError = true}
      />
    {/if}

    {#if !hasThumbnail || thumbError}
      <div class="thumb-placeholder">
        <span class="thumb-icon">📖</span>
      </div>
    {/if}

    <!-- Hover Overlay -->
    <div class="card-hover-overlay">
      <span class="continue-label">Continue</span>
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <polygon points="5 3 19 12 5 21 5 3"></polygon>
      </svg>
    </div>

    <!-- Location Badge -->
    {#if story.current_location}
      <span class="location-tag">📍 {story.current_location}</span>
    {/if}
  </div>

  <!-- Card Body -->
  <div class="card-body">
    <h3 class="card-title">{story.title}</h3>
    <p class="card-desc">{truncate(story.description, 90)}</p>

    <div class="card-meta">
      <span class="meta-item" title="Characters">
        <span class="meta-icon">👤</span> {story.character_count}
      </span>
      <span class="meta-sep">·</span>
      <span class="meta-item" title="Turns played">
        <span class="meta-icon">💬</span> {story.turn_count}
      </span>
      <span class="meta-sep">·</span>
      <span class="meta-item last-played" title="Last played: {story.last_played_at}">
        {relativeTime(story.last_played_at)}
      </span>
    </div>
  </div>

  <!-- Delete Button (top-right, stops propagation) -->
  <div
    class="card-delete-btn"
    on:click|stopPropagation={() => dispatch('delete', story.story_id)}
    on:keydown|stopPropagation={(e) => (e.key === 'Enter' || e.key === ' ') && dispatch('delete', story.story_id)}
    role="button"
    tabindex="0"
    title="Delete story"
  >
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="3 6 5 6 21 6"></polyline>
      <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"></path>
    </svg>
  </div>
</div>

<style>
  .story-card {
    position: relative;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary, #161b22);
    border: 1px solid var(--border-secondary, #21262d);
    border-radius: 12px;
    overflow: hidden;
    cursor: pointer;
    transition: transform 0.18s ease, box-shadow 0.18s ease, border-color 0.18s ease;
    text-align: left;
    color: inherit;
  }

  .story-card:focus {
    outline: 2px solid var(--accent-primary, #58a6ff);
    outline-offset: 2px;
  }

  .story-card:hover {
    transform: translateY(-3px);
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.25);
    border-color: var(--accent-primary, #58a6ff);
  }

  .story-card:active {
    transform: translateY(-1px);
  }

  /* ── Thumbnail ── */
  .card-thumbnail {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 10;
    overflow: hidden;
    background: var(--bg-tertiary, #21262d);
  }

  .thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.35s ease;
  }

  .thumb-img.loaded {
    opacity: 1;
  }

  .thumb-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .thumb-icon {
    font-size: 2.4rem;
    opacity: 0.25;
  }

  /* Hover overlay */
  .card-hover-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    opacity: 0;
    transition: opacity 0.2s ease;
    color: #fff;
    backdrop-filter: blur(2px);
  }

  .story-card:hover .card-hover-overlay {
    opacity: 1;
  }

  .continue-label {
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  /* Location tag */
  .location-tag {
    position: absolute;
    bottom: 6px;
    left: 6px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.65rem;
    font-weight: 500;
    background: rgba(0, 0, 0, 0.55);
    color: rgba(255, 255, 255, 0.8);
    backdrop-filter: blur(4px);
    pointer-events: none;
    text-transform: capitalize;
    max-width: 70%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Card Body ── */
  .card-body {
    padding: 12px 14px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .card-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-desc {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-secondary, #8b949e);
    line-height: 1.45;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    min-height: 2.1em;
  }

  /* ── Meta Row ── */
  .card-meta {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 4px;
    font-size: 0.7rem;
    color: var(--text-muted, #6e7681);
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: 3px;
    white-space: nowrap;
  }

  .meta-icon {
    font-size: 0.75rem;
  }

  .meta-sep {
    opacity: 0.35;
  }

  .last-played {
    margin-left: auto;
    font-style: italic;
  }

  /* ── Delete Button ── */
  .card-delete-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    border: none;
    color: rgba(255, 255, 255, 0.6);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.15s, background 0.15s, color 0.15s;
    z-index: 2;
  }

  .story-card:hover .card-delete-btn {
    opacity: 1;
  }

  .card-delete-btn:hover {
    background: var(--accent-danger, #f85149);
    color: #fff;
  }
</style>