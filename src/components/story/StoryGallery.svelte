<!-- src/components/story/StoryGallery.svelte — Image gallery for a story -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { getStoryImages } from '$lib/api/story';
  import { filePathToDataUrl } from '$lib/utils/image-url';
  import type { StoryImage } from '$lib/types';

  let {
    storyId,
    chatId = undefined,
    storyTitle = 'Gallery',
    onclose,
  }: {
    storyId: number;
    chatId?: number;
    storyTitle?: string;
    onclose?: () => void;
  } = $props();

  let images = $state<StoryImage[]>([]);
  let isLoading = $state(true);
  let error = $state('');
  let selectedImage = $state<StoryImage | null>(null);
  let imageUrls = $state<Map<number, string | null>>(new Map());

  onMount(async () => {
    await loadImages();
  });

  async function loadImages() {
    isLoading = true;
    error = '';
    console.log('[Gallery] Loading images for storyId:', storyId, 'chatId:', chatId);
    try {
      images = await getStoryImages(storyId, chatId);
      console.log('[Gallery] Loaded images:', images.length);
    } catch (e) {
      error = `Failed to load images: ${e}`;
      console.error('[Gallery]', error);
    }
    isLoading = false;
  }

  // Resolve file paths to data URIs whenever the images list changes
  $effect(() => {
    const imgs = images;
    Promise.all(
      imgs.map(async (img) => {
        const url = await filePathToDataUrl(img.file_path);
        return [img.id, url] as [number, string | null];
      })
    ).then((entries) => {
      imageUrls = new Map(entries);
    });
  });

  function openLightbox(img: StoryImage) {
    selectedImage = img;
  }

  function closeLightbox() {
    selectedImage = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!selectedImage) return;
    if (e.key === 'Escape') closeLightbox();
    if (e.key === 'ArrowRight') navigateLightbox(1);
    if (e.key === 'ArrowLeft') navigateLightbox(-1);
  }

  function navigateLightbox(direction: number) {
    if (!selectedImage) return;
    const idx = images.findIndex(i => i.id === selectedImage!.id);
    const next = idx + direction;
    if (next >= 0 && next < images.length) {
      selectedImage = images[next];
    }
  }

  function formatDate(timestamp: string): string {
    if (!timestamp) return '';
    try {
      return new Date(timestamp).toLocaleDateString(undefined, {
        month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
      });
    } catch { return ''; }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="gallery-panel">
  <div class="gallery-header">
    <h2>🖼 {storyTitle} — Gallery</h2>
    <span class="image-count">{images.length} image{images.length !== 1 ? 's' : ''}</span>
    {#if onclose}
      <button class="close-btn" onclick={onclose} title="Close gallery">✕</button>
    {/if}
  </div>

  {#if isLoading}
    <div class="gallery-loading">Loading images...</div>
  {:else if error}
    <div class="gallery-error">{error}</div>
  {:else if images.length === 0}
    <div class="gallery-empty">
      <div class="empty-icon">🎨</div>
      <p>No images generated yet.</p>
      <p class="empty-hint">Use "Illustrate Scene" in the chat to generate images for your story.</p>
    </div>
  {:else}
    <div class="gallery-grid">
      {#each images as img (img.id)}
        <button class="gallery-item" onclick={() => openLightbox(img)}>
          <img
            src={imageUrls.get(img.id) ?? ''}
            alt={img.caption || 'Scene image'}
            loading="lazy"
          />
          {#if img.caption}
            <div class="item-caption">{img.caption}</div>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<!-- Lightbox -->
{#if selectedImage}
  <div class="lightbox" onclick={closeLightbox}>
    <div class="lightbox-content" onclick={(e) => e.stopPropagation()}>
      <img src={imageUrls.get(selectedImage.id) ?? ''} alt={selectedImage.caption || 'Scene'} />

      <div class="lightbox-info">
        {#if selectedImage.caption}
          <p class="lightbox-caption">{selectedImage.caption}</p>
        {/if}
        <span class="lightbox-date">{formatDate(selectedImage.timestamp)}</span>
      </div>

      <button
        class="lightbox-nav prev"
        onclick={(e) => { e.stopPropagation(); navigateLightbox(-1); }}
        disabled={images.findIndex(i => i.id === selectedImage?.id) === 0}
      >◀</button>
      <button
        class="lightbox-nav next"
        onclick={(e) => { e.stopPropagation(); navigateLightbox(1); }}
        disabled={images.findIndex(i => i.id === selectedImage?.id) === images.length - 1}
      >▶</button>

      <button class="lightbox-close" onclick={closeLightbox}>✕</button>
    </div>
  </div>
{/if}

<style>
  .gallery-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-chat);
    overflow: hidden;
  }

  .gallery-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .gallery-header h2 {
    margin: 0;
    font-size: 1.1em;
    color: var(--text-primary);
    flex: 1;
  }

  .image-count {
    font-size: 0.8em;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 3px 10px;
    border-radius: 12px;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 1.2em;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
  }
  .close-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .gallery-grid {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
    align-content: start;
  }

  .gallery-item {
    position: relative;
    border: none;
    background: var(--bg-secondary);
    border-radius: 8px;
    overflow: hidden;
    cursor: pointer;
    transition: transform 0.15s, box-shadow 0.15s;
    padding: 0;
    aspect-ratio: 1;
  }

  .gallery-item:hover {
    transform: scale(1.03);
    box-shadow: 0 4px 16px var(--shadow);
  }

  .gallery-item img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .item-caption {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 8px 10px;
    background: linear-gradient(transparent, rgba(0,0,0,0.75));
    color: white;
    font-size: 0.75em;
    line-height: 1.3;
    pointer-events: none;
  }

  .gallery-loading, .gallery-error, .gallery-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    padding: 40px;
    text-align: center;
  }
  .gallery-error { color: var(--accent-danger); }
  .empty-icon { font-size: 3em; margin-bottom: 12px; opacity: 0.5; }
  .empty-hint { font-size: 0.85em; margin-top: 4px; }

  .lightbox {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.9);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 5000;
    cursor: pointer;
  }

  .lightbox-content {
    position: relative;
    max-width: 90vw;
    max-height: 90vh;
    cursor: default;
  }

  .lightbox-content img {
    max-width: 90vw;
    max-height: 85vh;
    object-fit: contain;
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .lightbox-info {
    text-align: center;
    margin-top: 10px;
  }

  .lightbox-caption {
    color: #e0e0e0;
    font-size: 0.9em;
    margin: 0 0 4px;
  }

  .lightbox-date {
    color: #888;
    font-size: 0.75em;
  }

  .lightbox-nav {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    background: rgba(255, 255, 255, 0.1);
    border: none;
    color: white;
    font-size: 1.5em;
    padding: 12px 16px;
    cursor: pointer;
    border-radius: 8px;
    transition: background 0.15s;
  }
  .lightbox-nav:hover:not(:disabled) { background: rgba(255, 255, 255, 0.2); }
  .lightbox-nav:disabled { opacity: 0.3; cursor: default; }
  .lightbox-nav.prev { left: -60px; }
  .lightbox-nav.next { right: -60px; }

  .lightbox-close {
    position: absolute;
    top: -40px;
    right: 0;
    background: none;
    border: none;
    color: #aaa;
    font-size: 1.5em;
    cursor: pointer;
    padding: 4px 10px;
  }
  .lightbox-close:hover { color: white; }
</style>
