/// Direct Simulation Test (No Queue)
/// 
/// Tests the simulation pipeline by directly calling buy/sell simulator
/// Bypasses all queue management to isolate simulation issues
///
/// This example:
/// 1. Runs transactions through router
/// 2. For creator transactions, directly calls buy/sell simulator
/// 3. Logs all simulation details to dev directory
/// 4. No queue, no manager - just direct simulation

use std::time::{Duration, Instant};
use std::sync::Arc;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use chrono::Local;

// Mempool processor imports
use mempool_processor::{
    mempool_fetcher::{MempoolFetcherIPCClient, MempoolTransaction},
    function_detector::FunctionDetector,
    tx_router::{TransactionRouter as TxRouter, TransactionCategory},
    simulator::MempoolSimulator,
    token_tracking::{TokenTrackingCache, TokenTrackingSubscriber},
};

#[derive(Parser, Debug)]
struct Args {
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    /// Reth database path
    #[arg(long, env = "RETH_DB_PATH", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_db_path: String,
    
    /// Number of transactions to process
    #[arg(long, default_value = "10000")]
    target_count: usize,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(if args.verbose { "debug" } else { "info" })
        .init();
    
    // Create log directory
    let log_dir = "/home/nima/code/crypto/logs/mempool/dev/simulation_only";
    create_dir_all(log_dir)?;
    
    // Create log file
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_path = format!("{}/direct_simulation_{}.log", log_dir, timestamp);
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;
    
    writeln!(log_file, "Direct Simulation Test - NO Queue/Manager")?;
    writeln!(log_file, "Started: {}", Local::now())?;
    writeln!(log_file, "Target: {} transactions", args.target_count)?;
    writeln!(log_file, "======================================\n")?;
    
    info!("🚀 Starting Direct Simulation Test");
    info!("📝 Logging to: {}", log_path);
    info!("🎯 Target: {} transactions", args.target_count);
    info!("⚠️  NO queue/manager - direct simulation only");
    
    // Initialize token tracking subscriber
    info!("\n📦 Initializing token tracking subscriber...");
    let mut token_subscriber = TokenTrackingSubscriber::new(0.1); // 0.1 ETH threshold
    let token_cache = token_subscriber.get_cache();
    
    // Start subscriber in background
    let _subscriber_handle = tokio::spawn(async move {
        if let Err(e) = token_subscriber.start_listening().await {
            warn!("Token subscriber error: {}", e);
        }
    });
    
    // Wait for initial cache population
    info!("⏳ Waiting for token cache population...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let initial_pools = token_cache.get_pool_count().await;
    let initial_creators = token_cache.get_creator_count().await;
    info!("📊 Token Cache Statistics:");
    info!("   Total Pools: {}", initial_pools);
    info!("   Total Creators: {}", initial_creators);
    
    // Initialize components
    info!("\n🔧 Initializing pipeline components...");
    
    // IPC client
    let ipc_client = MempoolFetcherIPCClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");
    
    // Function detector
    let function_detector = FunctionDetector::new();
    info!("✅ Function detector initialized");
    
    // TX Router
    let tx_router = TxRouter::new(Some(token_cache.clone()));
    info!("✅ Transaction router initialized");
    
    // Initialize MempoolSimulator 
    let simulator = Arc::new(MempoolSimulator::new(&args.reth_db_path)?);
    info!("✅ MempoolSimulator initialized (no database lock issues!)");
    
    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    info!("📦 Using latest block: {}", latest_block);
    
    info!("\n🏃 Starting direct simulation testing...\n");
    
    let mut total_processed = 0;
    let pipeline_start = Instant::now();
    
    // Stats tracking
    let mut total_simulations = 0;
    let mut successful_simulations = 0;
    let mut failed_simulations = 0;
    let mut creator_tx_count = 0;
    let mut contract_creation_count = 0;
    
    while total_processed < args.target_count {
        // Fetch transactions
        let new_txs = ipc_client.get_transactions_instant(100).await;
        
        if new_txs.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        // Process each transaction
        for tx in new_txs {
            if total_processed >= args.target_count {
                break;
            }
            
            total_processed += 1;
            
            // 1. Function Detection
            let _detected_function = function_detector.detect_function(&tx);
            
            // 2. Transaction Routing
            let classification = tx_router.classify(&tx).await;
            let category = classification.category;
            
            // Only process creator transactions and contract creations
            match &category {
                TransactionCategory::CreatorTransaction { creator, target_token, .. } => {
                    creator_tx_count += 1;
                    info!("=== CREATOR TRANSACTION FOUND ===");
                    info!("  TX: {}", tx.hash);
                    info!("  Creator: {}", creator);
                    info!("  Target Token: {:?}", target_token);
                    
                    writeln!(log_file, "\n=== CREATOR TRANSACTION {} ===\"", creator_tx_count)?;
                    writeln!(log_file, "Time: {}", Local::now())?;
                    writeln!(log_file, "TX Hash: {}", tx.hash)?;
                    writeln!(log_file, "Creator: {}", creator)?;
                    writeln!(log_file, "Target Token: {:?}", target_token)?;
                    
                    // Get token address
                    let token_address_str = match target_token {
                        Some(token) => token.clone(),
                        None => {
                            // Try to get token from cache if not provided
                            if let Some(token_info) = token_cache.get_token_for_creator(creator).await {
                                info!("  Found token {} in cache", token_info.address);
                                token_info.address.clone()
                            } else {
                                writeln!(log_file, "❌ No token address found for creator {}", creator)?;
                                warn!("No token address found for creator {}", creator);
                                continue;
                            }
                        }
                    };
                    
                    // Simulate buy/sell directly
                    // Create FullTransaction from MempoolTransaction
                    let full_tx = FullTransaction {
                        hash: tx.hash.clone(),
                        tx_data: tx.data.clone(),
                        detection_time: std::time::Instant::now(),
                        latency_ns: tx.detection_ns,
                    };
                    
                    simulate_buy_sell_direct(
                        &full_tx,
                        &token_address_str,
                        &token_cache,
                        &simulator,
                        &mut log_file,
                        &mut total_simulations,
                        &mut successful_simulations,
                        &mut failed_simulations,
                    ).await;
                }
                TransactionCategory::ContractCreation { contract_address, .. } => {
                    contract_creation_count += 1;
                    info!("=== CONTRACT CREATION FOUND ===");
                    info!("  TX: {}", tx.hash);
                    info!("  Contract: {}", contract_address);
                    
                    writeln!(log_file, "\n=== CONTRACT CREATION {} ===\"", contract_creation_count)?;
                    writeln!(log_file, "Time: {}", Local::now())?;
                    writeln!(log_file, "TX Hash: {}", tx.hash)?;
                    writeln!(log_file, "Contract: {}", contract_address)?;
                    
                    // Create FullTransaction from MempoolTransaction
                    let full_tx = FullTransaction {
                        hash: tx.hash.clone(),
                        tx_data: tx.data.clone(),
                        detection_time: std::time::Instant::now(),
                        latency_ns: tx.detection_ns,
                    };
                    
                    // Simulate buy/sell directly  
                    simulate_buy_sell_direct(
                        &full_tx,
                        contract_address,
                        &token_cache,
                        &simulator,
                        &mut log_file,
                        &mut total_simulations,
                        &mut successful_simulations,
                        &mut failed_simulations,
                    ).await;
                }
                _ => {
                    // Skip other transaction types
                    continue;
                }
            }
        }
        
        // Progress update every 1000 transactions
        if total_processed % 1000 == 0 {
            let elapsed = pipeline_start.elapsed();
            let rate = total_processed as f64 / elapsed.as_secs_f64();
            info!("Progress: {}/{} transactions ({:.1} tx/sec)", 
                total_processed, args.target_count, rate);
            info!("  Found: {} creator txs, {} contract creations", 
                creator_tx_count, contract_creation_count);
            
            if total_simulations > 0 {
                info!("  Simulations: {} total, {} success, {} failed", 
                    total_simulations, successful_simulations, failed_simulations);
            }
        }
    }
    
    let total_time = pipeline_start.elapsed();
    
    // Final summary
    let summary = format!(
        "\n📊 DIRECT SIMULATION SUMMARY\n\
        =====================================\n\
        Total Transactions Processed: {}\n\
        Creator Transactions Found: {}\n\
        Contract Creations Found: {}\n\
        Total Simulations Run: {}\n\
        Successful Simulations: {} ({:.1}%)\n\
        Failed Simulations: {} ({:.1}%)\n\
        Time Elapsed: {:.2}s\n\
        Processing Rate: {:.1} tx/sec",
        total_processed,
        creator_tx_count,
        contract_creation_count,
        total_simulations,
        successful_simulations,
        if total_simulations > 0 { successful_simulations as f64 / total_simulations as f64 * 100.0 } else { 0.0 },
        failed_simulations,
        if total_simulations > 0 { failed_simulations as f64 / total_simulations as f64 * 100.0 } else { 0.0 },
        total_time.as_secs_f64(),
        total_processed as f64 / total_time.as_secs_f64()
    );
    
    println!("{}", summary);
    writeln!(log_file, "{}", summary)?;
    
    info!("\n✅ Direct simulation test complete!");
    info!("📄 Results saved to: {}", log_path);
    
    Ok(())
}

/// Simulate buy/sell directly without queue/manager
async fn simulate_buy_sell_direct(
    tx: &MempoolTransaction,
    token_address_str: &str,
    token_cache: &TokenTrackingCache,
    simulator: &MempoolSimulator,
    log_file: &mut std::fs::File,
    total_simulations: &mut usize,
    successful_simulations: &mut usize,
    failed_simulations: &mut usize,
) {
    *total_simulations += 1;
    
    info!("  Starting direct buy/sell simulation...");
    writeln!(log_file, "\n--- Direct Buy/Sell Simulation ---").unwrap();
    writeln!(log_file, "Token Address: {}", token_address_str).unwrap();
    
    // Convert token address
    let token_address = match token_address_str.trim_start_matches("0x")
        .parse::<alloy_primitives::Address>() {
        Ok(addr) => addr,
        Err(e) => {
            writeln!(log_file, "❌ Invalid token address: {}", e).unwrap();
            *failed_simulations += 1;
            return;
        }
    };
    
    // Get pool address from cache
    let pool_info = token_cache.get_primary_pool(token_address_str).await;
    let pool_address = if let Some(pool) = pool_info {
        let addr = pool.address.trim_start_matches("0x")
            .parse::<alloy_primitives::Address>()
            .unwrap_or(alloy_primitives::Address::ZERO);
        writeln!(log_file, "Pool Address: {:?}", addr).unwrap();
        addr
    } else {
        writeln!(log_file, "Pool Address: Not found in cache").unwrap();
        alloy_primitives::Address::ZERO
    };
    
    // Get current block number
    let block_number = None; // Will use latest
    
    // Create transaction CallRequest
    let tx_call_request = match mempool_processor::common::convert::ipc_to_call_request(&tx.tx_data) {
        Ok(call) => call,
        Err(e) => {
            writeln!(log_file, "❌ Failed to convert transaction: {}", e).unwrap();
            *failed_simulations += 1;
            return;
        }
    };
    
    writeln!(log_file, "CallRequest Details:").unwrap();
    writeln!(log_file, "  From: {:?}", tx_call_request.from).unwrap();
    writeln!(log_file, "  To: {:?}", tx_call_request.to).unwrap();
    writeln!(log_file, "  Value: {:?}", tx_call_request.value).unwrap();
    writeln!(log_file, "  Nonce: {:?}", tx_call_request.nonce).unwrap();
    writeln!(log_file, "  Gas: {:?}", tx_call_request.gas).unwrap();
    
    info!("  Calling simulate_sequence_with_tx directly...");
    writeln!(log_file, "\nCalling simulator.simulate_sequence_with_tx()...").unwrap();
    
    let start = Instant::now();
    
    // Call buy/sell simulator directly
    match simulator.simulate_sequence_with_tx(
        Some(tx_call_request),
        token_address,
        pool_address,
        block_number,
    ).await {
        Ok(result) => {
            let elapsed = start.elapsed().as_millis();
            info!("  ✅ Simulation completed in {}ms", elapsed);
            writeln!(log_file, "✅ Simulation completed in {}ms", elapsed).unwrap();
            
            // Log original tx result
            if let Some(tx_result) = &result.given_tx_result {
                writeln!(log_file, "\nOriginal Transaction:").unwrap();
                writeln!(log_file, "  Success: {}", tx_result.success).unwrap();
                writeln!(log_file, "  Gas Used: {}", tx_result.gas_used).unwrap();
                if let Some(revert) = &tx_result.revert_reason {
                    writeln!(log_file, "  Revert: {}", revert).unwrap();
                }
            }
            
            // Log buy result
            writeln!(log_file, "\nBuy Transaction:").unwrap();
            writeln!(log_file, "  Success: {}", result.buy_result.success).unwrap();
            writeln!(log_file, "  Gas Used: {}", result.buy_result.gas_used).unwrap();
            if let Some(revert) = &result.buy_result.revert_reason {
                writeln!(log_file, "  Revert: {}", revert).unwrap();
            }
            
            // Log approve result
            writeln!(log_file, "\nApprove Transaction:").unwrap();
            writeln!(log_file, "  Success: {}", result.approve_result.success).unwrap();
            writeln!(log_file, "  Gas Used: {}", result.approve_result.gas_used).unwrap();
            if let Some(revert) = &result.approve_result.revert_reason {
                writeln!(log_file, "  Revert: {}", revert).unwrap();
            }
            
            // Log sell result
            writeln!(log_file, "\nSell Transaction:").unwrap();
            writeln!(log_file, "  Success: {}", result.sell_result.success).unwrap();
            writeln!(log_file, "  Gas Used: {}", result.sell_result.gas_used).unwrap();
            if let Some(revert) = &result.sell_result.revert_reason {
                writeln!(log_file, "  Revert: {}", revert).unwrap();
            }
            
            writeln!(log_file, "\nOverall: Can Buy = {}, Can Sell = {}", 
                result.buy_result.success, 
                result.sell_result.success).unwrap();
            
            *successful_simulations += 1;
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis();
            info!("  ❌ Simulation FAILED after {}ms: {}", elapsed, e);
            writeln!(log_file, "❌ Simulation FAILED after {}ms", elapsed).unwrap();
            writeln!(log_file, "Error: {}", e).unwrap();
            writeln!(log_file, "Error Details: {:?}", e).unwrap();
            
            // Check if it's the specific error code 11
            if e.to_string().contains("error code: 11") {
                writeln!(log_file, "\n⚠️  ERROR CODE 11 DETECTED!").unwrap();
                writeln!(log_file, "This is the database initialization error").unwrap();
            }
            
            *failed_simulations += 1;
        }
    }
    
    writeln!(log_file, "========================\n").unwrap();
}