/*
 * Comprehensive Scam Detection Integration Test
 * 
 * This test identifies the real issues preventing scam detection from working:
 * 1. Pool cache population from ZeroMQ
 * 2. Address format consistency
 * 3. Database connectivity and writing
 * 4. End-to-end transaction flow
 */

use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{info, error, warn, debug, Level};
use std::collections::HashMap;
use ethers::types::{H256, H160};

use mempool_processor::mempool_processor::db_logger::DbLogger;
use mempool_processor::pool_subscriber::{PoolSubscriber, cache::PoolStateCache, types::PoolUpdate};
use mempool_processor::scam_detection::{
    ScamDetectionService, 
    ScamDetectionEngine,
    SimulationResult, 
    PoolEffect,
    ScamDetectionConfig
};
use mempool_processor::tx_simulator::StateChange;

#[derive(Parser, Debug)]
struct Args {
    /// Database hostname
    #[arg(long, env = "DB_HOST", default_value = "localhost")]
    db_host: String,
    
    /// Database port
    #[arg(long, env = "DB_PORT", default_value = "5432")]
    db_port: u16,
    
    /// Database name
    #[arg(long, env = "DB_NAME", default_value = "eth_db")]
    db_name: String,
    
    /// Database user
    #[arg(long, env = "DB_USER", default_value = "postgres")]
    db_user: String,
    
    /// Database password
    #[arg(long, env = "DB_PASSWORD", default_value = "postgres")]
    db_password: String,
    
    /// ZeroMQ socket address for pool updates
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug)]
struct TestResults {
    db_connection_ok: bool,
    db_write_ok: bool,
    pool_cache_populated: bool,
    address_format_consistent: bool,
    scam_detection_working: bool,
    end_to_end_working: bool,
    issues_found: Vec<String>,
}

impl TestResults {
    fn new() -> Self {
        Self {
            db_connection_ok: false,
            db_write_ok: false,
            pool_cache_populated: false,
            address_format_consistent: false,
            scam_detection_working: false,
            end_to_end_working: false,
            issues_found: Vec::new(),
        }
    }
    
    fn add_issue(&mut self, issue: String) {
        error!("❌ ISSUE: {}", issue);
        self.issues_found.push(issue);
    }
    
    fn print_summary(&self) {
        info!("=== TEST RESULTS SUMMARY ===");
        info!("Database Connection: {}", if self.db_connection_ok { "✅ OK" } else { "❌ FAILED" });
        info!("Database Writing: {}", if self.db_write_ok { "✅ OK" } else { "❌ FAILED" });
        info!("Pool Cache Population: {}", if self.pool_cache_populated { "✅ OK" } else { "❌ FAILED" });
        info!("Address Format Consistency: {}", if self.address_format_consistent { "✅ OK" } else { "❌ FAILED" });
        info!("Scam Detection Logic: {}", if self.scam_detection_working { "✅ OK" } else { "❌ FAILED" });
        info!("End-to-End Flow: {}", if self.end_to_end_working { "✅ OK" } else { "❌ FAILED" });
        
        if !self.issues_found.is_empty() {
            error!("=== ISSUES FOUND ===");
            for (i, issue) in self.issues_found.iter().enumerate() {
                error!("{}. {}", i + 1, issue);
            }
        } else {
            info!("🎉 All tests passed!");
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();
    
    // Configure logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    let mut results = TestResults::new();
    
    info!("🔍 Starting Comprehensive Scam Detection Integration Test");
    
    // TEST 1: Database Connection and Writing
    info!("TEST 1: Database Connection and Writing");
    match test_database_connection(&args).await {
        Ok(()) => {
            results.db_connection_ok = true;
            info!("✅ Database connection successful");
        },
        Err(e) => {
            results.add_issue(format!("Database connection failed: {}", e));
        }
    }
    
    match test_database_writing(&args).await {
        Ok(()) => {
            results.db_write_ok = true;
            info!("✅ Database writing successful");
        },
        Err(e) => {
            results.add_issue(format!("Database writing failed: {}", e));
        }
    }
    
    // TEST 2: Pool Cache Population
    info!("TEST 2: Pool Cache Population");
    let pool_cache = test_pool_cache_population(&args, &mut results).await;
    
    // TEST 3: Address Format Consistency
    info!("TEST 3: Address Format Consistency");
    test_address_format_consistency(&mut results);
    
    // TEST 4: Scam Detection Logic
    info!("TEST 4: Scam Detection Logic");
    test_scam_detection_logic(&pool_cache, &mut results).await;
    
    // TEST 5: End-to-End Integration
    info!("TEST 5: End-to-End Integration");
    if results.db_connection_ok && results.pool_cache_populated {
        test_end_to_end_integration(&args, &pool_cache, &mut results).await;
    } else {
        results.add_issue("Skipping end-to-end test due to prerequisite failures".to_string());
    }
    
    results.print_summary();
    
    if results.issues_found.is_empty() {
        info!("🎉 All tests passed! Scam detection should be working.");
        Ok(())
    } else {
        error!("❌ {} issues found. Fix these to enable scam detection.", results.issues_found.len());
        std::process::exit(1);
    }
}

async fn test_database_connection(args: &Args) -> eyre::Result<()> {
    let db_logger = DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name,
    ).await?;
    
    // Test basic connectivity
    if !db_logger.is_connected().await {
        return Err(eyre::eyre!("Database not connected"));
    }
    
    Ok(())
}

async fn test_database_writing(args: &Args) -> eyre::Result<()> {
    let db_logger = DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name,
    ).await?;
    
    // Test 1: With foreign key constraints disabled (existing test)
    info!("Temporarily disabling foreign key constraints for testing");
    db_logger.execute_query("SET session_replication_role = 'replica';", &[]).await?;
    
    let test_token = "0xA97df2189c2E1A9C4EEc120371AA5DbEfA59C34E";
    let test_pool = "0x2222222222222222222222222222222222222222";
    let test_block = 19000000i64;
    
    db_logger.write_mempool_scam_prediction(
        test_token,
        test_pool,
        test_block,
        1.0,  // current_eth_level
        0.05, // simulated_eth_level
        0.15, // eth_threshold
    ).await?;
    
    info!("✅ Successfully wrote test scam prediction to database");
    
    let deleted = db_logger.delete_test_records(test_token, test_pool, test_block).await?;
    info!("✅ Cleaned up {} test records", deleted);
    
    // Restore foreign key constraints
    db_logger.execute_query("SET session_replication_role = 'origin';", &[]).await?;
    info!("✅ Restored foreign key constraints");
    
    // Test 2: With foreign key constraints enabled (production scenario)
    info!("Testing production scenario with foreign key constraints enabled");
    
    // Use a different token that likely doesn't exist in the tokens table
    let unknown_token = "0x9999999999999999999999999999999999999999";
    let test_block_2 = 19000001i64;
    
    // This should now work with our updated database logger
    match db_logger.write_mempool_scam_prediction(
        unknown_token,
        test_pool,
        test_block_2,
        1.0,  // current_eth_level
        0.05, // simulated_eth_level
        0.15, // eth_threshold
    ).await {
        Ok(_) => {
            info!("✅ Production scenario test passed - scam logged with unknown token");
            // Clean up
            let _ = db_logger.delete_test_records(unknown_token, test_pool, test_block_2).await;
        },
        Err(e) => {
            return Err(eyre::eyre!("Production scenario test failed: {}", e));
        }
    }
    
    Ok(())
}

async fn test_pool_cache_population(args: &Args, results: &mut TestResults) -> Arc<PoolStateCache> {
    // Create pool cache
    let pool_cache = Arc::new(PoolStateCache::new(0.15));
    
    // Try to get updates from ZeroMQ
    info!("Attempting to connect to ZeroMQ at: {}", args.pool_zmq_address);
    
    let context = zmq::Context::new();
    let receiver = match context.socket(zmq::SUB) {
        Ok(socket) => socket,
        Err(e) => {
            results.add_issue(format!("Failed to create ZeroMQ socket: {}", e));
            return create_mock_pool_cache(results);
        }
    };
    
    if let Err(e) = receiver.connect(&args.pool_zmq_address) {
        results.add_issue(format!("Failed to connect to ZeroMQ: {}", e));
        return create_mock_pool_cache(results);
    }
    
    if let Err(e) = receiver.set_subscribe(b"") {
        results.add_issue(format!("Failed to subscribe to ZeroMQ: {}", e));
        return create_mock_pool_cache(results);
    }
    
    // Try to receive updates for 5 seconds
    let start_time = Instant::now();
    let mut received_updates = false;
    
    while start_time.elapsed() < Duration::from_secs(5) && !received_updates {
        if let Ok(Ok(message)) = receiver.recv_string(zmq::DONTWAIT) {
            info!("✅ Received ZeroMQ message: {}", message);
            
            // Try to parse the message
            if let Ok(pool_message) = serde_json::from_str::<mempool_processor::pool_subscriber::types::PoolUpdatesMessage>(&message) {
                let updated = pool_cache.update_pools(pool_message.data.iter());
                info!("✅ Updated {} pools from ZeroMQ", updated.len());
                received_updates = true;
                results.pool_cache_populated = true;
            } else {
                results.add_issue("Failed to parse ZeroMQ message as pool updates".to_string());
            }
        }
        
        time::sleep(Duration::from_millis(100)).await;
    }
    
    if !received_updates {
        results.add_issue("No pool updates received from ZeroMQ - Python service may not be running".to_string());
        return create_mock_pool_cache(results);
    }
    
    pool_cache
}

fn create_mock_pool_cache(results: &mut TestResults) -> Arc<PoolStateCache> {
    info!("Creating mock pool cache for testing");
    let cache = Arc::new(PoolStateCache::new(0.15));
    
    // Add mock pool data with the real token address
    let mut updates = HashMap::new();
    updates.insert(
        "0x1234567890abcdef1234567890abcdef12345678".to_string(),
        PoolUpdate {
            eth_reserve: 0.1, // Low reserve to trigger scam detection
            token_address: "0xA97df2189c2E1A9C4EEc120371AA5DbEfA59C34E".to_string(), // Use real token
            block_number: 19000000,
            update_time: chrono::Utc::now().timestamp() as f64,
        }
    );
    
    cache.update_pools(updates.iter());
    info!("✅ Created mock pool cache with test data");
    results.pool_cache_populated = true; // Mark as populated for testing
    
    cache
}

fn test_address_format_consistency(results: &mut TestResults) {
    // Test the address formatting used in the main service
    let test_address = H160::from_low_u64_be(0x1234567890abcdef);
    
    // Format used in main service
    let main_format = format!("0x{:040x}", test_address);
    
    // Format used in old tests
    let debug_format = format!("{:?}", test_address);
    
    info!("Main service format: {}", main_format);
    info!("Debug format: {}", debug_format);
    
    if main_format != debug_format {
        results.add_issue(format!(
            "Address format mismatch: main service uses '{}' but some tests use '{}'", 
            main_format, debug_format
        ));
    } else {
        results.address_format_consistent = true;
        info!("✅ Address formats are consistent");
    }
}

async fn test_scam_detection_logic(pool_cache: &Arc<PoolStateCache>, results: &mut TestResults) {
    let config = ScamDetectionConfig {
        eth_threshold: 0.15,
        percentage_threshold: 0.5,
    };
    
    let engine = ScamDetectionEngine::new(pool_cache.clone(), config);
    
    // Test with a transaction that should trigger scam detection
    let tx_hash = H256::random();
    let alerts = engine.process_transaction(
        tx_hash,
        "0xscammer".to_string(),
        vec![
            ("0x1234567890abcdef1234567890abcdef12345678".to_string(), 0.1, 0.01)
        ]
    );
    
    if alerts.is_empty() {
        results.add_issue("Scam detection logic not working - no alerts generated for obvious scam".to_string());
    } else {
        results.scam_detection_working = true;
        info!("✅ Scam detection logic working - generated {} alerts", alerts.len());
    }
}

async fn test_end_to_end_integration(args: &Args, pool_cache: &Arc<PoolStateCache>, results: &mut TestResults) {
    // Create database logger
    let db_logger = match DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name,
    ).await {
        Ok(logger) => Arc::new(logger),
        Err(e) => {
            results.add_issue(format!("Failed to create DB logger for end-to-end test: {}", e));
            return;
        }
    };
    
    // Temporarily disable foreign key constraints for testing
    if let Err(e) = db_logger.execute_query("SET session_replication_role = 'replica';", &[]).await {
        results.add_issue(format!("Failed to disable foreign key constraints: {}", e));
        return;
    }
    
    // Create scam detection service
    let config = ScamDetectionConfig {
        eth_threshold: 0.15,
        percentage_threshold: 0.5,
    };
    
    let service = ScamDetectionService::new(
        pool_cache.clone(),
        db_logger.clone(),
        config,
    );
    
    // Create a mock simulation result that should trigger scam detection
    let tx_hash = H256::random();
    let mut affected_pools = HashMap::new();
    
    affected_pools.insert(
        "0x1234567890abcdef1234567890abcdef12345678".to_string(),
        PoolEffect {
            pool_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            current_eth_reserve: 0.1,
            simulated_eth_reserve: 0.01,
            eth_delta: -0.09,
            percentage_change: -0.9,
        }
    );
    
    let simulation = SimulationResult {
        tx_hash,
        from: "0xscammer".to_string(),
        affected_pools,
    };
    
    // Process the simulation
    match service.process_transaction(simulation).await {
        Ok(alerts) => {
            if alerts.is_empty() {
                results.add_issue("End-to-end test failed - no alerts generated".to_string());
            } else {
                results.end_to_end_working = true;
                info!("✅ End-to-end test successful - {} alerts generated and logged to DB", alerts.len());
                
                // Clean up test data
                for alert in &alerts {
                    let _ = db_logger.delete_test_records(
                        &alert.token_address,
                        &alert.pool_address,
                        alert.detection_block as i64,
                    ).await;
                }
            }
        },
        Err(e) => {
            results.add_issue(format!("End-to-end test failed with error: {}", e));
        }
    }
    
    // Restore foreign key constraints
    let _ = db_logger.execute_query("SET session_replication_role = 'origin';", &[]).await;
} 