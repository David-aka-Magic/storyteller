<!-- src/components/AddCharacterToSceneModal.svelte
     Picker modal: shows story characters so the user can add one to the active scene.
     Receives the story roster directly — no DB fetch needed. -->
<script lang="ts">
  import type { CharacterProfile } from '$lib/types';
  import { resolveCharacterImageUrl } from '$lib/utils/character-image';

  let {
    open,
    storyCharacters,
    excludeIds,
    title = 'Add Character to Scene',
    onSelect,
    onClose,
  }: {
    open: boolean;
    storyCharacters: CharacterProfile[];
    excludeIds: number[];
    title?: string;
    onSelect: (characterId: number) => void;
    onClose: () => void;
  } = $props();

  let portraitUrls: Map<number, string | null> = $state(new Map());
  let search = $state('');

  // Resolve portraits whenever the modal opens or the roster changes
  $effect(() => {
    if (!open) {
      portraitUrls = new Map();
      search = '';
      return;
    }
    Promise.all(
      storyCharacters.map(async (c) => {
        const url = await resolveCharacterImageUrl(c);
        return [c.id, url] as [number, string | null];
      })
    ).then((entries) => {
      portraitUrls = new Map(entries);
    }).catch((e) => console.error('[AddCharacterToSceneModal] portrait load failed:', e));
  });

  let available = $derived(
    storyCharacters.filter(c => !excludeIds.includes(c.id))
  );

  let filtered = $derived(
    search.trim()
      ? available.filter(c => c.name.toLowerCase().includes(search.trim().toLowerCase()))
      : available
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
      <h2>{title}</h2>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <div class="search-bar">
      <input
        type="search"
        placeholder="Filter by name…"
        bind:value={search}
        autofocus
      />
    </div>

    <div class="modal-body">
      {#if filtered.length === 0}
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
