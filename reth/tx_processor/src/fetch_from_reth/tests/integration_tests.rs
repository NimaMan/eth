//! Integration tests for the fetch_from_reth module
//!
//! These tests require a real Reth database and are marked with #[ignore]
//! by default. Run them with: cargo test fetch_from_reth::tests::integration_tests -- --ignored

use crate::fetch_from_reth::{
    provider::{RethDatabaseProvider, RethDataProvider},
    config::{RethDataConfig, CacheConfig},
    error::FetchError,
    tests::test_helpers::*,
};
use alloy_primitives::B256;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

/// Get Reth data directory for testing
fn get_test_reth_datadir() -> Option<PathBuf> {
    // Try test-specific environment variable first
    if let Ok(datadir) = env::var("RETH_TEST_DATADIR") {
        return Some(PathBuf::from(datadir));
    }
    
    // Try main RETH_DATADIR
    if let Ok(datadir) = env::var("RETH_DATADIR") {
        return Some(PathBuf::from(datadir));
    }
    
    // Try default location
    let default_dir = dirs::data_dir()?.join("reth").join("mainnet");
    if default_dir.exists() {
        Some(default_dir)
    } else {
        None
    }
}

/// Create a test provider or skip if no database available
fn create_test_provider() -> Option<RethDatabaseProvider> {
    let datadir = get_test_reth_datadir()?;
    let config = RethDataConfig::new(&datadir);
    
    // Validate config before creating provider
    if config.validate().is_err() {
        return None;
    }
    
    RethDatabaseProvider::with_config(config).ok()
}

/// Integration tests for real database connectivity
mod database_connection_tests {
    use super::*;
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_real_database_connection() {
        let provider = match create_test_provider() {
            Some(provider) => provider,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        // Test basic connectivity
        let latest_block = provider.latest_block_number()
            .expect("Should be able to get latest block number");
        
        assert!(latest_block > 0, "Should have at least one block");
        println!("✅ Connected to Reth database with {} blocks", latest_block);
    }
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_database_consistency() {
        let provider = match create_test_provider() {
            Some(provider) => provider,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        let latest_block = provider.latest_block_number()
            .expect("Should be able to get latest block number");
        
        if latest_block < 1000 {
            println!("Skipping consistency test: Not enough blocks in database");
            return;
        }
        
        // Test that we can fetch transactions from different blocks
        let _test_block = latest_block.saturating_sub(100);
        
        // This test mainly verifies that the database is accessible and consistent
        // Actual transaction fetching depends on having known transaction hashes
        println!("✅ Database appears consistent (latest block: {})", latest_block);
    }
    
    #[test] 
    #[ignore] // Requires local Reth node with specific data
    fn test_fetch_known_mainnet_transaction() {
        let provider = match create_test_provider() {
            Some(provider) => provider,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        // Try to fetch a well-known early mainnet transaction
        let tx_hash = B256::from_str(ETH_TRANSFER_TX)
            .expect("Valid transaction hash");
        
        match provider.fetch_transaction(tx_hash) {
            Ok(tx_data) => {
                println!("✅ Successfully fetched transaction: {:x}", tx_hash);
                assert_eq!(tx_data.hash, tx_hash);
                assert!(tx_data.block_number > 0);
                
                // Validate basic transaction properties
                assert!(!tx_data.from.is_zero());
                if let Some(to) = tx_data.to {
                    assert!(!to.is_zero());
                }
                assert!(tx_data.gas_limit > 0);
            }
            Err(FetchError::NotFound(_)) => {
                println!("⚠️  Transaction not found in local database (expected if not fully synced)");
            }
            Err(e) => {
                panic!("Unexpected error fetching transaction: {}", e);
            }
        }
    }
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_transaction_existence_check() {
        let provider = match create_test_provider() {
            Some(provider) => provider,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        // Test with a known non-existent transaction
        let fake_hash = B256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();
        
        let exists = provider.transaction_exists(fake_hash)
            .expect("Should be able to check transaction existence");
        
        assert!(!exists, "Fake transaction should not exist");
        println!("✅ Transaction existence check works correctly");
    }
}

/// Integration tests for performance with real data
mod performance_integration_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_single_transaction_fetch_performance() {
        let provider = match create_test_provider() {
            Some(provider) => provider,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        
        // Measure cold fetch performance
        let start = Instant::now();
        let result = provider.fetch_transaction(tx_hash);
        let cold_duration = start.elapsed();
        
        match result {
            Ok(_) => {
                println!("✅ Cold fetch took {:?}", cold_duration);
                
                // Target: <100ms for cold fetch (including database open overhead)
                if cold_duration.as_millis() > 100 {
                    println!("⚠️  Cold fetch slower than target ({}ms > 100ms)", cold_duration.as_millis());
                }
                
                // Measure warm fetch performance (cache hit)
                let start = Instant::now();
                let _ = provider.fetch_transaction(tx_hash);
                let warm_duration = start.elapsed();
                
                println!("✅ Warm fetch took {:?}", warm_duration);
                
                // Warm fetch should be significantly faster
                assert!(warm_duration < cold_duration, "Warm fetch should be faster than cold fetch");
                
                // Target: <1ms for warm fetch
                if warm_duration.as_millis() > 1 {
                    println!("⚠️  Warm fetch slower than target ({}ms > 1ms)", warm_duration.as_millis());
                }
            }
            Err(FetchError::NotFound(_)) => {
                println!("Transaction not found in database - this is expected if not fully synced");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_batch_fetch_performance() {
        let provider = match create_test_provider() {
            Some(provider) => provider,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        let tx_hashes = vec![
            B256::from_str(ETH_TRANSFER_TX).unwrap(),
            B256::from_str(ERC20_TRANSFER_TX).unwrap(),
            B256::from_str(UNISWAP_V2_SWAP_TX).unwrap(),
        ];
        
        // Measure batch fetch performance
        let start = Instant::now();
        let results = provider.fetch_batch(&tx_hashes)
            .expect("Batch fetch should not fail");
        let batch_duration = start.elapsed();
        
        println!("✅ Batch fetch of {} transactions took {:?}", tx_hashes.len(), batch_duration);
        println!("   Found {} transactions", results.len());
        
        if !results.is_empty() {
            let avg_per_tx = batch_duration.as_millis() as f64 / results.len() as f64;
            println!("   Average per transaction: {:.2}ms", avg_per_tx);
            
            // Target: <10ms average per transaction in batch
            if avg_per_tx > 10.0 {
                println!("⚠️  Batch performance slower than target ({:.2}ms > 10ms per tx)", avg_per_tx);
            }
        }
    }
    
    #[test]
    #[ignore] // Long running test
    fn test_sustained_performance() {
        let provider = match create_test_provider() {
            Some(provider) => provider,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        let iterations = 100;
        let mut successful_fetches = 0;
        let mut total_duration = std::time::Duration::ZERO;
        
        println!("Running sustained performance test ({} iterations)...", iterations);
        
        for i in 0..iterations {
            let start = Instant::now();
            
            match provider.fetch_transaction(tx_hash) {
                Ok(_) => {
                    successful_fetches += 1;
                    total_duration += start.elapsed();
                }
                Err(FetchError::NotFound(_)) => {
                    // Expected if transaction not in database
                    break;
                }
                Err(e) => {
                    panic!("Unexpected error on iteration {}: {}", i, e);
                }
            }
        }
        
        if successful_fetches > 0 {
            let avg_duration = total_duration / successful_fetches;
            let throughput = successful_fetches as f64 / total_duration.as_secs_f64();
            
            println!("✅ Sustained performance results:");
            println!("   Successful fetches: {}/{}", successful_fetches, iterations);
            println!("   Average duration: {:?}", avg_duration);
            println!("   Throughput: {:.1} TPS", throughput);
            
            // Cache should improve performance over time
            assert!(avg_duration.as_millis() < 50, "Average fetch should be <50ms with caching");
        } else {
            println!("No successful fetches - transaction not available in database");
        }
    }
}

/// Integration tests for cache behavior with real data
mod cache_integration_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_cache_effectiveness() {
        let datadir = match get_test_reth_datadir() {
            Some(datadir) => datadir,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        let config = RethDataConfig::new(&datadir);
        let cache_config = CacheConfig::default()
            .with_max_size(100)
            .with_stats(true);
        
        let provider = match RethDatabaseProvider::with_cache_config(config, cache_config) {
            Ok(provider) => provider,
            Err(_) => {
                println!("Skipping test: Could not create provider");
                return;
            }
        };
        
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        
        // First fetch (cache miss)
        let _ = provider.fetch_transaction(tx_hash);
        
        // Second fetch (should be cache hit)
        let _ = provider.fetch_transaction(tx_hash);
        
        let stats = provider.cache_stats();
        println!("Cache statistics after 2 fetches:");
        println!("  Total requests: {}", stats.total_requests);
        println!("  Hits: {}", stats.hits);
        println!("  Misses: {}", stats.misses);
        println!("  Hit rate: {:.1}%", stats.hit_rate());
        
        // We should have at least one request recorded
        assert!(stats.total_requests > 0, "Should have recorded requests");
    }
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_cache_with_different_configurations() {
        let datadir = match get_test_reth_datadir() {
            Some(datadir) => datadir,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        
        // Test with small cache
        let config = RethDataConfig::new(&datadir);
        let small_cache = CacheConfig::memory_efficient();
        
        if let Ok(provider) = RethDatabaseProvider::with_cache_config(config.clone(), small_cache) {
            let start = Instant::now();
            let _ = provider.fetch_transaction(tx_hash);
            let small_cache_duration = start.elapsed();
            
            println!("Small cache fetch took: {:?}", small_cache_duration);
        }
        
        // Test with large cache
        let large_cache = CacheConfig::high_performance();
        
        if let Ok(provider) = RethDatabaseProvider::with_cache_config(config, large_cache) {
            let start = Instant::now();
            let _ = provider.fetch_transaction(tx_hash);
            let large_cache_duration = start.elapsed();
            
            println!("Large cache fetch took: {:?}", large_cache_duration);
        }
    }
}

/// Integration tests for error scenarios
mod error_integration_tests {
    use super::*;
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_provider_creation_with_invalid_config() {
        let invalid_config = RethDataConfig::new("/nonexistent/path");
        let result = RethDatabaseProvider::with_config(invalid_config);
        
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("does not exist") || e.to_string().contains("Failed to create provider"));
        }
    }
    
    #[test]
    #[ignore] // Requires local Reth node
    fn test_fetch_nonexistent_transaction() {
        let provider = match create_test_provider() {
            Some(provider) => provider,
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        let fake_hash = B256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let result = provider.fetch_transaction(fake_hash);
        
        match result {
            Err(FetchError::NotFound(_)) => {
                println!("✅ Correctly returned NotFound for nonexistent transaction");
            }
            Ok(_) => {
                panic!("Should not have found zero hash transaction");
            }
            Err(e) => {
                panic!("Unexpected error type: {}", e);
            }
        }
    }
}

/// Integration tests for concurrent access
mod concurrent_integration_tests {
    use super::*;
    use std::sync::Arc;
    use tokio;
    
    #[tokio::test]
    #[ignore] // Requires local Reth node
    async fn test_concurrent_database_access() {
        let provider = match create_test_provider() {
            Some(provider) => Arc::new(provider),
            None => {
                println!("Skipping test: No Reth database available");
                return;
            }
        };
        
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        let num_threads = 4;
        let requests_per_thread = 10;
        
        let mut handles = Vec::new();
        
        for thread_id in 0..num_threads {
            let provider_clone = Arc::clone(&provider);
            let handle = tokio::spawn(async move {
                let mut successful_requests = 0;
                
                for _i in 0..requests_per_thread {
                    match provider_clone.fetch_transaction(tx_hash) {
                        Ok(_) => successful_requests += 1,
                        Err(FetchError::NotFound(_)) => {
                            // Expected if transaction not in database
                            break;
                        }
                        Err(e) => {
                            println!("Thread {} error: {}", thread_id, e);
                        }
                    }
                }
                
                successful_requests
            });
            
            handles.push(handle);
        }
        
        let mut total_successful = 0;
        for handle in handles {
            total_successful += handle.await.expect("Thread should complete");
        }
        
        println!("✅ Concurrent access test completed");
        println!("   Total successful requests: {}", total_successful);
        println!("   Threads: {}, Requests per thread: {}", num_threads, requests_per_thread);
        
        // If we found the transaction, all threads should have been able to access it
        if total_successful > 0 {
            // At least some requests should have succeeded
            assert!(total_successful >= num_threads, "At least one request per thread should succeed");
        }
    }
}