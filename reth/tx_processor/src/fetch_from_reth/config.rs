//! Configuration types for the fetch_from_reth module
//!
//! This module defines configuration structures for database access,
//! caching, and performance tuning.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Main configuration for Reth database access
#[derive(Debug, Clone)]
pub struct RethDataConfig {
    /// Path to Reth data directory containing the database
    pub datadir: PathBuf,
    
    /// Whether to open database in read-only mode (recommended)
    pub read_only: bool,
    
    /// Whether to enable static file access for older blocks
    pub enable_static_files: bool,
    
    /// Whether to check database consistency on open
    pub check_consistency: bool,
    
    /// Maximum time to wait for database operations
    pub operation_timeout: Duration,
    
    /// Whether to enable performance metrics collection
    pub enable_metrics: bool,
}

impl Default for RethDataConfig {
    fn default() -> Self {
        Self {
            datadir: Self::default_datadir(),
            read_only: true,           // Always use read-only for external access
            enable_static_files: true, // Required for complete data access
            check_consistency: true,   // Verify database integrity on open
            operation_timeout: Duration::from_secs(30),
            enable_metrics: false,     // Disabled by default for performance
        }
    }
}

impl RethDataConfig {
    /// Create a new configuration with the specified data directory
    pub fn new<P: AsRef<Path>>(datadir: P) -> Self {
        Self {
            datadir: datadir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }
    
    /// Get the default Reth data directory based on the environment
    pub fn default_datadir() -> PathBuf {
        // Try environment variable first
        if let Ok(datadir) = std::env::var("RETH_DATADIR") {
            return PathBuf::from(datadir);
        }
        
        // Fall back to platform-specific default
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."));
            
        base_dir.join("reth").join("mainnet")
    }
    
    /// Set a custom data directory
    pub fn with_datadir<P: AsRef<Path>>(mut self, datadir: P) -> Self {
        self.datadir = datadir.as_ref().to_path_buf();
        self
    }
    
    /// Enable or disable read-only mode
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
    
    /// Enable or disable static file access
    pub fn with_static_files(mut self, enable: bool) -> Self {
        self.enable_static_files = enable;
        self
    }
    
    /// Enable or disable consistency checking
    pub fn with_consistency_check(mut self, enable: bool) -> Self {
        self.check_consistency = enable;
        self
    }
    
    /// Set operation timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }
    
    /// Enable performance metrics
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Check if datadir exists
        if !self.datadir.exists() {
            return Err(format!("Data directory does not exist: {}", self.datadir.display()));
        }
        
        // Check if it's actually a directory
        if !self.datadir.is_dir() {
            return Err(format!("Data directory path is not a directory: {}", self.datadir.display()));
        }
        
        // Check if database subdirectory exists
        let db_path = self.datadir.join("db");
        if !db_path.exists() {
            return Err(format!("Database directory not found: {}", db_path.display()));
        }
        
        // Check for MDBX files
        let data_mdb = db_path.join("data.mdb");
        if !data_mdb.exists() {
            return Err(format!("MDBX data file not found: {}", data_mdb.display()));
        }
        
        // Validate timeout
        if self.operation_timeout.as_secs() == 0 {
            return Err("Operation timeout must be greater than 0".to_string());
        }
        
        Ok(())
    }
    
    /// Get the path to the database directory
    pub fn db_path(&self) -> PathBuf {
        self.datadir.join("db")
    }
    
    /// Get the path to the static files directory
    pub fn static_files_path(&self) -> PathBuf {
        self.datadir.join("static_files")
    }
}

/// Configuration for transaction caching
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of transactions to cache
    pub max_size: usize,
    
    /// Time-to-live for cached entries
    pub ttl: Duration,
    
    /// Whether to enable cache statistics
    pub enable_stats: bool,
    
    /// Initial cache capacity (for performance)
    pub initial_capacity: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 1000,        // Cache up to 1000 transactions
            ttl: Duration::from_secs(300), // 5 minutes TTL
            enable_stats: true,    // Track cache performance
            initial_capacity: 100, // Start with capacity for 100 entries
        }
    }
}

impl CacheConfig {
    /// Create a new cache configuration
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            max_size,
            ttl,
            ..Default::default()
        }
    }
    
    /// Create a high-performance cache configuration
    pub fn high_performance() -> Self {
        Self {
            max_size: 10000,       // Large cache
            ttl: Duration::from_secs(1800), // 30 minutes TTL
            enable_stats: false,   // Disable stats for max performance
            initial_capacity: 1000, // Large initial capacity
        }
    }
    
    /// Create a memory-efficient cache configuration
    pub fn memory_efficient() -> Self {
        Self {
            max_size: 100,         // Small cache
            ttl: Duration::from_secs(60), // 1 minute TTL
            enable_stats: true,    // Track performance
            initial_capacity: 10,  // Small initial capacity
        }
    }
    
    /// Disable caching entirely
    pub fn disabled() -> Self {
        Self {
            max_size: 0,           // No caching
            ttl: Duration::from_secs(0),
            enable_stats: false,
            initial_capacity: 0,
        }
    }
    
    /// Set maximum cache size
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }
    
    /// Set cache TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
    
    /// Enable or disable statistics
    pub fn with_stats(mut self, enable: bool) -> Self {
        self.enable_stats = enable;
        self
    }
    
    /// Set initial capacity
    pub fn with_initial_capacity(mut self, capacity: usize) -> Self {
        self.initial_capacity = capacity;
        self
    }
    
    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.max_size > 0
    }
    
    /// Validate cache configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.initial_capacity > self.max_size {
            return Err("Initial capacity cannot be greater than maximum size".to_string());
        }
        
        Ok(())
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Number of threads for concurrent operations
    pub thread_count: usize,
    
    /// Batch size for bulk operations
    pub batch_size: usize,
    
    /// Whether to use parallel processing for batches
    pub parallel_batches: bool,
    
    /// Memory pool size for allocations
    pub memory_pool_size: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            thread_count: num_cpus::get(),
            batch_size: 100,
            parallel_batches: true,
            memory_pool_size: 1024 * 1024, // 1MB
        }
    }
}

impl PerformanceConfig {
    /// Create a high-throughput configuration
    pub fn high_throughput() -> Self {
        Self {
            thread_count: num_cpus::get() * 2,
            batch_size: 1000,
            parallel_batches: true,
            memory_pool_size: 10 * 1024 * 1024, // 10MB
        }
    }
    
    /// Create a low-latency configuration
    pub fn low_latency() -> Self {
        Self {
            thread_count: 1,        // Single thread for consistency
            batch_size: 10,         // Small batches
            parallel_batches: false, // No parallelism overhead
            memory_pool_size: 256 * 1024, // 256KB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_reth_data_config_default() {
        let config = RethDataConfig::default();
        assert!(config.read_only);
        assert!(config.enable_static_files);
        assert!(config.check_consistency);
        assert!(config.operation_timeout.as_secs() > 0);
    }
    
    #[test]
    fn test_reth_data_config_builder() {
        let config = RethDataConfig::new("/test/path")
            .with_read_only(false)
            .with_static_files(false)
            .with_timeout(Duration::from_secs(60));
            
        assert_eq!(config.datadir, PathBuf::from("/test/path"));
        assert!(!config.read_only);
        assert!(!config.enable_static_files);
        assert_eq!(config.operation_timeout, Duration::from_secs(60));
    }
    
    #[test]
    fn test_cache_config_presets() {
        let high_perf = CacheConfig::high_performance();
        assert_eq!(high_perf.max_size, 10000);
        assert!(!high_perf.enable_stats);
        
        let memory_eff = CacheConfig::memory_efficient();
        assert_eq!(memory_eff.max_size, 100);
        assert!(memory_eff.enable_stats);
        
        let disabled = CacheConfig::disabled();
        assert_eq!(disabled.max_size, 0);
        assert!(!disabled.is_enabled());
    }
    
    #[test]
    fn test_cache_config_validation() {
        let mut config = CacheConfig::default();
        config.initial_capacity = config.max_size + 1;
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_performance_config_presets() {
        let high_throughput = PerformanceConfig::high_throughput();
        assert!(high_throughput.batch_size >= 1000);
        assert!(high_throughput.parallel_batches);
        
        let low_latency = PerformanceConfig::low_latency();
        assert_eq!(low_latency.thread_count, 1);
        assert!(!low_latency.parallel_batches);
    }
    
    #[test]
    fn test_default_datadir() {
        let datadir = RethDataConfig::default_datadir();
        // Should either use RETH_DATADIR or default path
        assert!(!datadir.as_os_str().is_empty());
    }
    
    #[test]
    fn test_environment_variable() {
        env::set_var("RETH_DATADIR", "/custom/reth/path");
        let datadir = RethDataConfig::default_datadir();
        assert_eq!(datadir, PathBuf::from("/custom/reth/path"));
        env::remove_var("RETH_DATADIR");
    }
}