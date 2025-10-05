// ProcessDistro Dashboard JavaScript

class ProcessDistroClient {
  constructor() {
    this.deviceId = this.generateDeviceId();
    this.capabilities = null;
    this.isRegistered = false;
    this.heartbeatInterval = null;
    this.nodes = [];
    this.websocket = null;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;

    this.init();
  }

  async init() {
    await this.detectDeviceCapabilities();
    this.setupEventListeners();
    this.connectWebSocket();
    this.updateDeviceDisplay();
    this.updateConnectionInfo();

    document.getElementById("device-id").textContent = this.deviceId;
  }

  generateDeviceId() {
    return "web-" + Math.random().toString(36).substr(2, 9) + "-" + Date.now();
  }

  async detectDeviceCapabilities() {
    const capabilities = {
      device_id: this.deviceId,
      device_name: this.getDeviceName(),
      device_type: this.getDeviceType(),
      cpu_cores: navigator.hardwareConcurrency || 4,
      cpu_threads: navigator.hardwareConcurrency || 4,
      cpu_frequency: 0.0, // Cannot be detected in browser
      total_memory: this.estimateMemory(),
      available_memory: this.estimateAvailableMemory(),
      gpu_info: this.getGPUInfo(),
      browser_info: this.getBrowserInfo(),
      platform: navigator.platform,
      user_agent: navigator.userAgent,
      benchmark_score: null,
      timestamp: new Date().toISOString(),
    };

    this.capabilities = capabilities;

    // Enable buttons once capabilities are detected
    document.getElementById("register-btn").disabled = false;
    document.getElementById("benchmark-btn").disabled = false;
  }

  getDeviceName() {
    const platform = navigator.platform.toLowerCase();
    const userAgent = navigator.userAgent.toLowerCase();

    if (userAgent.includes("mobile") || userAgent.includes("android")) {
      return "Mobile Device";
    } else if (userAgent.includes("tablet") || userAgent.includes("ipad")) {
      return "Tablet Device";
    } else if (platform.includes("mac")) {
      return "Mac Computer";
    } else if (platform.includes("win")) {
      return "Windows Computer";
    } else if (platform.includes("linux")) {
      return "Linux Computer";
    } else {
      return "Unknown Device";
    }
  }

  getDeviceType() {
    const userAgent = navigator.userAgent.toLowerCase();

    if (userAgent.includes("mobile") || userAgent.includes("android")) {
      return "mobile";
    } else if (userAgent.includes("tablet") || userAgent.includes("ipad")) {
      return "tablet";
    } else {
      return "desktop";
    }
  }

  estimateMemory() {
    // Try to get actual memory info if available
    if (navigator.deviceMemory) {
      return navigator.deviceMemory * 1024; // Convert GB to MB
    }

    // Fallback estimation based on device type
    const cores = navigator.hardwareConcurrency || 4;
    if (cores >= 8) return 16384; // 16GB
    if (cores >= 4) return 8192; // 8GB
    return 4096; // 4GB
  }

  estimateAvailableMemory() {
    return Math.floor(this.estimateMemory() * 0.6); // Assume 60% available
  }

  getGPUInfo() {
    try {
      const canvas = document.createElement("canvas");
      const gl =
        canvas.getContext("webgl") || canvas.getContext("experimental-webgl");

      if (gl) {
        const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
        if (debugInfo) {
          return gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);
        }
        return "WebGL Supported";
      }
    } catch (e) {
      // Ignore errors
    }
    return null;
  }

  getBrowserInfo() {
    const userAgent = navigator.userAgent;

    if (userAgent.includes("Chrome")) return "Google Chrome";
    if (userAgent.includes("Firefox")) return "Mozilla Firefox";
    if (userAgent.includes("Safari") && !userAgent.includes("Chrome"))
      return "Safari";
    if (userAgent.includes("Edge")) return "Microsoft Edge";
    if (userAgent.includes("Opera")) return "Opera";

    return "Unknown Browser";
  }

  updateDeviceDisplay() {
    const deviceInfo = document.getElementById("device-info");
    deviceInfo.innerHTML = `
            <h3>Device Detected: ${this.capabilities.device_name}</h3>
            <p><strong>Type:</strong> ${this.capabilities.device_type}</p>
            <p><strong>Browser:</strong> ${this.capabilities.browser_info}</p>
            <p><strong>Platform:</strong> ${this.capabilities.platform}</p>
        `;

    // Show capabilities
    const capabilitiesDiv = document.getElementById("device-capabilities");
    const capabilityGrid = document.getElementById("capability-grid");

    capabilityGrid.innerHTML = `
            <div class="capability-item">
                <span class="capability-label">CPU Cores</span>
                <span class="capability-value">${
                  this.capabilities.cpu_cores
                }</span>
            </div>
            <div class="capability-item">
                <span class="capability-label">Total Memory</span>
                <span class="capability-value">${Math.floor(
                  this.capabilities.total_memory / 1024
                )} GB</span>
            </div>
            <div class="capability-item">
                <span class="capability-label">Available Memory</span>
                <span class="capability-value">${Math.floor(
                  this.capabilities.available_memory / 1024
                )} GB</span>
            </div>
            <div class="capability-item">
                <span class="capability-label">GPU</span>
                <span class="capability-value">${
                  this.capabilities.gpu_info || "Not detected"
                }</span>
            </div>
        `;

    capabilitiesDiv.style.display = "block";
  }

  setupEventListeners() {
    // Register device button
    document.getElementById("register-btn").addEventListener("click", () => {
      this.registerDevice();
    });

    // Unregister device button
    document.getElementById("unregister-btn").addEventListener("click", () => {
      this.unregisterDevice();
    });

    // Benchmark button
    document.getElementById("benchmark-btn").addEventListener("click", () => {
      this.runBenchmark();
    });

    // Refresh nodes button - with WebSocket, this will re-render current data
    document.getElementById("refresh-nodes").addEventListener("click", () => {
      this.displayNodes(this.nodes);
      this.updateAllMetrics(this.nodes);
    });

    // Filter nodes
    document.getElementById("filter-nodes").addEventListener("change", (e) => {
      this.filterNodes(e.target.value);
    });
  }

  async registerDevice() {
    try {
      const response = await fetch("/api/register", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(this.capabilities),
      });

      const result = await response.json();

      if (result.status === "success") {
        this.isRegistered = true;
        this.startHeartbeat();

        // Update UI
        const registerBtn = document.getElementById("register-btn");
        const unregisterBtn = document.getElementById("unregister-btn");

        registerBtn.textContent = "Registered ✓";
        registerBtn.disabled = true;
        registerBtn.className = "btn btn-secondary";
        registerBtn.style.display = "none";

        unregisterBtn.disabled = false;
        unregisterBtn.style.display = "inline-block";

        // Update network status
        document.getElementById("network-status").textContent = "Registered";

        console.log("Device registered successfully");
        // Node updates will come via WebSocket
      } else {
        console.error("Registration failed:", result.message);
      }
    } catch (error) {
      console.error("Registration error:", error);
    }
  }

  async unregisterDevice() {
    try {
      const response = await fetch("/api/unregister", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          node_id: this.deviceId,
          timestamp: new Date().toISOString(),
        }),
      });

      const result = await response.json();

      if (result.status === "success") {
        this.isRegistered = false;
        this.stopHeartbeat();

        // Update UI
        const registerBtn = document.getElementById("register-btn");
        const unregisterBtn = document.getElementById("unregister-btn");

        registerBtn.textContent = "Register This Device";
        registerBtn.disabled = false;
        registerBtn.className = "btn btn-primary";
        registerBtn.style.display = "inline-block";

        unregisterBtn.style.display = "none";

        // Update network status
        document.getElementById("network-status").textContent = "Connected";

        console.log("Device unregistered successfully");
        // Node updates will come via WebSocket
      } else {
        console.error("Unregistration failed:", result.message);
      }
    } catch (error) {
      console.error("Unregistration error:", error);
    }
  }

  stopHeartbeat() {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  async cleanup() {
    if (this.isRegistered) {
      await this.unregisterDevice();
    }
  }

  connectWebSocket() {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    console.log("Connecting to WebSocket:", wsUrl);

    this.websocket = new WebSocket(wsUrl);

    this.websocket.onopen = () => {
      console.log("WebSocket connected");
      this.reconnectAttempts = 0;
      document.getElementById("network-status").textContent = "Connected";
      document.getElementById("network-status").className = "status-active";
    };

    this.websocket.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        this.handleWebSocketMessage(message);
      } catch (error) {
        console.error("Failed to parse WebSocket message:", error);
      }
    };

    this.websocket.onclose = () => {
      console.log("WebSocket disconnected");
      document.getElementById("network-status").textContent = "Disconnected";
      document.getElementById("network-status").className = "";
      this.attemptReconnect();
    };

    this.websocket.onerror = (error) => {
      console.error("WebSocket error:", error);
      document.getElementById("network-status").textContent = "Error";
      document.getElementById("network-status").className = "";
    };
  }

  handleWebSocketMessage(message) {
    switch (message.type) {
      case "node_update":
        this.updateNodesList(message.nodes);
        break;
      case "node_added":
        this.addNodeToList(message.node);
        break;
      case "node_removed":
        this.removeNodeFromList(message.node_id);
        break;
      case "metrics_update":
        this.updateMetrics(message);
        break;
      case "heartbeat":
        // Handle heartbeat if needed
        break;
      default:
        console.log("Unknown WebSocket message type:", message.type);
    }
  }

  updateMetrics(metrics) {
    document.getElementById("total-nodes").textContent = metrics.total_nodes;
    document.getElementById("active-tasks").textContent = metrics.active_tasks;
    document.getElementById("total-cores").textContent = metrics.total_cores;
    document.getElementById("total-memory").textContent = metrics.total_memory;
    document.getElementById("avg-benchmark").textContent =
      metrics.avg_benchmark > 0 ? Math.round(metrics.avg_benchmark) : "0";
  }

  updateNodesList(nodes) {
    this.nodes = nodes;
    this.displayNodes(nodes);
    this.updateAllMetrics(nodes);
  }

  addNodeToList(node) {
    const existingIndex = this.nodes.findIndex(
      (n) => n.node_id === node.node_id
    );
    if (existingIndex >= 0) {
      this.nodes[existingIndex] = node;
    } else {
      this.nodes.push(node);
    }
    this.displayNodes(this.nodes);
    this.updateAllMetrics(this.nodes);
  }

  removeNodeFromList(nodeId) {
    this.nodes = this.nodes.filter((n) => n.node_id !== nodeId);
    this.displayNodes(this.nodes);
    this.updateAllMetrics();
  }

  attemptReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.log("Max reconnection attempts reached");
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);

    console.log(
      `Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts})`
    );

    setTimeout(() => {
      this.connectWebSocket();
    }, delay);
  }

  startHeartbeat() {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
    }

    this.heartbeatInterval = setInterval(async () => {
      try {
        await fetch("/api/heartbeat", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            node_id: this.deviceId,
            timestamp: new Date().toISOString(),
          }),
        });
      } catch (error) {
        console.error("Heartbeat failed:", error);
      }
    }, 30000); // Every 30 seconds
  }

  async runBenchmark() {
    const modal = document.getElementById("benchmark-progress");
    const progressFill = document.getElementById("progress-fill");
    const statusText = document.getElementById("benchmark-status");

    modal.style.display = "flex";

    try {
      // Simulate WASM-like computation benchmark
      statusText.textContent = "Initializing benchmark...";
      progressFill.style.width = "10%";

      await this.sleep(500);

      statusText.textContent = "Running CPU intensive tasks...";
      progressFill.style.width = "30%";

      const startTime = performance.now();

      // CPU benchmark - calculate prime numbers
      const primeCount = this.calculatePrimes(100000);

      progressFill.style.width = "60%";
      statusText.textContent = "Running memory tests...";

      // Memory benchmark - array operations
      const memoryScore = this.memoryBenchmark();

      progressFill.style.width = "90%";
      statusText.textContent = "Finalizing results...";

      const endTime = performance.now();
      const duration = endTime - startTime;

      // Calculate score (operations per second)
      const score = Math.floor((primeCount + memoryScore) / (duration / 1000));

      progressFill.style.width = "100%";
      statusText.textContent = `Benchmark complete! Score: ${score} ops/sec`;

      // Update capabilities with benchmark score
      this.capabilities.benchmark_score = score;

      // Submit benchmark result
      await this.submitBenchmark(score);

      await this.sleep(2000);
      modal.style.display = "none";

      // Update display
      const capabilityGrid = document.getElementById("capability-grid");
      capabilityGrid.innerHTML += `
                <div class="capability-item">
                    <span class="capability-label">Benchmark Score</span>
                    <span class="capability-value">${score} ops/sec</span>
                </div>
            `;
    } catch (error) {
      console.error("Benchmark failed:", error);
      statusText.textContent = "Benchmark failed!";
      await this.sleep(2000);
      modal.style.display = "none";
    }
  }

  calculatePrimes(max) {
    let count = 0;
    for (let i = 2; i <= max; i++) {
      let isPrime = true;
      for (let j = 2; j <= Math.sqrt(i); j++) {
        if (i % j === 0) {
          isPrime = false;
          break;
        }
      }
      if (isPrime) count++;
    }
    return count;
  }

  memoryBenchmark() {
    const arraySize = 1000000;
    const array = new Array(arraySize);

    // Fill array
    for (let i = 0; i < arraySize; i++) {
      array[i] = Math.random();
    }

    // Sort array
    array.sort((a, b) => a - b);

    // Sum array
    let sum = 0;
    for (let i = 0; i < arraySize; i++) {
      sum += array[i];
    }

    return Math.floor(sum);
  }

  async submitBenchmark(score) {
    try {
      await fetch("/api/benchmark", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          node_id: this.deviceId,
          score: score,
          timestamp: new Date().toISOString(),
        }),
      });
    } catch (error) {
      console.error("Failed to submit benchmark:", error);
    }
  }

  async loadNodes() {
    try {
      const response = await fetch("/api/nodes");
      const nodes = await response.json();

      // Check if nodes have changed
      const hasChanged = this.hasNodesChanged(nodes);

      this.nodes = nodes;
      this.displayNodes(nodes);
      this.updateAllMetrics(nodes);

      // If nodes changed, trigger additional updates
      if (hasChanged) {
        this.onNodesChanged(nodes);
      }
    } catch (error) {
      console.error("Failed to load nodes:", error);
    }
  }

  hasNodesChanged(newNodes) {
    if (!this.nodes || this.nodes.length !== newNodes.length) {
      return true;
    }

    // Check if any node details changed
    for (let i = 0; i < newNodes.length; i++) {
      const oldNode = this.nodes.find((n) => n.node_id === newNodes[i].node_id);
      if (
        !oldNode ||
        oldNode.status !== newNodes[i].status ||
        oldNode.capabilities.benchmark_score !==
          newNodes[i].capabilities.benchmark_score ||
        oldNode.tasks_completed !== newNodes[i].tasks_completed
      ) {
        return true;
      }
    }

    return false;
  }

  onNodesChanged(nodes) {
    console.log("Node list changed - updating all metrics");

    // Update status bar with animation
    this.animateStatusUpdate();

    // Update connection guide if needed
    this.updateConnectionInfo();

    // Log the change for debugging
    console.log(
      `Network now has ${nodes.length} nodes:`,
      nodes.map((n) => `${n.capabilities.device_name} (${n.status})`)
    );
  }

  animateStatusUpdate() {
    // Add a brief highlight animation to show data updated
    const statusBar = document.querySelector(".status-bar");
    statusBar.classList.add("updating");
    setTimeout(() => {
      statusBar.classList.remove("updating");
    }, 1000);
  }

  displayNodes(nodes) {
    const nodesGrid = document.getElementById("nodes-grid");

    if (nodes.length === 0) {
      nodesGrid.innerHTML =
        "<p>No nodes detected. Register this device to get started!</p>";
      return;
    }

    nodesGrid.innerHTML = nodes
      .map(
        (node) => `
            <div class="node-card" data-status="${node.status}" data-node-id="${
          node.node_id
        }">
                <div class="node-header">
                    <span class="node-name">${
                      node.capabilities.device_name
                    }</span>
                    <span class="node-status status-${
                      node.status
                    }">${node.status.toUpperCase()}</span>
                </div>
                <div class="node-details">
                    <div class="node-detail"><strong>Type:</strong> ${
                      node.node_type
                    }</div>
                    <div class="node-detail"><strong>Platform:</strong> ${
                      node.capabilities.platform
                    }</div>
                    <div class="node-detail"><strong>Cores:</strong> ${
                      node.capabilities.cpu_cores
                    }</div>
                    <div class="node-detail"><strong>Memory:</strong> ${Math.floor(
                      node.capabilities.total_memory / 1024
                    )} GB 
                        <span class="memory-available">(${Math.floor(
                          node.capabilities.available_memory / 1024
                        )} GB available)</span>
                    </div>
                    <div class="node-detail"><strong>Tasks Completed:</strong> ${
                      node.tasks_completed
                    }</div>
                    ${
                      node.capabilities.benchmark_score
                        ? `<div class="node-detail"><strong>Benchmark:</strong> ${node.capabilities.benchmark_score} ops/sec</div>`
                        : '<div class="node-detail"><strong>Benchmark:</strong> <span class="not-tested">Not tested</span></div>'
                    }
                    <div class="node-detail"><strong>Browser:</strong> ${
                      node.capabilities.browser_info
                    }</div>
                    <div class="node-detail"><strong>Last Seen:</strong> 
                        <span class="last-seen" data-timestamp="${
                          node.last_seen
                        }">${this.formatTime(node.last_seen)}</span>
                    </div>
                </div>
            </div>
        `
      )
      .join("");

    // Update total nodes count with animation
    this.animateCountUpdate("total-nodes", nodes.length);

    // Update last seen times every 30 seconds
    this.updateLastSeenTimes();
  }

  animateCountUpdate(elementId, newValue) {
    const element = document.getElementById(elementId);
    const currentValue = parseInt(element.textContent) || 0;

    if (currentValue !== newValue) {
      element.classList.add("updating");
      element.textContent = newValue;
      setTimeout(() => {
        element.classList.remove("updating");
      }, 500);
    }
  }

  updateLastSeenTimes() {
    const lastSeenElements = document.querySelectorAll(".last-seen");
    lastSeenElements.forEach((element) => {
      const timestamp = element.getAttribute("data-timestamp");
      if (timestamp) {
        element.textContent = this.formatTime(timestamp);
      }
    });
  }

  filterNodes(filter) {
    if (filter === "all") {
      this.displayNodes(this.nodes);
    } else {
      const filtered = this.nodes.filter((node) => node.node_type === filter);
      this.displayNodes(filtered);
    }
  }

  updateAllMetrics(nodes) {
    // Basic node counts
    const totalNodes = nodes?.length;
    const activeNodes = nodes.filter((node) => node.status === "active").length;
    const idleNodes = nodes.filter((node) => node.status === "idle").length;
    const offlineNodes = nodes.filter(
      (node) => node.status === "offline"
    ).length;

    // Computing power metrics
    const totalCores = nodes.reduce(
      (sum, node) => sum + node.capabilities.cpu_cores,
      0
    );
    const totalMemory = nodes.reduce(
      (sum, node) => sum + node.capabilities.total_memory,
      0
    );
    const totalAvailableMemory = nodes.reduce(
      (sum, node) => sum + node.capabilities.available_memory,
      0
    );

    // Performance metrics
    const benchmarkScores = nodes
      .filter((node) => node.capabilities.benchmark_score)
      .map((node) => node.capabilities.benchmark_score);
    const avgBenchmark =
      benchmarkScores.length > 0
        ? Math.floor(
            benchmarkScores.reduce((sum, score) => sum + score, 0) /
              benchmarkScores.length
          )
        : 0;
    const totalBenchmarkPower = benchmarkScores.reduce(
      (sum, score) => sum + score,
      0
    );

    // Task metrics
    const totalTasksCompleted = nodes.reduce(
      (sum, node) => sum + node.tasks_completed,
      0
    );

    // Device type breakdown
    const deviceTypes = {};
    const platformTypes = {};
    nodes.forEach((node) => {
      const deviceType = node.capabilities.device_type || "unknown";
      const platform = node.capabilities.platform || "unknown";
      deviceTypes[deviceType] = (deviceTypes[deviceType] || 0) + 1;
      platformTypes[platform] = (platformTypes[platform] || 0) + 1;
    });

    // Update Status Bar
    document.getElementById("total-nodes").textContent = totalNodes;
    document.getElementById("network-status").textContent = this.isRegistered
      ? "Registered"
      : "Connected";

    // Update status bar with detailed info
    const statusItems = document.querySelectorAll(".status-item");
    if (statusItems.length >= 3) {
      // Update existing or add new status items
      this.updateStatusItem(1, "Active Nodes", `${activeNodes}/${totalNodes}`);
      this.updateStatusItem(2, "Total Tasks", totalTasksCompleted);
    }

    // Update Performance Metrics
    document.getElementById("total-cores").textContent = totalCores;
    document.getElementById("total-memory").textContent = Math.floor(
      totalMemory / 1024
    );
    document.getElementById("avg-benchmark").textContent = avgBenchmark;

    // Update Task Queue (simulated for now)
    this.updateTaskQueue(nodes);

    // Update detailed metrics
    this.updateDetailedMetrics(nodes, {
      totalCores,
      totalMemory,
      totalAvailableMemory,
      avgBenchmark,
      totalBenchmarkPower,
      activeNodes,
      idleNodes,
      offlineNodes,
      deviceTypes,
      platformTypes,
    });
  }

  updateStatusItem(index, label, value) {
    const statusItems = document.querySelectorAll(".status-item");
    if (statusItems[index]) {
      const labelSpan = statusItems[index].querySelector(".status-label");
      const valueSpan = statusItems[index].querySelector("span:last-child");
      if (labelSpan && valueSpan) {
        labelSpan.textContent = label + ":";
        valueSpan.textContent = value;
      }
    }
  }

  updateTaskQueue(nodes) {
    // For now, simulate task queue based on node activity
    const runningTasks = nodes.filter(
      (node) => node.status === "active"
    ).length;
    const pendingTasks = Math.max(0, nodes.length - runningTasks);
    const completedTasks = nodes.reduce(
      (sum, node) => sum + node.tasks_completed,
      0
    );

    document.getElementById("pending-tasks").textContent = pendingTasks;
    document.getElementById("running-tasks").textContent = runningTasks;
    document.getElementById("completed-tasks").textContent = completedTasks;
    document.getElementById("active-tasks").textContent = runningTasks;
  }

  updateDetailedMetrics(nodes, metrics) {
    // Update or create detailed metrics display
    let detailsSection = document.getElementById("detailed-metrics");
    if (!detailsSection) {
      // Create detailed metrics section if it doesn't exist
      detailsSection = this.createDetailedMetricsSection();
    }

    // Update memory utilization
    const memoryUtilization = (
      ((metrics.totalMemory - metrics.totalAvailableMemory) /
        metrics.totalMemory) *
      100
    ).toFixed(1);

    // Update node distribution
    const nodeDistribution = Object.entries(metrics.deviceTypes)
      .map(([type, count]) => `${type}: ${count}`)
      .join(", ");

    // Platform distribution
    const platformDistribution = Object.entries(metrics.platformTypes)
      .map(([platform, count]) => `${platform}: ${count}`)
      .join(", ");

    // Update the detailed metrics content
    detailsSection.innerHTML = `
      <h3>📊 Detailed Network Statistics</h3>
      <div class="detailed-metrics-grid">
        <div class="metric-detail">
          <span class="metric-label">Memory Utilization:</span>
          <span class="metric-value">${memoryUtilization}%</span>
        </div>
        <div class="metric-detail">
          <span class="metric-label">Total Computing Power:</span>
          <span class="metric-value">${
            metrics.totalBenchmarkPower
          } ops/sec</span>
        </div>
        <div class="metric-detail">
          <span class="metric-label">Available Memory:</span>
          <span class="metric-value">${Math.floor(
            metrics.totalAvailableMemory / 1024
          )} GB</span>
        </div>
        <div class="metric-detail">
          <span class="metric-label">Node Status:</span>
          <span class="metric-value">Active: ${metrics.activeNodes}, Idle: ${
      metrics.idleNodes
    }, Offline: ${metrics.offlineNodes}</span>
        </div>
        <div class="metric-detail">
          <span class="metric-label">Device Types:</span>
          <span class="metric-value">${nodeDistribution || "None"}</span>
        </div>
        <div class="metric-detail">
          <span class="metric-label">Platforms:</span>
          <span class="metric-value">${platformDistribution || "None"}</span>
        </div>
      </div>
    `;
  }

  createDetailedMetricsSection() {
    const performanceSection = document.querySelector(".performance-metrics");
    const detailsSection = document.createElement("section");
    detailsSection.className = "detailed-metrics";
    detailsSection.id = "detailed-metrics";

    // Insert after performance metrics
    performanceSection.parentNode.insertBefore(
      detailsSection,
      performanceSection.nextSibling
    );

    return detailsSection;
  }

  formatTime(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = Math.floor((now - date) / 1000);

    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  startPeriodicUpdates() {
    // Only update "last seen" times and connection info with timers
    // Node updates now come via WebSocket

    // Update "last seen" times every 10 seconds without full reload
    setInterval(() => {
      this.updateLastSeenTimes();
    }, 10000);

    // Update connection info every 60 seconds
    setInterval(() => {
      this.updateConnectionInfo();
    }, 60000);
  }

  sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  updateConnectionInfo() {
    // Get current URL info
    const currentUrl = window.location;
    const port =
      currentUrl.port || (currentUrl.protocol === "https:" ? "443" : "80");
    const hostname = currentUrl.hostname;

    // Update local URL
    const localUrl = `http://localhost:${port}`;
    document.getElementById("local-url").textContent = localUrl;

    // Try to detect network IP (this is limited in browsers for security)
    // We'll use the current hostname as a fallback
    let networkUrl;
    if (hostname !== "localhost" && hostname !== "127.0.0.1") {
      networkUrl = `http://${hostname}:${port}`;
    } else {
      // For localhost, we can try to suggest the likely network IP
      networkUrl = "http://192.168.1.7:" + port; // Based on the ipconfig output
    }

    document.getElementById("network-url").textContent = networkUrl;

    // Generate QR code for the network URL
    this.generateQRCode(networkUrl);

    // Setup copy buttons
    this.setupCopyButtons();
  }

  generateQRCode(url) {
    const canvas = document.getElementById("qr-code");
    const ctx = canvas.getContext("2d");

    // Simple QR code-like pattern (not a real QR code, but visually similar)
    // In a real implementation, you'd use a QR code library
    const size = 200;
    const moduleSize = 8;
    const modules = size / moduleSize;

    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, size, size);

    ctx.fillStyle = "#000000";

    // Create a pattern based on the URL
    const pattern = this.createPattern(url, modules);

    for (let i = 0; i < modules; i++) {
      for (let j = 0; j < modules; j++) {
        if (pattern[i] && pattern[i][j]) {
          ctx.fillRect(i * moduleSize, j * moduleSize, moduleSize, moduleSize);
        }
      }
    }

    // Add position markers (corner squares)
    this.drawPositionMarker(ctx, 0, 0, moduleSize);
    this.drawPositionMarker(ctx, (modules - 7) * moduleSize, 0, moduleSize);
    this.drawPositionMarker(ctx, 0, (modules - 7) * moduleSize, moduleSize);
  }

  createPattern(url, size) {
    // Create a deterministic pattern based on the URL
    const pattern = [];
    const hash = this.simpleHash(url);

    for (let i = 0; i < size; i++) {
      pattern[i] = [];
      for (let j = 0; j < size; j++) {
        // Skip corners for position markers
        if (
          (i < 7 && j < 7) ||
          (i >= size - 7 && j < 7) ||
          (i < 7 && j >= size - 7)
        ) {
          pattern[i][j] = false;
        } else {
          pattern[i][j] = (hash + i * j) % 3 === 0;
        }
      }
    }

    return pattern;
  }

  simpleHash(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = (hash << 5) - hash + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash);
  }

  drawPositionMarker(ctx, x, y, moduleSize) {
    // Draw position marker (7x7 square with pattern)
    ctx.fillStyle = "#000000";
    ctx.fillRect(x, y, 7 * moduleSize, 7 * moduleSize);
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(
      x + moduleSize,
      y + moduleSize,
      5 * moduleSize,
      5 * moduleSize
    );
    ctx.fillStyle = "#000000";
    ctx.fillRect(
      x + 2 * moduleSize,
      y + 2 * moduleSize,
      3 * moduleSize,
      3 * moduleSize
    );
  }

  setupCopyButtons() {
    document.querySelectorAll(".copy-btn").forEach((btn) => {
      btn.addEventListener("click", async (e) => {
        const urlId = e.target.getAttribute("data-url");
        const urlElement = document.getElementById(urlId);
        const url = urlElement.textContent;

        try {
          await navigator.clipboard.writeText(url);

          // Visual feedback
          const originalText = e.target.textContent;
          e.target.textContent = "✓";
          e.target.classList.add("copied");

          setTimeout(() => {
            e.target.textContent = originalText;
            e.target.classList.remove("copied");
          }, 2000);
        } catch (err) {
          console.error("Failed to copy URL:", err);
          // Fallback for older browsers
          this.fallbackCopyText(url);
        }
      });
    });
  }

  fallbackCopyText(text) {
    const textArea = document.createElement("textarea");
    textArea.value = text;
    textArea.style.position = "fixed";
    textArea.style.left = "-999999px";
    textArea.style.top = "-999999px";
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();

    try {
      document.execCommand("copy");
      alert("URL copied to clipboard!");
    } catch (err) {
      alert("Failed to copy URL. Please copy manually: " + text);
    }

    document.body.removeChild(textArea);
  }
}

// Initialize the client when the page loads
document.addEventListener("DOMContentLoaded", () => {
  window.processDistroClient = new ProcessDistroClient();
});

// Handle page visibility changes
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible" && window.processDistroClient) {
    // WebSocket will automatically sync when page becomes visible
    // Just re-render current data and reconnect if needed
    if (
      !window.processDistroClient.websocket ||
      window.processDistroClient.websocket.readyState !== WebSocket.OPEN
    ) {
      window.processDistroClient.connectWebSocket();
    }
  }
});

// Handle page unload (when user closes tab, navigates away, or reloads)
window.addEventListener("beforeunload", async (event) => {
  if (window.processDistroClient && window.processDistroClient.isRegistered) {
    // Try to unregister the device
    await window.processDistroClient.cleanup();
  }
});

// Handle page hide (more reliable than beforeunload in some browsers)
document.addEventListener("visibilitychange", () => {
  if (
    document.visibilityState === "hidden" &&
    window.processDistroClient &&
    window.processDistroClient.isRegistered
  ) {
    // Use sendBeacon for reliable cleanup when page is hidden
    navigator.sendBeacon(
      "/api/unregister",
      JSON.stringify({
        node_id: window.processDistroClient.deviceId,
        timestamp: new Date().toISOString(),
      })
    );
  }
});

// Handle browser close/refresh
window.addEventListener("pagehide", async () => {
  if (window.processDistroClient && window.processDistroClient.isRegistered) {
    // Use sendBeacon for more reliable cleanup
    navigator.sendBeacon(
      "/api/unregister",
      JSON.stringify({
        node_id: window.processDistroClient.deviceId,
        timestamp: new Date().toISOString(),
      })
    );
  }
});
