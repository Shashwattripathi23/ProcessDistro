/// Basic ProcessDistro Usage Example
/// 
/// Demonstrates how to start a controller and connect edge nodes

use common::types::*;
use common::protocol::*;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ProcessDistro Basic Usage Example");
    println!("================================");
    
    // Example configuration
    let config = SystemConfig {
        controller: ControllerConfig {
            port: 30000,
            max_nodes: 10,
            task_timeout: Duration::from_secs(300),
            heartbeat_interval: Duration::from_secs(30),
            work_stealing_enabled: true,
        },
        security: SecurityConfig {
            token_duration: Duration::from_secs(86400), // 24 hours
            require_tls: false,
            allowed_wasm_imports: vec![
                "wasi_snapshot_preview1".to_string(),
            ],
            sandbox_memory_limit: 64 * 1024 * 1024, // 64MB
        },
        performance: PerformanceConfig {
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            task_queue_size: 1000,
            result_cache_size: 500,
        },
    };
    
    println!("Configuration:");
    println!("  Controller port: {}", config.controller.port);
    println!("  Max nodes: {}", config.controller.max_nodes);
    println!("  Work stealing: {}", config.controller.work_stealing_enabled);
    
    // Example task creation
    let task = Task {
        id: uuid::Uuid::new_v4(),
        task_type: TaskType::MatrixMultiplication,
        wasm_url: "http://localhost:30000/tasks/matrix_mul.wasm".to_string(),
        parameters: {
            let mut params = std::collections::HashMap::new();
            params.insert("matrix_size".to_string(), serde_json::json!(256));
            params.insert("tile_size".to_string(), serde_json::json!(64));
            params
        },
        priority: TaskPriority::Normal,
        timeout: Duration::from_secs(300),
        retry_count: 0,
        created_at: std::time::SystemTime::now(),
    };
    
    println!("\nExample task:");
    println!("  ID: {}", task.id);
    println!("  Type: {:?}", task.task_type);
    println!("  Priority: {:?}", task.priority);
    
    // Example node capabilities
    let node_capabilities = NodeCapabilities {
        cpu_cores: 8,
        cpu_frequency: 3200,
        total_memory: 16 * 1024 * 1024 * 1024, // 16GB
        available_memory: 12 * 1024 * 1024 * 1024, // 12GB
        wasm_runtime: "wasmtime".to_string(),
        runtime_version: "25.0.0".to_string(),
        supported_task_types: vec![
            TaskType::MatrixMultiplication,
            TaskType::PasswordHashing,
            TaskType::MandelbrotRendering,
        ],
        max_concurrent_tasks: 4,
    };
    
    println!("\nExample node capabilities:");
    println!("  CPU cores: {}", node_capabilities.cpu_cores);
    println!("  Memory: {}GB", node_capabilities.total_memory / (1024*1024*1024));
    println!("  Runtime: {}", node_capabilities.wasm_runtime);
    println!("  Max concurrent tasks: {}", node_capabilities.max_concurrent_tasks);
    
    // Example message creation
    let registration_payload = NodeRegistrationPayload {
        node_info: NodeInfo {
            name: "example_node".to_string(),
            capabilities: node_capabilities,
            status: NodeStatus::Online,
            last_seen: std::time::SystemTime::now(),
        },
        auth_challenge: None,
    };
    
    let message = Message::with_json_payload(
        MessageType::NodeRegistration,
        &registration_payload
    )?;
    
    println!("\nExample registration message:");
    println!("  Message ID: {}", message.id);
    println!("  Type: {:?}", message.message_type);
    println!("  Payload size: {} bytes", message.payload.len());
    
    println!("\nTo run ProcessDistro:");
    println!("1. Start the controller: cargo run --bin controller");
    println!("2. Start edge nodes: cargo run --bin edge_node");
    println!("3. Submit tasks via CLI: processdistro submit-task ...");
    
    Ok(())
}