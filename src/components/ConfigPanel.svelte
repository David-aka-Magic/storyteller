<!-- src/components/ConfigPanel.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { StoryPremise, CharacterProfile } from '../lib/types';

  export let stories: StoryPremise[] = [];
  export let characters: CharacterProfile[] = [];
  export let selectedStoryId: string = '';
  export let selectedCharacterIds: Set<number> = new Set();
  export let collapsed: boolean = false;

  export let onToggleCollapse: (collapsed: boolean) => void = () => {};
  export let onSelectStory: (id: string) => void = () => {};
  export let onCreateStory: () => void = () => {};
  export let onEditStory: (story: StoryPremise) => void = () => {};
  export let onDeleteStory: (id: string) => void = () => {};
  export let onToggleCharacter: (id: number) => void = () => {};
  export let onCreateCharacter: () => void = () => {};
  export let onEditCharacter: (char: CharacterProfile) => void = () => {};
  export let onDeleteCharacter: (id: number) => void = () => {};
  export let onLinkToStory: (characterId: number) => void = () => {};

  // ── Add Existing picker state ──
  let showPicker = false;
  let pickerCharacters: CharacterProfile[] = [];
  let pickerSelected: Set<number> = new Set();
  let pickerLoading = false;
  let pickerError = '';

  function toggleCollapse() {
    collapsed = !collapsed;
    onToggleCollapse(collapsed);
  }

  async function openPicker() {
      if (!selectedStoryId || selectedStoryId === '1') return;
  
      pickerLoading = true;
      pickerError = '';
      pickerSelected = new Set();
      showPicker = true;
  
      try {
          const all = await invoke<CharacterProfile[]>('list_characters_for_story', { storyId: null });
          const currentStoryId = parseInt(selectedStoryId, 10);
          
          // Show characters not in this story — use loose == to handle number/string mismatch
          pickerCharacters = all.filter(c => {
              const charStoryId = c.story_id ? Number(c.story_id) : null;
              return charStoryId !== currentStoryId;
          });
  
          console.log('[Picker] All characters:', all.length, 'Filtered:', pickerCharacters.length, 'Current story:', currentStoryId);
      } catch (e) {
          pickerError = `Failed to load characters: ${e}`;
      } finally {
          pickerLoading = false;
      }
  }

  function togglePickerChar(id: number) {
    if (pickerSelected.has(id)) {
      pickerSelected.delete(id);
    } else {
      pickerSelected.add(id);
    }
    pickerSelected = pickerSelected;
  }

  async function confirmPicker() {
    if (pickerSelected.size === 0) return;

    pickerLoading = true;
    try {
      for (const charId of pickerSelected) {
        onLinkToStory(charId);
      }
      showPicker = false;
    } catch (e) {
      pickerError = `Failed to link characters: ${e}`;
    } finally {
      pickerLoading = false;
    }
  }

  function closePicker() {
    showPicker = false;
    pickerCharacters = [];
    pickerSelected = new Set();
    pickerError = '';
  }

  // Close picker on backdrop click
  function handlePickerBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) closePicker();
  }

  $: isStorySelected = selectedStoryId && selectedStoryId !== '1';
</script>

<div class="config-panel" class:collapsed>
  <!-- Collapse Toggle -->
  <button class="collapse-toggle" on:click={toggleCollapse} title={collapsed ? 'Expand panel' : 'Collapse panel'}>
    <span class="toggle-icon">{collapsed ? '◀' : '▶'}</span>
  </button>

  {#if !collapsed}
    <div class="panel-content">

      <!-- ── Story Setting ── -->
      <div class="config-section">
        <div class="section-header">
          <h2>Story Setting</h2>
          <button class="create-btn" on:click={onCreateStory}>+ Create Story</button>
        </div>

        <div class="story-list">
          {#each stories as story (story.id)}
            <label class="radio-item" class:selected-radio={selectedStoryId === story.id}>
              <input
                type="radio"
                name="storyGroup"
                value={story.id}
                checked={selectedStoryId === story.id}
                on:change={() => onSelectStory(story.id)}
              />
              <div class="radio-content">
                <span class="radio-title">{story.title}</span>
                <span class="radio-desc">{story.description}</span>
              </div>
              {#if story.id !== '1'}
                <div class="story-controls">
                  <button class="edit-btn" on:click|preventDefault={() => onEditStory(story)}>✎</button>
                  <button class="delete-btn" on:click|preventDefault={() => onDeleteStory(story.id)}>🗑️</button>
                </div>
              {/if}
            </label>
          {/each}
        </div>
      </div>

      <!-- ── Characters ── -->
      <div class="config-section">
        <div class="section-header">
          <h2>Characters</h2>
          <div class="header-btns">
            {#if isStorySelected}
              <button class="add-existing-btn" on:click={openPicker} title="Add a character from another story">
                + Add Existing
              </button>
            {/if}
            <button class="create-btn" on:click={onCreateCharacter}>+ Create</button>
          </div>
        </div>

        {#if characters.length === 0}
          <p class="empty-state">No characters yet.</p>
        {:else}
          <div class="char-list">
            {#each characters as char (char.id)}
              <div class="char-item" class:active={selectedCharacterIds.has(char.id)}>
                <label class="char-checkbox-area">
                  <input
                    type="checkbox"
                    checked={selectedCharacterIds.has(char.id)}
                    on:change={() => onToggleCharacter(char.id)}
                  />
                  <div class="char-info">
                    <span class="char-name">{char.name}</span>
                    <span class="char-meta">{char.gender}, {char.age}</span>
                  </div>
                </label>
                <div class="char-controls">
                  <button class="edit-icon" on:click={() => onEditCharacter(char)}>✎</button>
                  <button class="delete-icon" on:click={() => onDeleteCharacter(char.id)}>🗑️</button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

    </div>
  {:else}
    <!-- Collapsed View -->
    <div class="collapsed-content">
      <div class="collapsed-icon" title="Story Settings">📖</div>
      <div class="collapsed-icon" title="Characters">👤</div>
    </div>
  {/if}
</div>

<!-- ── Add Existing Character Picker Modal ── -->
{#if showPicker}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="picker-backdrop" on:click={handlePickerBackdrop}>
    <div class="picker-modal">
      <div class="picker-header">
        <h3>Add Existing Character</h3>
        <button class="picker-close" on:click={closePicker}>✕</button>
      </div>

      <p class="picker-subtitle">
        Select characters from other stories to add to this one.
      </p>

      {#if pickerLoading}
        <div class="picker-loading">Loading characters…</div>
      {:else if pickerError}
        <div class="picker-error">{pickerError}</div>
      {:else if pickerCharacters.length === 0}
        <div class="picker-empty">
          <span class="picker-empty-icon">👤</span>
          <p>No other characters found.</p>
          <p class="picker-empty-hint">Characters you create in other stories will appear here.</p>
        </div>
      {:else}
        <div class="picker-list">
          {#each pickerCharacters as char (char.id)}
            <!-- svelte-ignore a11y-click-events-have-key-events -->
            <!-- svelte-ignore a11y-no-static-element-interactions -->
            <div
              class="picker-char"
              class:selected={pickerSelected.has(char.id)}
              on:click={() => togglePickerChar(char.id)}
            >
              <div class="picker-avatar">
                {#if char.image}
                  <img src="data:image/png;base64,{char.image}" alt={char.name} />
                {:else}
                  <span class="picker-avatar-placeholder">
                    {char.name.charAt(0).toUpperCase()}
                  </span>
                {/if}
              </div>
              <div class="picker-char-info">
                <span class="picker-char-name">{char.name}</span>
                <span class="picker-char-meta">{char.gender ?? ''}
                  {char.age ? `· ${char.age}` : ''}
                  {char.master_image_path ? '· 🎨' : ''}
                </span>
              </div>
              <div class="picker-check" class:visible={pickerSelected.has(char.id)}>✓</div>
            </div>
          {/each}
        </div>
      {/if}

      <div class="picker-footer">
        <span class="picker-count">
          {pickerSelected.size > 0 ? `${pickerSelected.size} selected` : ''}
        </span>
        <div class="picker-actions">
          <button class="picker-cancel" on:click={closePicker}>Cancel</button>
          <button
            class="picker-confirm"
            disabled={pickerSelected.size === 0 || pickerLoading}
            on:click={confirmPicker}
          >
            Add {pickerSelected.size > 0 ? `(${pickerSelected.size})` : ''}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Panel ── */
  .config-panel {
    width: 300px;
    background: var(--bg-tertiary);
    border-left: 1px solid var(--border-primary);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    position: relative;
    transition: width 0.3s ease;
  }
  .config-panel.collapsed { width: 50px; }

  .collapse-toggle {
    position: absolute;
    left: -12px;
    top: 50%;
    transform: translateY(-50%);
    width: 24px;
    height: 48px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    border-radius: 4px 0 0 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 0.8em;
    z-index: 10;
    transition: all 0.2s;
  }
  .collapse-toggle:hover { background: var(--bg-hover); color: var(--text-primary); }

  .panel-content {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 30px;
  }

  .collapsed-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 20px;
    gap: 20px;
  }
  .collapsed-icon { font-size: 1.5em; cursor: default; opacity: 0.6; }

  /* ── Section ── */
  .config-section h2 {
    font-size: 1.2em;
    color: var(--text-primary);
    margin-bottom: 15px;
    padding-bottom: 5px;
    border-bottom: 2px solid var(--border-secondary);
  }
  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 15px;
  }
  .section-header h2 { margin-bottom: 0; border-bottom: none; }

  .header-btns {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .create-btn {
    background: var(--accent-success);
    color: white;
    border: none;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 0.8em;
    cursor: pointer;
    transition: opacity 0.2s;
  }
  .create-btn:hover { opacity: 0.9; }

  .add-existing-btn {
    background: transparent;
    color: var(--accent-primary, #58a6ff);
    border: 1px solid var(--accent-primary, #58a6ff);
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 0.8em;
    cursor: pointer;
    transition: all 0.2s;
  }
  .add-existing-btn:hover {
    background: var(--accent-primary, #58a6ff);
    color: white;
  }

  .empty-state { font-size: 0.9em; color: var(--text-muted); font-style: italic; }

  /* ── Story list ── */
  .story-list { display: flex; flex-direction: column; gap: 10px; }

  .radio-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border-secondary);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }
  .radio-item:hover { background: var(--bg-hover); border-color: var(--accent-primary); }
  .selected-radio { border-color: var(--accent-primary); background: var(--bg-hover); }

  .radio-content { flex: 1; min-width: 0; }
  .radio-title { display: block; font-weight: 600; font-size: 0.9em; color: var(--text-primary); }
  .radio-desc {
    display: block;
    font-size: 0.75em;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .story-controls { display: flex; gap: 4px; flex-shrink: 0; }
  .edit-btn, .delete-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 5px;
    border-radius: 3px;
    font-size: 0.9em;
    opacity: 0.6;
    transition: opacity 0.2s;
  }
  .edit-btn:hover, .delete-btn:hover { opacity: 1; }

  /* ── Character list ── */
  .char-list { display: flex; flex-direction: column; gap: 8px; }

  .char-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border-secondary);
    border-radius: 6px;
    transition: all 0.2s;
  }
  .char-item.active { border-color: var(--accent-primary); }
  .char-item:hover { background: var(--bg-hover); }

  .char-checkbox-area {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    flex: 1;
    min-width: 0;
  }
  .char-info { display: flex; flex-direction: column; min-width: 0; }
  .char-name { font-size: 0.9em; font-weight: 600; color: var(--text-primary); }
  .char-meta { font-size: 0.75em; color: var(--text-muted); }

  .char-controls { display: flex; gap: 4px; flex-shrink: 0; }
  .edit-icon, .delete-icon {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 5px;
    border-radius: 3px;
    font-size: 0.9em;
    opacity: 0.5;
    transition: opacity 0.2s;
  }
  .edit-icon:hover, .delete-icon:hover { opacity: 1; }

  /* ── Picker Modal ── */
  .picker-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .picker-modal {
    background: var(--bg-secondary, #161b22);
    border: 1px solid var(--border-primary, #30363d);
    border-radius: 12px;
    width: 380px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  .picker-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-primary, #30363d);
  }
  .picker-header h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--text-primary, #c9d1d9);
  }
  .picker-close {
    background: none;
    border: none;
    color: var(--text-muted, #6e7681);
    cursor: pointer;
    font-size: 1rem;
    padding: 4px 8px;
    border-radius: 4px;
    transition: color 0.2s;
  }
  .picker-close:hover { color: var(--text-primary, #c9d1d9); }

  .picker-subtitle {
    margin: 0;
    padding: 10px 20px;
    font-size: 0.82em;
    color: var(--text-muted, #6e7681);
    border-bottom: 1px solid var(--border-primary, #30363d);
  }

  .picker-loading, .picker-error {
    padding: 30px 20px;
    text-align: center;
    font-size: 0.9em;
    color: var(--text-muted, #6e7681);
  }
  .picker-error { color: var(--accent-danger, #f85149); }

  .picker-empty {
    padding: 40px 20px;
    text-align: center;
  }
  .picker-empty-icon { font-size: 2em; display: block; margin-bottom: 10px; }
  .picker-empty p { margin: 0 0 6px; color: var(--text-muted, #6e7681); font-size: 0.9em; }
  .picker-empty-hint { font-size: 0.8em !important; opacity: 0.7; }

  .picker-list {
    overflow-y: auto;
    flex: 1;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .picker-char {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 8px;
    cursor: pointer;
    border: 1px solid transparent;
    transition: all 0.15s;
    background: var(--bg-primary, #0d1117);
  }
  .picker-char:hover { background: var(--bg-hover, #1c2128); border-color: var(--border-secondary, #21262d); }
  .picker-char.selected {
    background: rgba(88, 166, 255, 0.1);
    border-color: var(--accent-primary, #58a6ff);
  }

  .picker-avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    overflow: hidden;
    flex-shrink: 0;
    background: var(--bg-tertiary, #21262d);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .picker-avatar img { width: 100%; height: 100%; object-fit: cover; }
  .picker-avatar-placeholder {
    font-size: 1.2em;
    font-weight: 700;
    color: var(--text-muted, #6e7681);
  }

  .picker-char-info { flex: 1; min-width: 0; }
  .picker-char-name {
    display: block;
    font-size: 0.9em;
    font-weight: 600;
    color: var(--text-primary, #c9d1d9);
  }
  .picker-char-meta {
    display: block;
    font-size: 0.75em;
    color: var(--text-muted, #6e7681);
  }

  .picker-check {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--accent-primary, #58a6ff);
    color: white;
    font-size: 0.8em;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.15s;
    flex-shrink: 0;
  }
  .picker-check.visible { opacity: 1; }

  .picker-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-top: 1px solid var(--border-primary, #30363d);
    background: var(--bg-secondary, #161b22);
  }
  .picker-count {
    font-size: 0.8em;
    color: var(--text-muted, #6e7681);
    min-width: 80px;
  }
  .picker-actions { display: flex; gap: 8px; }

  .picker-cancel {
    background: transparent;
    border: 1px solid var(--border-primary, #30363d);
    color: var(--text-secondary, #8b949e);
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.85em;
    cursor: pointer;
    transition: all 0.2s;
  }
  .picker-cancel:hover { border-color: var(--text-muted); color: var(--text-primary); }

  .picker-confirm {
    background: var(--accent-primary, #58a6ff);
    border: none;
    color: white;
    padding: 6px 16px;
    border-radius: 6px;
    font-size: 0.85em;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }
  .picker-confirm:hover:not(:disabled) { opacity: 0.85; }
  .picker-confirm:disabled { opacity: 0.4; cursor: not-allowed; }
</style>