/// Standalone test for pool_subscriber module
/// 
/// Run with: cargo test --test test_pool_subscriber -- --nocapture

use mempool_fetcher::pool_subscriber::{PoolSubscriber, types::*, cache::PoolStateCache};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_pool_state_cache_operations() {
    println!("Testing PoolStateCache basic operations...");
    
    let cache = PoolStateCache::new(0.1);
    
    // Test initial state
    assert_eq!(cache.get_pool_count(), 0);
    assert_eq!(cache.get_eth_threshold(), 0.1);
    println!("✓ Initial state verified");
    
    // Create test pool updates
    let mut updates = HashMap::new();
    
    // Pool 1 - Above threshold
    let pool1_addr = "0x1234567890abcdef1234567890abcdef12345678".to_string();
    let pool1_update = PoolUpdate {
        eth_reserve: 5.5,
        token_reserve: 10000.0,
        token_address: "0xTokenAAA".to_string(),
        block_number: 12345678,
        update_time: 1234567890.0,
    };
    updates.insert(pool1_addr.clone(), pool1_update);
    
    // Pool 2 - Below threshold  
    let pool2_addr = "0xfedcba0987654321fedcba0987654321fedcba09".to_string();
    let pool2_update = PoolUpdate {
        eth_reserve: 0.05,
        token_reserve: 100.0,
        token_address: "0xTokenBBB".to_string(),
        block_number: 12345679,
        update_time: 1234567891.0,
    };
    updates.insert(pool2_addr.clone(), pool2_update);
    
    // Update cache
    let updated = cache.update_pools(updates.iter());
    assert_eq!(updated.len(), 2);
    assert_eq!(cache.get_pool_count(), 2);
    println!("✓ Updated {} pools", updated.len());
    
    // Verify pool 1 data
    let pool1 = cache.get_pool(&pool1_addr).unwrap();
    assert_eq!(pool1.eth_reserve, 5.5);
    assert_eq!(pool1.token_address, "0xTokenAAA");
    assert_eq!(pool1.last_updated_block, 12345678);
    println!("✓ Pool 1 data verified (ETH: {})", pool1.eth_reserve);
    
    // Verify pool 2 data
    let pool2 = cache.get_pool(&pool2_addr).unwrap();
    assert_eq!(pool2.eth_reserve, 0.05);
    println!("✓ Pool 2 data verified (ETH: {})", pool2.eth_reserve);
    
    // Test get all pools
    let all_pools = cache.get_all_pools();
    assert_eq!(all_pools.len(), 2);
    println!("✓ Retrieved all {} pools", all_pools.len());
    
    // Test filtering by threshold
    let above_threshold: Vec<_> = all_pools
        .iter()
        .filter(|(_, pool)| pool.eth_reserve >= cache.get_eth_threshold())
        .collect();
    assert_eq!(above_threshold.len(), 1);
    println!("✓ {} pool(s) above threshold", above_threshold.len());
    
    println!("\n✅ All PoolStateCache tests passed!");
}

#[test]
fn test_pool_state_staleness() {
    println!("\nTesting pool state staleness checks...");
    
    let update = PoolUpdate {
        eth_reserve: 10.0,
        token_reserve: 20000.0,
        token_address: "0xTokenCCC".to_string(),
        block_number: 12345680,
        update_time: 1234567892.0,
    };
    
    let pool_state = PoolState::from(update);
    
    // Should not be stale immediately
    assert!(!pool_state.is_stale(Duration::from_secs(60)));
    println!("✓ Fresh pool state not marked as stale");
    
    // Age should be minimal
    let age = pool_state.age();
    assert!(age < Duration::from_millis(100));
    println!("✓ Pool state age: {:?}", age);
    
    println!("✅ Staleness tests passed!");
}

#[test]
fn test_message_deserialization() {
    println!("\nTesting JSON message deserialization...");
    
    let json_str = r#"{
        "type": "pool_updates",
        "timestamp": 1234567890.123,
        "data": {
            "0xPool1": {
                "eth_reserve": 123.456,
                "token_address": "0xToken1",
                "block_number": 12345678,
                "update_time": 1234567890.123
            },
            "0xPool2": {
                "eth_reserve": 78.901,
                "token_address": "0xToken2",
                "block_number": 12345679,
                "update_time": 1234567891.456
            }
        }
    }"#;
    
    let message: PoolUpdatesMessage = serde_json::from_str(json_str).unwrap();
    
    assert_eq!(message.message_type, "pool_updates");
    assert_eq!(message.timestamp, 1234567890.123);
    assert_eq!(message.data.len(), 2);
    println!("✓ Message deserialized successfully");
    
    let pool1 = message.data.get("0xPool1").unwrap();
    assert_eq!(pool1.eth_reserve, 123.456);
    assert_eq!(pool1.token_address, "0xToken1");
    println!("✓ Pool 1 data: {} ETH", pool1.eth_reserve);
    
    let pool2 = message.data.get("0xPool2").unwrap();
    assert_eq!(pool2.eth_reserve, 78.901);
    println!("✓ Pool 2 data: {} ETH", pool2.eth_reserve);
    
    println!("✅ Deserialization tests passed!");
}

#[test]
fn test_concurrent_cache_access() {
    use std::thread;
    
    println!("\nTesting concurrent cache access...");
    
    let cache = Arc::new(PoolStateCache::new(0.1));
    let mut handles = vec![];
    
    // Spawn 5 writer threads
    for i in 0..5 {
        let cache_clone = cache.clone();
        let handle = thread::spawn(move || {
            let pool_addr = format!("0xPool{:038}", i);
            let update = PoolUpdate {
                eth_reserve: (i + 1) as f64,
                token_reserve: (i + 1) as f64 * 1000.0,
                token_address: format!("0xToken{}", i),
                block_number: 1000000 + i as u64,
                update_time: 1234567890.0 + i as f64,
            };
            
            let mut updates = HashMap::new();
            updates.insert(pool_addr, update);
            cache_clone.update_pools(updates.iter());
        });
        handles.push(handle);
    }
    
    // Spawn 5 reader threads
    for i in 0..5 {
        let cache_clone = cache.clone();
        let handle = thread::spawn(move || {
            // Small delay to ensure writers have started
            thread::sleep(Duration::from_millis(10));
            
            let count = cache_clone.get_pool_count();
            assert!(count > 0);
            
            // Try to read a specific pool
            let pool_addr = format!("0xPool{:038}", i);
            if let Some(pool) = cache_clone.get_pool(&pool_addr) {
                assert_eq!(pool.eth_reserve, (i + 1) as f64);
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Final verification
    assert_eq!(cache.get_pool_count(), 5);
    println!("✓ All {} threads completed successfully", 10);
    println!("✓ Final pool count: {}", cache.get_pool_count());
    
    println!("✅ Concurrent access tests passed!");
}

#[test]
fn test_subscriber_creation() {
    println!("\nTesting PoolSubscriber creation...");
    
    // Test default creation
    let subscriber1 = PoolSubscriber::new(0.5);
    let cache1 = subscriber1.get_pool_cache();
    assert_eq!(cache1.get_eth_threshold(), 0.5);
    println!("✓ Default subscriber created with threshold: {}", cache1.get_eth_threshold());
    
    // Test with custom endpoint
    let subscriber2 = PoolSubscriber::with_endpoint(1.0, "tcp://localhost:15557");
    let cache2 = subscriber2.get_pool_cache();
    assert_eq!(cache2.get_eth_threshold(), 1.0);
    println!("✓ Custom subscriber created with threshold: {}", cache2.get_eth_threshold());
    
    println!("✅ Subscriber creation tests passed!");
}

fn main() {
    println!("Running pool_subscriber component tests...\n");
    
    test_pool_state_cache_operations();
    test_pool_state_staleness();
    test_message_deserialization();
    test_concurrent_cache_access();
    test_subscriber_creation();
    
    println!("\n🎉 All tests completed successfully!");
}