<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  
  interface SdDetails {
    name: string;
    view: string;
    look: string;
    features?: string;
    action_context?: string;
    clothing?: string;
  }
  
  interface StoryResponse {
    story: string;
    sd_prompt?: string;
    sd_details?: SdDetails;
  }
  
  interface Phase1Response {
    text: string;
    type: 'phase1';
  }
  
  interface ChatMessage {
    id: number;
    text: string;
    sender: 'user' | 'ai';
    data?: StoryResponse;
  }

  interface ChatSummary {
    id: number;
    title: string;
  }
  
  let chatList: ChatSummary[] = [];
  let currentChatId: number = 1;
  let message = "";
  let messages: ChatMessage[] = []; 
  let isLoading = false;

  function scrollToBottom() {
    setTimeout(() => {
      const msgBox = document.getElementById('message-box');
      if (msgBox) msgBox.scrollTop = msgBox.scrollHeight;
    }, 0);
  }

  function mapMessagesToFrontend(backendMessages: any[]): ChatMessage[] {
    let idCounter = Date.now();
    return backendMessages.map((msg, index) => {
        let chatMsg: ChatMessage = {
            id: idCounter + index,
            text: msg.content,
            sender: msg.role === 'user' ? 'user' : 'ai',
            data: undefined
        };

        if (chatMsg.sender === 'ai') {
            try {
                if (msg.content.trim().startsWith('{')) {
                    const parsedJson = JSON.parse(msg.content);
                    
                    if (parsedJson.story_json || parsedJson.sd_json) {
                        
                        let storyText = parsedJson.story_json?.response || parsedJson.story_json || parsedJson.response || "Story loaded from history.";
                        if (typeof storyText !== 'string') {
                            storyText = "Story data found, but text extraction failed.";
                        }
                        
                        let sdDetails: SdDetails | undefined = undefined;
                        if (parsedJson.sd_json) {
                            sdDetails = parsedJson.sd_json;
                        }

                        chatMsg.data = {
                            story: storyText,
                            sd_prompt: sdDetails?.look,
                            sd_details: sdDetails,
                        };
                        
                        chatMsg.text = "Story generated!";
                    }
                }
            } catch (e) {
                console.warn("Failed to parse AI message as JSON from history, treating as plain text:", e);
                chatMsg.text = msg.content;
            }
        }
        
        if (chatMsg.sender === 'user') {
            chatMsg.text = msg.content;
        }

        return chatMsg;
    });
  }

  async function fetchChatList() {
    try {
      const list = await invoke('get_chat_list') as ChatSummary[];
      chatList = list;
      
      if (chatList.length > 0 && !chatList.some(c => c.id === currentChatId)) {
        currentChatId = chatList[chatList.length - 1].id;
      }
    } catch (e) {
      console.error("Failed to fetch chat list:", e);
    }
  }

  async function loadSelectedChat(id: number) {
    if (isLoading || id === currentChatId) return;

    isLoading = true;
    currentChatId = id;
    messages = []; 

    try {
      const msgs = await invoke('load_chat', { id }) as any[]; 
      messages = mapMessagesToFrontend(msgs);
      scrollToBottom();

    } catch (e) {
      console.error(`Failed to load chat ${id}:`, e);
      messages = [{ id: Date.now(), text: `Error loading chat: ${e}`, sender: 'ai' }];
    } finally {
      isLoading = false;
    }
  }

  async function startNewChat() {
    if(isLoading) return;
    try {
      const newId = await invoke('new_chat') as number;
      
      currentChatId = newId;
      messages = [];
      message = ""; 
      
      await fetchChatList(); 
      scrollToBottom();

    } catch (e) {
      console.error("Failed to create new chat:", e);
    }
  }

  async function clearChat() {
    if (isLoading) return;
    try {
        await invoke('clear_history');
        messages = [];
        await fetchChatList();
    } catch (e) {
        console.error("Failed to clear history:", e);
    }
  }

  fetchChatList();


  async function sendMessage() {
    if (!message.trim() || isLoading) return;
    
    const userMsgText = message;
    message = "";
    const isNewChat = messages.length === 0; 

    isLoading = true;

    messages = [...messages, { 
        id: Date.now(),
        text: userMsgText, 
        sender: 'user' 
    }];
    
    scrollToBottom();
    
    try {
      const response = await invoke('generate_story', { prompt: userMsgText });
      
      let newAiMsg: ChatMessage = {
          id: Date.now() + 1, 
          text: '',
          sender: 'ai',
          data: undefined
      };

      if (response && typeof response === 'object') {
        if ('type' in response && (response as any).type === 'phase1') {
          newAiMsg.text = (response as Phase1Response).text;
        } else if ('story' in response) {
          newAiMsg.text = "Story generated!";
          newAiMsg.data = response as StoryResponse;
        } else {
            newAiMsg.text = JSON.stringify(response, null, 2);
        }
      } else if (typeof response === 'string') {
          newAiMsg.text = response;
      }

      messages = [...messages, newAiMsg];

      if (isNewChat) {
          await fetchChatList();
      }

    } catch (error) {
        console.error('Error:', error);
        let errorMessage = error instanceof Error ? error.message : String(error);
        
        messages = [...messages, { 
            id: Date.now(),
            text: `Error: ${errorMessage}`, 
            sender: 'ai' 
        }];
    } finally {
      isLoading = false;
      scrollToBottom();
    }
  }
</script>

<main>
  <div class="app-layout"> 
    
    <div class="sidebar">
      <h2>History</h2>
      <button on:click={startNewChat} class="new-chat-btn" disabled={isLoading}>
        + New Chat
      </button>
      <div class="chat-list">
        {#each chatList as chat (chat.id)}
          <div 
            class="chat-item"
            class:active={chat.id === currentChatId}
            on:click={() => loadSelectedChat(chat.id)}
          >
            {chat.title}
          </div>
        {/each}
      </div>
    </div>
    
    <div class="chat-main">
      <div class="header">
          <h1>AI Story Writer</h1>
          <button on:click={clearChat} disabled={isLoading || messages.length === 0} class="clear-btn">
              Clear Current Chat
          </button>
      </div>
      
      <div class="chat-container">
          <div class="messages" id="message-box">
              {#each messages as msg (msg.id)}
                <div class="message-wrapper {msg.sender}">
                  <div><strong>{msg.sender.toUpperCase()}:</strong> {msg.text}</div> 
                  
                  {#if msg.data}
                    <div class="story-block">
                      {#if msg.data.story && msg.data.story.trim().length > 0 && msg.data.story !== "{}"}
                        <h4>📖 Story:</h4>
                        <p class="story-text">{msg.data.story}</p>
                      {:else if msg.data.story === "{}"}
                          <p style="color:red">Error: The AI returned empty data. Please try again.</p>
                      {/if}
                      
                      {#if msg.data.sd_prompt}
                        <h4>🎨 SD Prompt:</h4>
                        <div class="sd-prompt">
                          {msg.data.sd_prompt}
                        </div>
                      {/if}
                      
                      {#if msg.data.sd_details}
                        <details>
                          <summary>View SD Details</summary>
                          <div class="sd-details">
                            <p><strong>Character:</strong> {msg.data.sd_details.name}</p>
                            <p><strong>View:</strong> {msg.data.sd_details.view}</p>
                            <p><strong>Look:</strong> {msg.data.sd_details.look}</p>
                          </div>
                        </details>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}

              {#if isLoading}
                  <div class="loading-indicator">AI is writing...</div>
              {/if}
          </div>

          <div class="input-area">
              <input 
                  type="text" 
                  bind:value={message}
                  placeholder="Give me a story prompt..."
                  on:keydown={(e) => e.key === 'Enter' && sendMessage()}
                  disabled={isLoading}
              />
              <button on:click={sendMessage} disabled={isLoading} class="send-btn">
                  {isLoading ? 'Writing...' : 'Send'}
              </button>
          </div>
      </div>
    </div>
  </div>
</main>

<style>
  main {
      margin: 0;
      padding: 0;
      height: 100vh;
      display: flex;
      flex-direction: column;
      box-sizing: border-box;
      font-family: Arial, sans-serif;
  }

  .app-layout {
      flex: 1;
      display: flex;
      overflow: hidden; 
  }

  .sidebar {
      width: 250px;
      padding: 20px 10px;
      background: #f4f4f9;
      border-right: 1px solid #ddd;
      display: flex;
      flex-direction: column;
      overflow-y: auto;
      flex-shrink: 0;
  }

  .sidebar h2 {
      margin-top: 0;
      margin-bottom: 15px;
      font-size: 1.5em;
      border-bottom: 1px solid #e0e0e0;
      padding-bottom: 10px;
      text-align: center;
  }
  
  .new-chat-btn {
      width: 100%;
      background: #007bff;
      color: white;
      padding: 10px;
      margin-bottom: 20px;
      font-weight: bold;
      border: none;
      border-radius: 4px;
      cursor: pointer;
  }
  .new-chat-btn:hover:not(:disabled) { background: #0056b3; }
  
  .chat-list {
      flex: 1;
      overflow-y: auto;
  }
  
  .chat-item {
      padding: 10px;
      margin-bottom: 5px;
      cursor: pointer;
      border-radius: 4px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      background: #fff;
      border: 1px solid #eee;
      transition: background 0.1s;
      font-size: 0.9em;
  }
  
  .chat-item:hover { background: #e0e0e5; }
  
  .chat-item.active {
      background: #4a9eff;
      color: white;
      font-weight: bold;
  }
  .chat-item.active:hover { background: #4a9eff; }

  .chat-main {
      flex: 1;
      display: flex;
      flex-direction: column;
      padding: 20px;
  }

  .header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 20px;
  }

  .chat-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    border: 1px solid #eee; 
    border-radius: 8px;
    overflow: hidden;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    scroll-behavior: smooth;
  }
  
  .loading-indicator {
      text-align: center;
      padding: 10px;
      color: #666;
      font-style: italic;
  }

  .message-wrapper {
      max-width: 85%;
  }

  .message-wrapper.user {
      align-self: flex-end;
      text-align: right;
  }

  .message-wrapper.ai {
      align-self: flex-start;
  }

  .story-block {
    margin-top: 15px;  
    background: #f8f9fa;  
    padding: 20px;  
    border-radius: 8px;
    border-left: 4px solid #4a9eff;
    text-align: left;
  }

  .story-text {
      white-space: pre-wrap;
      line-height: 1.6;
  }

  .sd-prompt {
    background: #eee;  
    padding: 10px;  
    border-radius: 3px;  
    font-family: 'Courier New', monospace;
    font-size: 0.9em;
    overflow-x: auto;
  }

  .sd-details {
    margin-top: 10px;  
    font-size: 0.9em;
    padding-left: 15px;
  }

  .input-area {
    padding: 20px;
    background: #fff;
    border-top: 1px solid #eee;
    display: flex;
    gap: 10px;
    flex-shrink: 0;
  }

  input {
    flex: 1;
    padding: 12px;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 16px;
  }

  button {
    padding: 12px 24px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
    transition: background 0.2s;
  }

  .send-btn {
      background: #4a9eff;
      color: white;
      padding: 12px 24px;
  }
  .send-btn:hover:not(:disabled) { background: #357abd; }

  .clear-btn {
      background: #ff4a4a;
      color: white;
      padding: 8px 16px;
      font-size: 14px;
  }
  .clear-btn:hover:not(:disabled) { background: #d43535; }

  button:disabled, input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>