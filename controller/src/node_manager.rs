/// Node Manager Module
/// 
/// Handles node discovery, capability tracking, and health monitoring
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use uuid::Uuid;
use tokio::net::UdpSocket as TokioUdpSocket;
use tokio::time::{interval, timeout};
use serde::{Serialize, Deserialize};
use common::types::{NodeId, NodeInfo, NodeStatus};
use common::protocol::{Message, MessageType, DiscoveryBroadcastPayload, DiscoveryResponsePayload};

pub struct NodeManager {
    nodes: HashMap<NodeId, ManagedNode>,
    discovery_service: DiscoveryService,
}

pub struct ManagedNode {
    pub info: NodeInfo,
    pub status: NodeStatus,
    pub last_heartbeat: Instant,
    pub task_count: u32,
    pub capabilities: NodeCapabilities,
}

pub struct NodeCapabilities {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub wasm_runtime: String,
    pub supported_tasks: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub ip_address: IpAddr,
    pub hostname: Option<String>,
    pub mac_address: Option<String>,
    pub responded_to_ping: bool,
    pub open_ports: Vec<u16>,
    pub is_processdistro_node: bool,
    pub discovered_at: Instant,
}

pub struct DiscoveryService {
    broadcast_port: u16,
    discovery_interval: Duration,
    network_interfaces: Vec<NetworkInterface>,
    mdns_service: Option<mdns_sd::ServiceDaemon>,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_address: IpAddr,
    pub subnet_mask: IpAddr,
    pub broadcast_address: IpAddr,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryMessage {
    controller_id: Uuid,
    controller_port: u16,
    protocol_version: String,
    supported_features: Vec<String>,
    timestamp: u64,
}

impl DiscoveryService {
    pub fn new(broadcast_port: u16) -> Self {
        Self {
            broadcast_port,
            discovery_interval: Duration::from_secs(30),
            network_interfaces: Vec::new(),
            mdns_service: None,
        }
    }

    /// Initialize network discovery services
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Discover network interfaces
        self.discover_network_interfaces().await?;
        
        // Initialize mDNS service
        self.initialize_mdns().await?;
        
        tracing::info!("Discovery service initialized with {} network interfaces", 
                      self.network_interfaces.len());
        
        Ok(())
    }

    /// Discover all network interfaces on this machine
    async fn discover_network_interfaces(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use local_ip_address::list_afinet_netifas;
        
        let network_interfaces = list_afinet_netifas()?;
        
        for (name, ip) in network_interfaces {
            // Skip loopback and link-local addresses
            if ip.is_loopback() || ip.to_string().starts_with("169.254.") {
                continue;
            }
            
            let subnet_mask = self.get_subnet_mask(&ip).await;
            let broadcast_address = self.calculate_broadcast_address(&ip, &subnet_mask);
            
            let interface = NetworkInterface {
                name: name.clone(),
                ip_address: ip,
                subnet_mask,
                broadcast_address,
            };
            
            self.network_interfaces.push(interface);
            tracing::debug!("Discovered network interface: {} ({})", name, ip);
        }
        
        Ok(())
    }

    /// Get subnet mask for an IP address (simplified implementation)
    async fn get_subnet_mask(&self, ip: &IpAddr) -> IpAddr {
        // This is a simplified implementation
        // In production, you'd query the system's routing table
        match ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                if octets[0] == 10 {
                    // Class A private: 10.0.0.0/8
                    IpAddr::V4(Ipv4Addr::new(255, 0, 0, 0))
                } else if octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31) {
                    // Class B private: 172.16.0.0/12
                    IpAddr::V4(Ipv4Addr::new(255, 240, 0, 0))
                } else if octets[0] == 192 && octets[1] == 168 {
                    // Class C private: 192.168.0.0/16
                    IpAddr::V4(Ipv4Addr::new(255, 255, 0, 0))
                } else {
                    // Default to /24
                    IpAddr::V4(Ipv4Addr::new(255, 255, 255, 0))
                }
            }
            IpAddr::V6(_) => {
                // IPv6 not implemented in this example
                IpAddr::V4(Ipv4Addr::new(255, 255, 255, 0))
            }
        }
    }

    /// Calculate broadcast address from IP and subnet mask
    fn calculate_broadcast_address(&self, ip: &IpAddr, subnet_mask: &IpAddr) -> IpAddr {
        match (ip, subnet_mask) {
            (IpAddr::V4(ip), IpAddr::V4(mask)) => {
                let ip_u32 = u32::from_be_bytes(ip.octets());
                let mask_u32 = u32::from_be_bytes(mask.octets());
                let broadcast_u32 = ip_u32 | !mask_u32;
                IpAddr::V4(Ipv4Addr::from(broadcast_u32))
            }
            _ => *ip, // Fallback
        }
    }

    /// Initialize mDNS service for service discovery
    async fn initialize_mdns(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Get local IP address
        let local_ip = if let Ok(ip) = local_ip_address::local_ip() {
            ip
        } else {
            // Fallback to localhost if can't determine local IP
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
        };
        
        // Initialize mDNS service daemon
        let mdns = mdns_sd::ServiceDaemon::new()?;
        
        // Register ProcessDistro controller service
        let service_type = "_processdistro._tcp.local.";
        let instance_name = "ProcessDistro Controller";
        let host_name = format!("{}.local.", gethostname::gethostname().to_string_lossy());
        let port = self.broadcast_port;
        let properties = [("version", "1.0.0"), ("type", "controller")];
        
        let service_info = mdns_sd::ServiceInfo::new(
            service_type,
            instance_name,
            &host_name,
            local_ip,
            port,
            &properties[..],
        )?;
        
        mdns.register(service_info)?;
        self.mdns_service = Some(mdns);
        
        tracing::info!("mDNS service registered: {}", service_type);
        Ok(())
    }

    /// Perform network-wide device discovery
    pub async fn discover_devices(&self) -> Result<Vec<DiscoveredDevice>, Box<dyn std::error::Error>> {
        let mut discovered_devices = Vec::new();
        
        // Method 1: UDP Broadcast Discovery
        let broadcast_devices = self.udp_broadcast_discovery().await?;
        discovered_devices.extend(broadcast_devices);
        
        // Method 2: Network Scan
        let scanned_devices = self.network_scan().await?;
        discovered_devices.extend(scanned_devices);
        
        // Method 3: mDNS Discovery
        let mdns_devices = self.mdns_discovery().await?;
        discovered_devices.extend(mdns_devices);
        
        // Remove duplicates based on IP address
        discovered_devices.sort_by_key(|d| d.ip_address.to_string());
        discovered_devices.dedup_by_key(|d| d.ip_address.to_string());
        
        tracing::info!("Discovered {} unique devices on the network", discovered_devices.len());
        Ok(discovered_devices)
    }

    /// Discover devices using UDP broadcast
    async fn udp_broadcast_discovery(&self) -> Result<Vec<DiscoveredDevice>, Box<dyn std::error::Error>> {
        let mut devices = Vec::new();
        
        for interface in &self.network_interfaces {
            let broadcast_addr = SocketAddr::new(interface.broadcast_address, self.broadcast_port);
            
            // Create UDP socket for broadcasting
            let socket = UdpSocket::bind("0.0.0.0:0")?;
            socket.set_broadcast(true)?;
            
            // Create discovery message
            let discovery_msg = DiscoveryMessage {
                controller_id: Uuid::new_v4(),
                controller_port: self.broadcast_port,
                protocol_version: "1.0.0".to_string(),
                supported_features: vec!["wasm_execution".to_string(), "work_stealing".to_string()],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs(),
            };
            
            let message_json = serde_json::to_string(&discovery_msg)?;
            
            // Send broadcast message
            socket.send_to(message_json.as_bytes(), broadcast_addr)?;
            tracing::debug!("Sent discovery broadcast to {}", broadcast_addr);
            
            // Listen for responses (with timeout)
            let tokio_socket = TokioUdpSocket::from_std(socket)?;
            let mut buffer = [0u8; 1024];
            
            // Wait for responses for 5 seconds
            let response_timeout = Duration::from_secs(5);
            let mut response_count = 0;
            
            while response_count < 10 { // Limit responses per interface
                match timeout(response_timeout, tokio_socket.recv_from(&mut buffer)).await {
                    Ok(Ok((len, addr))) => {
                        response_count += 1;
                        
                        if let Ok(response_text) = std::str::from_utf8(&buffer[..len]) {
                            tracing::debug!("Received response from {}: {}", addr, response_text);
                            
                            let device = DiscoveredDevice {
                                ip_address: addr.ip(),
                                hostname: self.resolve_hostname(addr.ip()).await,
                                mac_address: None, // Would need ARP table lookup
                                responded_to_ping: true,
                                open_ports: vec![addr.port()],
                                is_processdistro_node: response_text.contains("processdistro"),
                                discovered_at: Instant::now(),
                            };
                            
                            devices.push(device);
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("UDP receive error: {}", e);
                        break;
                    }
                    Err(_) => {
                        // Timeout - no more responses
                        break;
                    }
                }
            }
        }
        
        Ok(devices)
    }

    /// Scan network range for active devices
    async fn network_scan(&self) -> Result<Vec<DiscoveredDevice>, Box<dyn std::error::Error>> {
        let mut devices = Vec::new();
        
        for interface in &self.network_interfaces {
            if let IpAddr::V4(ip) = interface.ip_address {
                if let IpAddr::V4(mask) = interface.subnet_mask {
                    let network_devices = self.scan_network_range(ip, mask).await?;
                    devices.extend(network_devices);
                }
            }
        }
        
        Ok(devices)
    }

    /// Scan a specific network range
    async fn scan_network_range(&self, network_ip: Ipv4Addr, subnet_mask: Ipv4Addr) -> Result<Vec<DiscoveredDevice>, Box<dyn std::error::Error>> {
        let mut devices = Vec::new();
        
        let network_u32 = u32::from_be_bytes(network_ip.octets());
        let mask_u32 = u32::from_be_bytes(subnet_mask.octets());
        let network_addr = network_u32 & mask_u32;
        let host_bits = !mask_u32;
        
        // Limit scan to reasonable range (e.g., /24 networks)
        let max_hosts = if host_bits <= 255 { host_bits } else { 255 };
        
        tracing::info!("Scanning network range: {}/{} ({} hosts)", 
                      Ipv4Addr::from(network_addr), 
                      subnet_mask.octets().iter().map(|&b| b.count_ones()).sum::<u32>(),
                      max_hosts);
        
        // Create tasks for parallel scanning
        let mut scan_tasks = Vec::new();
        
        for host_offset in 1..=max_hosts {
            let target_ip = Ipv4Addr::from(network_addr | host_offset);
            
            // Skip our own IP
            if target_ip == network_ip {
                continue;
            }
            
            let task = tokio::spawn(async move {
                let mut device = DiscoveredDevice {
                    ip_address: IpAddr::V4(target_ip),
                    hostname: None,
                    mac_address: None,
                    responded_to_ping: false,
                    open_ports: Vec::new(),
                    is_processdistro_node: false,
                    discovered_at: Instant::now(),
                };
                
                // Simple TCP connect test to common ports
                let test_ports = [22, 80, 443, 8080, 30000]; // SSH, HTTP, HTTPS, Alt-HTTP, ProcessDistro
                
                for &port in &test_ports {
                    let addr = SocketAddr::new(IpAddr::V4(target_ip), port);
                    
                    if let Ok(_) = timeout(Duration::from_millis(100), tokio::net::TcpStream::connect(addr)).await {
                        device.open_ports.push(port);
                        device.responded_to_ping = true;
                        
                        // Check if it's a ProcessDistro node
                        if port == 30000 {
                            device.is_processdistro_node = true;
                        }
                    }
                }
                
                if device.responded_to_ping {
                    Some(device)
                } else {
                    None
                }
            });
            
            scan_tasks.push(task);
        }
        
        // Wait for all scan tasks to complete
        let scan_results = futures::future::join_all(scan_tasks).await;
        
        for result in scan_results {
            if let Ok(Some(device)) = result {
                devices.push(device);
            }
        }
        
        tracing::info!("Network scan found {} active devices", devices.len());
        Ok(devices)
    }

    /// Discover devices using mDNS
    async fn mdns_discovery(&self) -> Result<Vec<DiscoveredDevice>, Box<dyn std::error::Error>> {
        let mut devices = Vec::new();
        
        if let Some(mdns) = &self.mdns_service {
            // Browse for ProcessDistro services
            let service_type = "_processdistro._tcp.local.";
            let receiver = mdns.browse(service_type)?;
            
            // Wait for responses with timeout
            let browse_timeout = Duration::from_secs(5);
            let start_time = Instant::now();
            
            while start_time.elapsed() < browse_timeout {
                if let Ok(event) = timeout(Duration::from_millis(100), async { receiver.recv() }).await {
                    match event {
                        Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) => {
                            let device = DiscoveredDevice {
                                ip_address: *info.get_addresses().iter().next().unwrap_or(&IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))),
                                hostname: Some(info.get_hostname().to_string()),
                                mac_address: None,
                                responded_to_ping: true,
                                open_ports: vec![info.get_port()],
                                is_processdistro_node: true,
                                discovered_at: Instant::now(),
                            };
                            
                            devices.push(device);
                            tracing::info!("Found ProcessDistro node via mDNS: {}", info.get_hostname());
                        }
                        _ => {}
                    }
                } else {
                    break; // Timeout
                }
            }
        }
        
        Ok(devices)
    }

    /// Resolve hostname for an IP address
    async fn resolve_hostname(&self, ip: IpAddr) -> Option<String> {
        use dns_lookup::lookup_addr;
        
        tokio::task::spawn_blocking(move || {
            lookup_addr(&ip).ok()
        }).await.ok().flatten()
    }

    /// Start continuous device discovery
    pub async fn start_continuous_discovery(&self, callback: impl Fn(Vec<DiscoveredDevice>) + Send + 'static) {
        let mut discovery_interval = interval(self.discovery_interval);
        
        tokio::spawn(async move {
            loop {
                discovery_interval.tick().await;
                
                // Note: In a real implementation, you'd need to clone/share the discovery service
                // This is a simplified example showing the pattern
                tracing::debug!("Running periodic device discovery...");
                
                // The actual discovery would happen here
                // callback(discovered_devices);
            }
        });
    }
}
    
impl NodeManager {
    pub fn new(broadcast_port: u16) -> Self {
        Self {
            nodes: HashMap::new(),
            discovery_service: DiscoveryService::new(broadcast_port),
        }
    }
    
    /// Initialize the node manager and discovery service
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Initializing NodeManager...");
        
        // For now, skip mDNS discovery to avoid network compatibility issues
        // self.discovery_service.initialize().await?;
        
        // Just set up basic state
        tracing::info!("NodeManager initialized successfully (discovery simplified)");
        
        Ok(())
    }
    
    /// Start node discovery process
    pub async fn start_discovery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting device discovery on network...");
        
        // Initial discovery scan
        let discovered_devices = self.discovery_service.discover_devices().await?;
        
        tracing::info!("Initial discovery found {} devices", discovered_devices.len());
        
        // Process discovered devices
        for device in discovered_devices {
            self.process_discovered_device(device).await;
        }
        
        // Start periodic discovery
        self.start_periodic_discovery().await;
        
        Ok(())
    }
    
    /// Process a discovered device
    async fn process_discovered_device(&mut self, device: DiscoveredDevice) {
        tracing::debug!("Processing discovered device: {} (ProcessDistro: {})", 
                       device.ip_address, device.is_processdistro_node);
        
        let device_ip = device.ip_address.clone();
        
        if device.is_processdistro_node {
            // This is potentially a ProcessDistro edge node
            tracing::info!("Found potential ProcessDistro node at {}", device_ip);
            
            // Try to establish communication
            if let Err(e) = self.attempt_node_connection(device).await {
                tracing::warn!("Failed to connect to potential node at {}: {}", 
                              device_ip, e);
            }
        } else if !device.open_ports.is_empty() {
            // Log other interesting devices for potential manual addition
            tracing::debug!("Active device found: {} (ports: {:?})", 
                           device_ip, device.open_ports);
        }
    }
    
    /// Attempt to establish connection with a discovered device
    async fn attempt_node_connection(&mut self, device: DiscoveredDevice) -> Result<(), Box<dyn std::error::Error>> {
        // This would typically involve:
        // 1. Sending a ProcessDistro identification request
        // 2. Verifying the device is actually a ProcessDistro edge node
        // 3. Initiating the registration process
        
        tracing::info!("Attempting connection to ProcessDistro node at {}", device.ip_address);
        
        // For now, we'll just log the attempt
        // In a real implementation, this would use the network module
        // to establish actual communication
        
        Ok(())
    }
    
    /// Start periodic device discovery
    async fn start_periodic_discovery(&self) {
        let discovery_interval = Duration::from_secs(60); // Discover every minute
        
        tracing::info!("Starting periodic discovery (interval: {:?})", discovery_interval);
        
        // Note: In a real implementation, you'd need proper async handling
        // This is a simplified example showing the pattern
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(discovery_interval);
            
            loop {
                interval.tick().await;
                tracing::debug!("Running periodic network discovery...");
                
                // In a real implementation, you'd call the discovery service here
                // and process any new devices found
            }
        });
    }
    
    /// Manually add a node by IP address
    pub async fn add_manual_node(&mut self, ip_address: IpAddr, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Manually adding node at {}:{}", ip_address, port);
        
        // Create a discovered device entry for manual node
        let device = DiscoveredDevice {
            ip_address,
            hostname: None,
            mac_address: None,
            responded_to_ping: true,
            open_ports: vec![port],
            is_processdistro_node: true, // Assume it is since manually added
            discovered_at: Instant::now(),
        };
        
        self.attempt_node_connection(device).await?;
        
        Ok(())
    }
    
    /// Get all discovered devices (for debugging/monitoring)
    pub async fn get_discovered_devices(&self) -> Result<Vec<DiscoveredDevice>, Box<dyn std::error::Error>> {
        self.discovery_service.discover_devices().await
    }
    
    /// Register a new node
    pub fn register_node(&mut self, node_info: NodeInfo) -> NodeId {
        let node_id = Uuid::new_v4();
        let managed_node = ManagedNode {
            info: node_info,
            status: NodeStatus::Online,
            last_heartbeat: Instant::now(),
            task_count: 0,
            capabilities: NodeCapabilities {
                cpu_cores: 4, // TODO: Get from node
                memory_mb: 8192,
                wasm_runtime: "wasmtime".to_string(),
                supported_tasks: vec!["matrix_mul".to_string(), "password_hash".to_string()],
            },
        };
        
        self.nodes.insert(node_id, managed_node);
        tracing::info!("Registered new node: {} ({})", node_id, 
                      self.nodes.get(&node_id).unwrap().info.name);
        node_id
    }
    
    /// Update node heartbeat
    pub fn update_heartbeat(&mut self, node_id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.last_heartbeat = Instant::now();
            node.status = NodeStatus::Online;
        }
    }
    
    /// Check for offline nodes
    pub fn check_node_health(&mut self) {
        let timeout = Duration::from_secs(30);
        let now = Instant::now();
        
        for (node_id, node) in &mut self.nodes {
            if now.duration_since(node.last_heartbeat) > timeout {
                if !matches!(node.status, NodeStatus::Offline) {
                    tracing::warn!("Node {} went offline", node_id);
                    node.status = NodeStatus::Offline;
                    // TODO: Reassign tasks from offline nodes
                }
            }
        }
    }
    
    /// Get available nodes for task assignment
    pub fn get_available_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, node)| matches!(node.status, NodeStatus::Online))
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get node information
    pub fn get_node_info(&self, node_id: &NodeId) -> Option<&ManagedNode> {
        self.nodes.get(node_id)
    }
    
    /// Get all nodes with their status
    pub fn get_all_nodes(&self) -> &HashMap<NodeId, ManagedNode> {
        &self.nodes
    }
    
    /// Remove a node from management
    pub fn remove_node(&mut self, node_id: &NodeId) -> Option<ManagedNode> {
        if let Some(node) = self.nodes.remove(node_id) {
            tracing::info!("Removed node: {} ({})", node_id, node.info.name);
            Some(node)
        } else {
            None
        }
    }
    
    /// Update node task count
    pub fn update_node_task_count(&mut self, node_id: &NodeId, task_count: u32) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.task_count = task_count;
            
            // Update status based on load
            if task_count >= node.capabilities.cpu_cores {
                node.status = NodeStatus::Busy;
            } else if task_count > node.capabilities.cpu_cores * 2 {
                node.status = NodeStatus::Overloaded;
            } else {
                node.status = NodeStatus::Online;
            }
        }
    }
    
    /// Get network statistics
    pub fn get_network_stats(&self) -> NetworkStats {
        let total_nodes = self.nodes.len();
        let online_nodes = self.nodes.values()
            .filter(|n| matches!(n.status, NodeStatus::Online))
            .count();
        let busy_nodes = self.nodes.values()
            .filter(|n| matches!(n.status, NodeStatus::Busy))
            .count();
        let total_tasks = self.nodes.values()
            .map(|n| n.task_count)
            .sum();
        let total_cores = self.nodes.values()
            .map(|n| n.capabilities.cpu_cores)
            .sum();
        
        NetworkStats {
            total_nodes,
            online_nodes,
            busy_nodes,
            offline_nodes: total_nodes - online_nodes - busy_nodes,
            total_active_tasks: total_tasks,
            total_cpu_cores: total_cores,
            network_interfaces: self.discovery_service.network_interfaces.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub busy_nodes: usize,
    pub offline_nodes: usize,
    pub total_active_tasks: u32,
    pub total_cpu_cores: u32,
    pub network_interfaces: usize,
}