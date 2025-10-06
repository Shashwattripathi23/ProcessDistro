/// Metrics Module
/// 
/// Performance monitoring and metrics collection
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub nodes_online: u32,
    pub tasks_queued: u32,
    pub tasks_running: u32,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub avg_task_latency: Duration,
    pub bytes_transferred: u64,
    pub uptime: Duration,
    pub cpu_utilization: f32,
    pub memory_usage: u64,
}

pub struct MetricsCollector {
    start_time: Instant,
    task_completion_times: Vec<Duration>,
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            task_completion_times: Vec::new(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
        }
    }
    
    /// Increment a counter metric
    pub fn increment_counter(&mut self, name: &str, value: u64) {
        *self.counters.entry(name.to_string()).or_insert(0) += value;
    }
    
    /// Set a gauge metric
    pub fn set_gauge(&mut self, name: &str, value: f64) {
        self.gauges.insert(name.to_string(), value);
    }
    
    /// Record task completion time
    pub fn record_task_completion(&mut self, duration: Duration) {
        self.task_completion_times.push(duration);
        
        // Keep only last 1000 measurements for avg calculation
        if self.task_completion_times.len() > 1000 {
            self.task_completion_times.remove(0);
        }
    }
    
    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> Metrics {
        let avg_latency = if !self.task_completion_times.is_empty() {
            let total: Duration = self.task_completion_times.iter().sum();
            total / self.task_completion_times.len() as u32
        } else {
            Duration::from_secs(0)
        };
        
        Metrics {
            nodes_online: *self.gauges.get("nodes_online").unwrap_or(&0.0) as u32,
            tasks_queued: *self.gauges.get("tasks_queued").unwrap_or(&0.0) as u32,
            tasks_running: *self.gauges.get("tasks_running").unwrap_or(&0.0) as u32,
            tasks_completed: *self.counters.get("tasks_completed").unwrap_or(&0),
            tasks_failed: *self.counters.get("tasks_failed").unwrap_or(&0),
            avg_task_latency: avg_latency,
            bytes_transferred: *self.counters.get("bytes_transferred").unwrap_or(&0),
            uptime: Instant::now().duration_since(self.start_time),
            cpu_utilization: *self.gauges.get("cpu_utilization").unwrap_or(&0.0) as f32,
            memory_usage: *self.gauges.get("memory_usage").unwrap_or(&0.0) as u64,
        }
    }
    
    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();
        
        // Add counter metrics
        for (name, value) in &self.counters {
            output.push_str(&format!("# TYPE {} counter\n", name));
            output.push_str(&format!("{} {}\n", name, value));
        }
        
        // Add gauge metrics
        for (name, value) in &self.gauges {
            output.push_str(&format!("# TYPE {} gauge\n", name));
            output.push_str(&format!("{} {}\n", name, value));
        }
        
        output
    }
}