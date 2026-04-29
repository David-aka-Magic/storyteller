<!-- src/components/POVCharacterModal.svelte -->
<!--
  POV Character creation / edit modal.
  No portrait UI, no ComfyUI calls — only the descriptive fields the LLM needs.
  The "Appearance" field writes to sd_prompt (the field the context builder reads).
  is_pov is always true for characters managed here.
-->
<script lang="ts">
  import { addCharacter, updateCharacter, addCharacterToStory } from '$lib/api/character';
  import type { CharacterProfile } from '$lib/types';

  let {
    show = false,
    character = null,
    storyId = null,
    onsave,
    onclose,
  }: {
    show?: boolean;
    character?: CharacterProfile | null;
    storyId?: number | null;
    onsave?: (c: CharacterProfile) => void;
    onclose?: () => void;
  } = $props();

  const genders = ['Female', 'Male', 'Non-Binary', 'Other'];

  function makeEmpty(): CharacterProfile {
    return {
      id: 0,
      name: '',
      age: 25,
      gender: 'Female',
      skin_tone: '',
      hair_style: '',
      hair_color: '',
      eye_color: '',
      body_type: 'Average',
      height_scale: 3,
      weight_scale: 3,
      personality: '',
      additional_notes: '',
      default_clothing: '',
      sd_prompt: '',         // "Appearance" field writes here — this is what the context builder reads
      image: undefined,
      master_image_path: undefined,
      seed: undefined,
      art_style: 'Realistic',
      content_rating: 'sfw',
      is_pov: true,
    };
  }

  let form: CharacterProfile = $state(makeEmpty());
  let isSaving = $state(false);
  let saveError = $state('');
  let nameError = $state('');
  let initialFormJson = $state('');

  $effect(() => {
    if (show) {
      // Build the initial value into a plain local object first.
      // Do NOT read the reactive `form` $state after writing to it — that would
      // make `form` a tracked dependency of this effect and cause an infinite loop.
      const initial: CharacterProfile = character && character.id > 0
        ? { ...makeEmpty(), ...character, is_pov: true }
        : makeEmpty();
      form = initial;
      saveError = '';
      nameError = '';
      initialFormJson = JSON.stringify(initial);  // read local, not reactive `form`
    }
  });

  const isEditMode = $derived(!!character && character.id > 0);
  const isDirty = $derived(JSON.stringify(form) !== initialFormJson);

  function handleClose() {
    if (isDirty && !window.confirm('Discard unsaved changes?')) return;
    onclose?.();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) handleClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') handleClose();
  }

  async function handleSave() {
    nameError = '';
    saveError = '';

    if (!form.name.trim()) {
      nameError = 'Name is required.';
      return;
    }

    isSaving = true;
    try {
      let savedForm: CharacterProfile;
      if (isEditMode) {
        await updateCharacter({ ...form, is_pov: true });
        savedForm = { ...form, is_pov: true };
      } else {
        const newId = await addCharacter({ ...form, id: 0, is_pov: true });
        if (storyId != null) {
          await addCharacterToStory(newId, storyId);
        }
        savedForm = { ...form, id: newId, is_pov: true };
      }
      onsave?.(savedForm);
    } catch (e) {
      saveError = String(e);
    } finally {
      isSaving = false;
    }
  }
</script>

{#if show}
<div
  class="backdrop"
  onclick={handleBackdropClick}
  onkeydown={handleKeydown}
  role="button"
  tabindex="0"
>
  <div
    class="modal"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
  >
    <!-- Header -->
    <div class="modal-header">
      <div class="header-text">
        <h2 class="modal-title">
          {isEditMode ? `Edit ${form.name || 'Character'}` : 'Create Your Character'}
        </h2>
        <p class="modal-subtitle">
          This is the character you'll be playing — described to the AI for narrative
          consistency. No portrait will be generated.
        </p>
      </div>
      <button class="close-btn" onclick={handleClose} aria-label="Close">✕</button>
    </div>

    <!-- Body -->
    <div class="modal-body">
      <div class="pov-notice">
        <span class="pov-badge">POV</span>
        <span class="pov-notice-text">Player character — no portrait, no image generation</span>
      </div>

      <!-- Name -->
      <div class="form-group" class:has-error={!!nameError}>
        <label for="pov-name">
          Name <span class="required">*</span>
        </label>
        <input
          id="pov-name"
          type="text"
          bind:value={form.name}
          placeholder="Your character's name"
          maxlength="80"
          oninput={() => nameError = ''}
        />
        {#if nameError}
          <span class="field-error">{nameError}</span>
        {/if}
      </div>

      <!-- Age + Gender row -->
      <div class="form-row-2">
        <div class="form-group">
          <label for="pov-age">Age</label>
          <input
            id="pov-age"
            type="number"
            bind:value={form.age}
            min="1"
            max="999"
          />
        </div>
        <div class="form-group">
          <label for="pov-gender">Gender</label>
          <select id="pov-gender" bind:value={form.gender}>
            {#each genders as g}
              <option value={g}>{g}</option>
            {/each}
          </select>
        </div>
      </div>

      <!-- Personality -->
      <div class="form-group">
        <label for="pov-personality">Personality</label>
        <textarea
          id="pov-personality"
          bind:value={form.personality}
          placeholder="Describe your character's personality, quirks, and disposition…"
          rows="3"
        ></textarea>
      </div>

      <!-- Appearance (writes to sd_prompt — what the LLM context builder reads) -->
      <div class="form-group">
        <label for="pov-appearance">Appearance</label>
        <span class="field-hint">
          Physical description — how they look in the world, used by the AI for narrative
          consistency even though no portrait will be drawn.
        </span>
        <textarea
          id="pov-appearance"
          bind:value={form.sd_prompt}
          placeholder="Describe their build, hair, eyes, distinguishing features…"
          rows="3"
        ></textarea>
      </div>

      <!-- Default Clothing -->
      <div class="form-group">
        <label for="pov-clothing">Default Clothing</label>
        <input
          id="pov-clothing"
          type="text"
          bind:value={form.default_clothing}
          placeholder="What they typically wear…"
          maxlength="200"
        />
      </div>

      {#if saveError}
        <div class="save-error">{saveError}</div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="modal-footer">
      <button class="btn btn-secondary" onclick={handleClose} disabled={isSaving}>
        Cancel
      </button>
      <button class="btn btn-primary" onclick={handleSave} disabled={isSaving}>
        {isSaving ? 'Saving…' : 'Save Character'}
      </button>
    </div>
  </div>
</div>
{/if}

<style>
  /* ── Backdrop ── */
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1200;
    backdrop-filter: blur(4px);
  }

  .modal {
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-primary, #30363d);
    border-radius: 14px;
    width: 480px;
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 24px 56px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  /* ── Header ── */
  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 20px 22px 16px;
    border-bottom: 1px solid var(--border-primary, #30363d);
    background: var(--bg-secondary, #161b22);
  }

  .header-text {
    flex: 1;
    min-width: 0;
  }

  .modal-title {
    margin: 0 0 4px;
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary, #c9d1d9);
  }

  .modal-subtitle {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-muted, #6e7681);
    line-height: 1.5;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.3rem;
    cursor: pointer;
    color: var(--text-muted, #6e7681);
    padding: 2px;
    line-height: 1;
    transition: color 0.15s;
    flex-shrink: 0;
  }

  .close-btn:hover { color: var(--text-primary, #c9d1d9); }

  /* ── Body ── */
  .modal-body {
    padding: 20px 22px;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* ── POV Notice ── */
  .pov-notice {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(88, 166, 255, 0.08);
    border: 1px solid rgba(88, 166, 255, 0.2);
    border-radius: 7px;
  }

  .pov-badge {
    font-size: 0.62rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    padding: 2px 7px;
    border-radius: 4px;
    background: var(--accent-primary, #58a6ff);
    color: #0d1117;
    flex-shrink: 0;
  }

  .pov-notice-text {
    font-size: 0.78rem;
    color: var(--text-secondary, #8b949e);
  }

  /* ── Form ── */
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin: 0;
  }

  .form-group.has-error input,
  .form-group.has-error textarea,
  .form-group.has-error select {
    border-color: var(--accent-danger, #f85149);
  }

  .form-group label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary, #8b949e);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .required {
    color: var(--accent-danger, #f85149);
  }

  .field-hint {
    font-size: 0.72rem;
    color: var(--text-muted, #6e7681);
    line-height: 1.4;
    margin-top: -2px;
  }

  .form-group input,
  .form-group textarea,
  .form-group select {
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-secondary, #30363d);
    border-radius: 7px;
    padding: 9px 11px;
    color: var(--text-primary, #c9d1d9);
    font-size: 0.88rem;
    font-family: inherit;
    outline: none;
    transition: border-color 0.2s;
    box-sizing: border-box;
    width: 100%;
  }

  .form-group input::placeholder,
  .form-group textarea::placeholder {
    color: var(--text-muted, #6e7681);
  }

  .form-group input:focus,
  .form-group textarea:focus,
  .form-group select:focus {
    border-color: var(--accent-primary, #58a6ff);
  }

  .form-group textarea {
    resize: vertical;
    min-height: 72px;
    line-height: 1.5;
  }

  .form-row-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .field-error {
    font-size: 0.72rem;
    color: var(--accent-danger, #f85149);
  }

  .save-error {
    padding: 10px 12px;
    background: rgba(248, 81, 73, 0.08);
    border: 1px solid rgba(248, 81, 73, 0.25);
    border-radius: 7px;
    font-size: 0.8rem;
    color: var(--accent-danger, #f85149);
    line-height: 1.4;
  }

  /* ── Footer ── */
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 22px;
    border-top: 1px solid var(--border-primary, #30363d);
    background: var(--bg-secondary, #161b22);
  }

  .btn {
    padding: 9px 20px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
    font-family: inherit;
    transition: background 0.15s, opacity 0.15s;
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-tertiary, #21262d);
    color: var(--text-secondary, #8b949e);
    border: 1px solid var(--border-secondary, #30363d);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover, #30363d);
  }

  .btn-primary {
    background: var(--accent-primary, #58a6ff);
    color: #0d1117;
    font-weight: 700;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.88;
  }
</style>
