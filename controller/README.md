# Controller Module

The controller is the main orchestrator for ProcessDistro. It handles:

- Node discovery and management
- Task scheduling and distribution
- Result aggregation
- Fault tolerance and recovery
- CLI interface for user interaction

## Modules

- `scheduler/` - Task queue and work-stealing logic
- `aggregator/` - Result combination and validation
- `node_manager/` - Node discovery and capability tracking
- `cli/` - Command-line interface
- `metrics/` - Performance monitoring and metrics collection
- `network/` - Network communication layer
- `security/` - Authentication and authorization
- `fault_tolerance/` - Error recovery and resilience

## Network Discovery Features

The controller includes comprehensive network discovery capabilities:

### Discovery Methods

- **UDP Broadcast**: Discovers ProcessDistro nodes via network broadcasts
- **Network Scanning**: Scans IP ranges for active devices and open ports
- **mDNS Discovery**: Uses multicast DNS for service discovery
- **Manual Addition**: Allows manual addition of nodes by IP address

### Usage Examples

```bash
# Start controller with full discovery
cargo run --bin controller start --port 30000

# Scan network for devices only
cargo run --bin controller scan --port 30000 --debug

# View discovered network interfaces
cargo run --example network_discovery_example
```

### Supported Network Types

- IPv4 private networks (10.x.x.x, 172.16-31.x.x, 192.168.x.x)
- Automatic subnet detection
- Multi-interface support
- Broadcast and unicast discovery
