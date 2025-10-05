mod executor;
mod downloader;
mod monitor;
mod communication;
mod sandbox;

use anyhow::Result;
use clap::Parser;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "processdistro-edge")]
#[command(about = "ProcessDistro edge node worker")]
struct Args {
    /// Controller address to connect to
    #[arg(short, long, default_value = "127.0.0.1:30000")]
    controller: String,
    
    /// Node name/identifier
    #[arg(short, long)]
    name: Option<String>,
    
    /// Maximum concurrent tasks
    #[arg(short, long, default_value = "4")]
    max_tasks: u32,
    
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
    
    info!("Starting ProcessDistro Edge Node");
    info!("Controller: {}", args.controller);
    info!("Max concurrent tasks: {}", args.max_tasks);
    
    // TODO: Initialize edge node components
    // - Start system monitoring
    // - Connect to controller
    // - Start task execution loop
    // - Handle graceful shutdown
    
    Ok(())
}