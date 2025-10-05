/// Error Types
/// 
/// Common error definitions for ProcessDistro
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessDistroError {
    #[error("Network communication error: {0}")]
    NetworkError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("WASM execution error: {0}")]
    WasmError(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    
    #[error("Authorization failed: {0}")]
    AuthorizationError(String),
    
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: uuid::Uuid },
    
    #[error("Node not found: {node_id}")]
    NodeNotFound { node_id: uuid::Uuid },
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
    
    #[error("Timeout occurred after {duration:?}")]
    Timeout { duration: std::time::Duration },
    
    #[error("Validation failed: {0}")]
    ValidationError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type alias for ProcessDistro operations
pub type Result<T> = std::result::Result<T, ProcessDistroError>;

/// Task-specific error types
#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Task compilation failed: {0}")]
    CompilationError(String),
    
    #[error("Task runtime error: {0}")]
    RuntimeError(String),
    
    #[error("Task execution timeout")]
    TimeoutError,
    
    #[error("Memory limit exceeded")]
    MemoryError,
    
    #[error("Security violation: {0}")]
    SecurityViolation(String),
    
    #[error("Task validation failed: {0}")]
    ValidationError(String),
}

/// Network-specific error types
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed to {address}: {reason}")]
    ConnectionFailed { address: String, reason: String },
    
    #[error("Message parsing failed: {0}")]
    MessageParsingError(String),
    
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolMismatch { expected: String, actual: String },
    
    #[error("Network timeout after {duration:?}")]
    NetworkTimeout { duration: std::time::Duration },
    
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),
}

/// Security-specific error types
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Invalid authentication token")]
    InvalidToken,
    
    #[error("Token expired")]
    TokenExpired,
    
    #[error("Insufficient permissions for operation: {operation}")]
    InsufficientPermissions { operation: String },
    
    #[error("WASM module validation failed: {reason}")]
    WasmValidationFailed { reason: String },
    
    #[error("Untrusted WASM import: {import}")]
    UntrustedImport { import: String },
}

impl From<TaskError> for ProcessDistroError {
    fn from(err: TaskError) -> Self {
        match err {
            TaskError::CompilationError(msg) => ProcessDistroError::WasmError(format!("Compilation: {}", msg)),
            TaskError::RuntimeError(msg) => ProcessDistroError::WasmError(format!("Runtime: {}", msg)),
            TaskError::TimeoutError => ProcessDistroError::Timeout { duration: std::time::Duration::from_secs(0) },
            TaskError::MemoryError => ProcessDistroError::ResourceExhausted { resource: "memory".to_string() },
            TaskError::SecurityViolation(msg) => ProcessDistroError::AuthorizationError(msg),
            TaskError::ValidationError(msg) => ProcessDistroError::ValidationError(msg),
        }
    }
}

impl From<NetworkError> for ProcessDistroError {
    fn from(err: NetworkError) -> Self {
        match err {
            NetworkError::ConnectionFailed { address, reason } => {
                ProcessDistroError::NetworkError(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("Failed to connect to {}: {}", address, reason)
                ))
            },
            NetworkError::MessageParsingError(msg) => ProcessDistroError::SerializationError(
                serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, msg))
            ),
            NetworkError::ProtocolMismatch { expected, actual } => {
                ProcessDistroError::ValidationError(format!("Protocol mismatch: expected {}, got {}", expected, actual))
            },
            NetworkError::NetworkTimeout { duration } => ProcessDistroError::Timeout { duration },
            NetworkError::InvalidMessageFormat(msg) => ProcessDistroError::ValidationError(msg),
        }
    }
}

impl From<SecurityError> for ProcessDistroError {
    fn from(err: SecurityError) -> Self {
        match err {
            SecurityError::InvalidToken => ProcessDistroError::AuthenticationError("Invalid token".to_string()),
            SecurityError::TokenExpired => ProcessDistroError::AuthenticationError("Token expired".to_string()),
            SecurityError::InsufficientPermissions { operation } => {
                ProcessDistroError::AuthorizationError(format!("Insufficient permissions: {}", operation))
            },
            SecurityError::WasmValidationFailed { reason } => ProcessDistroError::WasmError(reason),
            SecurityError::UntrustedImport { import } => {
                ProcessDistroError::WasmError(format!("Untrusted import: {}", import))
            },
        }
    }
}