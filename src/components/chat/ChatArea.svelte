<!-- src/components/chat/ChatArea.svelte — Messages list + input -->
<script lang="ts">
  import ChatInput from './ChatInput.svelte';
  import MessageBubble from './MessageBubble.svelte';
  import { regenerateStory, generateSceneImageForTurn } from '$lib/api/text-gen';
  import { readFileBase64 } from '$lib/api/image-gen';
  import type { ChatMessage, StoryResponse, Phase1Response } from '$lib/types';

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
    onimagegenerated?: (msgId: number, src: string) => void;
    onregenerated?: (newMsg: ChatMessage) => void;
  } = $props();

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

  async function generateImage(msg: ChatMessage) {
    const prompt = msg.data?.sd_prompt || msg.data?.story;
    if (!prompt) {
      setImageError(msg.id, 'No text available to generate an image.');
      return;
    }

    setImageError(msg.id, null);
    generatingImages = new Set([...generatingImages, msg.id]);
    try {
      const path = await generateSceneImageForTurn(prompt, storyId ?? undefined);
      const base64 = await readFileBase64(path);
      onimagegenerated?.(msg.id, `data:image/png;base64,${base64}`);
    } catch (e) {
      console.error('Image gen failed:', e);
      setImageError(msg.id, String(e));
    } finally {
      generatingImages = new Set([...generatingImages].filter(id => id !== msg.id));
    }
  }

  async function regenerateText() {
    if (isLoading || isRegenerating) return;
    isRegenerating = true;

    try {
      const response = await regenerateStory(currentChatId);

      let newAiMsg: ChatMessage = { id: Date.now(), text: '', sender: 'ai', data: undefined };

      if (response && typeof response === 'object') {
        if ('type' in response && (response as any).type === 'phase1') {
          newAiMsg.text = (response as Phase1Response).text;
        } else if ('story' in response) {
          newAiMsg.text = '';
          newAiMsg.data = response as StoryResponse;
        }
      } else if (typeof response === 'string') {
        newAiMsg.text = response;
      }

      onregenerated?.(newAiMsg);
    } catch (e) {
      console.error('Regenerate failed:', e);
      onregenerated?.({ id: Date.now(), text: `Error regenerating: ${e}`, sender: 'ai' });
    } finally {
      isRegenerating = false;
    }
  }

  const isbusy = $derived(isLoading || isRegenerating);
</script>

<div class="chat-main">
  <div class="chat-container">
    <div class="messages" id="message-box">
      {#each messages as msg, i (msg.id)}
        <MessageBubble
          message={msg}
          isLast={i === messages.length - 1}
          isLoading={isbusy}
          isGeneratingImage={generatingImages.has(msg.id)}
          imageError={imageErrors.get(msg.id)}
          onGenerateImage={() => generateImage(msg)}
          onRegenerate={regenerateText}
        />
      {/each}

      {#if isLoading}
        <div class="loading">
          <span class="spinner">↻</span> AI is writing...
        </div>
      {/if}
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
