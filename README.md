# ProcessDistro — Distributed Computing using Edge Nodes

[![Build Status](https://img.shields.io/badge/build-passing-green)](https://github.com/username/processdistro)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-orange)](https://www.rust-lang.org)

ProcessDistro is a lightweight distributed execution framework for local networks (LAN). The master/controller node splits compute-heavy tasks into WASM-capable subtasks and distributes them to participating edge nodes. Edge nodes execute subtasks in sandboxed WASM runtimes and return results. The controller aggregates results and handles scheduling, work-stealing, failure recovery, and metrics.

## 🎯 Key Features

- **Cross-platform execution** - Uses WebAssembly (WASM) for portable task execution
- **Terminal-first CLI** - Simple command-line interface for headless operation
- **Dynamic work-stealing** - Automatic load balancing across nodes
- **Fault tolerance** - Automatic task reassignment and recovery
- **Security-focused** - Sandboxed WASM execution with resource limits
- **Real-time metrics** - Performance monitoring and reporting

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+ with `cargo`
- WASM target: `rustup target add wasm32-unknown-unknown`

### Building

```powershell
# Clone the repository
git clone https://github.com/username/processdistro.git
cd processdistro

# Build all components
./scripts/build_all.ps1 -Release

# Or build individually
cargo build --release
```

### Running

1. **Start the Controller**:

   ```powershell
   cargo run --bin controller -- start --port 30000
   ```

2. **Start Edge Nodes** (on each participating machine):

   ```powershell
   cargo run --bin edge_node -- --controller 192.168.1.100:30000 --max-tasks 4
   ```

3. **Submit Tasks**:
   ```powershell
   cargo run --bin controller -- submit-task --task-type matrix_mul --params '{"size":512}'
   ```

## 📁 Project Structure

```
processdistro/
├── controller/           # Main controller and scheduler
│   ├── src/
│   │   ├── main.rs      # CLI entry point
│   │   ├── scheduler.rs  # Task scheduling and work-stealing
│   │   ├── aggregator.rs # Result aggregation
│   │   ├── node_manager.rs # Node discovery and management
│   │   ├── network.rs   # Network communication
│   │   ├── security.rs  # Authentication and sandboxing
│   │   ├── fault_tolerance.rs # Error recovery
│   │   └── metrics.rs   # Performance monitoring
│   └── Cargo.toml
├── edge_node/           # Worker node implementation
│   ├── src/
│   │   ├── main.rs      # Edge node entry point
│   │   ├── executor.rs  # WASM task execution
│   │   ├── downloader.rs # Task downloading
│   │   ├── monitor.rs   # System monitoring
│   │   ├── communication.rs # Controller communication
│   │   └── sandbox.rs   # Security and isolation
│   └── Cargo.toml
├── common/              # Shared types and protocols
│   ├── src/
│   │   ├── types.rs     # Data structures
│   │   ├── protocol.rs  # Message definitions
│   │   └── errors.rs    # Error types
│   └── proto/           # Protocol schemas
├── wasm_tasks/          # Task implementations
│   ├── password_hash/   # CPU-intensive hashing
│   ├── matrix_mul/      # Parallelizable matrix ops
│   └── mandelbrot/      # Embarrassingly parallel rendering
├── tools/               # Development utilities
│   ├── benchmark.py     # Performance benchmarking
│   └── node_simulator.py # Testing utilities
├── examples/            # Usage examples
├── scripts/             # Build and deployment scripts
└── documentation/       # Project documentation
```

## 🔧 Supported Task Types

### Matrix Multiplication

- **Use case**: Large matrix computations, ML workloads
- **Parallelization**: Tile-based splitting
- **Parameters**: `matrix_size`, `tile_size`

### Password Hashing

- **Use case**: Bulk password processing, security auditing
- **Parallelization**: Batch processing
- **Algorithms**: bcrypt, SHA-256, SHA-512

### Mandelbrot Rendering

- **Use case**: Fractal generation, graphics processing
- **Parallelization**: Image tile rendering
- **Parameters**: `width`, `height`, `max_iterations`

## 🛡️ Security Features

- **WASM Sandboxing**: Tasks run in isolated WASM environments
- **Resource Limits**: Memory and CPU time constraints
- **Authentication**: Ephemeral tokens for node authentication
- **Import Validation**: Restricted WASM imports for security
- **Network Isolation**: Tasks cannot access host network by default

## 🎛️ Configuration

Example configuration file:

```json
{
  "controller": {
    "port": 30000,
    "max_nodes": 50,
    "task_timeout": "300s",
    "work_stealing_enabled": true
  },
  "security": {
    "token_duration": "24h",
    "require_tls": false,
    "sandbox_memory_limit": 67108864
  },
  "performance": {
    "max_retries": 3,
    "retry_delay": "5s",
    "task_queue_size": 1000
  }
}
```

## 📊 Monitoring & Metrics

ProcessDistro provides comprehensive monitoring:

- **Real-time metrics**: Tasks/sec, CPU usage, memory consumption
- **Node health**: Heartbeat monitoring, failure detection
- **Task tracking**: Progress, completion rates, error analysis
- **Prometheus export**: Optional metrics endpoint for external monitoring

Access metrics via CLI:

```powershell
cargo run --bin controller -- metrics
```

## 🧪 Testing

Run the complete test suite:

```powershell
# Unit tests
./scripts/test_all.ps1 -Unit

# Integration tests
./scripts/test_all.ps1 -Integration

# Performance benchmarks
python tools/benchmark.py
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Commit changes: `git commit -m 'Add amazing feature'`
4. Push to branch: `git push origin feature/amazing-feature`
5. Open a Pull Request

### Development Setup

```powershell
# Install dependencies
rustup target add wasm32-unknown-unknown
cargo install cargo-tarpaulin  # For coverage

# Setup development environment
./scripts/setup_dev.ps1

# Run development build
./scripts/build_all.ps1
```

## 📈 Roadmap

- [ ] **v0.1.0**: Basic controller/edge communication
- [ ] **v0.2.0**: WASM task execution and matrix multiplication
- [ ] **v0.3.0**: Work-stealing and fault tolerance
- [ ] **v0.4.0**: Security hardening and authentication
- [ ] **v0.5.0**: Web-based monitoring dashboard
- [ ] **v1.0.0**: Production-ready release

## 🐛 Known Issues

- WASM task debugging tools are limited
- No built-in task result persistence yet
- Limited to LAN environments (no internet-scale distribution)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [wasmtime](https://github.com/bytecodealliance/wasmtime) - WASM runtime
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime
- [serde](https://github.com/serde-rs/serde) - Serialization framework

## 📞 Support

- **Documentation**: [docs/](documentation/)
- **Issues**: [GitHub Issues](https://github.com/username/processdistro/issues)
- **Discussions**: [GitHub Discussions](https://github.com/username/processdistro/discussions)

---

**ProcessDistro** - Turning your local network into a distributed computing cluster, one WASM task at a time! 🚀
