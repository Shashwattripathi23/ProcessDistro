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

    /// Add a task to the queue
    pub async fn add_task(&self, task_type: String, payload: serde_json::Value, priority: u8) -> String {
        let task = Task {
            task_id: Uuid::new_v4().to_string(),
            task_type,
            status: TaskStatus::Pending,
            assigned_node: None,
            priority,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            estimated_duration: None,
            actual_duration: None,
            payload,
            result: None,
            error: None,
        };
        
        let task_id = task.task_id.clone();
        
        {
            let mut queue = self.task_queue.write().await;
            queue.push(task);
            // Sort by priority (highest first)
            queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        
        println!("📋 Task added to queue: {} (priority: {})", task_id, priority);
        self.broadcast_network_update().await;
        
        task_id
    }

    /// Assign next task to a node
    pub async fn assign_task_to_node(&self, node_id: &str) -> Option<Task> {
        let mut task = {
            let mut queue = self.task_queue.write().await;
            queue.pop()
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
        
        println!("🎯 Task assigned: {} -> {}", task_id, node_id);
        self.broadcast_network_update().await;
        
        Some(task)
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
}