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
    this.wasmAvailable = false;
    this.wasmInstance = null;
    this.runningTasks = {}; // map node_id -> { taskId, progress, intervalId, completed }
    this.taskHistory = []; // Array to store task history
    this.completedTasks = new Set(); // Track completed tasks to prevent duplicate messages

    this.init();
  }

  async init() {
    await this.detectDeviceCapabilities();
    this.setupEventListeners();
    await this.loadWasmModule();
    this.connectWebSocket();
    this.updateDeviceDisplay();
    this.updateConnectionInfo();

    document.getElementById("device-id").textContent = this.deviceId;
  }

  // Attempt to load wasm modules for available tasks
  async loadWasmModule() {
    try {
      if (typeof WebAssembly !== "undefined") {
        // Try to load Mandelbrot WASM module
        try {
          const res = await fetch("/static/mandelbrot.wasm");
          if (res.ok) {
            if (WebAssembly.instantiateStreaming) {
              const { instance } = await WebAssembly.instantiateStreaming(
                res,
                {}
              );
              this.mandelbrotWasm = instance;
              console.log("✅ Mandelbrot WASM module loaded");
            } else {
              const bytes = await res.arrayBuffer();
              const module = await WebAssembly.compile(bytes);
              const instance = await WebAssembly.instantiate(module, {});
              this.mandelbrotWasm = instance;
              console.log("✅ Mandelbrot WASM module loaded (fallback)");
            }
          }
        } catch (e) {
          console.warn("⚠️ Mandelbrot WASM failed to load:", e);
        }

        // Try to load password hash WASM module (fallback for legacy support)
        // try {
        //   const res = await fetch("/static/password_hash.wasm");
        //   if (res.ok) {
        //     if (WebAssembly.instantiateStreaming) {
        //       const { instance } = await WebAssembly.instantiateStreaming(
        //         res,
        //         {}
        //       );
        //       this.wasmInstance = instance;
        //       this.wasmAvailable =
        //         !!instance.exports &&
        //         typeof instance.exports.run === "function";
        //       console.log("✅ Password hash WASM module loaded");
        //     } else {
        //       const bytes = await res.arrayBuffer();
        //       const module = await WebAssembly.compile(bytes);
        //       const instance = await WebAssembly.instantiate(module, {});
        //       this.wasmInstance = instance;
        //       this.wasmAvailable =
        //         !!instance.exports &&
        //         typeof instance.exports.run === "function";
        //       console.log("✅ Password hash WASM module loaded (fallback)");
        //     }
        //   }
        // } catch (e) {
        //   console.warn(
        //     "⚠️ Password hash WASM failed to load (expected for Mandelbrot-only setup):",
        //     e
        //   );
        // }
      }
    } catch (e) {
      console.warn(
        "WASM load failed completely, falling back to JS simulation",
        e
      );
    }

    // Set availability flags
    this.mandelbrotAvailable = !!this.mandelbrotWasm;
    if (!this.wasmAvailable) this.wasmAvailable = false;
    if (!this.wasmInstance) this.wasmInstance = null;
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
      case "execute_task":
        // Execute task or simulate it on the dashboard
        this.handleExecuteTask(message);
        break;
      case "canvas_completed":
        // Handle canvas completion from other dashboards
        this.handleCanvasCompletion(message);
        break;
      case "canvas_progress":
        // Handle canvas progress from other dashboards
        this.handleCanvasProgress(message);
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

  // Handle an execute_task WS message
  async handleExecuteTask(message) {
    try {
      const taskId = message.task_id;
      const taskType = message.task_type;
      const payload = message.payload || {};
      const assigned = message.assigned_node;

      console.log("🎯 Received execute_task message:", {
        taskId,
        taskType,
        assigned,
        myDeviceId: this.deviceId,
        isForMe: assigned === this.deviceId,
      });

      // Prevent duplicate tasks for the same node
      if (this.runningTasks[assigned] && this.runningTasks[assigned].taskId) {
        console.warn(
          `⚠️ Node ${assigned} already has a running task, ignoring duplicate`
        );
        return;
      }

      // Add task to history
      this.addTaskToHistory(taskId, taskType, assigned, "running");

      // Mark the node as busy in local nodes list and start a progress simulation
      const nodeIndex = this.nodes.findIndex((n) => n.node_id === assigned);
      if (nodeIndex >= 0) {
        // update status locally to busy so UI shows worker illustration
        this.nodes[nodeIndex].status = "busy";
        this.displayNodes(this.nodes);
      }

      // If this device is the assigned node, actually run the work (WASM if available)
      if (assigned === this.deviceId) {
        console.log("🔥 This task is assigned to ME! Starting execution...");

        // Clear any previous completion tracking for this node
        const taskKey = `${assigned}-completion`;
        this.completedTasks.delete(taskKey);

        const start = performance.now();
        let result = null;

        // Handle different task types
        if (taskType === "mandelbrot") {
          console.log("🎨 Executing Mandelbrot fractal rendering...");
          result = await this.executeMandelbrotTask(payload);
        } else if (taskType === "password_hash") {
          console.log("🔐 Executing password hashing task...");
          if (
            this.wasmAvailable &&
            this.wasmInstance &&
            typeof this.wasmInstance.exports.run === "function"
          ) {
            console.log("⚡ Using WASM execution with payload:", payload);
            try {
              const iterations = payload.workload || 10000;
              result = this.wasmInstance.exports.run(iterations) || 0;
              console.log("✅ WASM execution completed, result:", result);
            } catch (e) {
              console.warn("WASM execution failed, falling back to JS", e);
              result = this.simulateCpuWork(payload.workload || 10000);
            }
          } else {
            console.log(
              "🔧 Using JavaScript fallback execution with payload:",
              payload
            );
            result = await this.simulateCpuWorkAsync(
              payload.workload || 10000,
              (progress) => {
                this.updateNodeProgress(this.deviceId, progress);
              }
            );
            console.log("✅ JavaScript execution completed, result:", result);
          }
        } else {
          console.log(
            "🔧 Using generic JavaScript execution for unknown task type:",
            taskType
          );
          result = await this.simulateCpuWorkAsync(
            payload.workload || 10000,
            (progress) => {
              this.updateNodeProgress(this.deviceId, progress);
            }
          );
        }

        const duration = performance.now() - start;

        // Ensure progress shows 100%
        this.updateNodeProgress(this.deviceId, 100);

        // Complete task in history
        this.completeTaskInHistory(taskId, "completed");

        // Clear the running task for this node
        delete this.runningTasks[this.deviceId];

        // send completion message via WebSocket (preferred) or HTTP fallback
        const completionMsg = {
          type: "task_complete",
          node_id: this.deviceId,
          task_id: taskId,
          result: result,
          duration_ms: duration,
          timestamp: new Date().toISOString(),
        };

        try {
          if (this.websocket && this.websocket.readyState === WebSocket.OPEN) {
            console.log(
              "🚀 Sending task completion via WebSocket:",
              completionMsg
            );
            this.websocket.send(JSON.stringify(completionMsg));
          } else {
            console.log("🔄 WebSocket not available, using HTTP fallback");
            // fallback to HTTP POST - FIX: correct endpoint
            await fetch("/api/submit_task_result", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify(completionMsg),
            });
          }
        } catch (e) {
          console.error("Failed to send task completion:", e);
        }

        // update local node status
        const idx = this.nodes.findIndex((n) => n.node_id === this.deviceId);
        if (idx >= 0) {
          this.nodes[idx].status = "idle";
          this.nodes[idx].tasks_completed =
            (this.nodes[idx].tasks_completed || 0) + 1;
          this.displayNodes(this.nodes);
        }
      } else {
        console.log(
          "👀 Task assigned to remote node:",
          assigned,
          "- starting simulation on dashboard"
        );
        // For other nodes, simulate progress on the dashboard for that node only
        this.startRemoteNodeSimulation(assigned, taskId);
      }
    } catch (e) {
      console.error("handleExecuteTask error:", e);
    }
  }

  // Start simulation progress for a remote node on the dashboard
  startRemoteNodeSimulation(nodeId, taskId) {
    // Avoid duplicate simulations
    if (this.runningTasks[nodeId] && this.runningTasks[nodeId].taskId) {
      console.warn(
        `⚠️ Node ${nodeId} already has a running simulation, skipping`
      );
      return;
    }

    console.log(`🎬 Starting simulation for remote node: ${nodeId}`);

    let progress = 0;
    const intervalId = setInterval(() => {
      progress = Math.min(100, progress + Math.floor(Math.random() * 8) + 3); // Slower, more stable progress
      this.updateNodeProgress(nodeId, progress);

      // Don't broadcast progress for simulated tasks (only real tasks should broadcast)
      // this.broadcastCanvasProgress(nodeId, progress);

      if (progress >= 100) {
        console.log(`✅ Simulation completed for node: ${nodeId}`);
        clearInterval(intervalId);

        // Complete task in history
        this.completeTaskInHistory(taskId, "completed");

        // Clear running task
        delete this.runningTasks[nodeId];

        // mark node back to idle
        const idx = this.nodes.findIndex((n) => n.node_id === nodeId);
        if (idx >= 0) {
          this.nodes[idx].status = "idle";
          this.nodes[idx].tasks_completed =
            (this.nodes[idx].tasks_completed || 0) + 1;
          this.displayNodes(this.nodes);
        }
      }
    }, 800 + Math.floor(Math.random() * 600)); // Slower interval for stability

    this.runningTasks[nodeId] = { taskId, progress: 0, intervalId };
    // set node status to busy
    const idx = this.nodes.findIndex((n) => n.node_id === nodeId);
    if (idx >= 0) {
      this.nodes[idx].status = "busy";
      this.displayNodes(this.nodes);
    }
  }

  // Update progress bar on the node card
  updateNodeProgress(nodeId, progress) {
    // Prevent progress from going backwards (except when starting new task at 0)
    if (
      this.runningTasks[nodeId] &&
      this.runningTasks[nodeId].progress > progress &&
      progress > 0
    ) {
      console.warn(
        `⚠️ Ignoring backwards progress for ${nodeId}: ${progress}% (was ${this.runningTasks[nodeId].progress}%)`
      );
      return;
    }

    // update runningTasks map
    if (!this.runningTasks[nodeId]) {
      this.runningTasks[nodeId] = {
        taskId: null,
        progress: 0,
        intervalId: null,
      };
    }
    this.runningTasks[nodeId].progress = progress;

    // Update DOM directly for the node card (faster than re-rendering all nodes)
    const card = document.querySelector(`.node-card[data-node-id="${nodeId}"]`);
    if (card) {
      let fill = card.querySelector(".worker-fill");
      if (!fill) {
        // If worker illustration not present, re-render nodes to include it
        this.displayNodes(this.nodes);
        fill = card.querySelector(".worker-fill");
      }
      if (fill) {
        fill.style.width = `${progress}%`;
      }
    }
  }

  // JS fallback CPU work - synchronous (blocking) - kept as small when used
  simulateCpuWork(iterations) {
    let acc = 0;
    for (let i = 0; i < iterations; i++) {
      acc = (acc + ((i * 9301 + 49297) % 233280)) % 1000000;
    }
    return acc;
  }

  // Async non-blocking CPU simulation that reports progress via callback
  async simulateCpuWorkAsync(iterations, onProgress) {
    const chunk = Math.max(1000, Math.floor(iterations / 50));
    let acc = 0;
    for (let start = 0; start < iterations; start += chunk) {
      const end = Math.min(iterations, start + chunk);
      // run chunk synchronously
      for (let i = start; i < end; i++) {
        acc = (acc + ((i * 9301 + 49297) % 233280)) % 1000000;
      }
      const progress = Math.floor((end / iterations) * 100);
      if (onProgress) onProgress(progress);
      // yield to event loop
      await this.sleep(20);
    }
    return acc;
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

  // Add task to history and update display
  addTaskToHistory(taskId, taskType, nodeId, status = "running") {
    const task = {
      id: taskId,
      type: taskType,
      nodeId: nodeId,
      status: status,
      startTime: new Date(),
      endTime: null,
      duration: null,
    };

    // Remove if already exists (update case)
    this.taskHistory = this.taskHistory.filter((t) => t.id !== taskId);

    // Add to beginning
    this.taskHistory.unshift(task);

    // Keep only last 10 tasks
    if (this.taskHistory.length > 10) {
      this.taskHistory = this.taskHistory.slice(0, 10);
    }

    this.updateTaskHistoryDisplay();
  }

  // Complete a task in history
  completeTaskInHistory(taskId, status = "completed") {
    const task = this.taskHistory.find((t) => t.id === taskId);
    if (task) {
      task.status = status;
      task.endTime = new Date();
      task.duration = task.endTime - task.startTime;

      // Log task completion to JSON file
      this.logTaskCompletion(task);

      this.updateTaskHistoryDisplay();
    }
  }

  // Log task completion details to server
  async logTaskCompletion(task) {
    try {
      const node = this.nodes.find((n) => n.node_id === task.nodeId);
      const taskLog = {
        task_id: task.id,
        task_type: task.type,
        node_id: task.nodeId,
        device_name: node ? node.capabilities.device_name : "Unknown",
        device_type: node ? node.capabilities.device_type : "unknown",
        platform: node ? node.capabilities.platform : "unknown",
        cpu_cores: node ? node.capabilities.cpu_cores : 0,
        memory_mb: node ? node.capabilities.total_memory : 0,
        benchmark_score: node ? node.capabilities.benchmark_score : null,
        status: task.status,
        start_time: task.startTime.toISOString(),
        end_time: task.endTime.toISOString(),
        duration_ms: task.duration,
        duration_seconds: Math.round(task.duration / 1000),
        timestamp: new Date().toISOString(),
        success: task.status === "completed",
      };

      console.log("📝 Logging task completion:", taskLog);

      // Send to server for JSON file logging
      const response = await fetch("/api/log_task", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(taskLog),
      });

      if (response.ok) {
        console.log("✅ Task logged successfully");
      } else {
        console.warn("⚠️ Failed to log task:", response.statusText);
      }
    } catch (error) {
      console.error("❌ Error logging task completion:", error);
    }
  }

  // Update task history display
  updateTaskHistoryDisplay() {
    const historyContainer = document.getElementById("task-history-list");

    if (this.taskHistory.length === 0) {
      historyContainer.innerHTML =
        "<p class=\"no-tasks\">No tasks executed yet. Use the controller terminal to run 'run-password-test'.</p>";
      return;
    }

    historyContainer.innerHTML = this.taskHistory
      .map((task) => {
        const duration = task.duration
          ? `${Math.round(task.duration / 1000)}s`
          : task.status === "running"
          ? "Running..."
          : "N/A";

        const node = this.nodes.find((n) => n.node_id === task.nodeId);
        const nodeName = node ? node.capabilities.device_name : "Unknown";

        return `
        <div class="task-item">
          <div class="task-info">
            <div class="task-title">🔐 ${task.type
              .replace("_", " ")
              .toUpperCase()}</div>
            <div class="task-meta">Node: ${nodeName} • Started: ${task.startTime.toLocaleTimeString()} • Duration: ${duration}</div>
          </div>
          <div class="task-status ${task.status}">${task.status}</div>
        </div>
      `;
      })
      .join("");
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
    // Clean up any running tasks for this node
    if (this.runningTasks[nodeId]) {
      console.log(
        `🧹 Cleaning up running task for disconnected node: ${nodeId}`
      );
      if (this.runningTasks[nodeId].intervalId) {
        clearInterval(this.runningTasks[nodeId].intervalId);
      }
      delete this.runningTasks[nodeId];
    }

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
      .map((node) => {
        const taskProgress = this.runningTasks[node.node_id] || { progress: 0 };
        const isWorking = node.status === "busy" && taskProgress.progress < 100;

        return `
            <div class="node-card" data-status="${node.status}" data-node-id="${
          node.node_id
        }">
                <div class="node-header">
                    <span class="node-name">${
                      node.capabilities.device_name
                    }</span>
                    <span class="node-status-indicator ${node.status}">
                        ${node.status.toUpperCase()}
                    </span>
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
                      node.tasks_completed || 0
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
                    
                    ${
                      isWorking
                        ? `
                    <div class="worker-progress">
                        <div class="worker-header">
                            <span class="worker-title">
                                <span class="worker-icon ${node.status}"></span>
                                Password Hash Task
                            </span>
                            <span class="task-indicator password-hash">🔐 BCRYPT</span>
                        </div>
                        <div class="worker-bar">
                            <div class="worker-fill active" style="width: ${
                              taskProgress.progress
                            }%"></div>
                        </div>
                        <div class="worker-stats">
                            <span>Progress: ${Math.round(
                              taskProgress.progress
                            )}%</span>
                            <span>Status: ${
                              taskProgress.progress < 100
                                ? "Processing..."
                                : "Completed"
                            }</span>
                        </div>
                    </div>
                    `
                        : ""
                    }
                    
                    ${
                      node.status === "idle" && (node.tasks_completed || 0) > 0
                        ? `
                    <div class="worker-progress">
                        <div class="worker-header">
                            <span class="worker-title">
                                <span class="worker-icon idle"></span>
                                Ready for Tasks
                            </span>
                            <span class="task-indicator completed">✅ READY</span>
                        </div>
                        <div class="worker-stats">
                            <span>Completed: ${node.tasks_completed} task(s)</span>
                            <span>Status: Idle</span>
                        </div>
                    </div>
                    `
                        : ""
                    }
                </div>
            </div>
        `;
      })
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

  // Execute Mandelbrot fractal rendering task
  async executeMandelbrotTask(payload) {
    console.log(
      "🎨 Starting Mandelbrot fractal rendering with payload:",
      payload
    );

    const width = payload.width || 800;
    const height = payload.height || 600;
    const maxIterations = payload.max_iterations || 100;
    const minReal = payload.min_real || -2.5;
    const maxReal = payload.max_real || 1.0;
    const minImag = payload.min_imag || -1.25;
    const maxImag = payload.max_imag || 1.25;

    // Create or get canvas for this node
    let canvas = this.getOrCreateCanvas(this.deviceId, width, height);
    const ctx = canvas.getContext("2d");
    const imageData = ctx.createImageData(width, height);

    let pixelsCompleted = 0;
    const totalPixels = width * height;

    console.log(`🖼️ Rendering ${width}x${height} Mandelbrot set...`);

    // Render in chunks to allow progress updates and avoid blocking UI
    const chunkSize = 1000; // Process 1000 pixels at a time
    let currentPixel = 0;

    while (currentPixel < totalPixels) {
      const endPixel = Math.min(currentPixel + chunkSize, totalPixels);

      for (let i = currentPixel; i < endPixel; i++) {
        const x = i % width;
        const y = Math.floor(i / width);

        // Map pixel to complex plane
        const real = minReal + (x / width) * (maxReal - minReal);
        const imag = minImag + (y / height) * (maxImag - minImag);

        // Calculate Mandelbrot iterations
        const iterations = this.mandelbrotIterations(real, imag, maxIterations);

        // Convert to color
        const color = this.iterationsToColor(iterations, maxIterations);

        // Set pixel in image data
        const pixelIndex = (y * width + x) * 4;
        imageData.data[pixelIndex] = (color >> 16) & 0xff; // R
        imageData.data[pixelIndex + 1] = (color >> 8) & 0xff; // G
        imageData.data[pixelIndex + 2] = color & 0xff; // B
        imageData.data[pixelIndex + 3] = 255; // A
      }

      pixelsCompleted = endPixel;
      const progress = Math.floor((pixelsCompleted / totalPixels) * 100);

      // Update progress and canvas
      this.updateNodeProgress(this.deviceId, progress);
      this.updateCanvasProgress(this.deviceId, progress);

      // Update canvas every few chunks
      if (
        currentPixel % (chunkSize * 5) === 0 ||
        pixelsCompleted === totalPixels
      ) {
        ctx.putImageData(imageData, 0, 0);
      }

      currentPixel = endPixel;

      // Allow UI to update
      await new Promise((resolve) => setTimeout(resolve, 1));
    }

    // Final canvas update
    ctx.putImageData(imageData, 0, 0);

    console.log("✅ Mandelbrot rendering completed!");

    return {
      pixels_rendered: totalPixels,
      width: width,
      height: height,
      iterations: maxIterations,
      canvas_id: `canvas-${this.deviceId}`,
    };
  }

  // Calculate Mandelbrot iterations for a point
  mandelbrotIterations(real, imag, maxIterations) {
    let zReal = 0;
    let zImag = 0;
    let iterations = 0;

    while (iterations < maxIterations && zReal * zReal + zImag * zImag <= 4) {
      const tempReal = zReal * zReal - zImag * zImag + real;
      zImag = 2 * zReal * zImag + imag;
      zReal = tempReal;
      iterations++;
    }

    return iterations;
  }

  // Convert iterations to color
  iterationsToColor(iterations, maxIterations) {
    if (iterations === maxIterations) {
      return 0x000000; // Black for points in the set
    }

    // Create colorful gradient
    const ratio = iterations / maxIterations;
    const hue = ratio * 360;
    const saturation = 1.0;
    const value = 1.0;

    return this.hsvToRgb(hue, saturation, value);
  }

  // Convert HSV to RGB color
  hsvToRgb(h, s, v) {
    const c = v * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = v - c;

    let r, g, b;
    if (h < 60) {
      r = c;
      g = x;
      b = 0;
    } else if (h < 120) {
      r = x;
      g = c;
      b = 0;
    } else if (h < 180) {
      r = 0;
      g = c;
      b = x;
    } else if (h < 240) {
      r = 0;
      g = x;
      b = c;
    } else if (h < 300) {
      r = x;
      g = 0;
      b = c;
    } else {
      r = c;
      g = 0;
      b = x;
    }

    const rInt = Math.floor((r + m) * 255);
    const gInt = Math.floor((g + m) * 255);
    const bInt = Math.floor((b + m) * 255);

    return (rInt << 16) | (gInt << 8) | bInt;
  }

  // Get or create canvas for a node in the persistent gallery
  getOrCreateCanvas(nodeId, width, height) {
    let canvasItem = document.getElementById(`canvas-item-${nodeId}`);

    if (!canvasItem) {
      // Create new canvas item in the gallery
      const gallery = document.getElementById("canvas-gallery-grid");
      const noCanvasMsg = gallery.querySelector(".no-canvas");
      if (noCanvasMsg) {
        noCanvasMsg.remove();
      }

      canvasItem = document.createElement("div");
      canvasItem.id = `canvas-item-${nodeId}`;
      canvasItem.className = "canvas-item new-canvas rendering";

      // Get device name from nodes list
      const node = this.nodes.find((n) => n.node_id === nodeId);
      const deviceName = node ? node.capabilities.device_name : nodeId;

      canvasItem.innerHTML = `
        <canvas id="canvas-${nodeId}" width="${width}" height="${height}"></canvas>
        <div class="canvas-info">
          <div class="canvas-title">
            🖥️ ${deviceName}
            <span class="canvas-status rendering">🎨 Rendering...</span>
          </div>
          <div class="canvas-meta">
            <span>${width}×${height} pixels</span>
            <span id="canvas-time-${nodeId}">Starting...</span>
          </div>
          <div class="canvas-progress">
            <div class="canvas-progress-bar" id="canvas-progress-${nodeId}" style="width: 0%"></div>
          </div>
        </div>
      `;

      gallery.appendChild(canvasItem);

      // Remove animation class after animation completes
      setTimeout(() => {
        canvasItem.classList.remove("new-canvas");
      }, 600);
    }

    const canvas = canvasItem.querySelector("canvas");
    canvas.width = width;
    canvas.height = height;

    return canvas;
  }

  // Update canvas progress and status
  updateCanvasProgress(nodeId, progress, status = "rendering") {
    // Skip if already completed to prevent spam
    const canvasItem = document.getElementById(`canvas-item-${nodeId}`);
    if (canvasItem && canvasItem.classList.contains("completed")) {
      return;
    }

    const progressBar = document.getElementById(`canvas-progress-${nodeId}`);
    const statusElement = document.querySelector(
      `#canvas-item-${nodeId} .canvas-status`
    );

    if (progressBar) {
      progressBar.style.width = `${progress}%`;
    }

    if (statusElement && canvasItem) {
      if (progress >= 100) {
        statusElement.textContent = "✅ Completed";
        statusElement.className = "canvas-status";
        canvasItem.classList.remove("rendering");
        canvasItem.classList.add("completed"); // Mark as completed

        // Update completion time
        const timeElement = document.getElementById(`canvas-time-${nodeId}`);
        if (timeElement) {
          timeElement.textContent = `Completed at ${new Date().toLocaleTimeString()}`;
        }

        // Only broadcast completion once for my device
        if (nodeId === this.deviceId) {
          this.broadcastCanvasCompletion(nodeId);
        }
      } else {
        statusElement.textContent = `🎨 Rendering ${progress}%`;
        statusElement.className = "canvas-status rendering";
        canvasItem.classList.add("rendering");

        // Only broadcast progress if this is my device and not completed
        if (nodeId === this.deviceId) {
          this.broadcastCanvasProgress(nodeId, progress);
        }
      }
    }
  }

  // Broadcast canvas completion to all connected dashboards
  broadcastCanvasCompletion(nodeId) {
    // Prevent duplicate completion broadcasts
    const taskKey = `${nodeId}-completion`;
    if (this.completedTasks.has(taskKey)) {
      console.log(
        `⚠️ Task completion already broadcast for ${nodeId}, skipping`
      );
      return;
    }

    if (this.websocket && this.websocket.readyState === WebSocket.OPEN) {
      const message = {
        type: "canvas_completed",
        node_id: nodeId,
        completed_at: new Date().toISOString(),
        status: "completed",
      };

      console.log("📡 Broadcasting canvas completion status:", message);
      this.websocket.send(JSON.stringify(message));

      // Mark as completed to prevent duplicates
      this.completedTasks.add(taskKey);
    }
  }

  // Broadcast progress updates to all connected dashboards
  broadcastCanvasProgress(nodeId, progress) {
    // Only broadcast if this is our own task execution, not simulations
    if (
      nodeId === this.deviceId &&
      this.websocket &&
      this.websocket.readyState === WebSocket.OPEN
    ) {
      const message = {
        type: "canvas_progress",
        node_id: nodeId,
        progress: progress,
        timestamp: new Date().toISOString(),
      };

      console.log("� Broadcasting canvas progress:", message);
      this.websocket.send(JSON.stringify(message));
    }
  }

  // Handle canvas completion messages from other dashboards
  handleCanvasCompletion(message) {
    const { node_id, completed_at, status } = message;

    console.log(
      `🎨 Canvas completion received for ${node_id}, status: ${status}`
    );

    // Update status to completed if we have this canvas item
    const canvasItem = document.getElementById(`canvas-item-${node_id}`);
    if (canvasItem) {
      this.updateCanvasProgress(node_id, 100, "completed");
    } else {
      // Create a placeholder canvas item for nodes we don't have locally
      this.createCanvasPlaceholder(node_id, completed_at);
    }
  }

  // Handle canvas progress messages from other dashboards
  handleCanvasProgress(message) {
    const { node_id, progress, timestamp } = message;

    // Ignore our own progress messages to prevent loops
    if (node_id === this.deviceId) {
      return;
    }

    // Ignore very old progress messages (older than 30 seconds)
    if (timestamp) {
      const messageTime = new Date(timestamp);
      const now = new Date();
      const ageInSeconds = (now - messageTime) / 1000;
      if (ageInSeconds > 30) {
        console.warn(
          `⚠️ Ignoring old progress message for ${node_id}: ${ageInSeconds}s old`
        );
        return;
      }
    }

    console.log(`📈 Canvas progress received for ${node_id}: ${progress}%`);

    // Update progress if we have this canvas item
    const canvasItem = document.getElementById(`canvas-item-${node_id}`);
    if (canvasItem) {
      this.updateCanvasProgress(node_id, progress, "rendering");
    } else if (progress < 100) {
      // Create a placeholder for remote rendering
      this.createCanvasPlaceholder(node_id, null, progress);
    }
  }

  // Create a placeholder canvas item for remote nodes (status-only)
  createCanvasPlaceholder(nodeId, completedAt, progress = 0) {
    const gallery = document.getElementById("canvas-gallery-grid");
    const noCanvasMsg = gallery.querySelector(".no-canvas");
    if (noCanvasMsg) {
      noCanvasMsg.remove();
    }

    const canvasItem = document.createElement("div");
    canvasItem.id = `canvas-item-${nodeId}`;
    canvasItem.className = "canvas-item new-canvas remote-canvas";

    // Get device name from nodes list
    const node = this.nodes.find((n) => n.node_id === nodeId);
    const deviceName = node ? node.capabilities.device_name : nodeId;

    const isCompleted = progress >= 100 || completedAt;
    const statusText = isCompleted
      ? "✅ Completed"
      : `🎨 Rendering ${progress}%`;
    const statusClass = isCompleted
      ? "canvas-status"
      : "canvas-status rendering";
    const timeText = completedAt
      ? `Completed at ${new Date(completedAt).toLocaleTimeString()}`
      : "Rendering on remote device...";

    canvasItem.innerHTML = `
      <div class="canvas-placeholder">
        <div class="placeholder-icon">🖥️</div>
        <div class="placeholder-text">Remote Device Rendering</div>
      </div>
      <div class="canvas-info">
        <div class="canvas-title">
          🖥️ ${deviceName}
          <span class="${statusClass}">${statusText}</span>
        </div>
        <div class="canvas-meta">
          <span>Mandelbrot Set</span>
          <span id="canvas-time-${nodeId}">${timeText}</span>
        </div>
        <div class="canvas-progress">
          <div class="canvas-progress-bar" id="canvas-progress-${nodeId}" style="width: ${progress}%"></div>
        </div>
      </div>
    `;

    if (!isCompleted) {
      canvasItem.classList.add("rendering");
    }

    gallery.appendChild(canvasItem);

    // Remove animation class after animation completes
    setTimeout(() => {
      canvasItem.classList.remove("new-canvas");
    }, 600);
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

// Metrics functionality
window.metricsManager = {
  modal: null,
  isLoading: false,

  init() {
    this.modal = document.getElementById("metricsModal");
    this.setupEventListeners();
  },

  setupEventListeners() {
    // View Metrics button
    const viewMetricsBtn = document.getElementById("viewMetricsBtn");
    if (viewMetricsBtn) {
      viewMetricsBtn.addEventListener("click", () => this.showMetrics());
    }

    // Close modal handlers
    const closeBtn = this.modal?.querySelector(".modal-close");
    if (closeBtn) {
      closeBtn.addEventListener("click", () => this.hideMetrics());
    }

    // Close on outside click
    if (this.modal) {
      this.modal.addEventListener("click", (e) => {
        if (e.target === this.modal) {
          this.hideMetrics();
        }
      });
    }

    // Close on Escape key
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && this.modal?.style.display === "block") {
        this.hideMetrics();
      }
    });
  },

  async showMetrics() {
    if (this.isLoading) return;

    this.modal.style.display = "block";
    this.isLoading = true;

    try {
      // Show loading state
      this.showLoadingState();

      // Fetch task logs data
      const response = await fetch("/api/task_logs");
      if (!response.ok) {
        throw new Error(`Failed to fetch task logs: ${response.status}`);
      }

      const taskLogs = await response.json();

      // Update metrics display
      this.updateMetricsDisplay(taskLogs);
    } catch (error) {
      console.error("Error loading metrics:", error);
      this.showErrorState(error.message);
    } finally {
      this.isLoading = false;
    }
  },

  hideMetrics() {
    this.modal.style.display = "none";
  },

  showLoadingState() {
    const modalBody = this.modal.querySelector(".modal-body");
    modalBody.innerHTML = `
      <div style="text-align: center; padding: 3rem; color: var(--text-secondary);">
        <div style="font-size: 2rem; margin-bottom: 1rem;">⏳</div>
        <div>Loading metrics data...</div>
      </div>
    `;
  },

  showErrorState(message) {
    const modalBody = this.modal.querySelector(".modal-body");
    modalBody.innerHTML = `
      <div style="text-align: center; padding: 3rem; color: var(--accent-danger);">
        <div style="font-size: 2rem; margin-bottom: 1rem;">❌</div>
        <div>Error loading metrics: ${message}</div>
        <button onclick="metricsManager.showMetrics()" style="margin-top: 1rem; padding: 0.5rem 1rem; background: var(--accent-primary); color: white; border: none; border-radius: 4px; cursor: pointer;">
          Retry
        </button>
      </div>
    `;
  },

  updateMetricsDisplay(taskLogs) {
    const metrics = this.calculateMetrics(taskLogs);

    const modalBody = this.modal.querySelector(".modal-body");
    modalBody.innerHTML = `
      <div class="metrics-summary">
        <div class="metric-card">
          <h4>Total Tasks</h4>
          <span class="metric-value">${metrics.totalTasks}</span>
        </div>
        <div class="metric-card">
          <h4>Completed</h4>
          <span class="metric-value">${metrics.completedTasks}</span>
        </div>
        <div class="metric-card">
          <h4>Success Rate</h4>
          <span class="metric-value">${metrics.successRate}%</span>
        </div>
        <div class="metric-card">
          <h4>Avg Duration</h4>
          <span class="metric-value">${metrics.avgDuration}s</span>
        </div>
        <div class="metric-card">
          <h4>Active Nodes</h4>
          <span class="metric-value">${metrics.activeNodes}</span>
        </div>
        <div class="metric-card">
          <h4>Last Task</h4>
          <span class="metric-value">${metrics.lastTaskTime}</span>
        </div>
      </div>
      
      <div class="metrics-charts">
        <div class="chart-section">
          <h4>Task Types Distribution</h4>
          <div class="chart-placeholder">
            ${this.renderTaskTypesChart(metrics.taskTypes)}
          </div>
        </div>
        <div class="chart-section">
          <h4>Node Performance</h4>
          <div class="chart-placeholder">
            ${this.renderNodePerformanceChart(metrics.nodeStats)}
          </div>
        </div>
      </div>
      
      <div class="detailed-logs">
        <h4>Recent Task History</h4>
        <div class="logs-table-container">
          <table class="logs-table">
            <thead>
              <tr>
                <th>Task ID</th>
                <th>Type</th>
                <th>Node</th>
                <th>Status</th>
                <th>Duration</th>
                <th>Started</th>
                <th>Completed</th>
              </tr>
            </thead>
            <tbody>
              ${this.renderTaskRows(taskLogs)}
            </tbody>
          </table>
        </div>
      </div>
    `;
  },

  calculateMetrics(taskLogs) {
    const totalTasks = taskLogs.length;
    const completedTasks = taskLogs.filter(
      (task) => task.status === "completed"
    ).length;
    const successRate =
      totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

    // Calculate average duration
    const completedTasksWithDuration = taskLogs.filter(
      (task) => task.status === "completed" && task.duration_ms
    );
    const avgDuration =
      completedTasksWithDuration.length > 0
        ? Math.round(
            (completedTasksWithDuration.reduce(
              (sum, task) => sum + task.duration_ms,
              0
            ) /
              completedTasksWithDuration.length /
              1000) *
              10
          ) / 10
        : 0;

    // Get unique nodes
    const activeNodes = new Set(taskLogs.map((task) => task.node_id)).size;

    // Get last task time
    const lastTask = taskLogs.length > 0 ? taskLogs[taskLogs.length - 1] : null;
    const lastTaskTime = lastTask
      ? new Date(
          lastTask.completed_at || lastTask.started_at
        ).toLocaleTimeString()
      : "Never";

    // Task types distribution
    const taskTypes = {};
    taskLogs.forEach((task) => {
      taskTypes[task.task_type] = (taskTypes[task.task_type] || 0) + 1;
    });

    // Node statistics
    const nodeStats = {};
    taskLogs.forEach((task) => {
      if (!nodeStats[task.node_id]) {
        nodeStats[task.node_id] = { total: 0, completed: 0, totalDuration: 0 };
      }
      nodeStats[task.node_id].total++;
      if (task.status === "completed") {
        nodeStats[task.node_id].completed++;
        if (task.duration_ms) {
          nodeStats[task.node_id].totalDuration += task.duration_ms;
        }
      }
    });

    return {
      totalTasks,
      completedTasks,
      successRate,
      avgDuration,
      activeNodes,
      lastTaskTime,
      taskTypes,
      nodeStats,
    };
  },

  renderTaskTypesChart(taskTypes) {
    const entries = Object.entries(taskTypes);
    if (entries.length === 0) {
      return '<div style="color: var(--text-muted); font-style: italic;">No task data available</div>';
    }

    return entries
      .map(
        ([type, count]) => `
      <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; margin: 0.25rem 0; background: var(--bg-card); border-radius: 4px;">
        <span style="color: var(--text-primary); font-weight: 500;">${type}</span>
        <div style="display: flex; align-items: center; gap: 0.5rem;">
          <div style="width: 100px; height: 8px; background: var(--bg-secondary); border-radius: 4px; overflow: hidden;">
            <div style="height: 100%; background: var(--accent-primary); width: ${Math.min(
              100,
              (count / Math.max(...Object.values(taskTypes))) * 100
            )}%;"></div>
          </div>
          <span style="color: var(--text-secondary); min-width: 30px; text-align: right;">${count}</span>
        </div>
      </div>
    `
      )
      .join("");
  },

  renderNodePerformanceChart(nodeStats) {
    const entries = Object.entries(nodeStats);
    if (entries.length === 0) {
      return '<div style="color: var(--text-muted); font-style: italic;">No node performance data</div>';
    }

    return entries
      .map(([nodeId, stats]) => {
        const successRate =
          stats.total > 0
            ? Math.round((stats.completed / stats.total) * 100)
            : 0;
        const avgDuration =
          stats.completed > 0 && stats.totalDuration > 0
            ? Math.round(stats.totalDuration / stats.completed / 100) / 10
            : 0;

        return `
        <div style="display: flex; flex-direction: column; padding: 0.75rem; margin: 0.5rem 0; background: var(--bg-card); border-radius: 6px; border: 1px solid var(--glass-border);">
          <div style="display: flex; justify-content: between; align-items: center; margin-bottom: 0.5rem;">
            <span style="color: var(--text-primary); font-weight: 600; font-size: 0.9rem;">${nodeId}</span>
            <span style="color: var(--text-secondary); font-size: 0.8rem;">${
              stats.completed
            }/${stats.total} tasks</span>
          </div>
          <div style="display: flex; align-items: center; gap: 0.5rem;">
            <span style="color: var(--text-secondary); font-size: 0.8rem; min-width: 60px;">Success:</span>
            <div style="flex: 1; height: 6px; background: var(--bg-secondary); border-radius: 3px; overflow: hidden;">
              <div style="height: 100%; background: ${
                successRate >= 90
                  ? "var(--accent-success)"
                  : successRate >= 70
                  ? "var(--accent-warning)"
                  : "var(--accent-danger)"
              }; width: ${successRate}%;"></div>
            </div>
            <span style="color: var(--text-primary); font-size: 0.8rem; min-width: 40px; text-align: right; font-weight: 500;">${successRate}%</span>
          </div>
          ${
            avgDuration > 0
              ? `
            <div style="margin-top: 0.25rem; color: var(--text-muted); font-size: 0.7rem;">
              Avg: ${avgDuration}s
            </div>
          `
              : ""
          }
        </div>
      `;
      })
      .join("");
  },

  renderTaskRows(taskLogs) {
    if (taskLogs.length === 0) {
      return `
        <tr>
          <td colspan="7" style="text-align: center; color: var(--text-muted); font-style: italic; padding: 2rem;">
            No task logs found
          </td>
        </tr>
      `;
    }

    // Show most recent tasks first
    return taskLogs
      .slice(-20)
      .reverse()
      .map((task) => {
        const startTime = new Date(task.started_at);
        const completedTime = task.completed_at
          ? new Date(task.completed_at)
          : null;
        const duration = task.duration_ms
          ? `${Math.round(task.duration_ms / 100) / 10}s`
          : "-";

        return `
        <tr>
          <td style="font-family: monospace; font-size: 0.8rem;">${task.task_id.substring(
            0,
            8
          )}...</td>
          <td><span style="font-weight: 500;">${task.task_type}</span></td>
          <td style="font-family: monospace; font-size: 0.8rem;">${
            task.node_id
          }</td>
          <td>
            <span class="status-badge status-${task.status}">
              ${task.status}
            </span>
          </td>
          <td>${duration}</td>
          <td style="font-size: 0.8rem;">${startTime.toLocaleTimeString()}</td>
          <td style="font-size: 0.8rem;">${
            completedTime ? completedTime.toLocaleTimeString() : "-"
          }</td>
        </tr>
      `;
      })
      .join("");
  },
};

// Initialize metrics when DOM is loaded
document.addEventListener("DOMContentLoaded", () => {
  window.metricsManager.init();
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
