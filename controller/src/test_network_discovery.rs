// Cargo.toml dependencies:
// [dependencies]
// tokio = { version = "1", features = ["full"] }
// socket2 = "0.5"
// local-ip-address = "0.6"
// futures = "0.3"
// mdns-sd = "0.11"
// dns-lookup = "2.0"
// anyhow = "1.0"
// libc = "0.2"  # For Unix hostname (optional)

use anyhow::{Context, Result};
use futures::future::join_all;
use local_ip_address;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use socket2::{Domain, SockAddr, Socket, Type};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Discovered device information
#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub ip: String,
    pub hostname: Option<String>,
    pub open_ports: Vec<u16>,
    pub mac_address: Option<String>,
    pub device_type: String,
    pub discovery_method: DiscoveryMethod,
    pub response_time_ms: u128,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiscoveryMethod {
    PortScan,
    MDns,
    ArpScan,
    Broadcast,
    Local, // Added for local machine detection
}

impl std::fmt::Display for DiscoveryMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DiscoveryMethod::PortScan => write!(f, "Port Scan"),
            DiscoveryMethod::MDns => write!(f, "mDNS"),
            DiscoveryMethod::ArpScan => write!(f, "ARP"),
            DiscoveryMethod::Broadcast => write!(f, "Broadcast"),
            DiscoveryMethod::Local => write!(f, "Local Detection"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║         ProcessDistro - Network Device Discovery             ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    let start = Instant::now();
    
    // Test 1: Get local network interfaces
    println!("📡 Step 1: Detecting Network Interfaces");
    println!("─────────────────────────────────────────");
    match local_ip_address::list_afinet_netifas() {
        Ok(interfaces) => {
            for (name, ip) in interfaces.iter() {
                println!("  ✓ {} → {}", name, ip);
            }
        }
        Err(e) => println!("  ✗ Error: {}", e),
    }

    // Test 2: UDP Broadcast capability
    println!("\n🔊 Step 2: Testing UDP Broadcast Capability");
    println!("─────────────────────────────────────────");
    match test_udp_broadcast().await {
        Ok(_) => println!("  ✓ UDP broadcast is functional"),
        Err(e) => println!("  ✗ UDP broadcast error: {}", e),
    }

    // Test 3: mDNS capability
    println!("\n🔍 Step 3: Testing mDNS Service Discovery");
    println!("─────────────────────────────────────────");
    match test_mdns().await {
        Ok(_) => println!("  ✓ mDNS service is functional"),
        Err(e) => println!("  ✗ mDNS error: {}", e),
    }

    // Test 4: Comprehensive Network Discovery
    println!("\n🌐 Step 4: Comprehensive Network Device Scan");
    println!("─────────────────────────────────────────");
    match discover_all_devices().await {
        Ok(devices) => {
            println!("\n✅ Discovery Complete!");
            println!("  Total scan time: {:.2}s", start.elapsed().as_secs_f32());
            println!("  Devices found: {}\n", devices.len());
            
            print_device_summary(&devices);
        }
        Err(e) => println!("  ✗ Discovery error: {}", e),
    }

    Ok(())
}

/// Comprehensive device discovery using multiple methods
async fn discover_all_devices() -> Result<Vec<DiscoveredDevice>> {
    let local_ip = local_ip_address::local_ip()?;
    
    if let IpAddr::V4(ipv4) = local_ip {
        let base = format!(
            "{}.{}.{}.",
            ipv4.octets()[0],
            ipv4.octets()[1],
            ipv4.octets()[2]
        );

        println!("  Subnet: {}0/24", base);
        println!("  Local IP: {} (will be excluded from scan)", local_ip);
        
        // Use all discovery methods in parallel
        let (scan_devices, arp_devices) = tokio::join!(
            parallel_port_scan(&base, &local_ip),
            arp_scan(&base, &local_ip)
        );

        // Merge results from different methods
        let mut all_devices = HashMap::new();
        
        // Always add local device first with full info
        let local_device = get_local_device_info(local_ip).await?;
        all_devices.insert(local_device.ip.clone(), local_device);
        
        // Add port scan results
        if let Ok(devices) = scan_devices {
            for device in devices {
                all_devices.insert(device.ip.clone(), device);
            }
        }

        // Merge ARP scan results
        if let Ok(devices) = arp_devices {
            for device in devices {
                all_devices.entry(device.ip.clone())
                    .and_modify(|d| {
                        if device.mac_address.is_some() {
                            d.mac_address = device.mac_address.clone();
                        }
                    })
                    .or_insert(device);
            }
        }

        let mut result: Vec<_> = all_devices.into_values().collect();
        result.sort_by(|a, b| {
            let a_octet: u8 = a.ip.split('.').last().unwrap().parse().unwrap_or(0);
            let b_octet: u8 = b.ip.split('.').last().unwrap().parse().unwrap_or(0);
            a_octet.cmp(&b_octet)
        });

        Ok(result)
    } else {
        Err(anyhow::anyhow!("IPv6 not currently supported"))
    }
}

/// Get information about the local device
async fn get_local_device_info(local_ip: IpAddr) -> Result<DiscoveredDevice> {
    // Try to get hostname using system call
    let hostname = tokio::task::spawn_blocking(|| {
        #[cfg(unix)]
        {
            use std::ffi::CStr;
            let mut buf = [0u8; 256];
            unsafe {
                if libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) == 0 {
                    CStr::from_ptr(buf.as_ptr() as *const libc::c_char)
                        .to_string_lossy()
                        .into_owned()
                        .into()
                } else {
                    None
                }
            }
        }
        #[cfg(windows)]
        {
            std::env::var("COMPUTERNAME").ok()
        }
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
    })
    .await
    .ok()
    .flatten();

    Ok(DiscoveredDevice {
        ip: local_ip.to_string(),
        hostname,
        open_ports: vec![9000], // ProcessDistro port
        mac_address: None,
        device_type: "ProcessDistro Node (LOCAL MACHINE)".to_string(),
        discovery_method: DiscoveryMethod::Local,
        response_time_ms: 0,
    })
}

/// Fast parallel port scanning with optimized port selection
async fn parallel_port_scan(base: &str, local_ip: &IpAddr) -> Result<Vec<DiscoveredDevice>> {
    // Optimized port list - most common ports first
    let ports: Vec<u16> = vec![
        // Most common first (higher hit rate)
        80, 443, 22, 8080, 9000,
        // Web services
        8000, 8443, 8888, 3000, 5000,
        // File sharing
        21, 445, 139, 137, 138, 135,
        // Remote access
        23, 3389, 5900,
        // DNS/Network
        53, 161, 162,
        // Media/Streaming
        554, 1900, 8089, 32400, 8096,
        // IoT/Smart devices
        1883, 8883, 1880, 7001, 8081,
        // Mobile
        5555, 62078,
    ];

    println!("  Scanning {} IPs across {} ports...", 254, ports.len());
    
    // Get local IP's last octet to skip it
    let local_last_octet = if let IpAddr::V4(ipv4) = local_ip {
        ipv4.octets()[3]
    } else {
        0
    };
    
    let mut tasks = Vec::new();
    let base = base.to_string();

    // Spawn parallel scan tasks for each IP (except local)
    for i in 1..=254 {
        if i == local_last_octet {
            println!("  Skipping local IP: {}{}", base, i);
            continue; // Skip scanning our own machine
        }
        
        let base_clone = base.clone();
        let ports_clone = ports.clone();

        let task = tokio::spawn(async move {
            scan_host(&base_clone, i, &ports_clone).await
        });

        tasks.push(task);
    }

    // Collect results
    let results = join_all(tasks).await;
    let mut devices = Vec::new();

    for result in results {
        if let Ok(Some(device)) = result {
            devices.push(device);
        }
    }

    Ok(devices)
}

/// Scan a single host for open ports
async fn scan_host(base: &str, host_num: u8, ports: &[u16]) -> Option<DiscoveredDevice> {
    let ip = format!("{}{}", base, host_num);
    let mut open_ports = Vec::new();
    let scan_start = Instant::now();

    // Quick initial check on most common port to avoid wasting time
    let quick_check = format!("{}:80", ip);
    let has_port_80 = timeout(Duration::from_millis(100), TcpStream::connect(&quick_check))
        .await
        .is_ok();

    if !has_port_80 {
        // If port 80 fails, do a faster scan with shorter timeout
        for &port in ports.iter().take(10) {
            // Only check first 10 ports
            if let Ok(Ok(_)) = timeout(
                Duration::from_millis(50),
                TcpStream::connect(format!("{}:{}", ip, port)),
            )
            .await
            {
                open_ports.push(port);
            }
        }
    } else {
        open_ports.push(80);
        // If port 80 is open, check all ports more thoroughly
        for &port in ports {
            if port == 80 {
                continue;
            }
            if let Ok(Ok(_)) = timeout(
                Duration::from_millis(100),
                TcpStream::connect(format!("{}:{}", ip, port)),
            )
            .await
            {
                open_ports.push(port);
            }
        }
    }

    if open_ports.is_empty() {
        return None;
    }

    let response_time = scan_start.elapsed().as_millis();

    // Try to get hostname (with timeout)
    let hostname = timeout(Duration::from_secs(1), tokio::task::spawn_blocking({
        let ip_clone = ip.clone();
        move || dns_lookup::lookup_addr(&ip_clone.parse().ok()?).ok()
    }))
    .await
    .ok()
    .and_then(|r| r.ok())
    .flatten();

    Some(DiscoveredDevice {
        ip: ip.clone(),
        hostname,
        open_ports: open_ports.clone(),
        mac_address: None,
        device_type: identify_device_type(&open_ports),
        discovery_method: DiscoveryMethod::PortScan,
        response_time_ms: response_time,
    })
}

/// ARP scan to find devices (works even if all ports are filtered)
async fn arp_scan(base: &str, local_ip: &IpAddr) -> Result<Vec<DiscoveredDevice>> {
    // Note: ARP scanning requires elevated privileges on most systems
    // This is a simplified version using ICMP ping as fallback
    
    println!("  Running ARP/Ping sweep...");
    
    // Get local IP's last octet to skip it
    let local_last_octet = if let IpAddr::V4(ipv4) = local_ip {
        ipv4.octets()[3]
    } else {
        0
    };
    
    let mut tasks = Vec::new();
    let base = base.to_string();

    for i in 1..=254 {
        if i == local_last_octet {
            continue; // Skip local machine
        }
        
        let ip = format!("{}{}", base, i);
        
        let task = tokio::spawn(async move {
            // Try ICMP ping using system ping command
            let output = tokio::process::Command::new("ping")
                .arg("-c")
                .arg("1")
                .arg("-W")
                .arg("1")
                .arg(&ip)
                .output()
                .await;

            if let Ok(output) = output {
                if output.status.success() {
                    return Some(DiscoveredDevice {
                        ip: ip.clone(),
                        hostname: None,
                        open_ports: vec![],
                        mac_address: None,
                        device_type: "Ping Responsive".to_string(),
                        discovery_method: DiscoveryMethod::ArpScan,
                        response_time_ms: 0,
                    });
                }
            }
            None
        });

        tasks.push(task);
    }

    let results = join_all(tasks).await;
    let devices: Vec<_> = results.into_iter().filter_map(|r| r.ok().flatten()).collect();

    Ok(devices)
}

/// Identify device type based on open ports
fn identify_device_type(ports: &[u16]) -> String {
    let port_set: HashSet<_> = ports.iter().copied().collect();
    let mut types = Vec::new();

    if port_set.contains(&9000) {
        types.push("ProcessDistro Node");
    }
    if port_set.contains(&80) || port_set.contains(&443) {
        types.push("Web Server");
    }
    if port_set.contains(&22) {
        types.push("SSH Server");
    }
    if port_set.contains(&445) || port_set.contains(&139) {
        types.push("Windows/SMB");
    }
    if port_set.contains(&3389) {
        types.push("Windows RDP");
    }
    if port_set.contains(&32400) {
        types.push("Plex Media Server");
    }
    if port_set.contains(&8096) {
        types.push("Jellyfin Server");
    }
    if port_set.contains(&554) {
        types.push("IP Camera/RTSP");
    }
    if port_set.contains(&1900) {
        types.push("UPnP/DLNA Device");
    }
    if port_set.contains(&1883) || port_set.contains(&8883) {
        types.push("MQTT/IoT Device");
    }
    if port_set.contains(&5555) {
        types.push("Android Device (ADB)");
    }

    if types.is_empty() {
        "Unknown Device".to_string()
    } else {
        types.join(", ")
    }
}

/// Test UDP broadcast capability
async fn test_udp_broadcast() -> Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    socket.set_broadcast(true)?;

    let broadcast_addr = SockAddr::from(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
        8080,
    ));

    let test_message = b"ProcessDistro-Discovery-Test";
    socket
        .send_to(test_message, &broadcast_addr)
        .context("Failed to send broadcast")?;

    Ok(())
}

/// Test mDNS service discovery
async fn test_mdns() -> Result<()> {
    let _daemon = ServiceDaemon::new().context("Failed to create mDNS daemon")?;
    let mut properties = HashMap::new();
    properties.insert("version".to_string(), "1.0".to_string());

    let service_info = ServiceInfo::new(
        "_processdistro._tcp.local.",
        "test-node",
        "localhost.",
        "127.0.0.1",
        8080,
        Some(properties),
    )
    .context("Failed to create service info")?;

    println!("  ✓ Created test service: {}", service_info.get_fullname());

    Ok(())
}

/// Print formatted device summary
fn print_device_summary(devices: &[DiscoveredDevice]) {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    DISCOVERED DEVICES                         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // Group by device type
    let mut by_type: HashMap<String, Vec<&DiscoveredDevice>> = HashMap::new();
    for device in devices {
        by_type
            .entry(device.device_type.clone())
            .or_default()
            .push(device);
    }

    for (device_type, devices) in by_type.iter() {
        println!("┌─ {} ({} devices)", device_type, devices.len());
        for device in devices {
            println!("│");
            println!("│  📍 IP: {}", device.ip);
            
            if let Some(hostname) = &device.hostname {
                println!("│  🏷️  Hostname: {}", hostname);
            }
            
            if !device.open_ports.is_empty() {
                println!("│  🔌 Open Ports: {:?}", device.open_ports);
            }
            
            if let Some(mac) = &device.mac_address {
                println!("│  🔗 MAC: {}", mac);
            }
            
            println!("│  🔍 Method: {}", device.discovery_method);
            println!("│  ⏱️  Response: {}ms", device.response_time_ms);
        }
        println!("└─\n");
    }

    // Statistics
    println!("📊 Statistics:");
    println!("  Total Devices: {}", devices.len());
    
    let with_hostnames = devices.iter().filter(|d| d.hostname.is_some()).count();
    println!("  With Hostnames: {}", with_hostnames);
    
    let with_open_ports = devices.iter().filter(|d| !d.open_ports.is_empty()).count();
    println!("  With Open Ports: {}", with_open_ports);
    
    let process_distro_nodes = devices
        .iter()
        .filter(|d| d.device_type.contains("ProcessDistro"))
        .count();
    println!("  ProcessDistro Nodes: {}", process_distro_nodes);
}