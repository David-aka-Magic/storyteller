<!-- src/components/layout/ScenePanel.svelte
     Right-side panel: Scenes list + Characters in active scene.
     280px wide when expanded, collapses to a toggle strip. -->
<script lang="ts">
  import SceneModal from '../SceneModal.svelte';
  import AddCharacterToSceneModal from '../AddCharacterToSceneModal.svelte';
  import type { Scene, CharacterProfile } from '$lib/types';
  import { resolveCharacterImageUrl } from '$lib/utils/character-image';
  import {
    listScenesForStory,
    createScene,
    updateScene,
    deleteScene,
    linkSceneToStory,
    setActiveScene,
    getActiveScene,
    getSceneCharacters,
    addCharacterToScene,
    removeCharacterFromScene,
  } from '$lib/api/scene';

  let {
    storyId = null as number | null,
    storyCharacters = [] as CharacterProfile[],
    allCharacters = [] as CharacterProfile[],
    collapsed = false,
    refreshKey = 0,
    onToggleCollapse,
    onCreateCharacter,
    onEditCharacter,
    onDeleteCharacter,
    onsceneselected,
    onaddcharactertostory,
  }: {
    storyId?: number | null;
    storyCharacters?: CharacterProfile[];
    allCharacters?: CharacterProfile[];
    collapsed?: boolean;
    /** Increment to force a reload (e.g. after auto-scene-sync from a turn). */
    refreshKey?: number;
    onToggleCollapse?: (val: boolean) => void;
    onCreateCharacter?: () => void;
    onEditCharacter?: (c: CharacterProfile) => void;
    onDeleteCharacter?: (id: number) => void;
    /** Called when the user manually clicks a scene to activate it. */
    onsceneselected?: (sceneId: number, sceneName: string, characterNames: string[]) => void;
    onaddcharactertostory?: (characterId: number) => void;
  } = $props();

  let scenes: Scene[] = $state([]);
  let activeSceneId: number | null = $state(null);
  let sceneCharacters: CharacterProfile[] = $state([]);
  let portraitUrls: Map<number, string | null> = $state(new Map());

  let showSceneModal = $state(false);
  let sceneToEdit: Scene | null = $state(null);
  let showAddExistingModal = $state(false);
  let showAddToStoryModal = $state(false);

  // ── Resolve portrait URLs whenever storyCharacters or allCharacters changes ─
  $effect(() => {
    const storySet = storyCharacters;
    const allSet = allCharacters;
    // Deduplicate by id
    const seen = new Set<number>();
    const combined: CharacterProfile[] = [];
    for (const c of [...storySet, ...allSet]) {
      if (!seen.has(c.id)) { seen.add(c.id); combined.push(c); }
    }
    Promise.all(
      combined.map(async (c) => {
        const url = await resolveCharacterImageUrl(c);
        return [c.id, url] as [number, string | null];
      })
    ).then((entries) => {
      portraitUrls = new Map(entries);
    });
  });

  // ── Load scenes whenever storyId or refreshKey changes ───────────────────
  $effect(() => {
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    refreshKey; // declare dependency so effect re-runs when key increments
    if (storyId != null) {
      loadScenes();
    } else {
      scenes = [];
      activeSceneId = null;
      sceneCharacters = [];
    }
  });

  async function loadScenes() {
    if (storyId == null) return;
    try {
      scenes = await listScenesForStory(storyId);
      const active = await getActiveScene(storyId);
      activeSceneId = active?.id ?? null;
      if (activeSceneId != null) {
        sceneCharacters = await getSceneCharacters(activeSceneId);
      } else {
        sceneCharacters = [];
      }
    } catch (e) {
      console.error('[ScenePanel] loadScenes failed:', e);
    }
  }

  async function selectScene(id: number) {
    if (storyId == null) return;
    const next = activeSceneId === id ? null : id;
    try {
      await setActiveScene(storyId, next);
      activeSceneId = next;
      sceneCharacters = next != null ? await getSceneCharacters(next) : [];

      // Notify parent so it can show the transition suggestion bar
      if (next != null) {
        const scene = scenes.find(s => s.id === next);
        const charNames = sceneCharacters.map(c => c.name);
        onsceneselected?.(next, scene?.name ?? '', charNames);
      }
    } catch (e) {
      console.error('[ScenePanel] selectScene failed:', e);
    }
  }

  async function handleSaveScene(form: any) {
    if (storyId == null) return;
    try {
      if (form.id) {
        await updateScene(form.id, form.name, form.description, form.location, form.location_type, form.time_of_day, form.mood);
      } else {
        const newId = await createScene(form.name, form.description, form.location, form.location_type, form.time_of_day, form.mood);
        await linkSceneToStory(newId, storyId);
      }
      showSceneModal = false;
      sceneToEdit = null;
      await loadScenes();
    } catch (e) {
      console.error('[ScenePanel] handleSaveScene failed:', e);
    }
  }

  async function handleDeleteScene(id: number) {
    try {
      await deleteScene(id);
      if (activeSceneId === id) {
        activeSceneId = null;
        sceneCharacters = [];
      }
      await loadScenes();
    } catch (e) {
      console.error('[ScenePanel] handleDeleteScene failed:', e);
    }
  }

  async function toggleCharacterInScene(charId: number) {
    if (activeSceneId == null) return;
    const inScene = sceneCharacters.some(c => c.id === charId);
    try {
      if (inScene) {
        await removeCharacterFromScene(activeSceneId, charId);
      } else {
        await addCharacterToScene(activeSceneId, charId);
      }
      sceneCharacters = await getSceneCharacters(activeSceneId);
    } catch (e) {
      console.error('[ScenePanel] toggleCharacterInScene failed:', e);
    }
  }

  async function handleAddExistingCharacter(characterId: number) {
    if (activeSceneId == null) return;
    try {
      await addCharacterToScene(activeSceneId, characterId);
      sceneCharacters = await getSceneCharacters(activeSceneId);
    } catch (e) {
      console.error('[ScenePanel] handleAddExistingCharacter failed:', e);
    }
  }

  function initials(name: string): string {
    return name.split(' ').map(w => w[0] ?? '').join('').toUpperCase().slice(0, 2);
  }
</script>

<div class="scene-panel" class:collapsed>
  <button
    class="collapse-toggle"
    onclick={() => onToggleCollapse?.(!collapsed)}
    title={collapsed ? 'Show Scenes panel' : 'Collapse Scenes panel'}
  >{collapsed ? '◀' : '▶'}</button>

  {#if !collapsed}
    <!-- ── Header ─────────────────────────────────────────── -->
    <div class="panel-header">
      <span class="panel-title">Scenes & Characters</span>
    </div>

    {#if storyId == null}
      <p class="empty-hint">Select a story to manage scenes.</p>
    {:else}
      <!-- ── Scenes list ─────────────────────────────────── -->
      <section class="section">
        <div class="section-header">
          <span class="section-title">Scenes</span>
          <button class="add-btn" onclick={() => { sceneToEdit = null; showSceneModal = true; }}>+ New</button>
        </div>

        {#if scenes.length === 0}
          <p class="empty-hint">No scenes yet.</p>
        {:else}
          <ul class="scene-list">
            {#each scenes as scene (scene.id)}
              <li class="scene-item" class:active={activeSceneId === scene.id}>
                <button
                  class="scene-name"
                  onclick={() => selectScene(scene.id)}
                  title={scene.location ?? ''}
                >
                  <span class="dot" class:dot-active={activeSceneId === scene.id}></span>
                  <span class="name-text">{scene.name}</span>
                  {#if scene.time_of_day}
                    <span class="badge">{scene.time_of_day}</span>
                  {/if}
                </button>
                <div class="scene-actions">
                  <button class="icon-btn small" onclick={() => { sceneToEdit = scene; showSceneModal = true; }} title="Edit">✏</button>
                  <button class="icon-btn small danger" onclick={() => handleDeleteScene(scene.id)} title="Delete">✕</button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <!-- ── Characters ─────────────────────────────────────── -->
      <section class="section">
        <div class="section-header">
          <span class="section-title">
            {activeSceneId != null ? 'In This Scene' : 'Story Cast'}
          </span>
          <div class="header-actions">
            {#if activeSceneId != null}
              <button
                class="add-btn"
                onclick={() => { showAddExistingModal = true; }}
                title="Add a story character to this scene"
              >+ Add</button>
            {:else}
              <button
                class="add-btn"
                onclick={() => { showAddToStoryModal = true; }}
                title="Add an existing character to this story"
              >+ Existing</button>
            {/if}
            <button class="add-btn" onclick={onCreateCharacter}>+ New</button>
          </div>
        </div>

        {#if activeSceneId != null}
          <!-- Characters pinned to this scene -->
          {#if sceneCharacters.length === 0}
            <p class="empty-hint">No characters in this scene. Add from story cast below.</p>
          {:else}
            <div class="char-grid">
              {#each sceneCharacters as char (char.id)}
                {@const portrait = portraitUrls.get(char.id) ?? null}
                <div class="char-tile in-scene">
                  <button
                    class="char-portrait"
                    onclick={() => toggleCharacterInScene(char.id)}
                    title="Remove from scene"
                  >
                    {#if portrait}
                      <img src={portrait} alt={char.name} />
                    {:else}
                      <span class="initials">{initials(char.name)}</span>
                    {/if}
                    <span class="scene-badge">✓</span>
                  </button>
                  <span class="char-name">
                    {char.name}
                    {#if char.content_rating === 'nsfw'}<span class="nsfw-pip" title="NSFW"></span>{/if}
                  </span>
                </div>
              {/each}
            </div>
          {/if}

          <!-- Available story characters not yet in this scene -->
          {@const availableChars = storyCharacters.filter(c => !sceneCharacters.some(sc => sc.id === c.id))}
          {#if availableChars.length > 0}
            <div class="subsection-header">
              <span class="subsection-title">Available</span>
            </div>
            <div class="char-grid available">
              {#each availableChars as char (char.id)}
                {@const portrait = portraitUrls.get(char.id) ?? null}
                <div class="char-tile">
                  <button
                    class="char-portrait"
                    onclick={() => toggleCharacterInScene(char.id)}
                    title="Add to scene"
                  >
                    {#if portrait}
                      <img src={portrait} alt={char.name} />
                    {:else}
                      <span class="initials">{initials(char.name)}</span>
                    {/if}
                  </button>
                  <span class="char-name">
                    {char.name}
                    {#if char.content_rating === 'nsfw'}<span class="nsfw-pip" title="NSFW"></span>{/if}
                  </span>
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          <!-- No active scene — show full story cast -->
          {#if storyCharacters.length === 0}
            <p class="empty-hint">No characters for this story.</p>
          {:else}
            <div class="char-grid">
              {#each storyCharacters as char (char.id)}
                {@const portrait = portraitUrls.get(char.id) ?? null}
                <div class="char-tile">
                  <div class="char-portrait static">
                    {#if portrait}
                      <img src={portrait} alt={char.name} />
                    {:else}
                      <span class="initials">{initials(char.name)}</span>
                    {/if}
                  </div>
                  <span class="char-name">
                    {char.name}
                    {#if char.content_rating === 'nsfw'}<span class="nsfw-pip" title="NSFW"></span>{/if}
                  </span>
                  <div class="char-actions">
                    <button class="icon-btn small" onclick={() => onEditCharacter?.(char)} title="Edit">✏</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </section>
    {/if}
  {/if}
</div>

<SceneModal
  show={showSceneModal}
  scene={sceneToEdit}
  onSave={handleSaveScene}
  onClose={() => { showSceneModal = false; sceneToEdit = null; }}
/>

<AddCharacterToSceneModal
  open={showAddExistingModal}
  storyCharacters={storyCharacters}
  excludeIds={sceneCharacters.map(c => c.id)}
  onSelect={handleAddExistingCharacter}
  onClose={() => { showAddExistingModal = false; }}
/>

{#if showAddToStoryModal}
  {@const storyCharIds = new Set(storyCharacters.map(c => c.id))}
  {@const unlinked = allCharacters.filter(c => !storyCharIds.has(c.id))}
  <div class="add-to-story-backdrop" onclick={() => { showAddToStoryModal = false; }} role="presentation">
    <div class="add-to-story-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Add character to story">
      <div class="ats-header">
        <span class="ats-title">Add to Story</span>
        <button class="icon-btn" onclick={() => { showAddToStoryModal = false; }} title="Close">✕</button>
      </div>
      {#if unlinked.length === 0}
        <p class="empty-hint">All characters are already in this story.</p>
      {:else}
        <div class="ats-grid">
          {#each unlinked as char (char.id)}
            {@const portrait = portraitUrls.get(char.id) ?? null}
            <button
              class="ats-tile"
              onclick={() => { onaddcharactertostory?.(char.id); showAddToStoryModal = false; }}
              title="Add {char.name} to story"
            >
              <div class="ats-portrait">
                {#if portrait}
                  <img src={portrait} alt={char.name} />
                {:else}
                  <span class="initials">{initials(char.name)}</span>
                {/if}
              </div>
              <span class="char-name">{char.name}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* ── Collapse tab (shared by both states) ────────────────── */
  .collapse-toggle {
    position: absolute;
    left: -14px;
    top: 50%;
    transform: translateY(-50%);
    width: 14px;
    height: 48px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-right: none;
    border-radius: 6px 0 0 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 0.65rem;
    padding: 0;
    z-index: 11;
    transition: color 0.15s, background 0.15s;
  }
  .collapse-toggle:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  /* ── Panel (expanded + collapsed) ───────────────────────── */
  .scene-panel {
    position: relative;
    width: 280px;
    min-width: 280px;
    border-left: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    display: flex;
    flex-direction: column;
    overflow: visible;
    flex-shrink: 0;
  }

  .scene-panel.collapsed {
    width: 0;
    min-width: 0;
  }

  .panel-header {
    display: flex;
    align-items: center;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }

  .panel-title {
    font-size: 0.85em;
    font-weight: 600;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  /* ── Sections ────────────────────────────────────────────── */
  .section {
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--border-primary);
    overflow: hidden;
    flex: 0 1 auto;
    max-height: 50%;
  }

  .section:last-child {
    flex: 1;
    max-height: none;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px 6px;
    flex-shrink: 0;
  }

  .section-title {
    font-size: 0.75em;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .header-actions {
    display: flex;
    gap: 4px;
  }

  .add-btn {
    background: none;
    border: 1px solid var(--border-secondary);
    color: var(--text-secondary);
    border-radius: 4px;
    padding: 2px 7px;
    font-size: 0.75em;
    cursor: pointer;
  }
  .add-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .empty-hint {
    margin: 0;
    padding: 10px 12px;
    font-size: 0.78em;
    color: var(--text-secondary);
    font-style: italic;
  }

  /* ── Scene list ──────────────────────────────────────────── */
  .scene-list {
    list-style: none;
    margin: 0;
    padding: 4px 0;
    overflow-y: auto;
    flex: 1;
  }

  .scene-item {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
  }
  .scene-item:hover { background: var(--bg-hover); }
  .scene-item.active { background: color-mix(in srgb, var(--accent-primary) 10%, transparent); }

  .scene-name {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 0.83em;
    text-align: left;
    cursor: pointer;
    padding: 4px 2px;
    min-width: 0;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--border-secondary);
    flex-shrink: 0;
  }
  .dot-active { background: var(--accent-primary); }

  .name-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .badge {
    font-size: 0.7em;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border-radius: 3px;
    padding: 1px 5px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .scene-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .scene-item:hover .scene-actions { opacity: 1; }

  /* ── Character grid ──────────────────────────────────────── */
  .char-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
    gap: 8px;
    padding: 8px 12px;
    overflow-y: auto;
    flex: 1;
  }

  .char-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
  }

  .char-portrait {
    position: relative;
    width: 60px;
    height: 60px;
    border-radius: 6px;
    overflow: hidden;
    border: 2px solid var(--border-secondary);
    background: var(--bg-tertiary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.15s;
    padding: 0;
  }
  .char-portrait:hover:not(.static) { border-color: var(--accent-primary); }
  .in-scene .char-portrait { border-color: var(--accent-primary); }

  /* Non-interactive portrait (story cast when no scene active) */
  .char-portrait.static {
    cursor: default;
  }

  .char-portrait img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .initials {
    font-size: 1.1em;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .scene-badge {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 14px;
    height: 14px;
    background: var(--accent-primary);
    border-radius: 50%;
    font-size: 0.55em;
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
  }

  .char-name {
    font-size: 0.7em;
    color: var(--text-secondary);
    text-align: center;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .nsfw-pip {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-danger, #f85149);
    vertical-align: middle;
    margin-left: 3px;
    flex-shrink: 0;
  }

  .char-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .char-tile:hover .char-actions { opacity: 1; }

  /* ── Available subgroup ──────────────────────────────────── */
  .subsection-header {
    display: flex;
    align-items: center;
    padding: 8px 12px 4px;
    flex-shrink: 0;
  }

  .subsection-title {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #6e7681);
  }

  .char-grid.available {
    padding-top: 0;
    flex: 0 0 auto;
    overflow-y: visible;
  }

  .char-grid.available .char-tile { opacity: 0.6; }
  .char-grid.available .char-tile:hover { opacity: 1; }

  /* ── Shared ──────────────────────────────────────────────── */
  .icon-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.85em;
    padding: 3px 5px;
    border-radius: 3px;
    line-height: 1;
  }
  .icon-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .icon-btn.small { font-size: 0.75em; padding: 2px 4px; }
  .icon-btn.danger:hover { color: var(--accent-danger); }

  /* ── Add-to-story inline modal ───────────────────────────── */
  .add-to-story-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 500;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .add-to-story-modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 10px;
    width: 320px;
    max-height: 420px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .ats-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }

  .ats-title {
    font-size: 0.85em;
    font-weight: 600;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .ats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
    gap: 8px;
    padding: 10px 12px;
    overflow-y: auto;
  }

  .ats-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
    transition: background 0.15s;
  }
  .ats-tile:hover { background: var(--bg-hover); }

  .ats-portrait {
    width: 56px;
    height: 56px;
    border-radius: 6px;
    overflow: hidden;
    border: 2px solid var(--border-secondary);
    background: var(--bg-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.15s;
  }
  .ats-tile:hover .ats-portrait { border-color: var(--accent-primary); }

  .ats-portrait img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
