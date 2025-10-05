# Edge Node Module

The edge node is the worker component that executes distributed tasks. It handles:

- WASM runtime execution
- Task downloading and execution
- Result reporting
- Resource monitoring
- Connection to controller

## Modules

- `executor/` - WASM task execution engine
- `downloader/` - Task and dependency downloading
- `monitor/` - System resource monitoring
- `communication/` - Controller communication
- `sandbox/` - Security and resource isolation
