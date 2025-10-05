/// Network Discovery Example
/// 
/// Demonstrates how to use the ProcessDistro network discovery system
// Note: This example requires the controller crate to be available
// Run with: cargo run --example network_discovery_example

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    println!("ProcessDistro Network Discovery Example");
    println!("======================================");
    
    // Example 1: Basic Network Scan
    println!("\n1. Basic Network Discovery");
    println!("--------------------------");
    
    let mut discovery = DiscoveryService::new(30000);
    discovery.initialize().await?;
    
    let devices = discovery.discover_devices().await?;
    
    println!("Found {} devices on the network:", devices.len());
    for (i, device) in devices.iter().enumerate() {
        println!("  {}. {} - {}", 
                i + 1, 
                device.ip_address,
                if device.is_processdistro_node { "ProcessDistro Node" } else { "Regular Device" }
        );
        
        if !device.open_ports.is_empty() {
            println!("     Open ports: {:?}", device.open_ports);
        }
        
        if let Some(hostname) = &device.hostname {
            println!("     Hostname: {}", hostname);
        }
    }
    
    // Example 2: Node Manager Integration
    println!("\n2. Node Manager Integration");
    println!("---------------------------");
    
    let mut node_manager = NodeManager::new(30000);
    node_manager.initialize().await?;
    
    // Show network interfaces
    let interfaces = node_manager.get_network_interfaces();
    println!("Network interfaces:");
    for interface in interfaces {
        println!("  {} - {} (broadcast: {})", 
                interface.name, 
                interface.ip_address, 
                interface.broadcast_address);
    }
    
    // Show network statistics
    let stats = node_manager.get_network_stats();
    println!("\nNetwork Statistics:");
    println!("  Total nodes managed: {}", stats.total_nodes);
    println!("  Online nodes: {}", stats.online_nodes);
    println!("  Total CPU cores: {}", stats.total_cpu_cores);
    println!("  Network interfaces: {}", stats.network_interfaces);
    
    // Example 3: Manual Node Addition
    println!("\n3. Manual Node Addition");
    println!("-----------------------");
    
    // Example of manually adding a node (would typically be done via CLI)
    // node_manager.add_manual_node("192.168.1.100".parse()?, 30000).await?;
    println!("Manual node addition available via:");
    println!("  cargo run --bin controller scan");
    println!("  cargo run --bin controller start");
    
    println!("\nExample completed successfully!");
    
    Ok(())
}