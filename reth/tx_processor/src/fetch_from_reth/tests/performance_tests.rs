//! Performance tests for the fetch_from_reth module
//!
//! These tests measure performance characteristics and can be run with:
//! cargo test fetch_from_reth::tests::performance_tests -- --nocapture

use crate::fetch_from_reth::{
    provider::{RethDataProvider},
    config::{CacheConfig},
    cache::TransactionCache,
    tests::test_helpers::*,
};
use alloy_primitives::B256;
use std::str::FromStr;
use std::time::{Duration, Instant};

/// Performance benchmarking structure
#[derive(Debug, Default)]
struct BenchmarkResults {
    operation: String,
    iterations: usize,
    total_duration: Duration,
    successful_operations: usize,
    cache_hits: u64,
    cache_misses: u64,
}

impl BenchmarkResults {
    fn new(operation: &str, iterations: usize) -> Self {
        Self {
            operation: operation.to_string(),
            iterations,
            ..Default::default()
        }
    }
    
    fn average_latency_ms(&self) -> f64 {
        if self.successful_operations == 0 {
            0.0
        } else {
            self.total_duration.as_millis() as f64 / self.successful_operations as f64
        }
    }
    
    fn throughput_per_second(&self) -> f64 {
        if self.total_duration.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.successful_operations as f64 / self.total_duration.as_secs_f64()
        }
    }
    
    fn success_rate(&self) -> f64 {
        if self.iterations == 0 {
            0.0
        } else {
            (self.successful_operations as f64 / self.iterations as f64) * 100.0
        }
    }
    
    fn cache_hit_rate(&self) -> f64 {
        let total_cache_requests = self.cache_hits + self.cache_misses;
        if total_cache_requests == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / total_cache_requests as f64) * 100.0
        }
    }
    
    fn print_summary(&self) {
        println!("\n📊 Benchmark Results: {}", self.operation);
        println!("┌─────────────────────┬─────────────────┐");
        println!("│ Metric              │ Value           │");
        println!("├─────────────────────┼─────────────────┤");
        println!("│ Iterations          │ {:>15} │", self.iterations);
        println!("│ Successful          │ {:>15} │", self.successful_operations);
        println!("│ Success Rate        │ {:>12.1}%   │", self.success_rate());
        println!("│ Total Duration      │ {:>12}ms   │", self.total_duration.as_millis());
        println!("│ Average Latency     │ {:>12.2}ms   │", self.average_latency_ms());
        println!("│ Throughput          │ {:>12.1} TPS │", self.throughput_per_second());
        println!("│ Cache Hits          │ {:>15} │", self.cache_hits);
        println!("│ Cache Misses        │ {:>15} │", self.cache_misses);
        println!("│ Cache Hit Rate      │ {:>12.1}%   │", self.cache_hit_rate());
        println!("└─────────────────────┴─────────────────┘");
    }
}

/// Benchmark cache operations
mod cache_performance_tests {
    use super::*;
    
    #[test]
    fn test_cache_put_performance() {
        let config = CacheConfig::default().with_max_size(1000);
        let cache = TransactionCache::new(config);
        let iterations = 1000;
        
        let mut results = BenchmarkResults::new("Cache Put Operations", iterations);
        let start = Instant::now();
        
        for i in 0..iterations {
            let tx_data = create_test_transaction_data(i as u8);
            cache.put(tx_data.hash, tx_data);
            results.successful_operations += 1;
        }
        
        results.total_duration = start.elapsed();
        results.print_summary();
        
        // Performance assertions
        assert!(results.average_latency_ms() < 0.1, "Cache put should be <0.1ms on average");
        assert!(results.throughput_per_second() > 10000.0, "Cache put should achieve >10K TPS");
    }
    
    #[test]
    fn test_cache_get_performance() {
        let config = CacheConfig::default().with_max_size(1000);
        let cache = TransactionCache::new(config);
        let iterations = 1000;
        
        // Pre-populate cache
        let test_data = create_test_transaction_data(1);
        cache.put(test_data.hash, test_data.clone());
        
        let mut results = BenchmarkResults::new("Cache Get Operations", iterations);
        let start = Instant::now();
        
        for _i in 0..iterations {
            if cache.get(&test_data.hash).is_some() {
                results.successful_operations += 1;
            }
        }
        
        results.total_duration = start.elapsed();
        results.print_summary();
        
        // Performance assertions
        assert!(results.average_latency_ms() < 0.01, "Cache get should be <0.01ms on average");
        assert!(results.success_rate() == 100.0, "All cache gets should succeed");
        assert!(results.throughput_per_second() > 100000.0, "Cache get should achieve >100K TPS");
    }
    
    #[test]
    fn test_cache_eviction_performance() {
        let config = CacheConfig::default().with_max_size(100); // Small cache to force evictions
        let cache = TransactionCache::new(config);
        let iterations = 500; // More items than cache size
        
        let mut results = BenchmarkResults::new("Cache with LRU Eviction", iterations);
        let start = Instant::now();
        
        for i in 0..iterations {
            let tx_data = create_test_transaction_data(i as u8);
            cache.put(tx_data.hash, tx_data);
            results.successful_operations += 1;
        }
        
        results.total_duration = start.elapsed();
        results.print_summary();
        
        // Performance assertions
        assert!(results.average_latency_ms() < 0.5, "Cache with eviction should be <0.5ms on average");
        assert_eq!(cache.size(), 100, "Cache should maintain max size");
    }
    
    #[test]
    fn test_cache_ttl_cleanup_performance() {
        let config = CacheConfig::default()
            .with_max_size(1000)
            .with_ttl(Duration::from_millis(1)); // Very short TTL
        let cache = TransactionCache::new(config);
        let iterations = 100;
        
        // Add entries that will expire
        for i in 0..iterations {
            let tx_data = create_test_transaction_data(i as u8);
            cache.put(tx_data.hash, tx_data);
        }
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(5));
        
        let mut results = BenchmarkResults::new("Cache TTL Cleanup", 1);
        let start = Instant::now();
        
        cache.cleanup_expired();
        results.successful_operations = 1;
        
        results.total_duration = start.elapsed();
        results.print_summary();
        
        // Performance assertion
        assert!(results.total_duration.as_millis() < 100, "TTL cleanup should be <100ms");
    }
}

/// Benchmark mock provider operations
mod mock_provider_performance_tests {
    use super::*;
    
    #[test]
    fn test_mock_provider_single_fetch_performance() {
        let provider = create_mock_provider_with_test_data();
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        let iterations = 10000;
        
        let mut results = BenchmarkResults::new("Mock Provider Single Fetch", iterations);
        let start = Instant::now();
        
        for _i in 0..iterations {
            if provider.fetch_transaction(tx_hash).is_ok() {
                results.successful_operations += 1;
            }
        }
        
        results.total_duration = start.elapsed();
        results.print_summary();
        
        // Performance assertions
        assert!(results.average_latency_ms() < 0.01, "Mock fetch should be <0.01ms");
        assert!(results.success_rate() == 100.0, "All mock fetches should succeed");
        assert!(results.throughput_per_second() > 100000.0, "Mock provider should achieve >100K TPS");
    }
    
    #[test]
    fn test_mock_provider_batch_fetch_performance() {
        let provider = create_mock_provider_with_test_data();
        let tx_hashes = vec![
            B256::from_str(ETH_TRANSFER_TX).unwrap(),
            B256::from_str(ERC20_TRANSFER_TX).unwrap(),
            B256::from_str(UNISWAP_V2_SWAP_TX).unwrap(),
        ];
        let iterations = 1000;
        
        let mut results = BenchmarkResults::new("Mock Provider Batch Fetch", iterations);
        let start = Instant::now();
        
        for _i in 0..iterations {
            if let Ok(batch_results) = provider.fetch_batch(&tx_hashes) {
                if !batch_results.is_empty() {
                    results.successful_operations += 1;
                }
            }
        }
        
        results.total_duration = start.elapsed();
        results.print_summary();
        
        // Performance assertions
        assert!(results.average_latency_ms() < 0.1, "Mock batch fetch should be <0.1ms");
        assert!(results.success_rate() == 100.0, "All mock batch fetches should succeed");
    }
    
    #[test]
    fn test_mock_provider_concurrent_access() {
        use std::sync::Arc;
        use std::thread;
        
        let provider = Arc::new(create_mock_provider_with_test_data());
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        let num_threads = 4;
        let iterations_per_thread = 1000;
        
        let mut results = BenchmarkResults::new("Mock Provider Concurrent Access", num_threads * iterations_per_thread);
        let start = Instant::now();
        
        let handles: Vec<_> = (0..num_threads)
            .map(|_| {
                let provider_clone = Arc::clone(&provider);
                thread::spawn(move || {
                    let mut successes = 0;
                    for _ in 0..iterations_per_thread {
                        if provider_clone.fetch_transaction(tx_hash).is_ok() {
                            successes += 1;
                        }
                    }
                    successes
                })
            })
            .collect();
        
        for handle in handles {
            results.successful_operations += handle.join().expect("Thread should complete");
        }
        
        results.total_duration = start.elapsed();
        results.print_summary();
        
        // Performance assertions
        assert!(results.success_rate() == 100.0, "All concurrent fetches should succeed");
        assert!(results.throughput_per_second() > 50000.0, "Concurrent access should maintain high throughput");
    }
}

/// Benchmark different cache configurations
mod cache_configuration_benchmarks {
    use super::*;
    
    #[test]
    fn test_cache_size_impact() {
        let cache_sizes = vec![10, 100, 1000, 10000];
        let iterations = 1000;
        
        println!("\n🧪 Cache Size Impact Benchmark");
        println!("==============================");
        
        for &cache_size in &cache_sizes {
            let config = CacheConfig::default().with_max_size(cache_size);
            let cache = TransactionCache::new(config);
            
            let mut results = BenchmarkResults::new(&format!("Cache Size {}", cache_size), iterations);
            let start = Instant::now();
            
            // Fill cache beyond capacity to test eviction
            for i in 0..iterations {
                let tx_data = create_test_transaction_data((i % 256) as u8);
                cache.put(tx_data.hash, tx_data);
                results.successful_operations += 1;
            }
            
            results.total_duration = start.elapsed();
            results.print_summary();
            
            // Verify cache size constraints
            assert!(cache.size() <= cache_size, "Cache should not exceed max size");
        }
    }
    
    #[test]
    fn test_cache_ttl_impact() {
        let ttl_values = vec![
            Duration::from_millis(10),
            Duration::from_millis(100),
            Duration::from_secs(1),
            Duration::from_secs(10),
        ];
        let iterations = 100;
        
        println!("\n🧪 Cache TTL Impact Benchmark");
        println!("=============================");
        
        for ttl in &ttl_values {
            let config = CacheConfig::default()
                .with_max_size(1000)
                .with_ttl(*ttl);
            let cache = TransactionCache::new(config);
            
            // Pre-populate cache
            for i in 0..50 {
                let tx_data = create_test_transaction_data(i);
                cache.put(tx_data.hash, tx_data);
            }
            
            let mut results = BenchmarkResults::new(&format!("TTL {:?}", ttl), iterations);
            let start = Instant::now();
            
            // Test get performance with TTL
            for i in 0..iterations {
                let tx_data = create_test_transaction_data((i % 50) as u8);
                if cache.get(&tx_data.hash).is_some() {
                    results.successful_operations += 1;
                }
            }
            
            results.total_duration = start.elapsed();
            results.print_summary();
        }
    }
    
    #[test]
    fn test_cache_stats_overhead() {
        let iterations = 10000;
        
        // Test with stats enabled
        let config_with_stats = CacheConfig::default()
            .with_max_size(1000)
            .with_stats(true);
        let cache_with_stats = TransactionCache::new(config_with_stats);
        
        let mut results_with_stats = BenchmarkResults::new("Cache with Stats", iterations);
        let tx_data = create_test_transaction_data(1);
        cache_with_stats.put(tx_data.hash, tx_data.clone());
        
        let start = Instant::now();
        for _i in 0..iterations {
            if cache_with_stats.get(&tx_data.hash).is_some() {
                results_with_stats.successful_operations += 1;
            }
        }
        results_with_stats.total_duration = start.elapsed();
        
        // Test with stats disabled
        let config_without_stats = CacheConfig::default()
            .with_max_size(1000)
            .with_stats(false);
        let cache_without_stats = TransactionCache::new(config_without_stats);
        
        let mut results_without_stats = BenchmarkResults::new("Cache without Stats", iterations);
        cache_without_stats.put(tx_data.hash, tx_data.clone());
        
        let start = Instant::now();
        for _i in 0..iterations {
            if cache_without_stats.get(&tx_data.hash).is_some() {
                results_without_stats.successful_operations += 1;
            }
        }
        results_without_stats.total_duration = start.elapsed();
        
        // Compare results
        results_with_stats.print_summary();
        results_without_stats.print_summary();
        
        let overhead_percent = ((results_with_stats.average_latency_ms() / results_without_stats.average_latency_ms()) - 1.0) * 100.0;
        println!("\n📊 Stats Overhead: {:.1}%", overhead_percent);
        
        // Stats overhead should be minimal
        assert!(overhead_percent < 50.0, "Stats overhead should be <50%");
    }
}

/// Memory usage and efficiency tests
mod memory_efficiency_tests {
    use super::*;
    
    #[test]
    fn test_memory_efficient_large_dataset() {
        let config = CacheConfig::memory_efficient(); // Small cache
        let cache = TransactionCache::new(config);
        let iterations = 10000; // Much larger than cache capacity
        
        let mut results = BenchmarkResults::new("Memory Efficient Large Dataset", iterations);
        let start = Instant::now();
        
        // Process large number of transactions with small cache
        for i in 0..iterations {
            let tx_data = create_test_transaction_data((i % 256) as u8);
            cache.put(tx_data.hash, tx_data);
            results.successful_operations += 1;
            
            // Periodically clean up expired entries
            if i % 100 == 0 {
                cache.cleanup_expired();
            }
        }
        
        results.total_duration = start.elapsed();
        results.print_summary();
        
        // Memory should be bounded
        assert!(cache.size() <= 100, "Cache size should be bounded by configuration");
        assert!(results.average_latency_ms() < 1.0, "Should maintain reasonable performance");
    }
    
    #[test]
    fn test_cache_utilization_patterns() {
        let config = CacheConfig::default().with_max_size(100);
        let cache = TransactionCache::new(config);
        
        println!("\n📊 Cache Utilization Pattern Test");
        println!("=================================");
        
        // Test different access patterns
        let patterns = vec![
            ("Sequential", (0..100).collect::<Vec<_>>()),
            ("Random", (0..100).cycle().take(200).collect::<Vec<_>>()),
            ("Hotspot", vec![1, 2, 3, 1, 2, 3, 1, 2, 3].into_iter().cycle().take(100).collect()),
        ];
        
        for (pattern_name, access_sequence) in patterns {
            cache.clear();
            
            let start = Instant::now();
            let mut hits = 0;
            let mut misses = 0;
            
            for i in access_sequence {
                let tx_data = create_test_transaction_data(i as u8);
                
                if cache.get(&tx_data.hash).is_some() {
                    hits += 1;
                } else {
                    cache.put(tx_data.hash, tx_data);
                    misses += 1;
                }
            }
            
            let duration = start.elapsed();
            let hit_rate = (hits as f64 / (hits + misses) as f64) * 100.0;
            
            println!("Pattern: {}", pattern_name);
            println!("  Hits: {}, Misses: {}", hits, misses);
            println!("  Hit Rate: {:.1}%", hit_rate);
            println!("  Duration: {:?}", duration);
            println!("  Utilization: {:.1}%", cache.utilization());
            println!();
        }
    }
}

/// Regression tests to ensure performance doesn't degrade
mod performance_regression_tests {
    use super::*;
    
    #[test]
    fn test_cache_performance_regression() {
        let config = CacheConfig::default();
        let cache = TransactionCache::new(config);
        let tx_data = create_test_transaction_data(1);
        
        // Baseline measurements
        let baseline_put_ms = 0.1; // 0.1ms
        let baseline_get_ms = 0.01; // 0.01ms
        
        // Test put performance
        let (_, put_duration) = measure_time(|| {
            cache.put(tx_data.hash, tx_data.clone());
        });
        
        // Test get performance
        let (_, get_duration) = measure_time(|| {
            cache.get(&tx_data.hash)
        });
        
        // Check for regression
        assert_performance(put_duration, (baseline_put_ms * 1000.0) as u64, "cache put");
        assert_performance(get_duration, (baseline_get_ms * 1000.0) as u64, "cache get");
        
        println!("✅ No performance regression detected");
        println!("   Put: {:?} (baseline: {}ms)", put_duration, baseline_put_ms);
        println!("   Get: {:?} (baseline: {}ms)", get_duration, baseline_get_ms);
    }
    
    #[test]
    fn test_mock_provider_performance_regression() {
        let provider = create_mock_provider_with_test_data();
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        
        // Baseline measurement
        let baseline_fetch_ms = 0.01; // 0.01ms
        
        // Test fetch performance
        let (result, fetch_duration) = measure_time(|| {
            provider.fetch_transaction(tx_hash)
        });
        
        assert!(result.is_ok(), "Mock fetch should succeed");
        assert_performance(fetch_duration, (baseline_fetch_ms * 1000.0) as u64, "mock provider fetch");
        
        println!("✅ No performance regression in mock provider");
        println!("   Fetch: {:?} (baseline: {}ms)", fetch_duration, baseline_fetch_ms);
    }
}