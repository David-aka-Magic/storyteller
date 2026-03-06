<!-- src/components/shared/ImageLightbox.svelte — Fullscreen image viewer -->
<script lang="ts">
    let {
        src,
        alt = 'Image',
        onclose,
    }: {
        src: string;
        alt?: string;
        onclose?: () => void;
    } = $props();

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') onclose?.();
    }

    function handleBackdropClick(e: MouseEvent) {
        if (e.target === e.currentTarget) onclose?.();
    }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="lightbox-backdrop" onclick={handleBackdropClick}>
    <div class="lightbox-body">
        <img {src} {alt} class="lightbox-img" />
        <button class="lightbox-close" onclick={() => onclose?.()} title="Close (Esc)">✕</button>
    </div>
</div>

<style>
    .lightbox-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.92);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 9999;
        cursor: pointer;
        padding: 20px;
    }

    .lightbox-body {
        position: relative;
        max-width: 95vw;
        max-height: 95vh;
        cursor: default;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .lightbox-img {
        max-width: 95vw;
        max-height: 92vh;
        object-fit: contain;
        border-radius: 6px;
        box-shadow: 0 8px 40px rgba(0, 0, 0, 0.6);
        user-select: none;
        -webkit-user-select: none;
    }

    .lightbox-close {
        position: absolute;
        top: -40px;
        right: -4px;
        background: rgba(255, 255, 255, 0.1);
        border: none;
        color: #ccc;
        font-size: 1.4em;
        cursor: pointer;
        padding: 4px 12px;
        border-radius: 6px;
        transition: background 0.15s, color 0.15s;
    }

    .lightbox-close:hover {
        background: rgba(255, 255, 255, 0.2);
        color: white;
    }
</style>
