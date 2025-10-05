/// Protocol Messages
/// 
/// Network communication protocol for controller-node communication
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Base message structure for all communications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
    pub timestamp: SystemTime,
}

/// All supported message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    // Node lifecycle messages
    NodeRegistration,
    RegistrationSuccess,
    RegistrationFailure,
    NodeDeregistration,
    
    // Heartbeat and health
    Heartbeat,
    HeartbeatAck,
    HealthCheck,
    HealthResponse,
    
    // Task management
    TaskRequest,
    TaskAssignment,
    NoTasksAvailable,
    TaskCancellation,
    TaskProgress,
    TaskResult,
    TaskFailure,
    
    // Work stealing
    WorkStealRequest,
    WorkStealResponse,
    WorkStealDenied,
    
    // System control
    Shutdown,
    Restart,
    UpdateConfig,
    
    // Metrics and monitoring
    MetricsRequest,
    MetricsResponse,
    
    // File transfer
    FileRequest,
    FileChunk,
    FileComplete,
    
    // Discovery
    DiscoveryBroadcast,
    DiscoveryResponse,
    
    // Error handling
    Error,
    Acknowledgment,
}

/// Node registration payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistrationPayload {
    pub node_info: crate::types::NodeInfo,
    pub auth_challenge: Option<String>,
}

/// Registration success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationSuccessPayload {
    pub node_id: crate::types::NodeId,
    pub auth_token: String,
    pub controller_config: ControllerConfigPayload,
}

/// Registration failure response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationFailurePayload {
    pub reason: String,
    pub retry_allowed: bool,
    pub retry_after: Option<std::time::Duration>,
}

/// Controller configuration sent to nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerConfigPayload {
    pub heartbeat_interval: std::time::Duration,
    pub task_timeout: std::time::Duration,
    pub max_concurrent_tasks: u32,
}

/// Task assignment payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignmentPayload {
    pub task: crate::types::Task,
    pub auth_token: String,
    pub wasm_hash: Option<String>,
}

/// Task progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressPayload {
    pub task_id: crate::types::TaskId,
    pub progress_percent: f32,
    pub estimated_completion: Option<SystemTime>,
    pub status_message: Option<String>,
}

/// Work stealing request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkStealRequestPayload {
    pub requesting_node: crate::types::NodeId,
    pub auth_token: String,
    pub available_capacity: u32,
    pub preferred_task_types: Vec<crate::types::TaskType>,
}

/// Discovery broadcast payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryBroadcastPayload {
    pub controller_address: String,
    pub controller_port: u16,
    pub protocol_version: String,
    pub supported_features: Vec<String>,
}

/// Discovery response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponsePayload {
    pub node_info: crate::types::NodeInfo,
    pub interested: bool,
    pub estimated_join_time: Option<std::time::Duration>,
}

/// Error message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub error_code: ErrorCode,
    pub error_message: String,
    pub related_message_id: Option<Uuid>,
}

/// Standard error codes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    InvalidMessage,
    AuthenticationFailed,
    AuthorizationFailed,
    TaskNotFound,
    NodeNotFound,
    InternalError,
    ResourceExhausted,
    Timeout,
    ValidationFailed,
    UnsupportedOperation,
}

/// File transfer chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunkPayload {
    pub file_id: String,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub chunk_data: Vec<u8>,
    pub checksum: String,
}

/// Metrics response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponsePayload {
    pub node_id: crate::types::NodeId,
    pub timestamp: SystemTime,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub active_tasks: u32,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub uptime: std::time::Duration,
}

impl Message {
    /// Create a new message
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            payload,
            timestamp: SystemTime::now(),
        }
    }
    
    /// Create a message with JSON payload
    pub fn with_json_payload<T: Serialize>(message_type: MessageType, payload: &T) -> Result<Self, serde_json::Error> {
        let payload_bytes = serde_json::to_vec(payload)?;
        Ok(Self::new(message_type, payload_bytes))
    }
    
    /// Deserialize payload as JSON
    pub fn deserialize_payload<T: for<'a> Deserialize<'a>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.payload)
    }
    
    /// Create an acknowledgment message for this message
    pub fn create_ack(&self) -> Self {
        let ack_payload = serde_json::json!({
            "original_message_id": self.id,
            "timestamp": SystemTime::now()
        });
        
        Self::new(
            MessageType::Acknowledgment,
            serde_json::to_vec(&ack_payload).unwrap()
        )
    }
    
    /// Create an error response for this message
    pub fn create_error_response(&self, error_code: ErrorCode, error_message: String) -> Self {
        let error_payload = ErrorPayload {
            error_code,
            error_message,
            related_message_id: Some(self.id),
        };
        
        Self::with_json_payload(MessageType::Error, &error_payload).unwrap()
    }
}