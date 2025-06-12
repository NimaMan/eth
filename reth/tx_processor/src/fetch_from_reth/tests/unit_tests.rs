//! Unit tests for the fetch_from_reth module
//!
//! These tests use mock providers and don't require a real Reth database.

use crate::fetch_from_reth::{
    provider::{RethDataProvider, TransactionData},
    error::{FetchError, ErrorCategory},
    config::{RethDataConfig, CacheConfig},
    cache::{TransactionCache, CacheStats},
    tests::test_helpers::*,
};
use alloy_primitives::{Address, B256, U256};
use std::str::FromStr;
use std::time::Duration;

/// Test basic mock provider functionality
mod mock_provider_tests {
    use super::*;
    
    #[test]
    fn test_mock_provider_fetch_success() {
        let provider = create_mock_provider_with_test_data();
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        
        let result = provider.fetch_transaction(tx_hash);
        assert!(result.is_ok());
        
        let tx_data = result.unwrap();
        assert_eq!(tx_data.hash, tx_hash);
        assert!(tx_data.receipt_status);
        assert_eq!(tx_data.gas_limit, 21000);
    }
    
    #[test]
    fn test_mock_provider_fetch_not_found() {
        let provider = MockRethProvider::new();
        let tx_hash = B256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        
        let result = provider.fetch_transaction(tx_hash);
        assert!(result.is_err());
        
        if let Err(FetchError::NotFound(_)) = result {
            // Expected error type
        } else {
            panic!("Expected NotFound error");
        }
    }
    
    #[test]
    fn test_mock_provider_error_simulation() {
        let provider = MockRethProvider::new()
            .with_error_on(ETH_TRANSFER_TX);
        
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        let result = provider.fetch_transaction(tx_hash);
        
        assert!(result.is_err());
        if let Err(FetchError::DatabaseError(_)) = result {
            // Expected error type
        } else {
            panic!("Expected DatabaseError");
        }
    }
    
    #[test]
    fn test_mock_provider_batch_fetch() {
        let provider = create_mock_provider_with_test_data();
        
        let tx_hashes = vec![
            B256::from_str(ETH_TRANSFER_TX).unwrap(),
            B256::from_str(ERC20_TRANSFER_TX).unwrap(),
            B256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap(), // Non-existent
        ];
        
        let results = provider.fetch_batch(&tx_hashes).unwrap();
        assert_eq!(results.len(), 2); // Two successful fetches
    }
    
    #[test]
    fn test_mock_provider_transaction_exists() {
        let provider = create_mock_provider_with_test_data();
        
        let existing_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        let non_existing_hash = B256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        
        assert!(provider.transaction_exists(existing_hash).unwrap());
        assert!(!provider.transaction_exists(non_existing_hash).unwrap());
    }
    
    #[test]
    fn test_mock_provider_latest_block() {
        let provider = MockRethProvider::new().with_latest_block(12345);
        assert_eq!(provider.latest_block_number().unwrap(), 12345);
    }
    
    #[test]
    fn test_mock_provider_fetch_by_block_and_index() {
        let provider = create_mock_provider_with_test_data();
        let result = provider.fetch_transaction_by_block_and_index(46147, 0);
        assert!(result.is_ok());
    }
}

/// Test error handling and categorization
mod error_tests {
    use super::*;
    
    #[test]
    fn test_error_categories() {
        assert_eq!(FetchError::NotFound("test".to_string()).category(), ErrorCategory::Permanent);
        assert_eq!(FetchError::TimeoutError("test".to_string()).category(), ErrorCategory::Temporary);
        assert_eq!(FetchError::ConfigError("test".to_string()).category(), ErrorCategory::User);
        assert_eq!(FetchError::DatabaseError("test".to_string()).category(), ErrorCategory::System);
    }
    
    #[test]
    fn test_error_retryable() {
        assert!(FetchError::TimeoutError("test".to_string()).is_retryable());
        assert!(FetchError::MdbxError("test".to_string()).is_retryable());
        assert!(!FetchError::NotFound("test".to_string()).is_retryable());
        assert!(!FetchError::ConfigError("test".to_string()).is_retryable());
    }
    
    #[test]
    fn test_error_helper_functions() {
        let tx_hash = "0x1234567890abcdef";
        let error = FetchError::transaction_not_found(tx_hash);
        assert!(error.is_not_found());
        assert!(!error.is_retryable());
        
        let error = FetchError::invalid_parameter("block_number", "must be positive");
        assert!(error.is_user_error());
        
        let error = FetchError::operation_timeout("fetch", 5000);
        assert!(error.is_retryable());
    }
    
    #[test]
    fn test_error_display() {
        let error = FetchError::transaction_not_found("0x1234");
        let display_string = error.to_string();
        assert!(display_string.contains("Transaction 0x1234 not found"));
    }
}

/// Test configuration validation and defaults
mod config_tests {
    use super::*;
    use std::path::PathBuf;
    
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
            .with_timeout(Duration::from_secs(60))
            .with_metrics(true);
        
        assert_eq!(config.datadir, PathBuf::from("/test/path"));
        assert!(!config.read_only);
        assert!(!config.enable_static_files);
        assert_eq!(config.operation_timeout, Duration::from_secs(60));
        assert!(config.enable_metrics);
    }
    
    #[test]
    fn test_reth_data_config_validation_invalid_path() {
        let config = RethDataConfig::new("/nonexistent/path");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
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
        
        let result = config.validate();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::new(500, Duration::from_secs(120))
            .with_stats(false)
            .with_initial_capacity(50);
        
        assert_eq!(config.max_size, 500);
        assert_eq!(config.ttl, Duration::from_secs(120));
        assert!(!config.enable_stats);
        assert_eq!(config.initial_capacity, 50);
    }
}

/// Test cache functionality
mod cache_tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_cache_basic_operations() {
        let config = CacheConfig::default();
        let cache = TransactionCache::new(config);
        
        let tx_data = create_test_transaction_data(1);
        let hash = tx_data.hash;
        
        // Should be empty initially
        assert!(cache.get(&hash).is_none());
        assert_eq!(cache.size(), 0);
        
        // Put and get
        cache.put(hash, tx_data.clone());
        assert_eq!(cache.size(), 1);
        assert!(cache.contains(&hash));
        
        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved.hash, hash);
        assert_eq!(retrieved.value, tx_data.value);
    }
    
    #[test]
    fn test_cache_lru_eviction() {
        let config = CacheConfig::default().with_max_size(2);
        let cache = TransactionCache::new(config);
        
        let tx1 = create_test_transaction_data(1);
        let tx2 = create_test_transaction_data(2);
        let tx3 = create_test_transaction_data(3);
        
        // Fill cache to capacity
        cache.put(tx1.hash, tx1.clone());
        cache.put(tx2.hash, tx2.clone());
        assert_eq!(cache.size(), 2);
        
        // Adding third should evict first (LRU)
        cache.put(tx3.hash, tx3.clone());
        assert_eq!(cache.size(), 2);
        assert!(!cache.contains(&tx1.hash)); // Should be evicted
        assert!(cache.contains(&tx2.hash));
        assert!(cache.contains(&tx3.hash));
    }
    
    #[test]
    fn test_cache_access_order() {
        let config = CacheConfig::default().with_max_size(2);
        let cache = TransactionCache::new(config);
        
        let tx1 = create_test_transaction_data(1);
        let tx2 = create_test_transaction_data(2);
        let tx3 = create_test_transaction_data(3);
        
        // Fill cache
        cache.put(tx1.hash, tx1.clone());
        cache.put(tx2.hash, tx2.clone());
        
        // Access tx1 to make it most recent
        cache.get(&tx1.hash);
        
        // Adding third should evict tx2 (least recently used)
        cache.put(tx3.hash, tx3.clone());
        assert!(cache.contains(&tx1.hash)); // Should remain (recently accessed)
        assert!(!cache.contains(&tx2.hash)); // Should be evicted
        assert!(cache.contains(&tx3.hash));
    }
    
    #[test]
    fn test_cache_ttl_expiration() {
        let config = CacheConfig::default()
            .with_ttl(Duration::from_millis(10))
            .with_max_size(10);
        let cache = TransactionCache::new(config);
        
        let tx_data = create_test_transaction_data(1);
        let hash = tx_data.hash;
        
        // Put data
        cache.put(hash, tx_data);
        assert!(cache.contains(&hash));
        
        // Should still be there immediately
        assert!(cache.get(&hash).is_some());
        
        // Wait for expiration
        thread::sleep(Duration::from_millis(15));
        
        // Should be expired now
        assert!(cache.get(&hash).is_none());
    }
    
    #[test]
    fn test_cache_statistics() {
        let config = CacheConfig::default().with_stats(true);
        let cache = TransactionCache::new(config);
        
        let tx1 = create_test_transaction_data(1);
        let tx2 = create_test_transaction_data(2);
        
        // Put data
        cache.put(tx1.hash, tx1.clone());
        
        // Get hit
        cache.get(&tx1.hash);
        
        // Get miss
        cache.get(&tx2.hash);
        
        let stats = cache.stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate(), 50.0);
    }
    
    #[test]
    fn test_cache_disabled() {
        let config = CacheConfig::disabled();
        let cache = TransactionCache::new(config);
        
        let tx_data = create_test_transaction_data(1);
        let hash = tx_data.hash;
        
        // Should not cache anything
        cache.put(hash, tx_data.clone());
        assert_eq!(cache.size(), 0);
        assert!(cache.get(&hash).is_none());
    }
    
    #[test]
    fn test_cache_cleanup_expired() {
        let config = CacheConfig::default()
            .with_ttl(Duration::from_millis(10))
            .with_max_size(10);
        let cache = TransactionCache::new(config);
        
        let tx1 = create_test_transaction_data(1);
        let tx2 = create_test_transaction_data(2);
        
        // Put data
        cache.put(tx1.hash, tx1.clone());
        thread::sleep(Duration::from_millis(15)); // Let first expire
        cache.put(tx2.hash, tx2.clone()); // This one won't expire yet
        
        assert_eq!(cache.size(), 2); // Both still in cache
        
        // Clean up expired
        cache.cleanup_expired();
        
        assert_eq!(cache.size(), 1); // Only non-expired remains
        assert!(!cache.contains(&tx1.hash));
        assert!(cache.contains(&tx2.hash));
    }
    
    #[test]
    fn test_cache_clear() {
        let config = CacheConfig::default();
        let cache = TransactionCache::new(config);
        
        let tx_data = create_test_transaction_data(1);
        cache.put(tx_data.hash, tx_data.clone());
        assert_eq!(cache.size(), 1);
        
        cache.clear();
        assert_eq!(cache.size(), 0);
    }
    
    #[test]
    fn test_cache_utilization() {
        let config = CacheConfig::default().with_max_size(10);
        let cache = TransactionCache::new(config);
        
        assert_eq!(cache.utilization(), 0.0);
        
        // Fill half the cache
        for i in 0..5 {
            let tx_data = create_test_transaction_data(i);
            cache.put(tx_data.hash, tx_data.clone());
        }
        
        assert_eq!(cache.utilization(), 50.0);
    }
    
    #[test]
    fn test_cache_stats_performance_indicators() {
        let stats = CacheStats {
            total_requests: 1000,
            hits: 850,
            misses: 150,
            expired_removals: 5,
            lru_evictions: 10,
            current_size: 500,
            max_size_reached: 1000,
        };
        
        assert_eq!(stats.hit_rate(), 85.0);
        assert_eq!(stats.miss_rate(), 15.0);
        assert!(stats.is_performing_well());
    }
}

/// Test transaction data structures
mod transaction_data_tests {
    use super::*;
    
    #[test]
    fn test_transaction_data_creation() {
        let tx_data = create_test_transaction_data(1);
        
        assert_eq!(tx_data.nonce, 1);
        assert_eq!(tx_data.block_number, 46148);
        assert!(tx_data.receipt_status);
        assert_eq!(tx_data.gas_limit, 21000);
        assert_eq!(tx_data.gas_used, 21000);
        assert!(tx_data.to.is_some());
        assert!(tx_data.contractaddress.is_none());
    }
    
    #[test]
    fn test_contract_creation_data() {
        let tx_data = create_contract_creation_data();
        
        assert!(tx_data.to.is_none());
        assert!(tx_data.contractaddress.is_some());
        assert!(!tx_data.input.is_empty());
        assert!(tx_data.receipt_status);
    }
    
    #[test]
    fn test_failed_transaction_data() {
        let tx_data = create_failed_transaction_data();
        
        assert!(!tx_data.receipt_status);
        assert_eq!(tx_data.gas_used, tx_data.gas_limit); // All gas consumed on failure
    }
    
    #[test]
    fn test_transaction_data_clone() {
        let tx_data = create_test_transaction_data(1);
        let cloned_data = tx_data.clone();
        
        assert_eq!(tx_data.hash, cloned_data.hash);
        assert_eq!(tx_data.value, cloned_data.value);
        assert_eq!(tx_data.receipt_status, cloned_data.receipt_status);
    }
}

/// Performance unit tests (lightweight, no actual database)
mod performance_unit_tests {
    use super::*;
    
    #[test]
    fn test_cache_performance_single_operation() {
        let config = CacheConfig::default();
        let cache = TransactionCache::new(config);
        let tx_data = create_test_transaction_data(1);
        
        // Measure cache put performance
        let (_, put_duration) = measure_time(|| {
            cache.put(tx_data.hash, tx_data.clone());
        });
        
        // Cache put should be very fast
        assert_performance(put_duration, 1, "cache put");
        
        // Measure cache get performance
        let (result, get_duration) = measure_time(|| {
            cache.get(&tx_data.hash)
        });
        
        assert!(result.is_some());
        assert_performance(get_duration, 1, "cache get");
    }
    
    #[test]
    fn test_mock_provider_performance() {
        let provider = create_mock_provider_with_test_data();
        let tx_hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        
        // Measure mock fetch performance
        let (result, duration) = measure_time(|| {
            provider.fetch_transaction(tx_hash)
        });
        
        assert!(result.is_ok());
        // Mock provider should be very fast
        assert_performance(duration, 1, "mock provider fetch");
    }
    
    #[test]
    fn test_batch_vs_individual_performance_mock() {
        let provider = create_mock_provider_with_test_data();
        let tx_hashes = vec![
            B256::from_str(ETH_TRANSFER_TX).unwrap(),
            B256::from_str(ERC20_TRANSFER_TX).unwrap(),
            B256::from_str(UNISWAP_V2_SWAP_TX).unwrap(),
        ];
        
        // Measure individual fetches
        let (individual_results, individual_duration) = measure_time(|| {
            let mut results = Vec::new();
            for hash in &tx_hashes {
                if let Ok(tx_data) = provider.fetch_transaction(*hash) {
                    results.push(tx_data);
                }
            }
            results
        });
        
        // Measure batch fetch
        let (batch_results, batch_duration) = measure_time(|| {
            provider.fetch_batch(&tx_hashes).unwrap_or_default()
        });
        
        assert_eq!(individual_results.len(), batch_results.len());
        
        // For mock provider, batch should be at least as fast as individual
        assert!(batch_duration <= individual_duration * 2); // Allow some variance
    }
}