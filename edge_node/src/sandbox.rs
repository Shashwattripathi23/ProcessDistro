/// Sandbox Module
/// 
/// Security and resource isolation for WASM task execution
use wasmtime::*;
use std::time::Duration;

pub struct SandboxConfig {
    pub memory_limit: u64,      // Memory limit in bytes
    pub cpu_time_limit: Duration, // Maximum execution time
    pub allowed_imports: Vec<String>, // Allowed WASM imports
    pub network_access: bool,   // Allow network access
    pub filesystem_access: bool, // Allow filesystem access
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit: 64 * 1024 * 1024, // 64MB
            cpu_time_limit: Duration::from_secs(300), // 5 minutes
            allowed_imports: vec![
                "wasi_snapshot_preview1".to_string(),
            ],
            network_access: false,
            filesystem_access: false,
        }
    }
}

pub struct WasmSandbox {
    config: SandboxConfig,
    engine: Engine,
}

impl WasmSandbox {
    pub fn new(config: SandboxConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut engine_config = Config::new();
        
        // Configure security settings
        engine_config.wasm_multi_memory(false);
        engine_config.wasm_memory64(false);
        engine_config.wasm_bulk_memory(true);
        engine_config.wasm_reference_types(false);
        
        // Set resource limits
        engine_config.max_wasm_stack(1024 * 1024); // 1MB stack
        
        let engine = Engine::new(&engine_config)?;
        
        Ok(Self {
            config,
            engine,
        })
    }
    
    /// Validate WASM module before execution
    pub fn validate_module(&self, wasm_bytes: &[u8]) -> Result<Module, Box<dyn std::error::Error>> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        
        // Check imports against allowed list
        for import in module.imports() {
            let module_name = import.module();
            if !self.config.allowed_imports.contains(&module_name.to_string()) {
                return Err(format!("Disallowed import: {}", module_name).into());
            }
        }
        
        // Validate exports (ensure no dangerous exports)
        for export in module.exports() {
            match export.ty() {
                ExternType::Memory(_) => {
                    // Memory exports are generally safe
                }
                ExternType::Func(_) => {
                    // Function exports are safe
                }
                ExternType::Global(_) => {
                    // Global exports might be concerning but generally safe
                }
                ExternType::Table(_) => {
                    // Table exports could be risky
                    if export.name().starts_with("__") {
                        return Err(format!("Suspicious table export: {}", export.name()).into());
                    }
                }
            }
        }
        
        Ok(module)
    }
    
    /// Create a sandboxed store with resource limits
    pub fn create_sandboxed_store<T>(&self, data: T) -> Store<T> {
        let mut store = Store::new(&self.engine, data);
        
        // Set memory limits
        store.limiter(|_| &mut ResourceLimiter::new(self.config.memory_limit));
        
        // Set CPU time limit
        store.set_epoch_deadline(1);
        store.out_of_fuel_async_yield(u64::MAX, 10000);
        
        store
    }
    
    /// Execute WASM with timeout and resource monitoring
    pub async fn execute_with_timeout<F, R>(&self, future: F) -> Result<R, Box<dyn std::error::Error>>
    where
        F: std::future::Future<Output = Result<R, Box<dyn std::error::Error>>>,
    {
        let timeout_future = tokio::time::timeout(self.config.cpu_time_limit, future);
        
        match timeout_future.await {
            Ok(result) => result,
            Err(_) => Err("Task execution timeout".into()),
        }
    }
    
    /// Check if WASM module has dangerous capabilities
    pub fn has_dangerous_capabilities(&self, module: &Module) -> Vec<String> {
        let mut warnings = Vec::new();
        
        for import in module.imports() {
            let module_name = import.module();
            let name = import.name();
            
            // Check for potentially dangerous imports
            if module_name.contains("env") && (
                name.contains("system") ||
                name.contains("exec") ||
                name.contains("fork") ||
                name.contains("socket") ||
                name.contains("network")
            ) {
                warnings.push(format!("Dangerous import: {}::{}", module_name, name));
            }
        }
        
        warnings
    }
}

/// Resource limiter for WASM execution
pub struct ResourceLimiter {
    memory_limit: u64,
    current_memory: u64,
}

impl ResourceLimiter {
    pub fn new(memory_limit: u64) -> Self {
        Self {
            memory_limit,
            current_memory: 0,
        }
    }
}

impl wasmtime::ResourceLimiter for ResourceLimiter {
    fn memory_growing(&mut self, current: usize, desired: usize, _maximum: Option<usize>) -> anyhow::Result<bool> {
        let new_memory = desired as u64;
        
        if new_memory > self.memory_limit {
            anyhow::bail!("Memory limit exceeded: {} > {}", new_memory, self.memory_limit);
        }
        
        self.current_memory = new_memory;
        Ok(true)
    }
    
    fn table_growing(&mut self, _current: u32, _desired: u32, _maximum: Option<u32>) -> anyhow::Result<bool> {
        // Allow table growth within reasonable limits
        Ok(true)
    }
}