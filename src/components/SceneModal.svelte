<!-- src/components/SceneModal.svelte — Create / edit a scene -->
<script lang="ts">
  import type { Scene } from '$lib/types';

  let {
    show = false,
    scene = null as Scene | null,
    onSave,
    onClose,
  }: {
    show?: boolean;
    scene?: Scene | null;
    onSave?: (form: Omit<Scene, 'id' | 'created_at'> & { id?: number }) => void;
    onClose?: () => void;
  } = $props();

  let name = $state('');
  let description = $state('');
  let location = $state('');
  let location_type = $state('interior');
  let time_of_day = $state('');
  let mood = $state('');

  $effect(() => {
    if (show) {
      name          = scene?.name          ?? '';
      description   = scene?.description   ?? '';
      location      = scene?.location      ?? '';
      location_type = scene?.location_type ?? 'interior';
      time_of_day   = scene?.time_of_day   ?? '';
      mood          = scene?.mood          ?? '';
    }
  });

  function handleSave() {
    if (!name.trim()) return;
    onSave?.({
      id: scene?.id,
      name: name.trim(),
      description: description.trim() || undefined,
      location: location.trim() || undefined,
      location_type: location_type || undefined,
      time_of_day: time_of_day.trim() || undefined,
      mood: mood.trim() || undefined,
    });
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose?.();
  }
</script>

{#if show}
<div class="overlay" role="dialog" aria-modal="true" onkeydown={handleKeydown}>
  <div class="modal">
    <div class="modal-header">
      <h2>{scene ? 'Edit Scene' : 'New Scene'}</h2>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <div class="modal-body">
      <label>
        Name <span class="required">*</span>
        <input bind:value={name} placeholder="e.g. The Tavern" />
      </label>

      <label>
        Description
        <textarea bind:value={description} rows={2} placeholder="Brief notes about this scene…"></textarea>
      </label>

      <label>
        Location
        <input bind:value={location} placeholder="e.g. a dimly lit medieval tavern" />
      </label>

      <div class="row">
        <label>
          Type
          <select bind:value={location_type}>
            <option value="interior">Interior</option>
            <option value="exterior">Exterior</option>
          </select>
        </label>

        <label>
          Time of Day
          <select bind:value={time_of_day}>
            <option value="">—</option>
            <option value="morning">Morning</option>
            <option value="afternoon">Afternoon</option>
            <option value="evening">Evening</option>
            <option value="night">Night</option>
          </select>
        </label>
      </div>

      <label>
        Mood / Atmosphere
        <input bind:value={mood} placeholder="e.g. tense, mysterious, warm" />
      </label>
    </div>

    <div class="modal-footer">
      <button class="cancel-btn" onclick={onClose}>Cancel</button>
      <button class="save-btn" onclick={handleSave} disabled={!name.trim()}>
        {scene ? 'Save Changes' : 'Create Scene'}
      </button>
    </div>
  </div>
</div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    width: 420px;
    max-width: 95vw;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border-primary);
  }

  h2 {
    margin: 0;
    font-size: 1em;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 1em;
    padding: 4px;
  }
  .close-btn:hover { color: var(--text-primary); }

  .modal-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.8em;
    color: var(--text-secondary);
  }

  .required { color: var(--accent-danger); }

  input, textarea, select {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.85em;
    padding: 6px 8px;
    outline: none;
    font-family: inherit;
  }
  input:focus, textarea:focus, select:focus {
    border-color: var(--border-active);
  }

  textarea { resize: vertical; }

  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px 16px;
    border-top: 1px solid var(--border-primary);
  }

  .cancel-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    color: var(--text-secondary);
    padding: 6px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85em;
  }
  .cancel-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .save-btn {
    background: var(--accent-primary);
    border: none;
    color: white;
    padding: 6px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85em;
    font-weight: 600;
  }
  .save-btn:hover:not(:disabled) { opacity: 0.9; }
  .save-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
