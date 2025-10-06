/// Controller Manager Module
/// 
/// Central management for distributed computing nodes, task queues, and real-time state tracking
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, Mutex};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::web_server::{NodeInfo, WsMessage};

/// Task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub task_type: String, // "compute", "benchmark", "wasm_execution"
    pub status: TaskStatus,
    pub assigned_node: Option<String>,
    pub priority: u8, // 1-10, 10 being highest
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub estimated_duration: Option<Duration>,
    pub actual_duration: Option<Duration>,
    pub payload: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Node connection tracking
#[derive(Debug, Clone)]
pub struct NodeConnection {
    pub node_id: String,
    pub websocket_id: Option<String>,
    pub last_heartbeat: DateTime<Utc>,
    pub connection_count: u32,
    pub is_active: bool,
}

/// Node statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStats {
    pub total_tasks_assigned: u64,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub average_task_duration: f64, // seconds
    pub uptime_seconds: u64,
    pub last_seen: DateTime<Utc>,
    pub connection_count: u32,
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub idle_nodes: usize,
    pub busy_nodes: usize,
    pub offline_nodes: usize,
    pub total_pending_tasks: usize,
    pub total_running_tasks: usize,
    pub total_completed_tasks: usize,
    pub total_failed_tasks: usize,
    pub network_uptime_seconds: u64,
    pub tasks_per_minute: f64,
}

/// Main controller manager
pub struct ControllerManager {
    /// Node registry with real-time state
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
    
    /// Node connection tracking
    connections: Arc<RwLock<HashMap<String, NodeConnection>>>,
    
    /// Node statistics
    node_stats: Arc<RwLock<HashMap<String, NodeStats>>>,
    
    /// Task queue (pending tasks)
    task_queue: Arc<RwLock<Vec<Task>>>,
    
    /// Running tasks (assigned to nodes)
    running_tasks: Arc<RwLock<HashMap<String, Task>>>,
    
    /// Completed tasks (for statistics)
    completed_tasks: Arc<RwLock<Vec<Task>>>,
    
    /// WebSocket broadcast channel
    broadcast_tx: broadcast::Sender<WsMessage>,
    
    /// Controller start time
    start_time: DateTime<Utc>,
    
    /// Cleanup task handle
    cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl ControllerManager {
    /// Create a new controller manager
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            node_stats: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(Vec::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(Vec::new())),
            broadcast_tx,
            start_time: Utc::now(),
            cleanup_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the controller manager with periodic cleanup
    pub async fn start(&self) {
        self.start_cleanup_task().await;
        println!("🚀 Controller Manager started");
    }

    /// Stop the controller manager
    pub async fn stop(&self) {
        if let Some(handle) = self.cleanup_handle.lock().await.take() {
            handle.abort();
        }
        println!("🛑 Controller Manager stopped");
    }

    /// Get broadcast sender for WebSocket integration
    pub fn get_broadcast_sender(&self) -> broadcast::Sender<WsMessage> {
        self.broadcast_tx.clone()
    }

    /// Register a new node
    pub async fn register_node(&self, node_info: NodeInfo, websocket_id: Option<String>) -> Result<(), String> {
        let node_id = node_info.node_id.clone();
        
        // Add/update node
        {
            let mut nodes = self.nodes.write().await;
            nodes.insert(node_id.clone(), node_info.clone());
        }
        
        // Track connection
        {
            let mut connections = self.connections.write().await;
            let connection = connections.entry(node_id.clone()).or_insert(NodeConnection {
                node_id: node_id.clone(),
                websocket_id: websocket_id.clone(),
                last_heartbeat: Utc::now(),
                connection_count: 0,
                is_active: true,
            });
            connection.connection_count += 1;
            connection.last_heartbeat = Utc::now();
            connection.is_active = true;
            connection.websocket_id = websocket_id;
        }
        
        // Initialize or update statistics
        {
            let mut stats = self.node_stats.write().await;
            stats.entry(node_id.clone()).or_insert(NodeStats {
                total_tasks_assigned: 0,
                total_tasks_completed: 0,
                total_tasks_failed: 0,
                average_task_duration: 0.0,
                uptime_seconds: 0,
                last_seen: Utc::now(),
                connection_count: 1,
            }).last_seen = Utc::now();
        }
        
        println!("✅ Node registered: {} ({})", node_info.capabilities.device_name, node_id);
        
        // Broadcast node added event
        let _ = self.broadcast_tx.send(WsMessage::NodeAdded { node: node_info });
        self.broadcast_network_update().await;
        
        Ok(())
    }

    /// Unregister a node (explicit unregistration)
    pub async fn unregister_node(&self, node_id: &str) -> Result<bool, String> {
        let removed = {
            let mut nodes = self.nodes.write().await;
            nodes.remove(node_id).is_some()
        };
        
        if removed {
            // Mark connection as inactive
            {
                let mut connections = self.connections.write().await;
                if let Some(connection) = connections.get_mut(node_id) {
                    connection.is_active = false;
                    connection.websocket_id = None;
                }
            }
            
            // Cancel any running tasks for this node
            self.cancel_node_tasks(node_id).await;
            
            println!("❌ Node unregistered: {}", node_id);
            
            // Broadcast node removed event
            let _ = self.broadcast_tx.send(WsMessage::NodeRemoved { 
                node_id: node_id.to_string() 
            });
            self.broadcast_network_update().await;
        }
        
        Ok(removed)
    }

    /// Handle node disconnection (WebSocket close, page reload, etc.)
    pub async fn handle_node_disconnect(&self, node_id: &str, websocket_id: Option<&str>) {
        let should_remove = {
            let mut connections = self.connections.write().await;
            if let Some(connection) = connections.get_mut(node_id) {
                // Only remove if this is the current WebSocket connection
                if websocket_id.is_none() || connection.websocket_id.as_deref() == websocket_id {
                    connection.is_active = false;
                    connection.websocket_id = None;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if should_remove {
            // Remove from active nodes
            {
                let mut nodes = self.nodes.write().await;
                nodes.remove(node_id);
            }
            
            // Cancel running tasks
            self.cancel_node_tasks(node_id).await;
            
            println!("🔌 Node disconnected: {}", node_id);
            
            // Broadcast node removed event
            let _ = self.broadcast_tx.send(WsMessage::NodeRemoved { 
                node_id: node_id.to_string() 
            });
            self.broadcast_network_update().await;
        }
    }

    /// Update node heartbeat
    pub async fn update_heartbeat(&self, node_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(node_id) {
            connection.last_heartbeat = Utc::now();
        }
        
        // Update node last_seen
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.last_seen = Utc::now();
        }
    }

    /// Update a node's benchmark score
    pub async fn update_benchmark_score(&self, node_id: &str, score: f64) -> Result<(), String> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.capabilities.benchmark_score = Some(score);
            node.last_seen = chrono::Utc::now();
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    /// Get all active nodes
    pub async fn get_active_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// Add a task to the queue with enhanced distribution logic
    pub async fn add_task(&self, task_type: String, payload: serde_json::Value, priority: u8) -> String {
        let task = Task {
            task_id: Uuid::new_v4().to_string(),
            task_type: task_type.clone(),
            status: TaskStatus::Pending,
            assigned_node: None,
            priority,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            estimated_duration: self.estimate_task_duration(&task_type, &payload),
            actual_duration: None,
            payload,
            result: None,
            error: None,
        };
        
        let task_id = task.task_id.clone();
        
        {
            let mut queue = self.task_queue.write().await;
            queue.push(task);
            // Enhanced sorting: priority first, then by estimated complexity for better distribution
            queue.sort_by(|a, b| {
                b.priority.cmp(&a.priority)
                    .then_with(|| {
                        // Prefer compute-intensive tasks for high-performance nodes
                        let a_compute_intensive = self.is_compute_intensive_task(&a.task_type);
                        let b_compute_intensive = self.is_compute_intensive_task(&b.task_type);
                        b_compute_intensive.cmp(&a_compute_intensive)
                    })
            });
        }
        
        println!("📋 Enhanced task added to queue: {} (type: {}, priority: {})", task_id, task_type, priority);
        self.broadcast_network_update().await;
        
        task_id
    }

    /// Estimate task duration based on type and payload for better scheduling
    fn estimate_task_duration(&self, task_type: &str, payload: &serde_json::Value) -> Option<Duration> {
        match task_type {
            "mandelbrot" => {
                let width = payload.get("width").and_then(|v| v.as_u64()).unwrap_or(800);
                let height = payload.get("height").and_then(|v| v.as_u64()).unwrap_or(600);
                let max_iterations = payload.get("max_iterations").and_then(|v| v.as_u64()).unwrap_or(100);
                
                // Estimate based on complexity
                let complexity_factor = (width * height * max_iterations) / 1_000_000;
                Some(Duration::from_secs(5 + complexity_factor.min(60)))
            },
            "password_hash" => {
                let workload = payload.get("workload").and_then(|v| v.as_u64()).unwrap_or(10000);
                Some(Duration::from_secs(workload / 2000 + 3))
            },
            "matrix_mul" => {
                let matrix_size = payload.get("matrix_size").and_then(|v| v.as_u64()).unwrap_or(256);
                let complexity = (matrix_size * matrix_size * matrix_size) / 1_000_000;
                Some(Duration::from_secs(complexity + 2))
            },
            _ => Some(Duration::from_secs(30)),
        }
    }

    /// Check if a task type is compute-intensive for better node assignment
    fn is_compute_intensive_task(&self, task_type: &str) -> bool {
        matches!(task_type, "mandelbrot" | "matrix_mul" | "password_hash")
    }

    /// Assign next task to a node with intelligent selection based on capabilities
    pub async fn assign_task_to_node(&self, node_id: &str) -> Option<Task> {
        // Get node info for capability-aware assignment
        let node_info = {
            let nodes = self.nodes.read().await;
            nodes.get(node_id).cloned()
        };
        
        let mut task = {
            let mut queue = self.task_queue.write().await;
            
            // Find the best task for this node based on its capabilities
            if let Some(node) = &node_info {
                // Look for a task that matches node capabilities
                let best_task_index = self.find_best_task_for_node(&*queue, node);
                if let Some(index) = best_task_index {
                    Some(queue.remove(index))
                } else {
                    queue.pop() // Fallback to any available task
                }
            } else {
                queue.pop() // If no node info, just get any task
            }
        }?;
        
        task.assigned_node = Some(node_id.to_string());
        task.status = TaskStatus::Assigned;
        task.started_at = Some(Utc::now());
        
        let task_id = task.task_id.clone();
        
        {
            let mut running = self.running_tasks.write().await;
            running.insert(task_id.clone(), task.clone());
        }
        
        // Update node stats
        {
            let mut stats = self.node_stats.write().await;
            if let Some(node_stats) = stats.get_mut(node_id) {
                node_stats.total_tasks_assigned += 1;
            }
        }
        
        println!("🎯 Intelligent task assignment: {} -> {} (type: {})", 
                 task_id, node_id, task.task_type);
        self.broadcast_network_update().await;
        
        Some(task)
    }

    /// Find the best task for a node based on its capabilities
    fn find_best_task_for_node(&self, tasks: &[Task], node_info: &NodeInfo) -> Option<usize> {
        let mut best_task_index = None;
        let mut best_score = 0.0;
        
        for (index, task) in tasks.iter().enumerate() {
            let score = self.calculate_task_node_affinity(task, node_info);
            if score > best_score {
                best_score = score;
                best_task_index = Some(index);
            }
        }
        
        best_task_index
    }

    /// Calculate affinity score between task and node (higher is better)
    fn calculate_task_node_affinity(&self, task: &Task, node_info: &NodeInfo) -> f64 {
        let mut score = 0.0;
        
        // Prefer nodes with higher benchmark scores for compute-intensive tasks
        if self.is_compute_intensive_task(&task.task_type) {
            if let Some(benchmark_score) = node_info.capabilities.benchmark_score {
                score += benchmark_score / 1000.0; // Normalize
            }
        }
        
        // Prefer nodes with more CPU cores for demanding tasks
        score += (node_info.capabilities.cpu_cores as f64) / 4.0; // Normalize assuming max 4 cores
        
        // Prefer nodes with more available memory
        let available_memory_gb = node_info.capabilities.available_memory as f64 / (1024.0 * 1024.0 * 1024.0);
        score += available_memory_gb;
        
        // Add priority bonus
        score += (task.priority as f64) / 10.0;
        
        score
    }

    /// Complete a task
    pub async fn complete_task(&self, task_id: &str, result: Option<serde_json::Value>) -> Result<(), String> {
        let mut task = {
            let mut running = self.running_tasks.write().await;
            running.remove(task_id).ok_or("Task not found")?
        };
        
        task.status = TaskStatus::Completed;
        task.completed_at = Some(Utc::now());
        task.result = result;
        
        if let (Some(started), Some(completed)) = (task.started_at, task.completed_at) {
            task.actual_duration = Some(Duration::from_secs(
                (completed.timestamp() - started.timestamp()) as u64
            ));
        }
        
        // Update node stats
        if let Some(node_id) = &task.assigned_node {
            let mut stats = self.node_stats.write().await;
            if let Some(node_stats) = stats.get_mut(node_id) {
                node_stats.total_tasks_completed += 1;
                
                // Update average duration
                if let Some(duration) = task.actual_duration {
                    let new_avg = (node_stats.average_task_duration * (node_stats.total_tasks_completed - 1) as f64 
                                  + duration.as_secs_f64()) / node_stats.total_tasks_completed as f64;
                    node_stats.average_task_duration = new_avg;
                }
            }
        }
        
        {
            let mut completed = self.completed_tasks.write().await;
            completed.push(task);
        }
        
        println!("✅ Task completed: {}", task_id);
        self.broadcast_network_update().await;
        
        Ok(())
    }

    /// Fail a task
    pub async fn fail_task(&self, task_id: &str, error: String) -> Result<(), String> {
        let mut task = {
            let mut running = self.running_tasks.write().await;
            running.remove(task_id).ok_or("Task not found")?
        };
        
        task.status = TaskStatus::Failed;
        task.completed_at = Some(Utc::now());
        task.error = Some(error);
        
        // Update node stats
        if let Some(node_id) = &task.assigned_node {
            let mut stats = self.node_stats.write().await;
            if let Some(node_stats) = stats.get_mut(node_id) {
                node_stats.total_tasks_failed += 1;
            }
        }
        
        {
            let mut completed = self.completed_tasks.write().await;
            completed.push(task);
        }
        
        println!("❌ Task failed: {}", task_id);
        self.broadcast_network_update().await;
        
        Ok(())
    }

    /// Cancel all tasks for a node
    async fn cancel_node_tasks(&self, node_id: &str) {
        let cancelled_tasks: Vec<Task> = {
            let mut running = self.running_tasks.write().await;
            let mut cancelled = Vec::new();
            
            running.retain(|_, task| {
                if task.assigned_node.as_deref() == Some(node_id) {
                    let mut cancelled_task = task.clone();
                    cancelled_task.status = TaskStatus::Cancelled;
                    cancelled_task.completed_at = Some(Utc::now());
                    cancelled.push(cancelled_task);
                    false
                } else {
                    true
                }
            });
            
            cancelled
        };
        
        if !cancelled_tasks.is_empty() {
            println!("🚫 Cancelled {} tasks for node: {}", cancelled_tasks.len(), node_id);
            
            {
                let mut completed = self.completed_tasks.write().await;
                completed.extend(cancelled_tasks);
            }
            
            self.broadcast_network_update().await;
        }
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> NetworkStats {
        let nodes = self.nodes.read().await;
        let queue = self.task_queue.read().await;
        let running = self.running_tasks.read().await;
        let completed = self.completed_tasks.read().await;
        
        let active_nodes = nodes.values().filter(|n| n.status == "active").count();
        let idle_nodes = nodes.values().filter(|n| n.status == "idle").count();
        let busy_nodes = nodes.values().filter(|n| n.status == "busy").count();
        let offline_nodes = nodes.values().filter(|n| n.status == "offline").count();
        
        let completed_tasks = completed.len();
        let failed_tasks = completed.iter().filter(|t| t.status == TaskStatus::Failed).count();
        
        let uptime = (Utc::now().timestamp() - self.start_time.timestamp()) as u64;
        let tasks_per_minute = if uptime > 0 {
            (completed_tasks as f64) / (uptime as f64 / 60.0)
        } else {
            0.0
        };
        
        NetworkStats {
            total_nodes: nodes.len(),
            active_nodes,
            idle_nodes,
            busy_nodes,
            offline_nodes,
            total_pending_tasks: queue.len(),
            total_running_tasks: running.len(),
            total_completed_tasks: completed_tasks,
            total_failed_tasks: failed_tasks,
            network_uptime_seconds: uptime,
            tasks_per_minute,
        }
    }

    /// Broadcast network update to all connected clients
    async fn broadcast_network_update(&self) {
        let nodes = self.get_active_nodes().await;
        let stats = self.get_network_stats().await;
        
        // Broadcast node updates
        let _ = self.broadcast_tx.send(WsMessage::NodeUpdate { nodes: nodes.clone() });
        
        // Calculate and broadcast metrics
        let total_cores: u32 = nodes.iter().map(|n| n.capabilities.cpu_cores).sum();
        let total_memory: u64 = nodes.iter().map(|n| n.capabilities.total_memory).sum();
        let benchmarks: Vec<f64> = nodes.iter()
            .filter_map(|n| n.capabilities.benchmark_score)
            .collect();
        let avg_benchmark = if benchmarks.is_empty() { 
            0.0 
        } else { 
            benchmarks.iter().sum::<f64>() / benchmarks.len() as f64 
        };

        let _ = self.broadcast_tx.send(WsMessage::MetricsUpdate {
            total_nodes: stats.total_nodes,
            active_tasks: stats.total_running_tasks as u64,
            total_cores,
            total_memory,
            avg_benchmark,
        });
    }

    /// Start cleanup task for dead connections
    async fn start_cleanup_task(&self) {
        let connections = self.connections.clone();
        let nodes = self.nodes.clone();
        let broadcast_tx = self.broadcast_tx.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let mut disconnected_nodes = Vec::new();
                let now = Utc::now();
                
                // Check for dead connections (no heartbeat for 60 seconds)
                {
                    let connections_guard = connections.read().await;
                    for (node_id, connection) in connections_guard.iter() {
                        if connection.is_active {
                            let time_since_heartbeat = now.timestamp() - connection.last_heartbeat.timestamp();
                            if time_since_heartbeat > 60 {
                                disconnected_nodes.push(node_id.clone());
                            }
                        }
                    }
                }
                
                // Remove dead connections
                for node_id in disconnected_nodes {
                    {
                        let mut nodes_guard = nodes.write().await;
                        nodes_guard.remove(&node_id);
                    }
                    
                    {
                        let mut connections_guard = connections.write().await;
                        if let Some(connection) = connections_guard.get_mut(&node_id) {
                            connection.is_active = false;
                            connection.websocket_id = None;
                        }
                    }
                    
                    println!("🧹 Cleaned up dead connection: {}", node_id);
                    
                    // Broadcast node removed
                    let _ = broadcast_tx.send(WsMessage::NodeRemoved { node_id });
                }
            }
        });
        
        *self.cleanup_handle.lock().await = Some(handle);
    }

    /// Update node status 
    pub async fn update_node_status(&self, node_id: &str, status: &str) {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.status = status.to_string();
            
            // Broadcast node update
            let _ = self.broadcast_tx.send(WsMessage::NodeUpdate { 
                nodes: nodes.values().cloned().collect() 
            });
        }
    }

    /// Run a Mandelbrot fractal rendering test on all nodes with intelligent distribution
    pub async fn run_mandelbrot_test(&self, width: u32, height: u32, max_iterations: u32) -> Vec<String> {
        let active_nodes = self.get_active_nodes().await;
        let mut task_ids = Vec::new();

        if active_nodes.is_empty() {
            println!("❌ No active nodes available for Mandelbrot test");
            return task_ids;
        }

        println!("🎨 Debug: Starting distributed Mandelbrot test with {} active nodes", active_nodes.len());
        println!("📐 Canvas dimensions: {}x{}, Max iterations: {}", width, height, max_iterations);

        // Calculate optimal tile distribution based on node capabilities
        let total_nodes = active_nodes.len();
        let tiles_per_dimension = (total_nodes as f64).sqrt().ceil() as u32;
        let tile_width = width / tiles_per_dimension;
        let tile_height = height / tiles_per_dimension;

        println!("🧩 Distributing {} tiles across {} nodes ({}x{} tiles of {}x{} pixels each)", 
                 tiles_per_dimension * tiles_per_dimension, total_nodes, 
                 tiles_per_dimension, tiles_per_dimension, tile_width, tile_height);

        // Sort nodes by performance capability for optimal task assignment
        let mut sorted_nodes = active_nodes.clone();
        sorted_nodes.sort_by(|a, b| {
            // First by benchmark score (higher is better)
            b.capabilities.benchmark_score.partial_cmp(&a.capabilities.benchmark_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    // Then by CPU cores (more cores handle complex tiles better)
                    b.capabilities.cpu_cores.cmp(&a.capabilities.cpu_cores)
                })
        });

        // Check current node loads for intelligent distribution
        let running_tasks = self.running_tasks.read().await;
        let mut node_loads: Vec<(NodeInfo, u32)> = Vec::new();

        for node in sorted_nodes {
            let current_load = running_tasks.values()
                .filter(|task| task.assigned_node.as_ref() == Some(&node.node_id))
                .count() as u32;
            node_loads.push((node, current_load));
        }

        // Sort by load (least loaded first) while maintaining performance preference
        node_loads.sort_by(|a, b| {
            a.1.cmp(&b.1).then_with(|| {
                b.0.capabilities.benchmark_score.partial_cmp(&a.0.capabilities.benchmark_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        drop(running_tasks);

        // Create Mandelbrot tasks with intelligent tile assignment
        let mut tile_index = 0;
        for tile_y in 0..tiles_per_dimension {
            for tile_x in 0..tiles_per_dimension {
                // Find next available node (skip overloaded ones)
                let mut attempts = 0;
                let (node, current_load) = loop {
                    if attempts >= node_loads.len() {
                        // If all nodes are overloaded, use the least loaded one
                        println!("⚠️ All nodes overloaded, using least loaded node");
                        let min_load_item = node_loads.iter().min_by_key(|(_, load)| *load).unwrap();
                        break (min_load_item.0.clone(), min_load_item.1.clone());
                    }
                    
                    let current_tile_index = tile_index % node_loads.len();
                    let (node, current_load) = &node_loads[current_tile_index];
                    
                    if *current_load < node.capabilities.cpu_cores * 2 {
                        break (node.clone(), *current_load);
                    }
                    
                    println!("⚠️ Skipping overloaded node: {} (load: {})", node.capabilities.device_name, current_load);
                    tile_index += 1;
                    attempts += 1;
                };
                
                // Ensure unique tile assignment
                println!("🎯 Assigning tile ({}, {}) to node: {} ({})", 
                         tile_x, tile_y, node.capabilities.device_name, node.node_id);

                let actual_tile_width = if tile_x == tiles_per_dimension - 1 { 
                    width - tile_x * tile_width 
                } else { 
                    tile_width 
                };
                let actual_tile_height = if tile_y == tiles_per_dimension - 1 { 
                    height - tile_y * tile_height 
                } else { 
                    tile_height 
                };

                // Calculate complexity-adjusted parameters for this node
                let node_performance_factor = node.capabilities.benchmark_score.unwrap_or(100.0) / 100.0;
                let adjusted_max_iterations = (max_iterations as f64 * node_performance_factor.min(2.0)) as u32;

                // Calculate the complex plane bounds for this specific tile
                let real_range = 1.0 - (-2.5); // 3.5 total range
                let imag_range = 1.25 - (-1.25); // 2.5 total range
                
                let tile_min_real = -2.5 + (tile_x as f64 * tile_width as f64 / width as f64) * real_range;
                let tile_max_real = -2.5 + ((tile_x + 1) as f64 * tile_width as f64 / width as f64) * real_range;
                let tile_min_imag = -1.25 + (tile_y as f64 * tile_height as f64 / height as f64) * imag_range;
                let tile_max_imag = -1.25 + ((tile_y + 1) as f64 * tile_height as f64 / height as f64) * imag_range;

                println!("🖼️ Creating optimized Mandelbrot tile for node: {} ({})", 
                         node.capabilities.device_name, node.node_id);
                println!("   📍 Tile position: ({}, {}), Size: {}x{}, Iterations: {}", 
                         tile_x, tile_y, actual_tile_width, actual_tile_height, adjusted_max_iterations);
                println!("   🎯 Complex bounds: real[{:.3}, {:.3}], imag[{:.3}, {:.3}]", 
                         tile_min_real, tile_max_real, tile_min_imag, tile_max_imag);

                let payload = serde_json::json!({
                    "test": "mandelbrot_distributed",
                    "width": actual_tile_width,     // ✅ Only render the tile size
                    "height": actual_tile_height,   // ✅ Only render the tile size
                    "full_canvas_width": width,     // 📐 For reference/positioning
                    "full_canvas_height": height,   // 📐 For reference/positioning
                    "tile_x": tile_x * tile_width,
                    "tile_y": tile_y * tile_height,
                    "tile_width": actual_tile_width,
                    "tile_height": actual_tile_height,
                    "min_real": tile_min_real,      // 🎯 Tile-specific complex bounds
                    "max_real": tile_max_real,      // 🎯 Tile-specific complex bounds
                    "min_imag": tile_min_imag,      // 🎯 Tile-specific complex bounds
                    "max_imag": tile_max_imag,      // 🎯 Tile-specific complex bounds
                    "max_iterations": adjusted_max_iterations,
                    "enable_progress": true,
                    "estimated_duration": self.estimate_mandelbrot_duration(actual_tile_width, actual_tile_height, adjusted_max_iterations, node_performance_factor),
                    "canvas_rendering": true,
                    "node_performance_factor": node_performance_factor,
                    "tile_id": format!("tile_{}_{}", tile_x, tile_y),
                    "tile_index_x": tile_x,         // 📍 For dashboard positioning
                    "tile_index_y": tile_y          // 📍 For dashboard positioning
                });

                // Calculate dynamic priority based on tile complexity and node capability
                let complexity_score = (actual_tile_width * actual_tile_height * adjusted_max_iterations) as f64;
                let priority = if complexity_score > 50_000_000.0 { 8 } 
                              else if complexity_score > 10_000_000.0 { 6 } 
                              else { 5 };

                let task_id = self.add_task("mandelbrot".to_string(), payload.clone(), priority).await;
                task_ids.push(task_id.clone());

                // 🎯 CRITICAL: Directly assign this task to the specific node we calculated
                // This prevents other nodes from getting the same tile
                {
                    let mut task_queue = self.task_queue.write().await;
                    let mut running_tasks = self.running_tasks.write().await;
                    
                    // Find and remove the task from the queue
                    if let Some(pos) = task_queue.iter().position(|t| t.task_id == task_id) {
                        let mut task = task_queue.remove(pos);
                        
                        // Update task to running state with specific node assignment
                        task.status = TaskStatus::Running;
                        task.assigned_node = Some(node.node_id.clone());
                        task.started_at = Some(chrono::Utc::now());
                        
                        running_tasks.insert(task_id.clone(), task.clone());
                        
                        println!("✅ Directly assigned task {} to node: {} (tile: {}, {})", 
                                task_id, node.node_id, tile_x, tile_y);
                        
                        // Broadcast the task directly to the specific node
                        let _ = self.broadcast_tx.send(WsMessage::ExecuteTask {
                            task_id: task.task_id.clone(),
                            task_type: task.task_type.clone(),
                            payload: task.payload.clone(),
                            assigned_node: node.node_id.clone(),
                        });
                    }
                }

                tile_index += 1; // Move to next node for next tile
            }
        }

        println!("📊 Total Mandelbrot tasks created and assigned: {}", task_ids.len());

        println!("📊 Total Mandelbrot tasks created: {}", task_ids.len());
        println!("🎨 Broadcasting network update...");
        self.broadcast_network_update().await;
        
        task_ids
    }

    /// Estimate Mandelbrot rendering duration based on complexity and node performance
    fn estimate_mandelbrot_duration(&self, width: u32, height: u32, iterations: u32, performance_factor: f64) -> u32 {
        let base_complexity = (width * height * iterations) as f64;
        let adjusted_duration = (base_complexity / 1_000_000.0) / performance_factor.max(0.1);
        (adjusted_duration.max(2.0).min(120.0)) as u32 // Between 2 and 120 seconds
    }

    /// Find the best available node based on current load and performance
    async fn find_best_available_node(&self) -> Option<NodeInfo> {
        let active_nodes = self.get_active_nodes().await;
        if active_nodes.is_empty() {
            return None;
        }

        let running_tasks = self.running_tasks.read().await;
        let mut best_node: Option<NodeInfo> = None;
        let mut best_score = f64::NEG_INFINITY;

        for node in active_nodes {
            let current_load = running_tasks.values()
                .filter(|task| task.assigned_node.as_ref() == Some(&node.node_id))
                .count() as u32;

            // Skip if node is at capacity
            if current_load >= node.capabilities.cpu_cores * 2 {
                continue;
            }

            // Calculate node score: performance / load ratio
            let performance_score = node.capabilities.benchmark_score.unwrap_or(100.0);
            let load_factor = 1.0 / (current_load as f64 + 1.0);
            let node_score = performance_score * load_factor;

            if node_score > best_score {
                best_score = node_score;
                best_node = Some(node);
            }
        }

        best_node
    }

    /// Run a password hashing test on all nodes with enhanced monitoring
    pub async fn run_password_hash_test(&self) -> Vec<String> {
        let nodes = self.get_active_nodes().await;
        let mut task_ids = Vec::new();

        println!("🔍 Debug: Starting password hash test with {} active nodes", nodes.len());

        for node in nodes.iter() {
            println!("🎯 Creating task for node: {} ({})", node.capabilities.device_name, node.node_id);
            
            let payload = serde_json::json!({
                "test": "password_hash",
                "workload": 50000, // Increased workload for better monitoring
                "algorithm": "bcrypt",
                "cost": 10, // Moderate cost for visible duration
                "batch_size": 1000, // Process in batches for progress updates
                "estimated_duration": 60, // Estimate 60 seconds
                "enable_progress": true
            });

            let task_id = self.add_task("password_hash".to_string(), payload.clone(), 5).await;
            task_ids.push(task_id.clone());

            println!("📋 Task created: {} for node: {}", task_id, node.node_id);

            // Broadcast execute_task to nodes (they will filter by assigned node)
            let broadcast_result = self.broadcast_tx.send(WsMessage::ExecuteTask {
                task_id: task_id.clone(),
                task_type: "password_hash".to_string(),
                payload: payload.clone(),
                assigned_node: node.node_id.clone(),
            });

            match broadcast_result {
                Ok(_) => {
                    println!("✅ WebSocket message sent successfully for task: {}", task_id);
                }
                Err(e) => {
                    println!("❌ Failed to send WebSocket message for task {}: {:?}", task_id, e);
                }
            }
        }

        println!("📊 Total tasks dispatched: {}", task_ids.len());
        println!("🔔 Broadcasting network update...");
        self.broadcast_network_update().await;
        
        task_ids
    }

    /// Distribute tasks intelligently based on node capabilities and load
    pub async fn distribute_tasks_intelligently(&self) -> Result<usize, String> {
        let active_nodes = self.get_active_nodes().await;
        if active_nodes.is_empty() {
            return Err("No active nodes available for task distribution".to_string());
        }

        let pending_tasks = {
            let queue = self.task_queue.read().await;
            queue.len()
        };

        if pending_tasks == 0 {
            return Ok(0);
        }

        let mut distributed_count = 0;

        // Calculate current load for each node
        let running_tasks = self.running_tasks.read().await;
        let mut node_loads: Vec<(NodeInfo, u32)> = Vec::new();

        for node in active_nodes {
            let current_load = running_tasks.values()
                .filter(|task| task.assigned_node.as_ref() == Some(&node.node_id))
                .count() as u32;
            node_loads.push((node, current_load));
        }

        // Sort by load and performance for intelligent distribution
        node_loads.sort_by(|a, b| {
            // First by load (least loaded first)
            a.1.cmp(&b.1)
                .then_with(|| {
                    // Then by performance (higher benchmark score first)
                    b.0.capabilities.benchmark_score.partial_cmp(&a.0.capabilities.benchmark_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        // Drop the running_tasks guard to avoid deadlock
        drop(running_tasks);

        // Distribute tasks to nodes in order of preference
        let node_count = node_loads.len();
        for (node, current_load) in node_loads {
            // Don't overload nodes - limit based on CPU cores
            if current_load < node.capabilities.cpu_cores {
                if let Some(task) = self.assign_task_to_node(&node.node_id).await {
                    distributed_count += 1;

                    // Broadcast the task
                    let _ = self.broadcast_tx.send(WsMessage::ExecuteTask {
                        task_id: task.task_id.clone(),
                        task_type: task.task_type.clone(),
                        payload: task.payload.clone(),
                        assigned_node: node.node_id.clone(),
                    });

                    // Check if we have more tasks to distribute
                    let remaining_tasks = {
                        let queue = self.task_queue.read().await;
                        queue.len()
                    };
                    if remaining_tasks == 0 {
                        break;
                    }
                }
            }
        }

        if distributed_count > 0 {
            println!("🎯 Intelligently distributed {} tasks across {} nodes", 
                     distributed_count, node_count);
        }

        Ok(distributed_count)
    }

    /// Get enhanced network statistics with load balancing metrics
    pub async fn get_enhanced_network_stats(&self) -> EnhancedNetworkStats {
        let base_stats = self.get_network_stats().await;
        let nodes = self.nodes.read().await;
        let running_tasks = self.running_tasks.read().await;
        let completed_tasks = self.completed_tasks.read().await;

        // Calculate load balance efficiency
        let mut node_task_counts: Vec<u32> = Vec::new();
        for node_id in nodes.keys() {
            let task_count = running_tasks.values()
                .filter(|task| task.assigned_node.as_ref() == Some(node_id))
                .count() as u32;
            node_task_counts.push(task_count);
        }

        let load_balance_efficiency = if node_task_counts.len() > 1 {
            let avg_load = node_task_counts.iter().sum::<u32>() as f64 / node_task_counts.len() as f64;
            let variance = node_task_counts.iter()
                .map(|&x| (x as f64 - avg_load).powi(2))
                .sum::<f64>() / node_task_counts.len() as f64;
            let std_dev = variance.sqrt();

            // Efficiency is higher when standard deviation is lower
            if avg_load > 0.0 {
                1.0 - (std_dev / (avg_load + 1.0)).min(1.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Calculate average completion time
        let average_completion_time = if !completed_tasks.is_empty() {
            completed_tasks.iter()
                .filter_map(|task| task.actual_duration)
                .map(|d| d.as_secs_f64())
                .sum::<f64>() / completed_tasks.len() as f64
        } else {
            0.0
        };

        // Calculate throughput
        let completed_in_last_minute = completed_tasks.iter()
            .filter(|task| {
                if let Some(completed_at) = task.completed_at {
                    (Utc::now() - completed_at).num_seconds() <= 60
                } else {
                    false
                }
            })
            .count();

        let throughput = completed_in_last_minute as f64 / 60.0;
        
        let node_utilization = if base_stats.total_nodes > 0 { 
            base_stats.busy_nodes as f64 / base_stats.total_nodes as f64 
        } else { 
            0.0 
        };

        EnhancedNetworkStats {
            base_stats,
            load_balance_efficiency,
            average_task_completion_time: average_completion_time,
            throughput_tasks_per_second: throughput,
            node_utilization,
        }
    }
}

/// Enhanced network statistics with distribution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedNetworkStats {
    #[serde(flatten)]
    pub base_stats: NetworkStats,
    pub load_balance_efficiency: f64,
    pub average_task_completion_time: f64,
    pub throughput_tasks_per_second: f64,
    pub node_utilization: f64,
}