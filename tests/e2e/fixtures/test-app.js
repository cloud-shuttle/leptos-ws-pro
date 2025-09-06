// Test application for leptos_ws library
// This simulates how the library would be used in a real browser environment

class LeptosWSTestApp {
  constructor() {
    this.ws = null;
    this.isConnected = false;
    this.messagesSent = 0;
    this.messagesReceived = 0;
    this.reconnectCount = 0;
    this.connectionQuality = 100;

    this.initializeElements();
    this.setupEventListeners();
    this.updateUI();
  }

  initializeElements() {
    // Status and controls
    this.statusEl = document.getElementById("status");
    this.connectBtn = document.getElementById("connect-btn");
    this.disconnectBtn = document.getElementById("disconnect-btn");
    this.reconnectBtn = document.getElementById("reconnect-btn");
    this.serverUrlInput = document.getElementById("server-url");

    // Message controls
    this.messageInput = document.getElementById("message-input");
    this.sendBtn = document.getElementById("send-btn");
    this.sendHeartbeatBtn = document.getElementById("send-heartbeat-btn");

    // Metrics
    this.messagesSentEl = document.getElementById("messages-sent");
    this.messagesReceivedEl = document.getElementById("messages-received");
    this.connectionQualityEl = document.getElementById("connection-quality");
    this.reconnectCountEl = document.getElementById("reconnect-count");

    // Messages display
    this.messagesEl = document.getElementById("messages");
    this.clearMessagesBtn = document.getElementById("clear-messages-btn");

    // RPC controls
    this.rpcEchoBtn = document.getElementById("rpc-echo-btn");
    this.rpcBroadcastBtn = document.getElementById("rpc-broadcast-btn");
    this.rpcStatsBtn = document.getElementById("rpc-stats-btn");

    // Error display
    this.errorDisplay = document.getElementById("error-display");
  }

  setupEventListeners() {
    this.connectBtn.addEventListener("click", () => this.connect());
    this.disconnectBtn.addEventListener("click", () => this.disconnect());
    this.reconnectBtn.addEventListener("click", () => this.reconnect());
    this.sendBtn.addEventListener("click", () => this.sendMessage());
    this.sendHeartbeatBtn.addEventListener("click", () => this.sendHeartbeat());
    this.clearMessagesBtn.addEventListener("click", () => this.clearMessages());

    // RPC event listeners
    this.rpcEchoBtn.addEventListener("click", () => this.rpcEchoTest());
    this.rpcBroadcastBtn.addEventListener("click", () =>
      this.rpcBroadcastTest(),
    );
    this.rpcStatsBtn.addEventListener("click", () => this.rpcGetStats());

    // Enter key for message input
    this.messageInput.addEventListener("keypress", (e) => {
      if (e.key === "Enter") {
        this.sendMessage();
      }
    });
  }

  connect() {
    const url = this.serverUrlInput.value;
    if (!url) {
      this.showError("Please enter a server URL");
      return;
    }

    try {
      this.updateStatus("connecting", "Connecting...");
      this.ws = new WebSocket(url);

      this.ws.onopen = () => {
        this.isConnected = true;
        this.updateStatus("connected", "Connected");
        this.addMessage("system", "Connected to server");
        this.updateUI();
      };

      this.ws.onmessage = (event) => {
        this.messagesReceived++;
        this.addMessage("received", `Received: ${event.data}`);
        this.updateMetrics();

        // Try to parse as JSON for RPC responses
        try {
          const data = JSON.parse(event.data);
          this.handleRpcResponse(data);
        } catch (e) {
          // Not JSON, treat as regular message
        }
      };

      this.ws.onclose = (event) => {
        this.isConnected = false;
        this.updateStatus("disconnected", `Disconnected (${event.code})`);
        this.addMessage(
          "system",
          `Connection closed: ${event.code} - ${event.reason}`,
        );
        this.updateUI();
      };

      this.ws.onerror = (error) => {
        this.showError(`WebSocket error: ${error}`);
        this.addMessage("system", `Error: ${error}`);
      };
    } catch (error) {
      this.showError(`Failed to connect: ${error.message}`);
      this.updateStatus("disconnected", "Connection Failed");
    }
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.isConnected = false;
    this.updateStatus("disconnected", "Disconnected");
    this.updateUI();
  }

  reconnect() {
    this.reconnectCount++;
    this.disconnect();
    setTimeout(() => {
      this.connect();
    }, 1000);
    this.updateMetrics();
  }

  sendMessage() {
    const message = this.messageInput.value.trim();
    if (!message || !this.isConnected) return;

    try {
      this.ws.send(message);
      this.messagesSent++;
      this.addMessage("sent", `Sent: ${message}`);
      this.messageInput.value = "";
      this.updateMetrics();
    } catch (error) {
      this.showError(`Failed to send message: ${error.message}`);
    }
  }

  sendHeartbeat() {
    if (!this.isConnected) return;

    const heartbeat = JSON.stringify({
      type: "heartbeat",
      timestamp: Date.now(),
    });

    try {
      this.ws.send(heartbeat);
      this.messagesSent++;
      this.addMessage("sent", "Sent: Heartbeat");
      this.updateMetrics();
    } catch (error) {
      this.showError(`Failed to send heartbeat: ${error.message}`);
    }
  }

  rpcEchoTest() {
    if (!this.isConnected) return;

    const rpcRequest = JSON.stringify({
      id: `echo_${Date.now()}`,
      method: "echo",
      params: {
        message: "Hello from RPC test!",
        timestamp: Date.now(),
      },
    });

    try {
      this.ws.send(rpcRequest);
      this.messagesSent++;
      this.addMessage("sent", "Sent: RPC Echo Request");
      this.updateMetrics();
    } catch (error) {
      this.showError(`Failed to send RPC request: ${error.message}`);
    }
  }

  rpcBroadcastTest() {
    if (!this.isConnected) return;

    const rpcRequest = JSON.stringify({
      id: `broadcast_${Date.now()}`,
      method: "broadcast",
      params: {
        message: "Broadcast message from RPC test!",
        timestamp: Date.now(),
      },
    });

    try {
      this.ws.send(rpcRequest);
      this.messagesSent++;
      this.addMessage("sent", "Sent: RPC Broadcast Request");
      this.updateMetrics();
    } catch (error) {
      this.showError(`Failed to send RPC request: ${error.message}`);
    }
  }

  rpcGetStats() {
    if (!this.isConnected) return;

    const rpcRequest = JSON.stringify({
      id: `stats_${Date.now()}`,
      method: "get_stats",
      params: {},
    });

    try {
      this.ws.send(rpcRequest);
      this.messagesSent++;
      this.addMessage("sent", "Sent: RPC Get Stats Request");
      this.updateMetrics();
    } catch (error) {
      this.showError(`Failed to send RPC request: ${error.message}`);
    }
  }

  handleRpcResponse(data) {
    if (data.id && data.result) {
      this.addMessage(
        "received",
        `RPC Response (${data.id}): ${JSON.stringify(data.result)}`,
      );
    } else if (data.id && data.error) {
      this.addMessage(
        "received",
        `RPC Error (${data.id}): ${JSON.stringify(data.error)}`,
      );
    }
  }

  addMessage(type, content) {
    const messageEl = document.createElement("div");
    messageEl.className = `message ${type}`;
    messageEl.textContent = `[${new Date().toLocaleTimeString()}] ${content}`;
    this.messagesEl.appendChild(messageEl);
    this.messagesEl.scrollTop = this.messagesEl.scrollHeight;
  }

  clearMessages() {
    this.messagesEl.innerHTML = "";
  }

  updateStatus(type, text) {
    this.statusEl.className = `status ${type}`;
    this.statusEl.textContent = text;
  }

  updateUI() {
    const connected = this.isConnected;

    this.connectBtn.disabled = connected;
    this.disconnectBtn.disabled = !connected;
    this.reconnectBtn.disabled = !connected;
    this.messageInput.disabled = !connected;
    this.sendBtn.disabled = !connected;
    this.sendHeartbeatBtn.disabled = !connected;
    this.rpcEchoBtn.disabled = !connected;
    this.rpcBroadcastBtn.disabled = !connected;
    this.rpcStatsBtn.disabled = !connected;
  }

  updateMetrics() {
    this.messagesSentEl.textContent = this.messagesSent;
    this.messagesReceivedEl.textContent = this.messagesReceived;
    this.connectionQualityEl.textContent = `${this.connectionQuality}%`;
    this.reconnectCountEl.textContent = this.reconnectCount;
  }

  showError(message) {
    this.errorDisplay.textContent = message;
    this.errorDisplay.style.display = "block";
    setTimeout(() => {
      this.errorDisplay.style.display = "none";
    }, 5000);
  }

  // Simulate connection quality changes
  simulateConnectionQualityChange() {
    if (this.isConnected) {
      // Simulate quality degradation over time
      this.connectionQuality = Math.max(
        10,
        this.connectionQuality - Math.random() * 5,
      );
      this.updateMetrics();

      // Trigger reconnect if quality is too low
      if (this.connectionQuality < 20) {
        this.addMessage(
          "system",
          "Connection quality low, triggering reconnect...",
        );
        this.reconnect();
      }
    }
  }
}

// Initialize the app when the page loads
document.addEventListener("DOMContentLoaded", () => {
  window.testApp = new LeptosWSTestApp();

  // Simulate connection quality changes every 10 seconds
  setInterval(() => {
    window.testApp.simulateConnectionQualityChange();
  }, 10000);
});

// Export for testing
window.LeptosWSTestApp = LeptosWSTestApp;
