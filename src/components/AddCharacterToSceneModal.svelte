<!-- src/components/AddCharacterToSceneModal.svelte
     Picker modal: shows all characters matching the scene's art_style so the
     user can add an existing character to the active scene. -->
<script lang="ts">
  import type { CharacterProfile } from '$lib/types';
  import { listCharactersByArtStyle } from '$lib/api/character';
  import { resolveCharacterImageUrl } from '$lib/utils/character-image';

  let {
    open,
    artStyle,
    excludeIds,
    onSelect,
    onClose,
  }: {
    open: boolean;
    artStyle: string | null;
    excludeIds: number[];
    onSelect: (characterId: number) => void;
    onClose: () => void;
  } = $props();

  let characters: CharacterProfile[] = $state([]);
  let portraitUrls: Map<number, string | null> = $state(new Map());
  let search = $state('');
  let loading = $state(false);

  // Reload character list whenever the modal opens or filter params change
  $effect(() => {
    if (!open) {
      characters = [];
      portraitUrls = new Map();
      search = '';
      return;
    }
    loading = true;
    listCharactersByArtStyle(artStyle, excludeIds)
      .then(async (chars) => {
        characters = chars;
        const entries = await Promise.all(
          chars.map(async (c) => {
            const url = await resolveCharacterImageUrl(c);
            return [c.id, url] as [number, string | null];
          })
        );
        portraitUrls = new Map(entries);
      })
      .catch((e) => console.error('[AddCharacterToSceneModal] load failed:', e))
      .finally(() => { loading = false; });
  });

  let filtered = $derived(
    search.trim()
      ? characters.filter(c => c.name.toLowerCase().includes(search.trim().toLowerCase()))
      : characters
  );

  function pick(id: number) {
    onSelect(id);
    onClose();
  }

  function initials(name: string): string {
    return name.split(' ').map(w => w[0] ?? '').join('').toUpperCase().slice(0, 2);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

{#if open}
<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  onkeydown={handleKeydown}
>
  <!-- Backdrop click to close -->
  <div class="backdrop" onclick={onClose}></div>

  <div class="modal">
    <div class="modal-header">
      <h2>Add Character to Scene</h2>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <div class="search-bar">
      <input
        type="search"
        placeholder="Filter by name…"
        bind:value={search}
        autofocus
      />
      {#if artStyle}
        <span class="style-badge">{artStyle}</span>
      {/if}
    </div>

    <div class="modal-body">
      {#if loading}
        <p class="hint">Loading…</p>
      {:else if filtered.length === 0}
        <p class="hint">No matching characters found.</p>
      {:else}
        <div class="char-grid">
          {#each filtered as char (char.id)}
            {@const portrait = portraitUrls.get(char.id) ?? null}
            <button class="char-tile" onclick={() => pick(char.id)} title="Add {char.name}">
              <div class="portrait">
                {#if portrait}
                  <img src={portrait} alt={char.name} />
                {:else}
                  <span class="initials">{initials(char.name)}</span>
                {/if}
              </div>
              <span class="name">{char.name}</span>
              {#if char.gender || char.age}
                <span class="sub">
                  {[char.gender, char.age != null ? `${char.age}y` : null].filter(Boolean).join(' · ')}
                </span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
  }

  .modal {
    position: relative;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    width: 500px;
    max-width: 95vw;
    max-height: 70vh;
    display: flex;
    flex-direction: column;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px 10px;
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }

  h2 {
    margin: 0;
    font-size: 0.95em;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 1em;
    padding: 4px;
    line-height: 1;
  }
  .close-btn:hover { color: var(--text-primary); }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }

  .search-bar input {
    flex: 1;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.83em;
    padding: 5px 8px;
    outline: none;
    font-family: inherit;
  }
  .search-bar input:focus { border-color: var(--border-active); }

  .style-badge {
    font-size: 0.7em;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: 4px;
    padding: 2px 7px;
    white-space: nowrap;
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 18px;
  }

  .hint {
    margin: 0;
    padding: 20px 0;
    font-size: 0.82em;
    color: var(--text-secondary);
    font-style: italic;
    text-align: center;
  }

  .char-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .char-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: 88px;
    padding: 8px 4px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 6px;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .char-tile:hover {
    border-color: var(--accent-primary);
    background: var(--bg-hover);
  }

  .portrait {
    width: 56px;
    height: 56px;
    border-radius: 5px;
    overflow: hidden;
    background: var(--bg-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .portrait img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .initials {
    font-size: 1.1em;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .name {
    font-size: 0.72em;
    color: var(--text-primary);
    text-align: center;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.3;
  }

  .sub {
    font-size: 0.65em;
    color: var(--text-secondary);
    text-align: center;
  }
</style>
