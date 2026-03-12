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
    collapsed = false,
    refreshKey = 0,
    onToggleCollapse,
    onCreateCharacter,
    onEditCharacter,
    onDeleteCharacter,
    onsceneselected,
  }: {
    storyId?: number | null;
    storyCharacters?: CharacterProfile[];
    collapsed?: boolean;
    /** Increment to force a reload (e.g. after auto-scene-sync from a turn). */
    refreshKey?: number;
    onToggleCollapse?: (val: boolean) => void;
    onCreateCharacter?: () => void;
    onEditCharacter?: (c: CharacterProfile) => void;
    onDeleteCharacter?: (id: number) => void;
    /** Called when the user manually clicks a scene to activate it. */
    onsceneselected?: (sceneId: number, sceneName: string, characterNames: string[]) => void;
  } = $props();

  let scenes: Scene[] = $state([]);
  let activeSceneId: number | null = $state(null);
  let sceneCharacters: CharacterProfile[] = $state([]);
  let portraitUrls: Map<number, string | null> = $state(new Map());

  let showSceneModal = $state(false);
  let sceneToEdit: Scene | null = $state(null);
  let showAddExistingModal = $state(false);

  // ── Resolve portrait URLs whenever storyCharacters changes ───────────────
  $effect(() => {
    const chars = storyCharacters;
    Promise.all(
      chars.map(async (c) => {
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

  function getSceneArtStyle(): string | null {
    if (sceneCharacters.length === 0) return null;
    const styles = new Set(sceneCharacters.map(c => c.art_style).filter(Boolean));
    if (styles.size === 1) return [...styles][0] ?? null;
    return null;
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
      <span class="panel-title">Scenes</span>
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

      <!-- ── Characters in active scene ───────────────────── -->
      <section class="section">
        <div class="section-header">
          <span class="section-title">
            {activeSceneId != null ? 'Characters in Scene' : 'Characters'}
          </span>
          <div class="header-actions">
            {#if activeSceneId != null}
              <button
                class="add-btn"
                onclick={() => { showAddExistingModal = true; }}
                title="Add an existing character to this scene"
              >+ Existing</button>
            {/if}
            <button class="add-btn" onclick={onCreateCharacter}>+ New</button>
          </div>
        </div>

        {#if storyCharacters.length === 0}
          <p class="empty-hint">No characters for this story.</p>
        {:else}
          <div class="char-grid">
            {#each storyCharacters as char (char.id)}
              {@const inScene = sceneCharacters.some(c => c.id === char.id)}
              {@const portrait = portraitUrls.get(char.id) ?? null}
              <div class="char-tile" class:in-scene={inScene}>
                <button
                  class="char-portrait"
                  onclick={() => activeSceneId != null && toggleCharacterInScene(char.id)}
                  title={activeSceneId != null ? (inScene ? 'Remove from scene' : 'Add to scene') : 'Select an active scene first'}
                  disabled={activeSceneId == null}
                >
                  {#if portrait}
                    <img src={portrait} alt={char.name} />
                  {:else}
                    <span class="initials">{initials(char.name)}</span>
                  {/if}
                  {#if inScene}
                    <span class="scene-badge">✓</span>
                  {/if}
                </button>
                <span class="char-name">{char.name}</span>
                <div class="char-actions">
                  <button class="icon-btn small" onclick={() => onEditCharacter?.(char)} title="Edit">✏</button>
                  <button class="icon-btn small danger" onclick={() => onDeleteCharacter?.(char.id)} title="Delete">✕</button>
                </div>
              </div>
            {/each}
          </div>
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
  artStyle={getSceneArtStyle()}
  excludeIds={sceneCharacters.map(c => c.id)}
  onSelect={handleAddExistingCharacter}
  onClose={() => { showAddExistingModal = false; }}
/>

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
  .char-portrait:hover:not(:disabled) { border-color: var(--accent-primary); }
  .char-portrait:disabled { cursor: default; opacity: 0.6; }
  .in-scene .char-portrait { border-color: var(--accent-primary); }

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

  .char-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .char-tile:hover .char-actions { opacity: 1; }

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
</style>
