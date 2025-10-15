class ChatApp {
  constructor() {
    this.ws = null;
    this.username = "";
    this.isConnected = false;
    this.isConnecting = false;

    this.elements = {
      messages: document.getElementById("messages"),
      messageInput: document.getElementById("messageInput"),
      usernameInput: document.getElementById("usernameInput"),
      sendButton: document.getElementById("sendButton"),
      status: document.getElementById("status"),
      userCount: document.getElementById("userCount"),
    };

    this.init();
  }

  init() {
    // Load username from localStorage
    const savedUsername = localStorage.getItem("chatUsername");
    if (savedUsername) {
      this.elements.usernameInput.value = savedUsername;
    }

    // Event listeners
    this.elements.usernameInput.addEventListener("keyup", (e) => {
      if (e.key === "Enter" && e.target.value.trim() && !this.username) {
        e.preventDefault(); // Prevent form submission
        this.setUsername(e.target.value.trim());
      }
    });

    this.elements.messageInput.addEventListener("keyup", (e) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault(); // Prevent form submission
        this.sendMessage();
      }
    });

    // Prevent default form submission on button click
    this.elements.sendButton.addEventListener("click", (e) => {
      e.preventDefault(); // Prevent form submission
      this.sendMessage();
    });

    // Auto-connect if username exists
    if (savedUsername) {
      this.setUsername(savedUsername);
    }
  }

  setUsername(username) {
    if (this.username || this.isConnecting) return;

    this.username = username;
    localStorage.setItem("chatUsername", username);
    this.elements.usernameInput.disabled = true;
    this.connect();
  }

  connect() {
    if (
      this.isConnecting ||
      (this.ws && this.ws.readyState === WebSocket.OPEN)
    ) {
      return;
    }

    this.isConnecting = true;
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}`;

    console.log("Connecting to:", wsUrl);
    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = () => {
      console.log("✅ Connected to WebSocket");
      this.isConnected = true;
      this.isConnecting = false;
      this.updateStatus(true);
      this.elements.messageInput.disabled = false;
      this.elements.sendButton.disabled = false;
      this.elements.messageInput.focus();
    };

    this.ws.onmessage = (event) => {
      console.log("📨 Received:", event.data);
      try {
        const data = JSON.parse(event.data);
        this.handleMessage(data);
      } catch (e) {
        console.error("Failed to parse message:", e, event.data);
      }
    };

    this.ws.onerror = (error) => {
      console.error("❌ WebSocket error:", error);
      this.isConnecting = false;
    };

    this.ws.onclose = (event) => {
      console.log("🔌 Disconnected from WebSocket", event.code, event.reason);
      this.isConnected = false;
      this.isConnecting = false;
      this.updateStatus(false);
      this.elements.messageInput.disabled = true;
      this.elements.sendButton.disabled = true;

      // Reconnect after 3 seconds
      setTimeout(() => {
        if (!this.isConnected && this.username) {
          console.log("🔄 Attempting to reconnect...");
          this.connect();
        }
      }, 3000);
    };
  }

  handleMessage(data) {
    console.log("Handling message:", data);

    if (data.type === "stats") {
      this.updateUserCount(data.count);
      return;
    }

    const messageDiv = document.createElement("div");

    if (data.msg_type === "user") {
      const isOwnMessage = data.username === this.username;
      messageDiv.className = `message ${isOwnMessage ? "user" : "other"}`;

      messageDiv.innerHTML = `
                <div class="message-header">${this.escapeHtml(data.username)}</div>
                <div class="message-text">${this.escapeHtml(data.message)}</div>
                <div class="message-time">${this.formatTime(data.timestamp)}</div>
            `;
    } else {
      // System, join, leave messages
      messageDiv.className = `message ${data.msg_type || "system"}`;
      messageDiv.innerHTML = `
                <div class="message-text">${this.escapeHtml(data.message)}</div>
            `;
    }

    this.elements.messages.appendChild(messageDiv);
    this.scrollToBottom();
  }

  sendMessage() {
    const message = this.elements.messageInput.value.trim();

    if (!message || !this.isConnected) {
      console.warn("Cannot send: not connected or empty message");
      return;
    }

    const chatMessage = {
      username: this.username,
      message: message,
      timestamp: Math.floor(Date.now() / 1000),
      msg_type: "user",
    };

    console.log("📤 Sending:", chatMessage);
    this.ws.send(JSON.stringify(chatMessage));
    this.elements.messageInput.value = "";
    this.elements.messageInput.focus();
  }

  updateStatus(connected) {
    this.elements.status.textContent = connected ? "Connected" : "Disconnected";
    this.elements.status.className = `status ${connected ? "connected" : "disconnected"}`;
  }

  updateUserCount(count) {
    this.elements.userCount.textContent = `${count} user${count !== 1 ? "s" : ""}`;
  }

  scrollToBottom() {
    this.elements.messages.scrollTop = this.elements.messages.scrollHeight;
  }

  formatTime(timestamp) {
    const date = new Date(timestamp * 1000);
    return date.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }
}

// Initialize app when DOM is ready
document.addEventListener("DOMContentLoaded", () => {
  new ChatApp();
});
