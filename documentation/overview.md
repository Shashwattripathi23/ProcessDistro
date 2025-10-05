ProcessDistro — Distributed Computing using Edge Nodes

Project: ProcessDistro
Goal: Allow multiple devices on the same network to act as complementary CPU processors for a main node. Use Rust for core logic and WebAssembly (WASM) to enable cross-platform execution of subtasks; aggregate results at the controller.

Table of Contents

Overview

Important Key Points

Test Cases

High-level Architecture

Modules (with responsibilities)

Project Directory Layout (module-based)

Protocols & Data Flow

Scheduling & Task Stealing

Security & Permissions

Fault Tolerance & Recovery

Monitoring & Metrics

Development / Build / Run Quickstart

Roadmap & TODOs

License

1. Overview

ProcessDistro is a lightweight distributed execution framework for local networks (LAN).
The controller (main node) splits compute-heavy tasks into WASM-capable subtasks and distributes them to participating edge nodes.

Unlike traditional agent installs, ProcessDistro introduces a browser-based detection and participation system:
Any device on the network can open a webpage hosted by the controller to automatically advertise its computational capability (cores, memory, etc.) and optionally execute small benchmark tasks in-browser (via JS/WASM).

This design makes the system zero-install, cross-platform, and immediately scalable across laptops, phones, and tablets connected to the same network.

2. Important Key Points

Main UI: Terminal-first (CLI) controller for simplicity and headless operation.

Edge detection via web: Devices join by visiting a web page served by the controller.
The page collects device specs via browser APIs and reports back automatically.

Cross-platform execution: Use WASM for task portability and safe sandboxing across browsers and native runtimes.

Security trade-offs: Ephemeral auth tokens and limited browser API access to avoid privacy risks.

Test-cases: Password hashing, Matrix multiplication, Mandelbrot renderer — all can run as WASM workloads.

Task stealing: Dynamic work-stealing to balance load and enable scaling.

3. Test Cases

Matrix multiplication: Split matrices into tiles; parallelize tile multiplications.

Password hashing: Work units = password/salt pairs or iterations. Good for benchmarking CPU throughput.

Mandelbrot renderer: Split image into tiles; each node renders tiles and returns image data.

4. High-level Architecture
         ┌───────────────────────────────┐
         │        Controller (CLI)       │
         │  - Scheduler / Aggregator     │
         │  - Web Server (hosts webpage) │
         └──────────┬────────────────────┘
                    │
     (HTTP / WebSocket over LAN)
                    │
         ┌──────────┴───────────┐
         │                      │
   [Browser Edge Node]    [Browser Edge Node]
   - Opens webpage        - JS/WASM runtime
   - Reports specs        - Executes subtasks
   - Runs probes          - Returns results


Flow:

Controller hosts a web dashboard or /agent endpoint.

Any device on the LAN opens it → JS detects available resources.

Browser runs lightweight probes (CPU/memory/WASM microbenchmarks).

Data is POSTed or streamed to controller.

Controller adds node to pool and dispatches WASM subtasks via the same channel.

5. Modules (with responsibilities)

Node Detection Module (Web-based)

Serve a discovery webpage via controller.

Browser collects specs (cores, memory, GPU info) using JS APIs.

Optionally run short WASM benchmarks to estimate compute capacity.

Send JSON report back to controller via HTTP/WebSocket.

Scheduler Module (Controller)

Maintain global task queue, assign subtasks.

Implement prioritization and work-stealing logic.

Track node capabilities and performance scores.

Execution Module (Browser Edge)

Receive WASM workloads dynamically from controller.

Execute inside browser’s WebAssembly runtime (no native install).

Return results as JSON or binary payloads.

Aggregate Module

Combine partial results (e.g., matrix tiles, hashes, or render chunks).

Validate consistency and handle resubmission on errors.

Logging Module

Centralized logs on controller, client-side events logged via JS console or API.

Network Layer

HTTP + WebSocket based message passing.

Discovery via page visit (no UDP broadcast needed).

Fault Tolerance / Recovery Module

Detect disconnected browser clients via WebSocket heartbeats.

Reassign tasks automatically on timeout.

Security Layer

Tokens embedded in webpage (signed/short-lived).

Enforce CORS, origin checks, and rate limiting.

WASM sandbox ensures safe code execution.

Monitoring & Metrics Module

Controller tracks connected devices, CPU scores, completed tasks.

Optional Prometheus endpoint or simple JSON /metrics route.

Data Transfer Module

Chunked result upload (HTTP POST or WebSocket binary frames).

Compression optional for large outputs.

Resource Optimizer

Decide how many subtasks each browser runs concurrently based on benchmark results.

Configuration Module

CLI-based configs for controller server port, task repo path, and max concurrency.

6. Project Directory Layout (module-based)
processdistro/
├── controller/                # Scheduler, web server, aggregator
│   ├── src/
│   ├── static/                # Served webpages (HTML/JS/WASM agents)
│   ├── Cargo.toml
│   └── README.md
├── edge_browser/              # Web client scripts (JS/WASM)
│   ├── agent.js               # Collects device info, runs probes
│   ├── wasm_runner.js         # Executes subtasks in browser
│   └── wasm/                  # Precompiled task WASMs
├── common/                    # Shared types, protocol messages
│   ├── src/
│   ├── Cargo.toml
│   └── schema/
├── wasm_tasks/                # Native + browser WASM tasks
│   ├── password_hash/
│   ├── matrix_mul/
│   └── mandelbrot/
├── tools/
├── docs/
├── scripts/
└── README.md

7. Protocols & Data Flow

Discovery: Occurs when a user opens the controller’s web page (e.g., http://controller.local:30000/agent).

Device Reporting: The page’s JS gathers:

{
  "device_id": "uuid",
  "cores": 8,
  "memory_gb": 16,
  "browser": "Chrome 141",
  "os": "Windows 10",
  "benchmark_score": 12450
}


and sends it to the controller over HTTP or WebSocket.

Task Dispatch: Controller sends subtasks encoded as WASM binaries + JSON args through WebSocket.

Execution: Browser runs WASM, streams progress, returns result.

Aggregation: Controller validates and merges final results.

8. Scheduling & Task Stealing

Controller maintains a global work-stealing queue.

Browsers can “pull” new tasks if idle — reducing central scheduling overhead.

Dynamic node scoring adjusts based on measured benchmark performance.

9. Security & Permissions

WASM sandboxing: Browsers isolate execution natively — no file or network access beyond the script.

Auth tokens: Page embeds a short-lived access token per node session.

CORS & Origin: Only controller’s domain allowed.

HTTPS recommended: Required for advanced browser APIs (Battery, NetworkInfo, WebGL introspection).

Consent prompt: Before running any probe or WASM workload, user must explicitly confirm participation.

10. Fault Tolerance & Recovery

Heartbeat (WebSocket pings): Controller detects inactive browser sessions automatically.

Reassign tasks: Lost subtasks are redistributed to active nodes.

Checkpointing (optional): Long WASM tasks can periodically report progress.

Redundancy: Run occasional duplicate tasks across multiple nodes to ensure correctness.

11. Monitoring & Metrics

CLI view + optional /metrics JSON endpoint.

Example metrics:

nodes_connected  = 12
avg_score        = 8750 ops/sec
tasks_queued     = 8
tasks_running    = 24
tasks_completed  = 156
avg_latency_ms   = 73


Optional simple browser dashboard for live visual tracking.

12. Development / Build / Run Quickstart

Install Rust & Cargo

rustc --version
cargo --version


Create workspace

cargo new --workspace processdistro
cd processdistro


Add controller + web server

Serve /static/agent.html and /static/agent.js

Expose /node-report API to collect device info.

Build sample WASM tasks

rustup target add wasm32-unknown-unknown
cd wasm_tasks/matrix_mul
cargo build --target wasm32-unknown-unknown --release


Run controller

cargo run --bin controller start --port 30000


On any LAN device:
Open http://controller.local:30000/agent → click “Join Network” → device auto-registers.

13. Roadmap & TODOs

 Switch to browser-based node detection and participation

 Implement /node-report handler in Rust controller

 Build benchmark scoring in JS/WASM

 Add task assignment over WebSocket

 Implement matrix multiplication task

 Add security & token-based access

 Add optional GUI monitor dashboard

14. License

MIT or Apache-2.0 (choose based on contribution policy)

Appendix: Notes & Recommendations

Prefer WASM tasks that run consistently both natively and in-browser.

Keep web probes short (<1s) and ask consent before running them.

For production, serve pages via HTTPS and restrict allowed origins.

CLI remains the central command and monitoring interface — webpage nodes are just temporary compute peers.

End of document.