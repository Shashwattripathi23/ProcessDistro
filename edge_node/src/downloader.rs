/// Downloader Module
/// 
/// Handles downloading tasks and dependencies from the controller
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

pub struct TaskDownloader {
    cache_dir: PathBuf,
    cached_tasks: HashMap<String, CachedTask>,
}

pub struct CachedTask {
    pub task_type: String,
    pub version: String,
    pub wasm_bytes: Vec<u8>,
    pub downloaded_at: std::time::SystemTime,
}

impl TaskDownloader {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            cached_tasks: HashMap::new(),
        }
    }
    
    /// Download a WASM task from URL
    pub async fn download_task(&mut self, task_url: &str, task_type: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(cached) = self.cached_tasks.get(task_url) {
            return Ok(cached.wasm_bytes.clone());
        }
        
        // Download from URL
        let response = reqwest::get(task_url).await?;
        let wasm_bytes = response.bytes().await?.to_vec();
        
        // Cache the downloaded task
        let cached_task = CachedTask {
            task_type: task_type.to_string(),
            version: "1.0.0".to_string(), // TODO: Extract from metadata
            wasm_bytes: wasm_bytes.clone(),
            downloaded_at: std::time::SystemTime::now(),
        };
        
        self.cached_tasks.insert(task_url.to_string(), cached_task);
        
        // Save to disk cache
        self.save_to_disk_cache(task_url, &wasm_bytes).await?;
        
        Ok(wasm_bytes)
    }
    
    /// Download task from controller directly
    pub async fn download_from_controller(&mut self, controller_addr: &str, task_id: Uuid) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let url = format!("http://{}/tasks/{}.wasm", controller_addr, task_id);
        self.download_task(&url, "unknown").await
    }
    
    /// Save downloaded task to disk cache
    async fn save_to_disk_cache(&self, task_url: &str, wasm_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&self.cache_dir).await?;
        
        // Generate cache filename from URL hash
        let hash = format!("{:x}", md5::compute(task_url.as_bytes()));
        let cache_file = self.cache_dir.join(format!("{}.wasm", hash));
        
        // Write to cache file
        fs::write(cache_file, wasm_bytes).await?;
        
        Ok(())
    }
    
    /// Load task from disk cache
    pub async fn load_from_disk_cache(&self, task_url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let hash = format!("{:x}", md5::compute(task_url.as_bytes()));
        let cache_file = self.cache_dir.join(format!("{}.wasm", hash));
        
        let wasm_bytes = fs::read(cache_file).await?;
        Ok(wasm_bytes)
    }
    
    /// Clear old cache entries
    pub async fn cleanup_cache(&mut self, max_age_days: u64) {
        let max_age = std::time::Duration::from_secs(max_age_days * 24 * 60 * 60);
        let now = std::time::SystemTime::now();
        
        self.cached_tasks.retain(|_, cached| {
            now.duration_since(cached.downloaded_at).unwrap_or_default() < max_age
        });
        
        // TODO: Also cleanup disk cache files
    }
    
    /// Verify downloaded WASM integrity
    pub fn verify_wasm_integrity(&self, wasm_bytes: &[u8], expected_hash: Option<&str>) -> bool {
        if let Some(hash) = expected_hash {
            let actual_hash = format!("{:x}", md5::compute(wasm_bytes));
            actual_hash == hash
        } else {
            // Basic WASM magic number check
            wasm_bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d])
        }
    }
}