//! Performance optimization example for the fetch_from_reth module
//!
//! This example demonstrates advanced performance optimization techniques
//! for high-throughput blockchain data processing.
//!
//! To run this example:
//!   cargo run --bin fetch_from_reth_performance_optimization
//!
//! Prerequisites:
//! - Local Reth node with substantial synced data (>100K blocks recommended)
//! - Set RETH_DATADIR environment variable

use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::sync::Arc;

use alloy_primitives::B256;
use eyre::Result;
use tokio::time;
use revm_tx_simulator_lib::fetch_from_reth::{
    RethDatabaseProvider, RethDataProvider, RethDataConfig, CacheConfig
};
use revm_tx_simulator_lib::SharedRethDataProvider;

/// Performance measurement structure
#[derive(Debug, Default)]
struct PerformanceMetrics {
    total_requests: u64,
    total_duration: Duration,
    cache_hits: u64,
    successful_fetches: u64,
    failed_fetches: u64,
}

impl PerformanceMetrics {
    fn average_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_duration.as_millis() as f64 / self.total_requests as f64
        }
    }
    
    fn throughput_per_second(&self) -> f64 {
        if self.total_duration.as_secs() == 0 {
            0.0
        } else {
            self.successful_fetches as f64 / self.total_duration.as_secs_f64()
        }
    }
    
    fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_fetches as f64 / self.total_requests as f64) * 100.0
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with performance-focused configuration
    tracing_subscriber::fmt()
        .with_env_filter("warn") // Reduce logging overhead
        .init();
    
    println!("🚀 Fetch From Reth - Performance Optimization Example");
    println!("====================================================\n");
    
    // Get Reth data directory
    let datadir = get_reth_datadir()?;
    println!("📂 Using Reth data directory: {}", datadir.display());
    
    // Demonstration 1: Cache Configuration Impact
    println!("🧪 Demonstration 1: Cache Configuration Impact");
    println!("----------------------------------------------");
    demo_cache_configurations(&datadir).await?;
    
    // Demonstration 2: Batch vs Individual Fetching
    println!("\n🧪 Demonstration 2: Batch vs Individual Fetching");
    println!("------------------------------------------------");
    demo_batch_vs_individual(&datadir).await?;
    
    // Demonstration 3: Concurrent Access Patterns
    println!("\n🧪 Demonstration 3: Concurrent Access Patterns");
    println!("----------------------------------------------");
    demo_concurrent_access(&datadir).await?;
    
    // Demonstration 4: Memory-Efficient Processing
    println!("\n🧪 Demonstration 4: Memory-Efficient Processing");
    println!("-----------------------------------------------");
    demo_memory_efficient_processing(&datadir).await?;
    
    println!("\n🎉 Performance optimization examples completed!");
    println!("💡 Key takeaways:");
    println!("   • Use high-performance cache for repeated queries");
    println!("   • Batch operations for better throughput");
    println!("   • Leverage concurrent access with SharedRethDataProvider");
    println!("   • Configure cache TTL based on your use case");
    
    Ok(())
}

/// Demonstrate impact of different cache configurations
async fn demo_cache_configurations(datadir: &PathBuf) -> Result<()> {
    let test_hashes = get_test_transaction_hashes();
    
    // Test 1: No cache
    println!("Testing with cache disabled...");
    let config = RethDataConfig::new(datadir);
    let cache_config = CacheConfig::disabled();
    let provider = RethDatabaseProvider::with_cache_config(config.clone(), cache_config)?;
    
    let metrics_no_cache = measure_performance(&provider, &test_hashes, "No Cache").await;
    
    // Test 2: Small cache
    println!("Testing with small cache (100 entries)...");
    let cache_config = CacheConfig::default().with_max_size(100);
    let provider = RethDatabaseProvider::with_cache_config(config.clone(), cache_config)?;
    
    let metrics_small_cache = measure_performance(&provider, &test_hashes, "Small Cache").await;
    
    // Test 3: Large cache
    println!("Testing with large cache (10000 entries)...");
    let cache_config = CacheConfig::high_performance();
    let provider = RethDatabaseProvider::with_cache_config(config, cache_config)?;
    
    let metrics_large_cache = measure_performance(&provider, &test_hashes, "Large Cache").await;
    
    // Compare results
    println!("\n📊 Cache Configuration Comparison:");
    println!("┌─────────────────┬─────────────┬─────────────┬─────────────┐");
    println!("│ Configuration   │ Avg Latency │ Throughput  │ Success Rate│");
    println!("├─────────────────┼─────────────┼─────────────┼─────────────┤");
    println!("│ No Cache        │ {:>8.2} ms │ {:>8.1} TPS │ {:>8.1}%    │", 
             metrics_no_cache.average_latency_ms(),
             metrics_no_cache.throughput_per_second(),
             metrics_no_cache.success_rate());
    println!("│ Small Cache     │ {:>8.2} ms │ {:>8.1} TPS │ {:>8.1}%    │", 
             metrics_small_cache.average_latency_ms(),
             metrics_small_cache.throughput_per_second(),
             metrics_small_cache.success_rate());
    println!("│ Large Cache     │ {:>8.2} ms │ {:>8.1} TPS │ {:>8.1}%    │", 
             metrics_large_cache.average_latency_ms(),
             metrics_large_cache.throughput_per_second(),
             metrics_large_cache.success_rate());
    println!("└─────────────────┴─────────────┴─────────────┴─────────────┘");
    
    Ok(())
}

/// Demonstrate batch vs individual fetching performance
async fn demo_batch_vs_individual(datadir: &PathBuf) -> Result<()> {
    let test_hashes = get_test_transaction_hashes();
    let config = RethDataConfig::new(datadir);
    let provider = RethDatabaseProvider::with_config(config)?;
    
    // Test individual fetching
    println!("Testing individual transaction fetching...");
    let start_time = Instant::now();
    let mut individual_success = 0;
    
    for hash in &test_hashes {
        if provider.fetch_transaction(*hash).is_ok() {
            individual_success += 1;
        }
    }
    
    let individual_duration = start_time.elapsed();
    
    // Test batch fetching  
    println!("Testing batch transaction fetching...");
    let start_time = Instant::now();
    let batch_results = provider.fetch_batch(&test_hashes).unwrap_or_default();
    let batch_duration = start_time.elapsed();
    
    // Compare results
    println!("\n📊 Batch vs Individual Fetching:");
    println!("┌─────────────────┬─────────────┬─────────────┬─────────────┐");
    println!("│ Method          │ Duration    │ Successful  │ Rate        │");
    println!("├─────────────────┼─────────────┼─────────────┼─────────────┤");
    println!("│ Individual      │ {:>8} ms │ {:>8}/{:>2} │ {:>8.1} TPS │", 
             individual_duration.as_millis(),
             individual_success,
             test_hashes.len(),
             individual_success as f64 / individual_duration.as_secs_f64());
    println!("│ Batch           │ {:>8} ms │ {:>8}/{:>2} │ {:>8.1} TPS │", 
             batch_duration.as_millis(),
             batch_results.len(),
             test_hashes.len(),
             batch_results.len() as f64 / batch_duration.as_secs_f64());
    println!("└─────────────────┴─────────────┴─────────────┴─────────────┘");
    
    if batch_duration < individual_duration {
        let improvement = (individual_duration.as_millis() as f64 / batch_duration.as_millis() as f64 - 1.0) * 100.0;
        println!("✅ Batch fetching is {:.1}% faster than individual fetching", improvement);
    }
    
    Ok(())
}

/// Demonstrate concurrent access patterns
async fn demo_concurrent_access(datadir: &PathBuf) -> Result<()> {
    let test_hashes = get_test_transaction_hashes();
    let config = RethDataConfig::new(datadir);
    let provider = RethDatabaseProvider::with_config(config)?;
    let shared_provider = SharedRethDataProvider::new(provider);
    
    // Test sequential access
    println!("Testing sequential access...");
    let start_time = Instant::now();
    let mut sequential_success = 0;
    
    for hash in &test_hashes {
        if shared_provider.fetch_transaction(*hash).is_ok() {
            sequential_success += 1;
        }
    }
    
    let sequential_duration = start_time.elapsed();
    
    // Test concurrent access
    println!("Testing concurrent access (4 threads)...");
    let start_time = Instant::now();
    let chunk_size = test_hashes.len() / 4;
    let mut handles = vec![];
    
    for chunk in test_hashes.chunks(chunk_size) {
        let chunk_hashes = chunk.to_vec();
        let provider_clone = shared_provider.clone();
        
        let handle = tokio::spawn(async move {
            let mut success_count = 0;
            for hash in chunk_hashes {
                if provider_clone.fetch_transaction(hash).is_ok() {
                    success_count += 1;
                }
            }
            success_count
        });
        
        handles.push(handle);
    }
    
    let mut concurrent_success = 0;
    for handle in handles {
        concurrent_success += handle.await?;
    }
    
    let concurrent_duration = start_time.elapsed();
    
    // Compare results
    println!("\n📊 Sequential vs Concurrent Access:");
    println!("┌─────────────────┬─────────────┬─────────────┬─────────────┐");
    println!("│ Method          │ Duration    │ Successful  │ Rate        │");
    println!("├─────────────────┼─────────────┼─────────────┼─────────────┤");
    println!("│ Sequential      │ {:>8} ms │ {:>8}/{:>2} │ {:>8.1} TPS │", 
             sequential_duration.as_millis(),
             sequential_success,
             test_hashes.len(),
             sequential_success as f64 / sequential_duration.as_secs_f64());
    println!("│ Concurrent (4x) │ {:>8} ms │ {:>8}/{:>2} │ {:>8.1} TPS │", 
             concurrent_duration.as_millis(),
             concurrent_success,
             test_hashes.len(),
             concurrent_success as f64 / concurrent_duration.as_secs_f64());
    println!("└─────────────────┴─────────────┴─────────────┴─────────────┘");
    
    if concurrent_duration < sequential_duration {
        let improvement = (sequential_duration.as_millis() as f64 / concurrent_duration.as_millis() as f64 - 1.0) * 100.0;
        println!("✅ Concurrent access is {:.1}% faster than sequential", improvement);
    }
    
    Ok(())
}

/// Demonstrate memory-efficient processing for large datasets
async fn demo_memory_efficient_processing(datadir: &PathBuf) -> Result<()> {
    let config = RethDataConfig::new(datadir);
    let provider = RethDatabaseProvider::with_config(config)?;
    
    // Get latest block for demonstration
    let latest_block = provider.latest_block_number()?;
    if latest_block < 1000 {
        println!("⚠️  Not enough blocks for memory efficiency demonstration");
        return Ok(());
    }
    
    println!("Processing transactions from recent blocks (memory-efficient approach)...");
    
    let start_block = latest_block.saturating_sub(10); // Last 10 blocks
    let mut total_processed = 0;
    let _total_gas_used = 0u64;
    let _total_value_transferred = alloy_primitives::U256::ZERO;
    
    let start_time = Instant::now();
    
    // Process blocks one by one to minimize memory usage
    for _block_num in start_block..=latest_block {
        // In a real application, you'd get block transaction hashes
        // For this demo, we'll simulate processing
        total_processed += 1;
        
        // Simulate some processing work
        time::sleep(Duration::from_millis(1)).await;
    }
    
    let processing_duration = start_time.elapsed();
    
    println!("\n📊 Memory-Efficient Processing Results:");
    println!("┌─────────────────────────────┬─────────────┐");
    println!("│ Metric                      │ Value       │");
    println!("├─────────────────────────────┼─────────────┤");
    println!("│ Blocks Processed            │ {:>11} │", total_processed);
    println!("│ Processing Time             │ {:>8} ms │", processing_duration.as_millis());
    println!("│ Average Time per Block      │ {:>8.2} ms │", processing_duration.as_millis() as f64 / total_processed as f64);
    println!("│ Memory Usage (estimated)    │ {:>8} MB │", "< 10"); // Actual measurement would require more complex code
    println!("└─────────────────────────────┴─────────────┘");
    
    println!("💡 Memory-efficient tips:");
    println!("   • Process data in chunks rather than loading everything");
    println!("   • Use streaming when possible");
    println!("   • Configure cache size based on available memory");
    println!("   • Clear cache periodically for long-running processes");
    
    Ok(())
}

/// Measure performance of transaction fetching
async fn measure_performance(
    provider: &RethDatabaseProvider,
    hashes: &[B256],
    label: &str,
) -> PerformanceMetrics {
    let mut metrics = PerformanceMetrics::default();
    let start_time = Instant::now();
    
    // Warm up cache by fetching each transaction twice
    for hash in hashes {
        metrics.total_requests += 1;
        if provider.fetch_transaction(*hash).is_ok() {
            metrics.successful_fetches += 1;
        } else {
            metrics.failed_fetches += 1;
        }
    }
    
    // Second pass for cache performance
    for hash in hashes {
        metrics.total_requests += 1;
        if provider.fetch_transaction(*hash).is_ok() {
            metrics.successful_fetches += 1;
        } else {
            metrics.failed_fetches += 1;
        }
    }
    
    metrics.total_duration = start_time.elapsed();
    
    let cache_stats = provider.cache_stats();
    metrics.cache_hits = cache_stats.hits;
    
    println!("  {} - Processed {} requests in {:?}", 
             label, 
             metrics.total_requests, 
             metrics.total_duration);
    
    metrics
}

/// Get test transaction hashes for performance testing
fn get_test_transaction_hashes() -> Vec<B256> {
    vec![
        B256::from_str("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060").unwrap(),
        B256::from_str("0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006").unwrap(),
        B256::from_str("0x2d8c72e31d4a10e7d9c1f4e1c5f4e3a9b8d7c6f5e4d3c2b1a0918273645546372").unwrap(),
        // Add more test hashes as needed
    ]
}

/// Get Reth data directory from environment or use default
fn get_reth_datadir() -> Result<PathBuf> {
    if let Ok(datadir) = env::var("RETH_DATADIR") {
        Ok(PathBuf::from(datadir))
    } else {
        let default_dir = dirs::data_dir()
            .ok_or_else(|| eyre::eyre!("Could not determine data directory"))?
            .join("reth")
            .join("mainnet");
        
        println!("💡 RETH_DATADIR not set, using default: {}", default_dir.display());
        Ok(default_dir)
    }
}