/// Security Module
/// 
/// Handles authentication, authorization, and security policies
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    pub node_id: Uuid,
    pub expires_at: u64,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    ExecuteTasks,
    SubmitResults,
    RequestTasks,
    ViewMetrics,
}

pub struct SecurityManager {
    active_tokens: HashMap<String, AuthToken>,
    token_duration: Duration,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            active_tokens: HashMap::new(),
            token_duration: Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }
    
    /// Generate a new authentication token for a node
    pub fn generate_token(&mut self, node_id: Uuid, permissions: Vec<Permission>) -> String {
        let token = format!("pd_{}", Uuid::new_v4().to_string().replace("-", ""));
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + self.token_duration.as_secs();
        
        let auth_token = AuthToken {
            token: token.clone(),
            node_id,
            expires_at,
            permissions,
        };
        
        self.active_tokens.insert(token.clone(), auth_token);
        token
    }
    
    /// Validate an authentication token
    pub fn validate_token(&self, token: &str) -> Option<&AuthToken> {
        if let Some(auth_token) = self.active_tokens.get(token) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            if auth_token.expires_at > now {
                Some(auth_token)
            } else {
                None // Token expired
            }
        } else {
            None // Token not found
        }
    }
    
    /// Check if a token has a specific permission
    pub fn has_permission(&self, token: &str, permission: &Permission) -> bool {
        if let Some(auth_token) = self.validate_token(token) {
            auth_token.permissions.contains(permission)
        } else {
            false
        }
    }
    
    /// Revoke a token
    pub fn revoke_token(&mut self, token: &str) {
        self.active_tokens.remove(token);
    }
    
    /// Clean up expired tokens
    pub fn cleanup_expired_tokens(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.active_tokens.retain(|_, token| token.expires_at > now);
    }
    
    /// Validate WASM module safety
    pub fn validate_wasm_module(&self, wasm_bytes: &[u8]) -> bool {
        // TODO: Implement WASM module validation
        // - Check for restricted imports
        // - Validate module structure
        // - Ensure no host bindings to sensitive functions
        true
    }
}