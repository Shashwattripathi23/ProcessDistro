/// Simple Web Server Launcher
/// 
/// A standalone launcher for the ProcessDistro web interface
use anyhow::Result;
use clap::Parser;
use tracing::{info, error};
use std::net::{IpAddr, Ipv4Addr};

mod web_server;
mod controller_manager;

#[derive(Parser)]
#[command(name = "processdistro-web")]
#[command(about = "ProcessDistro web interface")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "30100")]
    port: u16,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if args.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
    
    info!("Starting ProcessDistro Web Interface on port {}", args.port);
    
    // Get local IP addresses
    let local_ips = get_local_ip_addresses();
    
    // Initialize web server state
    let web_state = web_server::WebServerState::new();
    
    info!("Web dashboard will be available at:");
    info!("  Local: http://localhost:{}", args.port);
    for ip in &local_ips {
        info!("  Network: http://{}:{}", ip, args.port);
    }
    info!("");
    info!("📱 For other devices to connect:");
    info!("  1. Make sure all devices are on the same WiFi network");
    info!("  2. On other devices, open a web browser");
    for ip in &local_ips {
        info!("  3. Go to: http://{}:{}", ip, args.port);
    }
    info!("  4. The device will automatically detect capabilities and register");
    info!("");
    
    // Start web server
    if let Err(e) = web_server::start_web_server(args.port, web_state).await {
        error!("Web server failed: {}", e);
        return Err(anyhow::anyhow!("Web server startup failed: {}", e));
    }
    
    Ok(())
}

fn get_local_ip_addresses() -> Vec<IpAddr> {
    let mut ips = Vec::new();
    
    // Try to get the local IP address using the library
    if let Ok(ip) = local_ip_address::local_ip() {
        ips.push(ip);
    }
    
    // Add the specific IP we saw from ipconfig
    if let Ok(ip) = "192.168.1.7".parse::<IpAddr>() {
        if !ips.contains(&ip) {
            ips.push(ip);
        }
    }
    
    // If we couldn't detect any, add localhost
    if ips.is_empty() {
        ips.push(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }
    
    ips
}