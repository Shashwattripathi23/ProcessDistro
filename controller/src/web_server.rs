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
    #[serde(rename = "heartbeat")]
    Heartbeat,
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
            // Handle incoming WebSocket messages
            if let Ok(message) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(msg_type) = message.get("type").and_then(|v| v.as_str()) {
                    match msg_type {
                        "register" => {
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
                                let _ = controller_clone.register_node(node_info, Some(websocket_id_clone.clone())).await;
                            }
                        }
                        "heartbeat" => {
                            if let Some(node_id) = &current_node_id {
                                controller_clone.update_heartbeat(node_id).await;
                            }
                        }
                        _ => {
                            println!("Unknown WebSocket message type: {}", msg_type);
                        }
                    }
                }
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
            Response::builder()
                .header(header::CONTENT_TYPE, "application/javascript")
                .body(content.to_string())
                .unwrap()
        }
        "dashboard.css" => {
            let content = include_str!("../static/dashboard.css");
            Response::builder()
                .header(header::CONTENT_TYPE, "text/css")
                .body(content.to_string())
                .unwrap()
        }
        _ => {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("File not found".to_string())
                .unwrap()
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

// Helper functions for system info
fn get_total_memory() -> u64 {
    // Simplified - in a real implementation you'd use a proper system info crate
    8192 // 8GB default
}

fn get_available_memory() -> u64 {
    // Simplified - in a real implementation you'd use a proper system info crate
    4096 // 4GB default
}