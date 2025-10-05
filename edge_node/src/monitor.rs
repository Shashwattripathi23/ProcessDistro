/// Monitor Module
/// 
/// System resource monitoring and capability reporting
use sysinfo::{System, SystemExt, CpuExt, ComponentExt};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    pub cpu_cores: u32,
    pub cpu_frequency: u64,
    pub total_memory: u64,
    pub available_memory: u64,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub temperature: Option<f32>,
    pub battery_level: Option<f32>,
    pub wasm_runtime: String,
    pub supported_tasks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub avg_execution_time: Duration,
    pub cpu_utilization: f32,
    pub memory_utilization: f32,
    pub uptime: Duration,
}

pub struct SystemMonitor {
    system: System,
    start_time: Instant,
    task_metrics: TaskMetrics,
}

struct TaskMetrics {
    completed: u64,
    failed: u64,
    execution_times: Vec<Duration>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system,
            start_time: Instant::now(),
            task_metrics: TaskMetrics {
                completed: 0,
                failed: 0,
                execution_times: Vec::new(),
            },
        }
    }
    
    /// Get current system capabilities
    pub fn get_capabilities(&mut self) -> SystemCapabilities {
        self.system.refresh_all();
        
        let cpu_count = self.system.cpus().len() as u32;
        let cpu_frequency = self.system.cpus()
            .first()
            .map(|cpu| cpu.frequency())
            .unwrap_or(0);
        
        let total_memory = self.system.total_memory();
        let available_memory = self.system.available_memory();
        
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        let memory_usage = ((total_memory - available_memory) as f32 / total_memory as f32) * 100.0;
        
        // Get temperature from first thermal sensor
        let temperature = self.system.components()
            .first()
            .map(|component| component.temperature());
        
        SystemCapabilities {
            cpu_cores: cpu_count,
            cpu_frequency,
            total_memory,
            available_memory,
            cpu_usage,
            memory_usage,
            temperature,
            battery_level: None, // TODO: Implement battery monitoring
            wasm_runtime: "wasmtime-25.0".to_string(),
            supported_tasks: vec![
                "matrix_mul".to_string(),
                "password_hash".to_string(),
                "mandelbrot".to_string(),
            ],
        }
    }
    
    /// Get current performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        let avg_execution_time = if !self.task_metrics.execution_times.is_empty() {
            let total: Duration = self.task_metrics.execution_times.iter().sum();
            total / self.task_metrics.execution_times.len() as u32
        } else {
            Duration::from_secs(0)
        };
        
        PerformanceMetrics {
            tasks_completed: self.task_metrics.completed,
            tasks_failed: self.task_metrics.failed,
            avg_execution_time,
            cpu_utilization: self.system.global_cpu_info().cpu_usage(),
            memory_utilization: ((self.system.total_memory() - self.system.available_memory()) as f32 / self.system.total_memory() as f32) * 100.0,
            uptime: Instant::now().duration_since(self.start_time),
        }
    }
    
    /// Record task completion
    pub fn record_task_completion(&mut self, execution_time: Duration) {
        self.task_metrics.completed += 1;
        self.task_metrics.execution_times.push(execution_time);
        
        // Keep only last 100 measurements
        if self.task_metrics.execution_times.len() > 100 {
            self.task_metrics.execution_times.remove(0);
        }
    }
    
    /// Record task failure
    pub fn record_task_failure(&mut self) {
        self.task_metrics.failed += 1;
    }
    
    /// Check if system is under heavy load
    pub fn is_overloaded(&mut self) -> bool {
        self.system.refresh_cpu();
        
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        let memory_usage = ((self.system.total_memory() - self.system.available_memory()) as f32 / self.system.total_memory() as f32) * 100.0;
        
        cpu_usage > 90.0 || memory_usage > 90.0
    }
    
    /// Get recommended number of concurrent tasks
    pub fn get_recommended_task_count(&mut self) -> u32 {
        self.system.refresh_all();
        
        let cpu_count = self.system.cpus().len() as u32;
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        let memory_usage = ((self.system.total_memory() - self.system.available_memory()) as f32 / self.system.total_memory() as f32) * 100.0;
        
        // Adjust based on current load
        let load_factor = if cpu_usage > 70.0 || memory_usage > 70.0 {
            0.5
        } else if cpu_usage > 50.0 || memory_usage > 50.0 {
            0.75
        } else {
            1.0
        };
        
        ((cpu_count as f32 * load_factor) as u32).max(1)
    }
}