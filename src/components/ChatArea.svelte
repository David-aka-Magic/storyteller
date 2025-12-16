<script lang="ts">
  import { createEventDispatcher, afterUpdate } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { ChatMessage, StoryResponse, Phase1Response } from '../lib/types';

  export let messages: ChatMessage[] = [];
  export let isLoading: boolean = false;
  export let currentChatId: number; // NEW PROP

  let generatingImages = new Set<number>();
  let messageInput = "";
  const dispatch = createEventDispatcher();

  function sendMessage() {
    if (!messageInput.trim() || isLoading) return;
    dispatch('sendMessage', messageInput);
    messageInput = "";
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault(); 
      sendMessage();
    }
  }

  async function regenerateText() {
     if (isLoading) return;
     isLoading = true;
     if (messages.length > 0 && messages[messages.length - 1].sender === 'ai') {
        messages = messages.slice(0, -1);
     }
     try {
        const response = await invoke('regenerate_story');
        let newAiMsg: ChatMessage = { id: Date.now(), text: '', sender: 'ai' };
        
        if (response && typeof response === 'object') {
             if ('type' in response && (response as any).type === 'phase1') {
                newAiMsg.text = (response as Phase1Response).text;
            } else if ('story' in response) {
                newAiMsg.text = ""; 
                newAiMsg.data = response as StoryResponse;
            }
        } else if (typeof response === 'string') {
            newAiMsg.text = response;
        }
        messages = [...messages, newAiMsg];
     } catch (e) {
        console.error(e);
     } finally { isLoading = false; }
  }

  async function generateImage(msg: ChatMessage, index: number) {
    const promptToUse = msg.data?.sd_prompt || msg.data?.story;
    if (!promptToUse) {
        alert("No text available to generate an image.");
        return;
    }

    generatingImages.add(msg.id);
    generatingImages = generatingImages; 

    try {
        const base64 = await invoke('generate_image', { 
            prompt: promptToUse,
            chatId: currentChatId,
            msgIndex: index
        });
        
        const msgIdx = messages.findIndex(m => m.id === msg.id);
        if (msgIdx !== -1) {
            messages[msgIdx].image = base64 as string;
            messages = messages; 
        }
    } catch (e) {
        console.error("Image gen failed:", e);
        alert("Failed to generate image.");
    } finally {
        generatingImages.delete(msg.id);
        generatingImages = generatingImages; 
    }
  }

  afterUpdate(() => {
    if (isLoading) {
        const msgBox = document.getElementById('message-box');
        if (msgBox) msgBox.scrollTop = msgBox.scrollHeight;
    }
  });
</script>

<div class="chat-main">
  <div class="header">
      <h1>AI Story Writer</h1>
      <button 
        type="button"
        on:click={() => dispatch('clearChat')} 
        disabled={isLoading || messages.length === 0} 
        class="clear-btn"
      >
          Clear Current Chat
      </button>
  </div>
  
  <div class="chat-container">
      <div class="messages" id="message-box">
          {#each messages as msg, i (msg.id)}
            <div class="message-wrapper {msg.sender}">
              
              {#if msg.text}
                <div><strong>{msg.sender.toUpperCase()}:</strong> {msg.text}</div> 
              {/if}
              
              {#if msg.data}
                <div class="story-block">
                    <div class="story-text">{msg.data.story}</div>
                    
                    {#if msg.data.sd_prompt}
                        <div class="sd-details">
                            <div class="sd-prompt"><strong>Visual Prompt:</strong> {msg.data.sd_prompt}</div>
                        </div>
                    {/if}

                    <div class="actions-row">
                        <button 
                            class="action-btn img-btn" 
                            on:click={() => generateImage(msg, i)} 
                            disabled={isLoading || generatingImages.has(msg.id)}
                        >
                            {#if generatingImages.has(msg.id)}
                                <span class="spinner">↻</span> Generating...
                            {:else}
                                {msg.image ? '↻ Redraw Image' : '🎨 Illustrate Scene'}
                            {/if}
                        </button>

                        {#if i === messages.length - 1 && msg.sender === 'ai'}
                             <button class="action-btn regen-btn" on:click={regenerateText} disabled={isLoading}>
                                ↻ Rewrite Story
                             </button>
                        {/if}
                    </div>
                </div>
              {/if}

              {#if msg.image}
                <div class="img-container">
                    <img 
                        src="data:image/png;base64,{msg.image}" 
                        alt="Generated Scene" 
                        class:dimmed={generatingImages.has(msg.id)}
                    />
                </div>
              {/if}
            </div>
          {/each}
          
          {#if isLoading}
            <div class="loading">
                <span class="spinner">↻</span> AI is writing...
            </div>
          {/if}
      </div>

      <div class="input-area">
          <textarea 
            bind:value={messageInput} 
            on:keydown={handleKeydown}
            placeholder="Type your action..."
            rows="1"
            disabled={isLoading}
          ></textarea>
          <button type="button" on:click={sendMessage} disabled={isLoading || !messageInput.trim()}>
            Send
          </button>
      </div>
  </div>
</div>

<style>
  .chat-main { flex: 1; display: flex; flex-direction: column; height: 100%; background: #fff; }
  .header { padding: 15px 20px; border-bottom: 1px solid #eee; display: flex; justify-content: space-between; align-items: center; background: #fcfcfc; }
  h1 { margin: 0; font-size: 1.2em; color: #333; }

  .clear-btn { background: #ff4d4d; color: white; border: none; padding: 5px 10px; border-radius: 4px; cursor: pointer; font-size: 0.8em; }
  .clear-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .chat-container { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
  .messages { flex: 1; overflow-y: auto; padding: 20px; display: flex; flex-direction: column; gap: 15px; }

  .message-wrapper { max-width: 80%; padding: 10px 15px; border-radius: 8px; font-size: 0.95em; }
  .message-wrapper.user { align-self: flex-end; text-align: right; background: #e3f2fd; border-bottom-right-radius: 0; }
  .message-wrapper.ai { align-self: flex-start; background: #fff; border: 1px solid #eee; border-bottom-left-radius: 0; }

  .story-block { margin-top: 15px; background: #f8f9fa; padding: 15px; border-radius: 8px; border-left: 4px solid #4a9eff; text-align: left; }
  .story-text { white-space: pre-wrap; line-height: 1.6; }

  .sd-details { margin-top: 15px; border-top: 1px solid #eee; padding-top: 10px; }
  .sd-prompt { background: #eee; padding: 8px; border-radius: 4px; font-family: monospace; font-size: 0.85em; color: #555; margin-bottom: 8px; }

  .actions-row { margin-top: 15px; display: flex; gap: 10px; justify-content: flex-end; }
  
  .action-btn { border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer; font-size: 0.85em; display: inline-flex; align-items: center; gap: 5px; font-weight: bold; transition: background 0.2s;}
  
  .img-btn { background: #e3f2fd; color: #1976d2; border: 1px solid #bbdefb; }
  .img-btn:hover { background: #bbdefb; }
  .img-btn:disabled { opacity: 0.7; cursor: wait; }
  
  .regen-btn { background: #fff3e0; color: #e65100; border: 1px solid #ffe0b2; }
  .regen-btn:hover { background: #ffe0b2; }

  .img-container { margin-top: 15px; text-align: center; }
  .img-container img { max-width: 100%; border-radius: 8px; box-shadow: 0 4px 8px rgba(0,0,0,0.1); transition: opacity 0.3s; }
  .img-container img.dimmed { opacity: 0.5; }

  .input-area { padding: 20px; background: #fff; border-top: 1px solid #eee; display: flex; gap: 10px; flex-shrink: 0; }
  textarea { flex: 1; padding: 10px; border: 1px solid #ccc; border-radius: 4px; font-family: inherit; resize: none; }
  button { padding: 0 20px; background: #4a9eff; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: bold; }
  button:disabled { background: #ccc; cursor: not-allowed; }
  
  .loading { text-align: center; color: #888; font-style: italic; font-size: 0.9em; margin-bottom: 10px; display: flex; align-items: center; justify-content: center; gap: 8px;}
  
  .spinner { animation: spin 1s infinite linear; display: inline-block; font-weight: normal;}
  @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
</style>