/// Simulation Only Test
/// 
/// Tests the simulation pipeline WITHOUT signal detection
/// Logs detailed simulation results for debugging
///
/// This example:
/// 1. Runs transactions through simulation
/// 2. Logs all simulation details to dev directory
/// 3. Skips signal manager processing entirely

use std::time::{Duration, Instant};
use std::sync::Arc;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, debug};
use tokio::sync::Mutex;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use chrono::Local;
use ethers::types::H256;
use hex;

// Mempool processor imports
use mempool_processor::{
    mempool_fetcher::NonBlockingIpcClient,
    function_detector::FunctionDetector,
    tx_router::{TransactionRouter as TxRouter, TransactionCategory, SimulationPriority},
    simulator::{SimulationManager, SimulationRequest, SimulationType, SequentialBuySellSimulator, BuySellSimulatorConfig, TxSimulator},
    signal_detector::SignalManagerConfig,
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
    let log_path = format!("{}/simulation_test_{}.log", log_dir, timestamp);
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;
    
    writeln!(log_file, "Simulation Only Test - NO Signal Detection")?;
    writeln!(log_file, "Started: {}", Local::now())?;
    writeln!(log_file, "Target: {} transactions", args.target_count)?;
    writeln!(log_file, "======================================\n")?;
    
    info!("🚀 Starting Simulation Only Test");
    info!("📝 Logging to: {}", log_path);
    info!("🎯 Target: {} transactions", args.target_count);
    info!("⚠️  Signal detection is DISABLED for this test");
    
    // Initialize token tracking subscriber
    info!("\n📦 Initializing token tracking subscriber...");
    let mut token_subscriber = TokenTrackingSubscriber::new(0.1); // 0.1 ETH threshold
    let token_cache = token_subscriber.get_cache();
    
    // Start subscriber in background
    let subscriber_handle = tokio::spawn(async move {
        if let Err(e) = token_subscriber.start_listening().await {
            warn!("Token subscriber error: {}", e);
        }
    });
    
    // Wait for initial cache population
    info!("⏳ Waiting for token cache population...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let initial_pools = token_cache.pools.get_pool_count().await;
    let initial_creators = token_cache.get_creator_count().await;
    info!("📊 Token Cache Statistics:");
    info!("   Total Pools: {}", initial_pools);
    info!("   Total Creators: {}", initial_creators);
    
    // Log some sample pools
    info!("\n📋 Sample pools in cache:");
    let all_pools = token_cache.pools.get_all_pools().await;
    for (i, (pool_addr, pool_state)) in all_pools.iter().take(5).enumerate() {
        info!("   {}: {} - {:.4} ETH, token: {}", 
            i+1, 
            pool_addr, 
            pool_state.eth_reserve,
            pool_state.token_address
        );
    }
    
    // Log cache details
    info!("\n📡 Cache configuration:");
    info!("   ETH threshold: 0.1 ETH");
    info!("   Monitoring pools above threshold: {}", initial_pools);
    
    // Initialize components
    info!("\n🔧 Initializing pipeline components...");
    
    // IPC client
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");
    
    // Function detector
    let function_detector = FunctionDetector::new();
    info!("✅ Function detector initialized");
    
    // TX Router
    let tx_router = TxRouter::new(Some(token_cache.clone()));
    info!("✅ Transaction router initialized");
    
    // Initialize simulators
    let tx_simulator = Arc::new(TxSimulator::new(&args.reth_db_path).unwrap());
    let buy_sell_config = BuySellSimulatorConfig::default();
    let buy_sell_simulator = Arc::new(SequentialBuySellSimulator::with_config(&args.reth_db_path, buy_sell_config).unwrap());
    
    // Simulation Manager (we'll skip the signal manager part)
    let signal_config = SignalManagerConfig::default();
    let simulation_manager = SimulationManager::new(
        tx_simulator,
        buy_sell_simulator,
        token_cache.clone(),
        signal_config,
        10, // max concurrent simulations
    );
    info!("✅ Simulation manager initialized");
    
    info!("\n🏃 Starting simulation testing...\n");
    
    let mut total_processed = 0;
    let pipeline_start = Instant::now();
    
    // Stats tracking
    let mut total_simulations = 0;
    let mut successful_simulations = 0;
    let mut failed_simulations = 0;
    let mut can_buy_count = 0;
    let mut can_sell_count = 0;
    let mut both_tradeable = 0;
    
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
            let detected_function = function_detector.detect_function(&tx);
            
            // 2. Transaction Routing
            let classification = tx_router.classify(&tx).await;
            let category = classification.category;
            
            // Skip non-relevant transactions
            match &category {
                TransactionCategory::ContractCreation { .. } |
                TransactionCategory::CreatorTransaction { .. } => {
                    // Continue with simulation
                }
                _ => {
                    continue;
                }
            }
            
            // 3. Create simulation request
            let sim_request = SimulationRequest {
                tx: tx.clone(),
                category: category.clone(),
                priority: match &category {
                    TransactionCategory::ContractCreation { .. } => SimulationPriority::High,
                    TransactionCategory::CreatorTransaction { .. } => SimulationPriority::Normal,
                    _ => SimulationPriority::Low,
                },
                simulation_type: SimulationType::TransactionWithBuySell,
                tx_hash: H256::from_slice(hex::decode(&tx.hash.trim_start_matches("0x")).unwrap_or_default().as_slice()),
            };
            
            // 4. Submit for simulation
            if let Err(e) = simulation_manager.submit(sim_request).await {
                warn!("Failed to submit simulation for {}: {}", tx.hash, e);
                continue;
            }
        }
        
        // Process simulations in batches
        let results = simulation_manager.process_queue().await;
        
        for result in results {
            total_simulations += 1;
            
            // Log detailed simulation result
            writeln!(log_file, "\n=== Simulation Result ===")?;
            writeln!(log_file, "Transaction: {}", result.request.tx.hash)?;
            writeln!(log_file, "Category: {:?}", result.request.category)?;
            writeln!(log_file, "Simulation Time: {:.2}ms", result.simulation_time_ms)?;
            
            // For creator transactions, log available token info
            if let TransactionCategory::CreatorTransaction { creator, target_token, .. } = &result.request.category {
                writeln!(log_file, "\n--- Creator Transaction Details ---")?;
                writeln!(log_file, "Creator: {}", creator)?;
                writeln!(log_file, "Target Token from Router: {:?}", target_token)?;
                
                // Try to get token info from cache
                if let Some(token_info) = token_cache.get_token_for_creator(creator).await {
                    writeln!(log_file, "\n🔍 Token Info from Cache:")?;
                    writeln!(log_file, "  Token Address: {}", token_info.token_address)?;
                    writeln!(log_file, "  Symbol: {}", token_info.symbol.as_ref().unwrap_or(&"Unknown".to_string()))?;
                    writeln!(log_file, "  Name: {}", token_info.name.as_ref().unwrap_or(&"Unknown".to_string()))?;
                    writeln!(log_file, "  Trading Enabled: {}", token_info.trading_enabled)?;
                    writeln!(log_file, "  Creation Block: {}", token_info.creation_block)?;
                    writeln!(log_file, "  Current Owner: {}", token_info.current_owner)?;
                    writeln!(log_file, "  Pools: {}", token_info.pools.len())?;
                    
                    if let Some(primary_pool) = token_cache.get_primary_pool(&token_info.token_address).await {
                        writeln!(log_file, "  Primary Pool: {} ({:.4} {} liquidity)", 
                            primary_pool.pool_address, 
                            primary_pool.denom_reserve, 
                            primary_pool.denom_currency)?;
                    }
                    
                    // Show tax info if available
                    if let Some(buy_tax) = token_info.buy_tax_python {
                        writeln!(log_file, "  Buy Tax: {:.1}%", buy_tax)?;
                    }
                    if let Some(sell_tax) = token_info.sell_tax_python {
                        writeln!(log_file, "  Sell Tax: {:.1}%", sell_tax)?;
                    }
                } else {
                    writeln!(log_file, "  ⚠️  No token info found in cache for creator {}", creator)?;
                }
            }
            
            if let Some(error) = &result.error {
                failed_simulations += 1;
                writeln!(log_file, "Error: {}", error)?;
            } else {
                successful_simulations += 1;
                
                // Log token/pool addresses
                if let Some(token_addr) = &result.token_address {
                    writeln!(log_file, "Token Address: {:?}", token_addr)?;
                }
                if let Some(pool_addr) = &result.pool_address {
                    writeln!(log_file, "Pool Address: {:?}", pool_addr)?;
                }
                
                // Log transaction simulation results
                if let Some(tx_changes) = &result.tx_state_changes {
                    writeln!(log_file, "\nTransaction State Changes:")?;
                    writeln!(log_file, "  Addresses affected: {}", tx_changes.len())?;
                    
                    // Log first few state changes
                    for (i, (addr, changes)) in tx_changes.iter().enumerate() {
                        if i >= 3 { 
                            writeln!(log_file, "  ... and {} more addresses", tx_changes.len() - 3)?;
                            break;
                        }
                        writeln!(log_file, "  Address {:?}:", addr)?;
                        if !changes.eth_net.is_zero() {
                            let eth_value = format!("{:?}", changes.eth_net);
                            writeln!(log_file, "    ETH change: {}", eth_value)?;
                        }
                        if !changes.token_net.is_empty() {
                            writeln!(log_file, "    Token changes: {} tokens", changes.token_net.len())?;
                        }
                    }
                }
                
                // Log buy/sell results
                if let Some(bs_result) = &result.buy_sell_result {
                    writeln!(log_file, "\nBuy/Sell Test Results:")?;
                    writeln!(log_file, "  Can Buy: {}", bs_result.can_buy)?;
                    writeln!(log_file, "  Can Sell: {}", bs_result.can_sell)?;
                    
                    if bs_result.can_buy {
                        can_buy_count += 1;
                    }
                    if bs_result.can_sell {
                        can_sell_count += 1;
                    }
                    if bs_result.can_buy && bs_result.can_sell {
                        both_tradeable += 1;
                    }
                    
                    // Log state changes from buy simulation
                    if let Some(buy_changes) = &bs_result.buy_state_changes {
                        writeln!(log_file, "  Buy simulation: {} addresses affected", buy_changes.len())?;
                    }
                    
                    // Log state changes from sell simulation
                    if let Some(sell_changes) = &bs_result.sell_state_changes {
                        writeln!(log_file, "  Sell simulation: {} addresses affected", sell_changes.len())?;
                    }
                }
            }
            
            writeln!(log_file, "========================\n")?;
        }
        
        // Progress update every 1000 transactions
        if total_processed % 1000 == 0 {
            let elapsed = pipeline_start.elapsed();
            let rate = total_processed as f64 / elapsed.as_secs_f64();
            info!("Progress: {}/{} transactions ({:.1} tx/sec)", 
                total_processed, args.target_count, rate);
            
            if total_simulations > 0 {
                info!("  Simulations: {} total, {} success, {} failed", 
                    total_simulations, successful_simulations, failed_simulations);
                info!("  Trading: {} can buy, {} can sell, {} both", 
                    can_buy_count, can_sell_count, both_tradeable);
            }
        }
    }
    
    let total_time = pipeline_start.elapsed();
    
    // Final summary - only show meaningful metrics
    let summary = if total_simulations > 0 {
        format!(
            "\n📊 SIMULATION TEST SUMMARY\n\
            =====================================\n\
            Total Transactions Processed: {}\n\
            Total Simulations Run: {}\n\
            Successful Simulations: {} ({:.1}%)\n\
            Failed Simulations: {} ({:.1}%)\n\
            \n\
            Trading Results:\n\
            - Can Buy: {} ({:.1}% of successful)\n\
            - Can Sell: {} ({:.1}% of successful)\n\
            - Both (Tradeable): {} ({:.1}% of successful)\n\
            \n\
            Time Elapsed: {:.2}s\n\
            Avg Simulation Time: {:.1}ms",
            total_processed,
            total_simulations,
            successful_simulations,
            successful_simulations as f64 / total_simulations as f64 * 100.0,
            failed_simulations,
            failed_simulations as f64 / total_simulations as f64 * 100.0,
            can_buy_count,
            can_buy_count as f64 / successful_simulations as f64 * 100.0,
            can_sell_count,
            can_sell_count as f64 / successful_simulations as f64 * 100.0,
            both_tradeable,
            both_tradeable as f64 / successful_simulations as f64 * 100.0,
            total_time.as_secs_f64(),
            total_time.as_millis() as f64 / total_simulations as f64
        )
    } else {
        format!(
            "\n📊 SIMULATION TEST SUMMARY\n\
            =====================================\n\
            Total Transactions Processed: {}\n\
            No simulations were run (no contract creations or creator transactions found)\n\
            Time Elapsed: {:.2}s",
            total_processed,
            total_time.as_secs_f64()
        )
    };
    
    println!("{}", summary);
    writeln!(log_file, "{}", summary)?;
    
    info!("\n✅ Simulation test complete!");
    info!("📄 Results saved to: {}", log_path);
    info!("🔍 This test ran WITHOUT signal detection");
    info!("   Use this log to debug simulation issues");
    
    Ok(())
}