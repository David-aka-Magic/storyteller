<!-- src/components/chat/ChatArea.svelte — Messages list + input -->
<script lang="ts">
  import { tick } from 'svelte';
  import ChatInput from './ChatInput.svelte';
  import MessageBubble from './MessageBubble.svelte';
  import { regenerateStory, regenerateStoryWithInput, generateSceneImageForTurn } from '$lib/api/text-gen';
  import { readFileBase64 } from '$lib/api/image-gen';
  import type { ChatMessage, StoryTurnResult } from '$lib/types';

  let {
    messages = [],
    isLoading = false,
    currentChatId,
    activeCharacterId = null,
    storyId = null,
    onsendmessage,
    onimagegenerated,
    onregenerated,
  }: {
    messages?: ChatMessage[];
    isLoading?: boolean;
    currentChatId: number;
    activeCharacterId?: string | null;
    storyId?: number | null;
    onsendmessage?: (text: string) => void;
    onimagegenerated?: (msgId: number, src: string, filePath: string) => void;
    onregenerated?: (newMsg: ChatMessage) => void;
  } = $props();

  let messagesContainer: HTMLDivElement;

  async function scrollToBottom() {
    await tick();
    if (messagesContainer) {
      messagesContainer.scrollTo({ top: messagesContainer.scrollHeight, behavior: 'smooth' });
    }
  }

  $effect(() => {
    messages.length;
    isLoading;
    scrollToBottom();
  });

  // Local loading flag for regenerate (separate from parent isLoading)
  let isRegenerating = $state(false);
  let generatingImages = $state(new Set<number>());
  let imageErrors = $state(new Map<number, string>());

  function setImageError(msgId: number, err: string | null) {
    const next = new Map(imageErrors);
    if (err === null) next.delete(msgId);
    else next.set(msgId, err);
    imageErrors = next;
  }

  async function generateImage(msg: ChatMessage, positivePrompt?: string, negativePrompt?: string) {
    if (generatingImages.has(msg.id)) return;
    const basePrompt = msg.data?.sd_prompt || msg.data?.story;
    if (!basePrompt && !positivePrompt) {
      setImageError(msg.id, 'No text available to generate an image.');
      return;
    }

    setImageError(msg.id, null);
    generatingImages = new Set([...generatingImages, msg.id]);
    try {
      const path = await generateSceneImageForTurn(basePrompt ?? '', storyId ?? undefined, msg.sceneCharacterNames, msg.sceneCharacterPoses, positivePrompt, negativePrompt);
      const base64 = await readFileBase64(path);
      onimagegenerated?.(msg.id, `data:image/png;base64,${base64}`, path);
      await scrollToBottom();
    } catch (e) {
      console.error('Image gen failed:', e);
      setImageError(msg.id, String(e));
    } finally {
      generatingImages = new Set([...generatingImages].filter(id => id !== msg.id));
    }
  }

  async function regenerateText() {
    if (isLoading || isRegenerating) return;
    if (!messages.some(m => m.sender === 'user')) return;
    isRegenerating = true;

    try {
      const result: StoryTurnResult = await regenerateStory(currentChatId, storyId ?? undefined);

      const sdPrompt = result.scene
        ? [result.scene.location, result.scene.lighting, result.scene.mood]
            .filter(Boolean).join(', ')
        : undefined;

      const newAiMsg: ChatMessage = {
        id: Date.now(),
        text: '',
        sender: 'ai',
        data: {
          story: result.story_text,
          sd_prompt: sdPrompt,
        },
        sceneCharacterNames: result.characters.map(c => c.name),
        dbMessageId: result.assistant_message_id ?? undefined,
      };

      onregenerated?.(newAiMsg);
    } catch (e) {
      console.error('Regenerate failed:', e);
      onregenerated?.({ id: Date.now(), text: `Error regenerating: ${e}`, sender: 'ai' });
    } finally {
      isRegenerating = false;
    }
  }

  const lastUserInput = $derived(
    [...messages].reverse().find(m => m.sender === 'user')?.text ?? ''
  );

  async function regenerateWithInput(editedInput: string) {
    if (isLoading || isRegenerating) return;
    isRegenerating = true;
    try {
      const result: StoryTurnResult = await regenerateStoryWithInput(currentChatId, editedInput, storyId ?? undefined);
      const sdPrompt = result.scene
        ? [result.scene.location, result.scene.lighting, result.scene.mood]
            .filter(Boolean).join(', ')
        : undefined;
      const newAiMsg: ChatMessage = {
        id: Date.now(),
        text: '',
        sender: 'ai',
        data: { story: result.story_text, sd_prompt: sdPrompt },
        sceneCharacterNames: result.characters.map(c => c.name),
        dbMessageId: result.assistant_message_id ?? undefined,
      };
      onregenerated?.(newAiMsg);
    } catch (e) {
      console.error('Regenerate with input failed:', e);
      onregenerated?.({ id: Date.now(), text: `Error regenerating: ${e}`, sender: 'ai' });
    } finally {
      isRegenerating = false;
    }
  }

  const isbusy = $derived(isLoading || isRegenerating);
</script>

<div class="chat-main">
  <div class="chat-container">
    <div class="messages" id="message-box" bind:this={messagesContainer}>
      <div class="messages-inner">
        {#each messages as msg, i (msg.id)}
          <MessageBubble
            message={msg}
            isLast={i === messages.length - 1}
            isLoading={isbusy}
            isGeneratingImage={generatingImages.has(msg.id)}
            imageError={imageErrors.get(msg.id)}
            onGenerateImage={(pos, neg) => generateImage(msg, pos, neg)}
            onRegenerate={regenerateText}
            onRewriteWithInput={regenerateWithInput}
            lastUserInput={lastUserInput}
          />
        {/each}

        {#if isLoading}
          <div class="loading">
            <span class="spinner">↻</span> AI is writing...
          </div>
        {/if}
      </div>
    </div>

    <ChatInput disabled={isbusy} onsubmit={onsendmessage} />
  </div>
</div>

<style>
  .chat-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-chat);
    overflow: hidden;
  }

  .chat-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .messages-inner {
    max-width: 800px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 15px;
  }

  .loading {
    text-align: center;
    color: var(--text-muted);
    font-style: italic;
    font-size: 0.9em;
    margin-bottom: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .spinner {
    animation: spin 1s infinite linear;
    display: inline-block;
    font-weight: normal;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>
