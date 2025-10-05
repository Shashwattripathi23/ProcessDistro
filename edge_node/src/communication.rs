/// Communication Module
/// 
/// Handles communication with the controller
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json;
use uuid::Uuid;
use std::time::Duration;
use common::protocol::{Message, MessageType};
use common::types::{NodeInfo, TaskResult};

pub struct ControllerClient {
    stream: Option<TcpStream>,
    controller_addr: String,
    node_id: Option<Uuid>,
    auth_token: Option<String>,
    heartbeat_interval: Duration,
}

impl ControllerClient {
    pub fn new(controller_addr: String) -> Self {
        Self {
            stream: None,
            controller_addr,
            node_id: None,
            auth_token: None,
            heartbeat_interval: Duration::from_secs(30),
        }
    }
    
    /// Connect to the controller
    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(&self.controller_addr).await?;
        self.stream = Some(stream);
        
        println!("Connected to controller at {}", self.controller_addr);
        Ok(())
    }
    
    /// Register this node with the controller
    pub async fn register_node(&mut self, node_info: NodeInfo) -> Result<(), Box<dyn std::error::Error>> {
        let message = Message {
            id: Uuid::new_v4(),
            message_type: MessageType::NodeRegistration,
            payload: serde_json::to_vec(&node_info)?,
            timestamp: std::time::SystemTime::now(),
        };
        
        self.send_message(message).await?;
        
        // Wait for registration response
        if let Some(response) = self.receive_message().await? {
            match response.message_type {
                MessageType::RegistrationSuccess => {
                    // Extract node_id and auth_token from response
                    let response_data: serde_json::Value = serde_json::from_slice(&response.payload)?;
                    self.node_id = Some(Uuid::parse_str(response_data["node_id"].as_str().unwrap())?);
                    self.auth_token = Some(response_data["auth_token"].as_str().unwrap().to_string());
                    
                    println!("Successfully registered with controller");
                }
                MessageType::RegistrationFailure => {
                    return Err("Node registration failed".into());
                }
                _ => {
                    return Err("Unexpected response to registration".into());
                }
            }
        }
        
        Ok(())
    }
    
    /// Send heartbeat to controller
    pub async fn send_heartbeat(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(node_id), Some(token)) = (&self.node_id, &self.auth_token) {
            let heartbeat_data = serde_json::json!({
                "node_id": node_id,
                "auth_token": token,
                "status": "online"
            });
            
            let message = Message {
                id: Uuid::new_v4(),
                message_type: MessageType::Heartbeat,
                payload: serde_json::to_vec(&heartbeat_data)?,
                timestamp: std::time::SystemTime::now(),
            };
            
            self.send_message(message).await?;
        }
        
        Ok(())
    }
    
    /// Request a task from the controller
    pub async fn request_task(&mut self) -> Result<Option<common::types::Task>, Box<dyn std::error::Error>> {
        if let (Some(node_id), Some(token)) = (&self.node_id, &self.auth_token) {
            let request_data = serde_json::json!({
                "node_id": node_id,
                "auth_token": token
            });
            
            let message = Message {
                id: Uuid::new_v4(),
                message_type: MessageType::TaskRequest,
                payload: serde_json::to_vec(&request_data)?,
                timestamp: std::time::SystemTime::now(),
            };
            
            self.send_message(message).await?;
            
            // Wait for task assignment or no-task response
            if let Some(response) = self.receive_message().await? {
                match response.message_type {
                    MessageType::TaskAssignment => {
                        let task: common::types::Task = serde_json::from_slice(&response.payload)?;
                        return Ok(Some(task));
                    }
                    MessageType::NoTasksAvailable => {
                        return Ok(None);
                    }
                    _ => {
                        return Err("Unexpected response to task request".into());
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Submit task result to controller
    pub async fn submit_result(&mut self, result: TaskResult) -> Result<(), Box<dyn std::error::Error>> {
        let message = Message {
            id: Uuid::new_v4(),
            message_type: MessageType::TaskResult,
            payload: serde_json::to_vec(&result)?,
            timestamp: std::time::SystemTime::now(),
        };
        
        self.send_message(message).await
    }
    
    /// Send a message to the controller
    async fn send_message(&mut self, message: Message) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(stream) = &mut self.stream {
            let message_bytes = serde_json::to_vec(&message)?;
            let length = message_bytes.len() as u32;
            
            // Send length prefix
            stream.write_u32(length).await?;
            // Send message
            stream.write_all(&message_bytes).await?;
            
            Ok(())
        } else {
            Err("Not connected to controller".into())
        }
    }
    
    /// Receive a message from the controller
    async fn receive_message(&mut self) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        if let Some(stream) = &mut self.stream {
            // Read length prefix
            let length = stream.read_u32().await?;
            
            // Read message
            let mut buffer = vec![0u8; length as usize];
            stream.read_exact(&mut buffer).await?;
            
            let message: Message = serde_json::from_slice(&buffer)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
    
    /// Start heartbeat loop
    pub async fn start_heartbeat_loop(&mut self) {
        let mut interval = tokio::time::interval(self.heartbeat_interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.send_heartbeat().await {
                eprintln!("Failed to send heartbeat: {}", e);
                // TODO: Attempt reconnection
            }
        }
    }
}