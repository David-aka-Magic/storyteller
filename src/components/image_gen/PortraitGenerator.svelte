<!-- src/components/image_gen/PortraitGenerator.svelte — Wizard: composes PortraitForm + PortraitGallery -->
<script lang="ts">
  import Modal from '../shared/Modal.svelte';
  import PortraitForm from './PortraitForm.svelte';
  import PortraitGallery from './PortraitGallery.svelte';
  import {
    previewPortraitPrompt,
    generateMasterPortrait,
    saveMasterPortrait as apiSaveMasterPortrait,
  } from '$lib/api/image-gen';
  import type { CharacterProfile } from '$lib/types';

  type PortraitFormData = {
    name: string;
    age: number | undefined;
    gender: string;
    skin_tone: string;
    hair_color: string;
    hair_style: string;
    body_type: string;
    default_clothing: string;
    physical_features: string;
    art_style: string;
    custom_prompt: string;
  };

  let {
    show = false,
    character = null,
    storyId = undefined,
    onsaved,
    onclose,
  }: {
    show?: boolean;
    character?: Partial<CharacterProfile> | null;
    storyId?: number;
    onsaved?: (data: { characterId: number; masterImagePath: string }) => void;
    onclose?: () => void;
  } = $props();

  // ── State ──
  let form            = $state<PortraitFormData>(getEmptyForm());
  let generatedImages = $state<string[]>([]);
  let generatedPaths  = $state<string[]>([]);
  let selectedIndex   = $state(-1);
  let isGenerating    = $state(false);
  let isSaving        = $state(false);
  let promptPreview   = $state('');
  let generationError = $state('');
  let seedUsed        = $state(-1);
  let promptId        = $state('');

  // ── Reset + populate form when modal opens ──
  $effect(() => {
    if (show) {
      form = character
        ? {
            name:             character.name || '',
            age:              character.age,
            gender:           character.gender || 'Male',
            skin_tone:        character.skin_tone || '',
            hair_color:       character.hair_color || '',
            hair_style:       character.hair_style || '',
            body_type:        character.body_type || 'Average',
            default_clothing: character.default_clothing || '',
            physical_features: character.additional_notes || '',
            art_style:        character.art_style || 'Realistic',
            custom_prompt:    '',
          }
        : getEmptyForm();
      generatedImages = [];
      generatedPaths  = [];
      selectedIndex   = -1;
      generationError = '';
      updatePromptPreview();
    }
  });

  // ── Form defaults ──
  function getEmptyForm(): PortraitFormData {
    return {
      name: '', age: undefined, gender: 'Male',
      skin_tone: '', hair_color: '', hair_style: '',
      body_type: 'Average', default_clothing: '',
      physical_features: '', art_style: 'Realistic', custom_prompt: '',
    };
  }

  // ── Prompt preview ──
  async function updatePromptPreview() {
    try {
      promptPreview = await previewPortraitPrompt({
        name:              form.name,
        age:               form.age || null,
        gender:            form.gender || null,
        skin_tone:         form.skin_tone || null,
        hair_color:        form.hair_color || null,
        hair_style:        form.hair_style || null,
        body_type:         form.body_type || null,
        default_clothing:  form.default_clothing || null,
        physical_features: form.physical_features || null,
        art_style:         form.art_style || null,
        custom_prompt:     form.custom_prompt || null,
      });
    } catch {
      promptPreview = buildLocalPromptPreview();
    }
  }

  function buildLocalPromptPreview(): string {
    if (form.custom_prompt) return form.custom_prompt;
    const parts = ['(masterpiece, best quality)'];
    const gTag = form.gender === 'Female' ? '1girl' : form.gender === 'Male' ? '1boy' : '1person';
    parts.push(`solo, ${gTag}`);
    parts.push('portrait, upper body, looking at viewer');
    if (form.age) parts.push(`${form.age} year old`);
    if (form.skin_tone) parts.push(`${form.skin_tone.toLowerCase()} skin`);
    if (form.hair_color && form.hair_style) {
      parts.push(`${form.hair_color.toLowerCase()} ${form.hair_style.toLowerCase()} hair`);
    } else if (form.hair_color) {
      parts.push(`${form.hair_color.toLowerCase()} hair`);
    }
    if (form.body_type && form.body_type !== 'Average') parts.push(`${form.body_type.toLowerCase()} body`);
    if (form.physical_features) parts.push(form.physical_features.toLowerCase());
    if (form.default_clothing) parts.push(`wearing ${form.default_clothing}`);
    parts.push('neutral gray background, detailed face, sharp focus, soft studio lighting');
    return parts.join(', ');
  }

  // Debounced prompt update
  let promptTimeout: ReturnType<typeof setTimeout>;
  function onFormChange() {
    clearTimeout(promptTimeout);
    promptTimeout = setTimeout(updatePromptPreview, 300);
  }

  // ── Generate portraits ──
  async function generatePortraits() {
    if (isGenerating) return;
    if (!form.name.trim()) {
      generationError = 'Please enter a character name.';
      return;
    }

    isGenerating    = true;
    generationError = '';
    generatedImages = [];
    generatedPaths  = [];
    selectedIndex   = -1;

    try {
      const result = await generateMasterPortrait({
        name:              form.name,
        age:               form.age || null,
        gender:            form.gender || null,
        skin_tone:         form.skin_tone || null,
        hair_color:        form.hair_color || null,
        hair_style:        form.hair_style || null,
        body_type:         form.body_type || null,
        default_clothing:  form.default_clothing || null,
        physical_features: form.physical_features || null,
        art_style:         form.art_style || null,
        custom_prompt:     form.custom_prompt || null,
        seed:              null,
      });

      generatedImages = result.images_base64;
      generatedPaths  = result.image_paths;
      promptPreview   = result.prompt_used;
      seedUsed        = result.seed;
      promptId        = result.prompt_id;
    } catch (e) {
      generationError = `Generation failed: ${e}`;
      console.error('[PortraitGenerator]', e);
    } finally {
      isGenerating = false;
    }
  }

  // ── Save selected master ──
  async function savePortrait() {
    if (selectedIndex < 0 || isSaving) return;
    if (!character?.id) {
      generationError = 'Character must be saved first before setting a master image.';
      return;
    }

    isSaving        = true;
    generationError = '';

    try {
      const masterPath = await apiSaveMasterPortrait({
        character_id:    character.id!,
        selected_index:  selectedIndex,
        image_paths:     generatedPaths,
        character_name:  form.name,
      });

      onsaved?.({ characterId: character.id, masterImagePath: masterPath });
    } catch (e) {
      generationError = `Failed to save master image: ${e}`;
      console.error('[PortraitGenerator]', e);
    } finally {
      isSaving = false;
    }
  }
</script>

{#if show}
  <Modal title="🎨 Master Portrait Generator" {onclose} width="1050px">
    <p class="subtitle">Create the reference image for IP-Adapter consistency</p>

    <div class="content-layout">
      <PortraitForm
        {form}
        {promptPreview}
        {isGenerating}
        onchange={onFormChange}
        ongenerate={generatePortraits}
      />

      <PortraitGallery
        images={generatedImages}
        {selectedIndex}
        {isGenerating}
        {generationError}
        {seedUsed}
        {character}
        {isSaving}
        onselect={(i) => (selectedIndex = i)}
        onsave={savePortrait}
      />
    </div>

    <div class="actions">
      <button class="cancel-btn" onclick={onclose}>Cancel</button>
      {#if generatedImages.length > 0}
        <button class="regen-btn" onclick={generatePortraits} disabled={isGenerating}>
          🔄 Regenerate
        </button>
      {/if}
    </div>
  </Modal>
{/if}

<style>
  .subtitle {
    margin: 0;
    padding: 0 25px 12px;
    font-size: 0.85em;
    color: var(--text-secondary, #888);
    border-bottom: 1px solid var(--border-secondary, #2a2a4a);
  }

  .content-layout {
    display: flex;
    gap: 0;
    min-height: 500px;
  }

  .actions {
    padding: 15px 25px;
    border-top: 1px solid var(--border-secondary, #2a2a4a);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .cancel-btn {
    padding: 10px 20px;
    background: var(--bg-secondary, #2a2a4a);
    color: var(--text-primary, #ccc);
    border: 1px solid var(--border-secondary, #444);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9em;
  }

  .cancel-btn:hover {
    background: #3a3a5a;
  }

  .regen-btn {
    padding: 10px 20px;
    background: var(--bg-secondary, #2a2a4a);
    color: var(--accent, #4a9eff);
    border: 1px solid var(--accent, #4a9eff);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9em;
  }

  .regen-btn:hover:not(:disabled) {
    background: rgba(74, 158, 255, 0.1);
  }

  .regen-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
