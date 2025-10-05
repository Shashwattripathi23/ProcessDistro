mod scheduler;
mod aggregator;
mod node_manager;
mod cli;
mod metrics;
mod network;
mod security;
mod fault_tolerance;
mod web_server;
mod controller_manager;

use anyhow::Result;
use clap::Parser;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "processdistro-controller")]
#[command(about = "ProcessDistro distributed computing controller")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Start the controller
    Start {
        /// Port to listen on
        #[arg(short, long, default_value = "30000")]
        port: u16,
        
        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,
        
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    /// Scan network for devices
    Scan {
        /// Port to use for discovery
        #[arg(short, long, default_value = "30000")]
        port: u16,
        
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    match args.command {
        Commands::Start { port, config, debug } => {
            start_controller(port, config, debug).await
        }
        Commands::Scan { port, debug } => {
            scan_network(port, debug).await
        }
    }
}

async fn start_controller(port: u16, _config: Option<String>, debug: bool) -> Result<()> {
    // Initialize logging
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
    
    info!("Starting ProcessDistro Controller on port {}", port);
    
    // Initialize web server state
    let web_state = web_server::WebServerState::new();
    
    // Initialize node manager with discovery service
    let mut node_manager = node_manager::NodeManager::new(port);
    
    match node_manager.initialize().await {
        Ok(()) => {
            info!("Node manager and discovery service initialized successfully");
            
            // Display network information
            info!("Controller networking initialized");
            
            // Start device discovery
            match node_manager.get_discovered_devices().await {
                Ok(devices) => {
                    info!("Initial network scan found {} devices:", devices.len());
                    for device in &devices {
                        if device.is_processdistro_node {
                            info!("  ProcessDistro Node: {} ({})", 
                                 device.ip_address, 
                                 device.hostname.as_deref().unwrap_or("unknown"));
                        } else {
                            info!("  Device: {} - ports: {:?}", 
                                 device.ip_address, device.open_ports);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to perform initial device discovery: {}", e);
                }
            }
            
            // Show network statistics
            let stats = node_manager.get_network_stats();
            info!("Network Statistics:");
            info!("  Total nodes: {}", stats.total_nodes);
            info!("  Online nodes: {}", stats.online_nodes);
            info!("  Network interfaces: {}", stats.network_interfaces);
            
            // Start web server on a different port (HTTP)
            let web_port = port + 100; // Use port + 100 for web interface
            let web_state_clone = web_state.clone();
            
            tokio::spawn(async move {
                if let Err(e) = web_server::start_web_server(web_port, web_state_clone).await {
                    error!("Web server failed: {}", e);
                }
            });
            
            info!("Web dashboard available at: http://localhost:{}", web_port);
            info!("Devices can register by visiting the dashboard URL");
            
            // TODO: Start other controller services
            // - Start scheduler
            // - Start metrics collection
            // - Start CLI interface
            // - Start network listener
            
            // Keep the controller running
            info!("Controller started successfully. Press Ctrl+C to stop.");
            
            // Wait for shutdown signal
            tokio::signal::ctrl_c().await?;
            info!("Shutdown signal received, stopping controller...");
        }
        Err(e) => {
            error!("Failed to initialize node manager: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn scan_network(port: u16, debug: bool) -> Result<()> {
    // Initialize logging
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
    
    info!("Scanning network for ProcessDistro devices...");
    
    // Initialize discovery service
    let mut discovery_service = node_manager::DiscoveryService::new(port);
    match discovery_service.initialize().await {
        Ok(_) => info!("Discovery service initialized"),
        Err(e) => {
            error!("Failed to initialize discovery service: {}", e);
            return Err(anyhow::anyhow!("Discovery initialization failed"));
        }
    }
    
    // Perform device discovery
    let devices = match discovery_service.discover_devices().await {
        Ok(devices) => devices,
        Err(e) => {
            error!("Failed to discover devices: {}", e);
            return Err(anyhow::anyhow!("Device discovery failed"));
        }
    };
    
    println!("\n🔍 Network Scan Results");
    println!("=======================");
    
    if devices.is_empty() {
        println!("No devices found on the network.");
    } else {
        println!("Found {} devices:\n", devices.len());
        
        let mut processdistro_nodes = 0;
        let mut other_devices = 0;
        
        for device in &devices {
            if device.is_processdistro_node {
                processdistro_nodes += 1;
                println!("✅ ProcessDistro Node");
            } else {
                other_devices += 1;
                println!("📱 Network Device");
            }
            
            println!("   IP: {}", device.ip_address);
            if let Some(hostname) = &device.hostname {
                println!("   Hostname: {}", hostname);
            }
            println!("   Open Ports: {:?}", device.open_ports);
            println!("   Discovered: {:?} ago", device.discovered_at.elapsed());
            println!();
        }
        
        println!("Summary:");
        println!("  ProcessDistro Nodes: {}", processdistro_nodes);
        println!("  Other Devices: {}", other_devices);
        println!("  Total: {}", devices.len());
    }
    
    Ok(())
}