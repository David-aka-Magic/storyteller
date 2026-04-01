<!-- src/components/AllCharactersModal.svelte -->
<script lang="ts">
  import type { CharacterProfile } from '$lib/types';
  import { resolveCharacterImageUrl } from '$lib/utils/character-image';

  let {
    show = false,
    characters = [],
    onclose,
    onedit,
    ondelete,
  }: {
    show?: boolean;
    characters?: CharacterProfile[];
    onclose?: () => void;
    onedit?: (char: CharacterProfile) => void;
    ondelete?: (id: number) => void;
  } = $props();

  let search = $state('');
  let portraitUrls: Map<number, string | null> = $state(new Map());

  $effect(() => {
    if (!show) { portraitUrls = new Map(); return; }
    Promise.all(
      characters.map(async (c) => {
        const url = await resolveCharacterImageUrl(c);
        return [c.id, url] as [number, string | null];
      })
    ).then((entries) => {
      portraitUrls = new Map(entries);
    });
  });

  const filtered = $derived(
    search.trim() === ''
      ? characters
      : characters.filter(c => c.name.toLowerCase().includes(search.trim().toLowerCase()))
  );

  function initials(name: string): string {
    return name.split(' ').map(w => w[0] ?? '').join('').toUpperCase().slice(0, 2);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose?.();
  }
</script>

{#if show}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onkeydown={handleKeydown} role="dialog" aria-modal="true">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="backdrop-click" onclick={onclose}></div>
    <div class="modal">
      <div class="modal-header">
        <h2>All Characters ({characters.length})</h2>
        <input
          class="search-input"
          type="text"
          placeholder="Search by name..."
          bind:value={search}
        />
        <button class="close-btn" onclick={onclose}>✕</button>
      </div>

      <div class="modal-body">
        {#if filtered.length === 0}
          <p class="empty">No characters found.</p>
        {:else}
          <div class="grid">
            {#each filtered as char (char.id)}
              {@const url = portraitUrls.get(char.id)}
              <div class="card">
                <div class="portrait">
                  {#if url}
                    <img src={url} alt={char.name} />
                  {:else}
                    <div class="initials">{initials(char.name)}</div>
                  {/if}
                </div>
                <div class="card-info">
                  <div class="char-name">{char.name}</div>
                  <div class="char-meta">{char.age ?? '?'} · {char.gender ?? '—'}</div>
                  {#if char.art_style}
                    <span class="badge">{char.art_style}</span>
                  {/if}
                </div>
                <div class="card-actions">
                  <button class="btn-edit" onclick={() => onedit?.(char)}>Edit</button>
                  <button class="btn-delete" onclick={() => ondelete?.(char.id)}>Delete</button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .backdrop-click {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
  }

  .modal {
    position: relative;
    z-index: 1;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    width: 90vw;
    max-width: 960px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.1em;
    color: var(--text-primary);
    white-space: nowrap;
  }

  .search-input {
    flex: 1;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 4px;
    color: var(--text-primary);
    padding: 6px 10px;
    font-size: 0.9em;
  }
  .search-input::placeholder { color: var(--text-muted); }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 1.1em;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
  }
  .close-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .modal-body {
    overflow-y: auto;
    padding: 20px;
    flex: 1;
  }

  .empty {
    color: var(--text-muted);
    text-align: center;
    margin-top: 40px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
  }

  .card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: 8px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .portrait {
    width: 100%;
    aspect-ratio: 3/4;
    background: var(--bg-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .portrait img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .initials {
    font-size: 2em;
    font-weight: bold;
    color: var(--text-muted);
  }

  .card-info {
    padding: 10px 12px 6px;
    flex: 1;
  }

  .char-name {
    font-weight: bold;
    color: var(--text-primary);
    font-size: 0.95em;
    margin-bottom: 2px;
  }

  .char-meta {
    font-size: 0.8em;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .badge {
    display: inline-block;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    color: var(--text-muted);
    font-size: 0.7em;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .card-actions {
    display: flex;
    gap: 6px;
    padding: 8px 12px;
    border-top: 1px solid var(--border-primary);
  }

  .btn-edit, .btn-delete {
    flex: 1;
    padding: 5px 0;
    border-radius: 4px;
    font-size: 0.8em;
    cursor: pointer;
    border: 1px solid var(--border-secondary);
    transition: all 0.15s;
  }

  .btn-edit {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }
  .btn-edit:hover { background: var(--bg-hover); color: var(--text-primary); }

  .btn-delete {
    background: transparent;
    color: var(--accent-danger);
    border-color: var(--accent-danger);
  }
  .btn-delete:hover { background: var(--accent-danger); color: white; }
</style>
