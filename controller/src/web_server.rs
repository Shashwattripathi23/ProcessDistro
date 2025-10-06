/// Web Server Module
/// 
/// Provides HTTP server for device discovery and node management interface
use axum::{
    extract::{Json, State, WebSocketUpgrade, ws::{WebSocket, Message}},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::controller_manager::ControllerManager;

/// Device capabilities reported from web interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String, // "desktop", "laptop", "mobile", "tablet"
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub cpu_frequency: f64, // GHz
    pub total_memory: u64,  // MB
    pub available_memory: u64, // MB
    pub gpu_info: Option<String>,
    pub browser_info: String,
    pub platform: String,
    pub user_agent: String,
    pub benchmark_score: Option<f64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Node information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub node_type: String, // "controller", "edge_node", "web_client"
    pub status: String,    // "active", "idle", "busy", "offline"
    pub capabilities: DeviceCapabilities,
    pub tasks_completed: u64,
    pub uptime: u64, // seconds
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "node_update")]
    NodeUpdate { nodes: Vec<NodeInfo> },
    #[serde(rename = "node_added")]
    NodeAdded { node: NodeInfo },
    #[serde(rename = "node_removed")]
    NodeRemoved { node_id: String },
    #[serde(rename = "metrics_update")]
    MetricsUpdate {
        total_nodes: usize,
        active_tasks: u64,
        total_cores: u32,
        total_memory: u64,
        avg_benchmark: f64,
    },
    #[serde(rename = "execute_task")]
    ExecuteTask {
        task_id: String,
        task_type: String,
        payload: serde_json::Value,
        assigned_node: String,
    },
    #[serde(rename = "canvas_completed")]
    CanvasCompleted {
        node_id: String,
        completed_at: String,
        status: String,
    },
    #[serde(rename = "canvas_progress")]
    CanvasProgress {
        node_id: String,
        progress: u32,
        timestamp: String,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat,
}

/// Task completion log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLog {
    pub task_id: String,
    pub task_type: String,
    pub node_id: String,
    pub device_name: String,
    pub device_type: String,
    pub platform: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub benchmark_score: Option<u32>,
    pub status: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_ms: u64,
    pub duration_seconds: u64,
    pub timestamp: String,
    pub success: bool,
}

/// Web server state
#[derive(Clone)]
pub struct WebServerState {
    pub controller: Arc<ControllerManager>,
}

impl WebServerState {
    pub fn new() -> Self {
        let controller = Arc::new(ControllerManager::new());
        
        Self {
            controller,
        }
    }

    pub async fn start(&self) {
        self.controller.start().await;
        
        // Register the main controller node
        let controller_info = NodeInfo {
            node_id: "controller-main".to_string(),
            node_type: "controller".to_string(),
            status: "active".to_string(),
            capabilities: DeviceCapabilities {
                device_id: "controller-main".to_string(),
                device_name: "ProcessDistro Controller".to_string(),
                device_type: "server".to_string(),
                cpu_cores: num_cpus::get() as u32,
                cpu_threads: num_cpus::get() as u32,
                cpu_frequency: 0.0,
                total_memory: get_total_memory(),
                available_memory: get_available_memory(),
                gpu_info: None,
                browser_info: "N/A".to_string(),
                platform: std::env::consts::OS.to_string(),
                user_agent: "ProcessDistro Controller".to_string(),
                benchmark_score: None,
                timestamp: chrono::Utc::now(),
            },
            tasks_completed: 0,
            uptime: 0,
            last_seen: chrono::Utc::now(),
        };
        
        let _ = self.controller.register_node(controller_info, None).await;
    }

    pub async fn stop(&self) {
        self.controller.stop().await;
    }
}

/// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<WebServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

/// Handle WebSocket connection
async fn handle_websocket(socket: WebSocket, state: WebServerState) {
    let (mut sender, mut receiver) = socket.split();
    let websocket_id = Uuid::new_v4().to_string();
    let mut rx = state.controller.get_broadcast_sender().subscribe();

    // Send initial data
    let nodes = state.controller.get_active_nodes().await;
    let initial_message = WsMessage::NodeUpdate { nodes };
    if let Ok(msg) = serde_json::to_string(&initial_message) {
        let _ = sender.send(Message::Text(msg)).await;
    }

    // Store WebSocket ID for cleanup tracking
    let websocket_id_clone = websocket_id.clone();
    let controller_clone = state.controller.clone();

    // Handle incoming messages and broadcast updates
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        let mut current_node_id: Option<String> = None;
        
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            println!("📨 Received WebSocket message from {}: {}", current_node_id.as_deref().unwrap_or("unknown"), text);
            
            // Handle incoming WebSocket messages
            if let Ok(message) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(msg_type) = message.get("type").and_then(|v| v.as_str()) {
                    println!("🔍 Processing message type: {}", msg_type);
                    match msg_type {
                        "register" => {
                            println!("🆕 Processing registration message");
                            if let Ok(capabilities) = serde_json::from_value::<DeviceCapabilities>(message.clone()) {
                                let node_info = NodeInfo {
                                    node_id: capabilities.device_id.clone(),
                                    node_type: "web_client".to_string(),
                                    status: "active".to_string(),
                                    capabilities,
                                    tasks_completed: 0,
                                    uptime: 0,
                                    last_seen: chrono::Utc::now(),
                                };
                                
                                current_node_id = Some(node_info.node_id.clone());
                                println!("✅ Registering node: {}", node_info.node_id);
                                let _ = controller_clone.register_node(node_info, Some(websocket_id_clone.clone())).await;
                            }
                        }
                        "heartbeat" => {
                            if let Some(node_id) = &current_node_id {
                                println!("💓 Heartbeat from node: {}", node_id);
                                controller_clone.update_heartbeat(node_id).await;
                            }
                        }
                        "canvas_completed" => {
                            println!("🎨 Processing canvas completion message");
                            if let Some(node_id) = message.get("node_id").and_then(|v| v.as_str()) {
                                let completed_at = message.get("completed_at").and_then(|v| v.as_str()).unwrap_or("");
                                let status = message.get("status").and_then(|v| v.as_str()).unwrap_or("completed");
                                
                                println!("✅ Canvas completed by node: {}", node_id);
                                
                                // Broadcast to all connected dashboards
                                let broadcast_msg = WsMessage::CanvasCompleted {
                                    node_id: node_id.to_string(),
                                    completed_at: completed_at.to_string(),
                                    status: status.to_string(),
                                };
                                
                                match controller_clone.get_broadcast_sender().send(broadcast_msg) {
                                    Ok(_) => println!("📡 Canvas completion broadcasted to all dashboards"),
                                    Err(e) => println!("❌ Failed to broadcast canvas completion: {:?}", e),
                                }
                            } else {
                                println!("⚠️ Canvas completion message missing node_id");
                            }
                        }
                        "canvas_progress" => {
                            if let Some(node_id) = message.get("node_id").and_then(|v| v.as_str()) {
                                let progress = message.get("progress").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                let timestamp = message.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
                                
                                // Only log significant progress milestones to reduce spam
                                if progress % 10 == 0 || progress >= 95 {
                                    println!("📊 Canvas progress from node {}: {}%", node_id, progress);
                                }
                                
                                // Broadcast to all connected dashboards
                                let broadcast_msg = WsMessage::CanvasProgress {
                                    node_id: node_id.to_string(),
                                    progress,
                                    timestamp: timestamp.to_string(),
                                };
                                
                                match controller_clone.get_broadcast_sender().send(broadcast_msg) {
                                    Ok(_) => {
                                        // Only log broadcast success for completion or every 25%
                                        if progress >= 100 || progress % 25 == 0 {
                                            println!("📡 Canvas progress broadcasted to all dashboards");
                                        }
                                    },
                                    Err(e) => println!("❌ Failed to broadcast canvas progress: {:?}", e),
                                }
                            } else {
                                println!("⚠️ Canvas progress message missing node_id");
                            }
                        }
                        "task_complete" => {
                            println!("🏁 Processing task completion message");
                            // Expect fields: task_id, node_id, result, duration_ms
                            if let Some(task_id) = message.get("task_id").and_then(|v| v.as_str()) {
                                println!("✅ Task completed: {} by node: {:?}", task_id, current_node_id);
                                let _ = controller_clone.complete_task(task_id, message.get("result").cloned()).await;
                                
                                // Update node status to idle
                                if let Some(node_id) = &current_node_id {
                                    controller_clone.update_node_status(node_id, "idle").await;
                                }
                            } else {
                                println!("⚠️ Task completion message missing task_id");
                            }
                        }
                        _ => {
                            println!("❓ Unknown WebSocket message type: {}", msg_type);
                        }
                    }
                } else {
                    println!("⚠️ WebSocket message missing 'type' field: {}", text);
                }
            } else {
                println!("⚠️ Failed to parse WebSocket message as JSON: {}", text);
            }
        }
        
        // Handle disconnect
        if let Some(node_id) = current_node_id {
            controller_clone.handle_node_disconnect(&node_id, Some(&websocket_id_clone)).await;
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        },
    }
}

/// Start the web server
pub async fn start_web_server(port: u16, state: WebServerState) -> Result<(), Box<dyn std::error::Error>> {
    // Create the app with routes
    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/ws", get(websocket_handler))
        .route("/api/nodes", get(get_nodes))
        .route("/api/register", post(register_device))
        .route("/api/unregister", post(unregister_device))
        .route("/api/heartbeat", post(heartbeat))
        .route("/api/benchmark", post(submit_benchmark))
        .route("/api/submit_result", post(submit_task_result))
        .route("/api/log_task", post(log_task_completion))
        .route("/api/task_logs", get(get_task_logs))
        .route("/static/*file", get(serve_static))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start the server
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    println!("Web server starting on http://{}", addr);
    println!("Dashboard available at: http://localhost:{}", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Serve the main dashboard
async fn serve_dashboard() -> impl IntoResponse {
    let html = include_str!("../static/dashboard.html");
    Html(html)
}

/// Serve static files
async fn serve_static(uri: axum::extract::Path<String>) -> impl IntoResponse {
    let path = uri.as_str();
    
    match path {
        "dashboard.js" => {
            let content = include_str!("../static/dashboard.js");
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/javascript")],
                content.as_bytes().to_vec(),
            )
        }
        "dashboard.css" => {
            let content = include_str!("../static/dashboard.css");
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/css")],
                content.as_bytes().to_vec(),
            )
        }
        "mandelbrot.wasm" => {
            let wasm_bytes = include_bytes!("../static/mandelbrot.wasm");
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/wasm")],
                wasm_bytes.to_vec(),
            )
        }
        "password_hash.wasm" => {
            let wasm_bytes = include_bytes!("../static/password_hash.wasm");
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/wasm")],
                wasm_bytes.to_vec(),
            )
        }
        _ => {
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/plain")],
                b"File not found".to_vec(),
            )
        }
    }
}

/// Get all nodes API endpoint
async fn get_nodes(State(state): State<WebServerState>) -> impl IntoResponse {
    let nodes = state.controller.get_active_nodes().await;
    Json(nodes)
}

/// Register a new device
async fn register_device(
    State(state): State<WebServerState>,
    Json(capabilities): Json<DeviceCapabilities>,
) -> impl IntoResponse {
    let node = NodeInfo {
        node_id: capabilities.device_id.clone(),
        node_type: "web_client".to_string(),
        status: "active".to_string(),
        capabilities,
        tasks_completed: 0,
        uptime: 0,
        last_seen: chrono::Utc::now(),
    };

    match state.controller.register_node(node.clone(), None).await {
        Ok(_) => {
            println!("New device registered: {} ({})", node.capabilities.device_name, node.node_id);
            Json(serde_json::json!({
                "status": "success",
                "message": "Device registered successfully",
                "node_id": node.node_id
            }))
        }
        Err(error) => {
            Json(serde_json::json!({
                "status": "error",
                "message": error
            }))
        }
    }
}

/// Handle heartbeat from devices
async fn heartbeat(
    State(state): State<WebServerState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(node_id) = payload.get("node_id").and_then(|v| v.as_str()) {
        state.controller.update_heartbeat(node_id).await;
        Json(serde_json::json!({"status": "ok"}))
    } else {
        Json(serde_json::json!({"status": "error", "message": "Missing node_id"}))
    }
}

/// Submit benchmark results
async fn submit_benchmark(
    State(state): State<WebServerState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let node_id = payload.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
    let score = payload.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
    
    // Update benchmark score through controller
    if let Ok(_) = state.controller.update_benchmark_score(node_id, score).await {
        println!("Benchmark result for {}: {:.2}", node_id, score);
    }
    
    Json(serde_json::json!({"status": "success"}))
}

/// Unregister a device
async fn unregister_device(
    State(state): State<WebServerState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(node_id) = payload.get("node_id").and_then(|v| v.as_str()) {
        match state.controller.unregister_node(node_id).await {
            Ok(removed) => {
                if removed {
                    println!("Device unregistered: {}", node_id);
                    Json(serde_json::json!({
                        "status": "success",
                        "message": "Device unregistered successfully"
                    }))
                } else {
                    Json(serde_json::json!({
                        "status": "error",
                        "message": "Device not found or already unregistered"
                    }))
                }
            }
            Err(error) => {
                Json(serde_json::json!({
                    "status": "error",
                    "message": error
                }))
            }
        }
    } else {
        Json(serde_json::json!({
            "status": "error",
            "message": "Missing node_id"
        }))
    }
}

/// Submit task result
async fn submit_task_result(
    State(state): State<WebServerState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(task_id) = payload.get("task_id").and_then(|v| v.as_str()) {
        let result = payload.get("result").cloned();
        
        match state.controller.complete_task(task_id, result).await {
            Ok(()) => {
                println!("Task completed: {}", task_id);
                
                // Update node status to idle if this was their task
                if let Some(node_id) = payload.get("node_id").and_then(|v| v.as_str()) {
                    state.controller.update_node_status(node_id, "idle").await;
                }
                
                Json(serde_json::json!({
                    "status": "success",
                    "message": "Task result submitted successfully"
                }))
            }
            Err(error) => {
                Json(serde_json::json!({
                    "status": "error",
                    "message": error
                }))
            }
        }
    } else {
        Json(serde_json::json!({
            "status": "error",
            "message": "Missing task_id"
        }))
    }
}

/// Log task completion to JSON file
async fn log_task_completion(
    State(_state): State<WebServerState>,
    Json(task_log): Json<TaskLog>,
) -> impl IntoResponse {
    let log_file_path = "task_logs.json";
    
    match log_task_to_file(&task_log, log_file_path).await {
        Ok(()) => {
            println!("📝 Task logged: {} - {} ({}ms)", 
                task_log.task_id, 
                task_log.task_type, 
                task_log.duration_ms
            );
            
            Json(serde_json::json!({
                "status": "success",
                "message": "Task logged successfully"
            }))
        }
        Err(error) => {
            eprintln!("❌ Failed to log task: {}", error);
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to log task: {}", error)
            }))
        }
    }
}

/// Write task log to JSON file
async fn log_task_to_file(task_log: &TaskLog, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::fs::{File, OpenOptions};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    // Read existing logs or create empty array
    let mut logs: Vec<TaskLog> = if std::path::Path::new(file_path).exists() {
        let mut file = File::open(file_path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        
        if contents.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new())
        }
    } else {
        Vec::new()
    };
    
    // Add new log entry
    logs.push(task_log.clone());
    
    // Keep only last 1000 entries to prevent file from growing too large
    if logs.len() > 1000 {
        logs = logs.into_iter().rev().take(1000).rev().collect();
    }
    
    // Write back to file
    let json_data = serde_json::to_string_pretty(&logs)?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_path)
        .await?;
    
    file.write_all(json_data.as_bytes()).await?;
    file.flush().await?;
    
    Ok(())
}

/// Get task logs for metrics dashboard
async fn get_task_logs(
    State(_state): State<WebServerState>,
) -> impl IntoResponse {
    let log_file_path = "task_logs.json";
    
    match read_task_logs_from_file(log_file_path).await {
        Ok(logs) => {
            Json(logs)
        }
        Err(error) => {
            eprintln!("❌ Failed to read task logs: {}", error);
            // Return empty array instead of error for better UX
            Json(Vec::<TaskLog>::new())
        }
    }
}

/// Read task logs from JSON file
async fn read_task_logs_from_file(file_path: &str) -> Result<Vec<TaskLog>, Box<dyn std::error::Error>> {
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;
    
    if !std::path::Path::new(file_path).exists() {
        return Ok(Vec::new());
    }
    
    let mut file = File::open(file_path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    
    if contents.trim().is_empty() {
        Ok(Vec::new())
    } else {
        let logs: Vec<TaskLog> = serde_json::from_str(&contents)
            .unwrap_or_else(|_| Vec::new());
        Ok(logs)
    }
}

// Helper functions for system info
fn get_total_memory() -> u64 {
    // Simplified - in a real implementation you'd use a proper system info crate
    8192 // 8GB default
}

fn get_available_memory() -> u64 {
    // Simplified - in a real implementation you'd use a proper system info crate
    4096 // 4GB default
}