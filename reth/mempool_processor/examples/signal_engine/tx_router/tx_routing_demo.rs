/// Demonstration of Transaction Router
/// 
/// This example shows how the tx_router routes transactions to appropriate
/// simulation strategies based on their characteristics.
///
/// The tx_router:
/// - Categorizes transactions (contract creation, creator tx, DEX, regular)
/// - Assigns simulation priorities (Critical, High, Normal, Low)
/// - Determines simulation strategy (tx only, tx+buy/sell, buy/sell only)
/// - Routes to appropriate simulation path

use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use mempool_processor::signal_engine::FunctionDetector;
use mempool_processor::signal_engine::tx_router::{TransactionRouter, SimulationPriority};
use mempool_processor::signal_engine::simulator::{
    SimulationOrchestrator, SimulationType, SimulationRequest,
    buy_sell_simulator::SequentialBuySellSimulator,
};
use mempool_processor::tx_simulator::RethDirectTxSimulator;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use eyre::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Transaction Router Demo");
    info!("   This demo shows how transactions are routed to different simulation strategies");

    // Initialize components
    let ipc_client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start().await?;
    info!("✅ Connected to IPC");

    let function_detector = FunctionDetector::new();
    
    // Create tx router (no token cache for this demo)
    let tx_router = TransactionRouter::new(None);
    info!("✅ Transaction router initialized");

    // Initialize simulators
    let reth_db_path = std::env::var("RETH_DB_PATH")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet/db".to_string());
    let tx_simulator = Arc::new(RethDirectTxSimulator::new(&reth_db_path)?);
    let buy_sell_simulator = Arc::new(SequentialBuySellSimulator::new(
        std::env::var("ETH_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string()),
        0.01, // 0.01 ETH for testing
    ));
    
    let simulation_orchestrator = Arc::new(SimulationOrchestrator::new(
        tx_simulator,
        buy_sell_simulator,
        10, // max concurrent simulations
    ));
    info!("✅ Simulation orchestrator initialized");

    // Statistics tracking
    let mut stats = HashMap::new();
    stats.insert("contract_creations", 0u64);
    stats.insert("creator_actions", 0u64);
    stats.insert("dex_interactions", 0u64);
    stats.insert("regular_txs", 0u64);
    stats.insert("critical_priority", 0u64);
    stats.insert("high_priority", 0u64);
    stats.insert("normal_priority", 0u64);
    stats.insert("low_priority", 0u64);
    stats.insert("simulations_requested", 0u64);
    stats.insert("buy_sell_tests_requested", 0u64);

    // Process transactions
    let target_count = 100u64;
    let mut processed = 0u64;

    info!("\n📊 Processing {} transactions...\n", target_count);

    while processed < target_count {
        // Get batch of transactions
        let new_txs = ipc_client.get_transactions(20).await?;
        
        if new_txs.is_empty() {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            continue;
        }

        // Detect functions
        let transactions_with_functions = function_detector.detect_batch(new_txs);
        
        // Route each transaction
        for tx in transactions_with_functions {
            let routing_result = tx_router.classify(&tx).await;
            
            // Update category stats
            match &routing_result.category {
                mempool_processor::signal_engine::tx_router::TransactionCategory::ContractCreation { is_token, .. } => {
                    *stats.get_mut("contract_creations").unwrap() += 1;
                    if *is_token {
                        info!("📝 Token Contract Creation detected! Hash: {}", tx.hash);
                    }
                }
                mempool_processor::signal_engine::tx_router::TransactionCategory::CreatorTransaction { function_type, target_token, .. } => {
                    *stats.get_mut("creator_actions").unwrap() += 1;
                    info!("👤 Creator Action: {:?} on token {:?}", function_type, target_token);
                }
                mempool_processor::signal_engine::tx_router::TransactionCategory::DexInteraction { dex_type, action, .. } => {
                    *stats.get_mut("dex_interactions").unwrap() += 1;
                    info!("💱 DEX: {:?} - {:?}", dex_type, action);
                }
                mempool_processor::signal_engine::tx_router::TransactionCategory::Regular { .. } => {
                    *stats.get_mut("regular_txs").unwrap() += 1;
                }
            }
            
            // Update priority stats
            match routing_result.priority {
                SimulationPriority::Critical => *stats.get_mut("critical_priority").unwrap() += 1,
                SimulationPriority::High => *stats.get_mut("high_priority").unwrap() += 1,
                SimulationPriority::Normal => *stats.get_mut("normal_priority").unwrap() += 1,
                SimulationPriority::Low => *stats.get_mut("low_priority").unwrap() += 1,
            }
            
            // Update simulation strategy stats
            if routing_result.requires_simulation {
                *stats.get_mut("simulations_requested").unwrap() += 1;
            }
            if routing_result.requires_buy_sell_test {
                *stats.get_mut("buy_sell_tests_requested").unwrap() += 1;
            }
            
            // Determine simulation type based on routing
            let simulation_type = match (&routing_result.category, routing_result.requires_buy_sell_test) {
                (_, true) if routing_result.requires_simulation => SimulationType::TransactionWithBuySell,
                (_, true) => SimulationType::BuySellOnly,
                _ if routing_result.requires_simulation => SimulationType::TransactionOnly,
                _ => continue, // Skip if no simulation needed
            };
            
            // Create simulation request
            if routing_result.priority <= SimulationPriority::High {
                let request = SimulationRequest {
                    tx: tx.clone(),
                    category: routing_result.category.clone(),
                    priority: routing_result.priority,
                    simulation_type,
                    tx_hash: ethers::types::H256::from_slice(&hex::decode(&tx.hash[2..]).unwrap()),
                };
                
                // Queue for simulation (in production, this would go to a queue)
                info!("   → Routing to {:?} simulation with {:?} priority", 
                      simulation_type, routing_result.priority);
                
                // For demo purposes, we just track it
                simulation_orchestrator.queue_simulation(request).await;
            }
            
            processed += 1;
            if processed >= target_count {
                break;
            }
        }
    }

    // Display routing statistics
    info!("\n📊 === Transaction Routing Statistics ===");
    info!("✅ Total transactions processed: {}", processed);
    
    info!("\n🔍 Transaction Categories:");
    info!("   📝 Contract creations: {}", stats["contract_creations"]);
    info!("   👤 Creator actions: {}", stats["creator_actions"]);
    info!("   💱 DEX interactions: {}", stats["dex_interactions"]);
    info!("   💸 Regular transactions: {}", stats["regular_txs"]);
    
    info!("\n⚡ Routing Priorities:");
    info!("   🔴 Critical: {}", stats["critical_priority"]);
    info!("   🟠 High: {}", stats["high_priority"]);
    info!("   🟡 Normal: {}", stats["normal_priority"]);
    info!("   🟢 Low: {}", stats["low_priority"]);
    
    info!("\n🔬 Simulation Strategies:");
    info!("   🧪 Transactions requiring simulation: {}", stats["simulations_requested"]);
    info!("   💰 Transactions requiring buy/sell test: {}", stats["buy_sell_tests_requested"]);
    
    let orchestrator_stats = simulation_orchestrator.get_stats().await;
    info!("\n📈 Simulation Orchestrator Stats:");
    info!("   Total requests: {}", orchestrator_stats.total_requests);
    info!("   Successful simulations: {}", orchestrator_stats.successful_simulations);
    info!("   Failed simulations: {}", orchestrator_stats.failed_simulations);
    
    Ok(())
}