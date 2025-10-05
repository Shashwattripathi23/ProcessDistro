/// Common Types
/// 
/// Shared data structures used across controller and edge nodes
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds, TimestampSeconds};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Unique identifier for tasks
pub type TaskId = Uuid;

/// Unique identifier for nodes
pub type NodeId = Uuid;

/// Task definition and metadata
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub task_type: TaskType,
    pub wasm_url: String,
    pub parameters: TaskParameters,
    pub priority: TaskPriority,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub timeout: Duration,
    pub retry_count: u32,
    #[serde_as(as = "TimestampSeconds<i64>")]
    pub created_at: SystemTime,
}

/// Types of tasks that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    MatrixMultiplication,
    PasswordHashing,
    MandelbrotRendering,
    Custom(String),
}

/// Task parameters as key-value pairs
pub type TaskParameters = HashMap<String, serde_json::Value>;

/// Task priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Assigned(NodeId),
    Running(NodeId),
    Completed,
    Failed(String),
    Cancelled,
}

/// Result of task execution
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub success: bool,
    pub result_data: Vec<u8>,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub execution_time: Duration,
    pub error_message: Option<String>,
}

/// Node information and capabilities
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    pub capabilities: NodeCapabilities,
    pub status: NodeStatus,
    #[serde_as(as = "TimestampSeconds<i64>")]
    pub last_seen: SystemTime,
}

/// Node computational capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub cpu_cores: u32,
    pub cpu_frequency: u64,
    pub total_memory: u64,
    pub available_memory: u64,
    pub wasm_runtime: String,
    pub runtime_version: String,
    pub supported_task_types: Vec<TaskType>,
    pub max_concurrent_tasks: u32,
}

/// Node operational status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Offline,
    Busy,
    Overloaded,
    Maintenance,
}

/// Configuration for the entire system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub controller: ControllerConfig,
    pub security: SecurityConfig,
    pub performance: PerformanceConfig,
}

/// Controller-specific configuration
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerConfig {
    pub port: u16,
    pub max_nodes: u32,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub task_timeout: Duration,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub heartbeat_interval: Duration,
    pub work_stealing_enabled: bool,
}

/// Security configuration
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde_as(as = "DurationSeconds<u64>")]
    pub token_duration: Duration,
    pub require_tls: bool,
    pub allowed_wasm_imports: Vec<String>,
    pub sandbox_memory_limit: u64,
}

/// Performance tuning configuration
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_retries: u32,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub retry_delay: Duration,
    pub task_queue_size: usize,
    pub result_cache_size: usize,
}

/// Error types for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskError {
    CompilationError(String),
    RuntimeError(String),
    TimeoutError,
    MemoryError,
    SecurityViolation(String),
    NetworkError(String),
    ValidationError(String),
}

/// Work stealing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkStealRequest {
    pub requesting_node: NodeId,
    pub available_capacity: u32,
    pub preferred_task_types: Vec<TaskType>,
}

/// Work stealing response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkStealResponse {
    pub success: bool,
    pub stolen_tasks: Vec<Task>,
    pub reason: Option<String>,
}