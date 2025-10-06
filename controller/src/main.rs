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
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start { port } => {
            start_controller(port, false).await
        }
        Commands::Status => {
            show_status().await
        }
        Commands::ListNodes => {
            list_nodes().await
        }
        Commands::SubmitTask { task_type, params, input: _ } => {
            submit_task(task_type, params).await
        }
        Commands::CancelTask { task_id } => {
            cancel_task(task_id).await
        }
        Commands::Metrics => {
            show_metrics().await
        }
        Commands::Progress { task_id } => {
            show_progress(task_id).await
        }
        Commands::DistributeTasks { count } => {
            distribute_tasks(count).await
        }
        Commands::ShowStats => {
            show_enhanced_stats().await
        }
        Commands::ShowNodes => {
            show_node_details().await
        }
        Commands::Scan { port } => {
            scan_network(port, false).await
        }
        Commands::TestMandelbrot { width, height, max_iterations } => {
            test_mandelbrot_distributed(width, height, max_iterations).await
        }
    }
}

async fn start_controller(port: u16, debug: bool) -> Result<()> {
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
            // Additionally listen for a simple terminal command to run password-hash test
            let controller_clone = web_state.clone();
            tokio::spawn(async move {
                use std::io::{self, Write};
                loop {
                    print!("\n🔧 ProcessDistro Controller Commands:\n");
                    print!("  'run-mandelbrot-test' - Execute Mandelbrot fractal rendering on all nodes\n");
                    print!("  'exit' - Quit controller\n");
                    print!("📝 Enter command: ");
                    io::stdout().flush().ok();
                    
                    let mut input = String::new();
                    if let Ok(_) = io::stdin().read_line(&mut input) {
                        let cmd = input.trim();
                        if cmd.eq_ignore_ascii_case("run-mandelbrot-test") {
                            // Get active nodes count for confirmation
                            let active_nodes = controller_clone.controller.get_active_nodes().await;
                            
                            if active_nodes.is_empty() {
                                println!("❌ No active nodes found! Please ensure nodes are connected before running tests.");
                                continue;
                            }
                            
                            println!("\n🎨 Enhanced Distributed Mandelbrot Test Configuration:");
                            println!("  • Canvas: 1200x900 pixels (intelligently tiled across nodes)");
                            println!("  • Algorithm: Enhanced distributed Mandelbrot with intelligent load balancing");
                            println!("  • Max iterations: 1000 (adjusted per node capability)");
                            println!("  • Target nodes: {} active node(s)", active_nodes.len());
                            println!("  • Distribution: Performance-based tile assignment");
                            println!("  • Estimated duration: Optimized based on node capabilities");
                            
                            print!("\n✨ Ready to render beautiful fractals? (y/N): ");
                            io::stdout().flush().ok();
                            
                            let mut confirmation = String::new();
                            if let Ok(_) = io::stdin().read_line(&mut confirmation) {
                                let confirm = confirmation.trim();
                                if confirm.eq_ignore_ascii_case("y") || confirm.eq_ignore_ascii_case("yes") {
                                    println!("\n🚀 Starting Enhanced Distributed Mandelbrot rendering on all {} nodes...", active_nodes.len());
                                    let task_ids = controller_clone.controller.run_mandelbrot_test(1200, 900, 1000).await;
                                    println!("✅ Successfully dispatched {} distributed tasks", task_ids.len());
                                    println!("🎯 Task IDs: {:?}", task_ids);
                                    println!("🎨 Watch live canvas rendering on dashboard: http://localhost:30100");
                                } else {
                                    println!("❌ Test cancelled.");
                                }
                            }
                        } else if cmd.eq_ignore_ascii_case("exit") {
                            println!("🛑 Exit command received, shutting down controller...");
                            std::process::exit(0);
                        } else if !cmd.is_empty() {
                            println!("❓ Unknown command: '{}'. Try 'run-mandelbrot-test' or 'exit'.", cmd);
                        }
                    }
                }
            });

            tokio::signal::ctrl_c().await?;
            info!("Shutdown signal received, stopping controller...");
        }
        Err(e) => {
            error!("Failed to initialize node manager: {}", e);
            return Err(anyhow::anyhow!("Node manager initialization failed: {}", e));
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

async fn show_status() -> Result<()> {
    println!("Controller Status: Running");
    // TODO: Implement actual status check
    Ok(())
}

async fn list_nodes() -> Result<()> {
    println!("Connected Nodes:");
    // TODO: List actual connected nodes
    Ok(())
}

async fn submit_task(task_type: String, params: String) -> Result<()> {
    println!("Submitting {} task with params: {}", task_type, params);
    // TODO: Submit actual task
    Ok(())
}

async fn cancel_task(task_id: String) -> Result<()> {
    println!("Cancelling task: {}", task_id);
    // TODO: Cancel actual task
    Ok(())
}

async fn show_metrics() -> Result<()> {
    println!("Performance Metrics:");
    // TODO: Show actual metrics
    Ok(())
}

async fn show_progress(task_id: Option<String>) -> Result<()> {
    match task_id {
        Some(id) => println!("Progress for task {}: 0%", id),
        None => println!("All task progress: No active tasks"),
    }
    // TODO: Show actual progress
    Ok(())
}

async fn distribute_tasks(count: usize) -> Result<()> {
    println!("Distributing {} tasks intelligently across connected nodes...", count);
    // TODO: Implement intelligent task distribution using enhanced controller manager
    Ok(())
}

async fn show_enhanced_stats() -> Result<()> {
    println!("Enhanced Network Statistics:");
    // TODO: Show enhanced network statistics using get_enhanced_network_stats()
    Ok(())
}

async fn show_node_details() -> Result<()> {
    println!("Detailed Node Information:");
    // TODO: Show detailed node capabilities and performance metrics
    Ok(())
}

async fn test_mandelbrot_distributed(width: u32, height: u32, max_iterations: u32) -> Result<()> {
    println!("🎨 Starting Enhanced Distributed Mandelbrot Test");
    println!("📐 Canvas: {}x{}, Iterations: {}", width, height, max_iterations);
    println!("🧠 Using intelligent node distribution algorithm...");
    
    // TODO: Initialize controller manager and call the enhanced run_mandelbrot_test
    // For now, just show what would happen
    println!("✅ This would distribute Mandelbrot tiles intelligently across connected nodes");
    println!("🎯 Each node would receive optimized tiles based on their performance capabilities");
    println!("⚡ Load balancing would ensure efficient resource utilization");
    
    Ok(())
}