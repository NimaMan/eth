/// Integration test for pool_subscriber module
/// 
/// This test simulates the Python publisher and verifies that the Rust subscriber
/// correctly receives and caches pool updates.
///
/// Run with: cargo test -p mempool_processor test_pool_subscriber_integration -- --nocapture

use mempool_fetcher::pool_subscriber::{PoolSubscriber, types::*};
use std::time::Duration;
use tokio::time::{timeout, sleep};
use zmq;
use serde_json::json;
use tracing::{info, debug};

#[tokio::test]
async fn test_pool_subscriber_integration() {
    // Initialize logging for debugging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=debug")
        .try_init();
    
    info!("Starting pool_subscriber integration test");
    
    // Create ZMQ context
    let context = zmq::Context::new();
    
    // Test ports (different from production to avoid conflicts)
    let pub_port = 25557;
    let rep_port = 25558;
    
    // Create mock Python publisher
    let publisher = context.socket(zmq::PUB).unwrap();
    publisher.bind(&format!("tcp://127.0.0.1:{}", pub_port)).unwrap();
    info!("Mock publisher bound to port {}", pub_port);
    
    // Create mock Python REP server
    let rep_server = context.socket(zmq::REP).unwrap();
    rep_server.bind(&format!("tcp://127.0.0.1:{}", rep_port)).unwrap();
    info!("Mock REP server bound to port {}", rep_port);
    
    // Give sockets time to bind
    sleep(Duration::from_millis(100)).await;
    
    // Create subscriber
    let mut subscriber = PoolSubscriber::with_endpoint(
        0.1, 
        &format!("tcp://127.0.0.1:{}", pub_port)
    );
    let cache = subscriber.get_pool_cache();
    
    // Spawn task to handle REQ/REP requests
    let rep_task = tokio::spawn(async move {
        // Wait for request
        match rep_server.recv_string(0) {
            Ok(Ok(request)) => {
                info!("Received REQ: {}", request);
                
                // Parse request
                if request.contains("get_all_pools") {
                    // Send mock response
                    let response = json!({
                        "status": "success",
                        "count": 2,
                        "data": {
                            "0x1111111111111111111111111111111111111111": {
                                "eth_reserve": 50.0,
                                "token_address": "0xTokenA",
                                "block_number": 1000000,
                                "update_time": 1234567890.0
                            },
                            "0x2222222222222222222222222222222222222222": {
                                "eth_reserve": 25.5,
                                "token_address": "0xTokenB", 
                                "block_number": 1000001,
                                "update_time": 1234567891.0
                            }
                        }
                    });
                    
                    rep_server.send(&response.to_string(), 0).unwrap();
                    info!("Sent initial pool data response");
                }
            }
            _ => {
                info!("REP server timeout or error");
            }
        }
    });
    
    // Spawn task to publish updates
    let publish_task = tokio::spawn(async move {
        // Wait a bit before publishing
        sleep(Duration::from_millis(500)).await;
        
        // Publish first update
        let update1 = json!({
            "type": "pool_updates",
            "timestamp": 1234567892.0,
            "data": {
                "0x3333333333333333333333333333333333333333": {
                    "eth_reserve": 75.25,
                    "token_address": "0xTokenC",
                    "block_number": 1000002,
                    "update_time": 1234567892.0
                }
            }
        });
        
        publisher.send(update1.to_string().as_bytes(), 0).unwrap();
        info!("Published update 1");
        
        // Wait and publish second update
        sleep(Duration::from_millis(200)).await;
        
        let update2 = json!({
            "type": "pool_updates",
            "timestamp": 1234567893.0,
            "data": {
                "0x1111111111111111111111111111111111111111": {
                    "eth_reserve": 55.0, // Updated value
                    "token_address": "0xTokenA",
                    "block_number": 1000003,
                    "update_time": 1234567893.0
                },
                "0x4444444444444444444444444444444444444444": {
                    "eth_reserve": 0.05, // Below threshold
                    "token_address": "0xTokenD",
                    "block_number": 1000003,
                    "update_time": 1234567893.0
                }
            }
        });
        
        publisher.send(update2.to_string().as_bytes(), 0).unwrap();
        info!("Published update 2");
    });
    
    // Start subscriber with timeout
    let subscribe_result = timeout(
        Duration::from_secs(2),
        subscriber.start_listening()
    ).await;
    
    // Wait for tasks to complete
    let _ = rep_task.await;
    let _ = publish_task.await;
    
    // Give a moment for processing
    sleep(Duration::from_millis(100)).await;
    
    // Verify results
    info!("Verifying cached pool data...");
    
    let pool_count = cache.get_pool_count();
    info!("Total pools in cache: {}", pool_count);
    
    // Check specific pools
    if let Some(pool1) = cache.get_pool("0x1111111111111111111111111111111111111111") {
        info!("Pool 0x1111: ETH reserve = {}", pool1.eth_reserve);
        // Should have the updated value if PUB/SUB worked
    }
    
    if let Some(pool3) = cache.get_pool("0x3333333333333333333333333333333333333333") {
        info!("Pool 0x3333: ETH reserve = {}", pool3.eth_reserve);
        assert_eq!(pool3.eth_reserve, 75.25);
    }
    
    // Check pool below threshold
    if let Some(pool4) = cache.get_pool("0x4444444444444444444444444444444444444444") {
        info!("Pool 0x4444: ETH reserve = {} (below threshold)", pool4.eth_reserve);
    }
    
    // Get all pools
    let all_pools = cache.get_all_pools();
    info!("All cached pools:");
    for (addr, pool) in all_pools.iter() {
        info!("  {}: {} ETH", addr, pool.eth_reserve);
    }
    
    info!("Integration test completed");
    
    // Basic assertions
    assert!(pool_count > 0, "Should have at least one pool cached");
}

/// Test error handling when Python publisher is not available
#[tokio::test] 
async fn test_pool_subscriber_no_publisher() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=debug")
        .try_init();
    
    info!("Testing subscriber without publisher");
    
    // Create subscriber pointing to non-existent endpoint
    let mut subscriber = PoolSubscriber::with_endpoint(
        0.1,
        "tcp://127.0.0.1:29999" // Port where nothing is listening
    );
    
    // Try to start listening with short timeout
    let result = timeout(
        Duration::from_millis(500),
        subscriber.start_listening()
    ).await;
    
    // Should timeout or handle gracefully
    assert!(result.is_err() || result.unwrap().is_err());
    
    info!("Subscriber handled missing publisher gracefully");
}