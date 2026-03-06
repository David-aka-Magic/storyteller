<!-- src/components/layout/MainView.svelte — Chat view or welcome screen -->
<script lang="ts">
  import ChatArea from '../chat/ChatArea.svelte';
  import type { ChatMessage } from '$lib/types';

  let {
    messages = [],
    isLoading = false,
    currentChatId = 0,
    storyId = null,
    activeCharacterId = null,
    onsendmessage,
    onimagegenerated,
    onregenerated,
  }: {
    messages?: ChatMessage[];
    isLoading?: boolean;
    currentChatId?: number;
    storyId?: number | null;
    activeCharacterId?: string | null;
    onsendmessage?: (text: string) => void;
    onimagegenerated?: (msgId: number, src: string, filePath: string) => void;
    onregenerated?: (newMsg: ChatMessage) => void;
  } = $props();
</script>

{#if currentChatId}
  <ChatArea
    {messages}
    {isLoading}
    {currentChatId}
    {storyId}
    {activeCharacterId}
    {onsendmessage}
    {onimagegenerated}
    {onregenerated}
  />
{:else}
  <div class="welcome">
    <div class="welcome-content">
      <h2>AI Story Writer</h2>
      <p>Start a new chat or select one from the sidebar to begin your story.</p>
    </div>
  </div>
{/if}

<style>
  .welcome {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-chat, var(--bg-primary));
  }

  .welcome-content {
    text-align: center;
    color: var(--text-muted);
  }

  .welcome-content h2 {
    font-size: 1.8em;
    margin-bottom: 10px;
    color: var(--text-secondary);
  }

  .welcome-content p {
    font-size: 1em;
  }
</style>
