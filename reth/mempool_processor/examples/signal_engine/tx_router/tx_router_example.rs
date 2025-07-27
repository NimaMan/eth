/// Transaction Router Example
/// 
/// Fetches 100 transactions, detects their functions, routes them,
/// and logs the routing results to /home/nima/code/crypto/logs/mempool/dev/

use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use mempool_processor::signal_engine::FunctionDetector;
use mempool_processor::signal_engine::tx_router::{TransactionRouter, TransactionCategory, SimulationPriority};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use eyre::Result;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Transaction Router Example");

    // Create log file
    let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let log_path = format!("/home/nima/code/crypto/logs/mempool/dev/tx_router_{}.log", timestamp);
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&log_path)?;
    
    writeln!(log_file, "Transaction Router Log - {}", timestamp)?;
    writeln!(log_file, "=")?;
    info!("📁 Logging to: {}", log_path);

    // Initialize components
    let ipc_client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start().await?;
    info!("✅ Connected to IPC");

    let function_detector = FunctionDetector::new();
    let tx_router = TransactionRouter::new(None);
    info!("✅ Router initialized");

    // Process 100 transactions
    let target_count = 100u64;
    let mut processed = 0u64;
    
    // Statistics
    let mut contract_creations = 0u64;
    let mut creator_actions = 0u64;
    let mut dex_interactions = 0u64;
    let mut regular_txs = 0u64;

    info!("📊 Processing {} transactions...", target_count);

    while processed < target_count {
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
            
            // Write to log file
            writeln!(log_file, "\nTransaction: {}", tx.hash)?;
            writeln!(log_file, "  From: 0x{}", hex::encode(&tx.from))?;
            writeln!(log_file, "  To: {}", 
                tx.to.as_ref()
                    .map(|addr| format!("0x{}", hex::encode(addr)))
                    .unwrap_or_else(|| "CONTRACT_CREATION".to_string()))?;
            writeln!(log_file, "  Functions: {:?}", tx.functions)?;
            
            // Log routing result
            match &routing_result.category {
                TransactionCategory::ContractCreation { deployer, contract_address, is_token, has_liquidity_in_calldata } => {
                    contract_creations += 1;
                    writeln!(log_file, "  Category: CONTRACT_CREATION")?;
                    writeln!(log_file, "    Deployer: {}", deployer)?;
                    writeln!(log_file, "    Contract: {}", contract_address)?;
                    writeln!(log_file, "    Is Token: {}", is_token)?;
                    writeln!(log_file, "    Has Liquidity: {}", has_liquidity_in_calldata)?;
                }
                TransactionCategory::CreatorTransaction { creator, target_address, target_token, function_type } => {
                    creator_actions += 1;
                    writeln!(log_file, "  Category: CREATOR_TRANSACTION")?;
                    writeln!(log_file, "    Creator: {}", creator)?;
                    writeln!(log_file, "    Target: {}", target_address)?;
                    writeln!(log_file, "    Token: {:?}", target_token)?;
                    writeln!(log_file, "    Function: {:?}", function_type)?;
                }
                TransactionCategory::DexInteraction { dex_type, action, token_address, pool_address } => {
                    dex_interactions += 1;
                    writeln!(log_file, "  Category: DEX_INTERACTION")?;
                    writeln!(log_file, "    DEX: {:?}", dex_type)?;
                    writeln!(log_file, "    Action: {:?}", action)?;
                    writeln!(log_file, "    Token: {:?}", token_address)?;
                    writeln!(log_file, "    Pool: {:?}", pool_address)?;
                }
                TransactionCategory::Regular { is_transfer, is_approval } => {
                    regular_txs += 1;
                    writeln!(log_file, "  Category: REGULAR")?;
                    writeln!(log_file, "    Is Transfer: {}", is_transfer)?;
                    writeln!(log_file, "    Is Approval: {}", is_approval)?;
                }
            }
            
            writeln!(log_file, "  Priority: {:?}", routing_result.priority)?;
            writeln!(log_file, "  Requires Simulation: {}", routing_result.requires_simulation)?;
            writeln!(log_file, "  Requires Buy/Sell Test: {}", routing_result.requires_buy_sell_test)?;
            
            processed += 1;
            if processed >= target_count {
                break;
            }
        }
    }

    // Write summary
    writeln!(log_file, "\n\nSUMMARY")?;
    writeln!(log_file, "=")?;
    writeln!(log_file, "Total Transactions: {}", processed)?;
    writeln!(log_file, "Contract Creations: {}", contract_creations)?;
    writeln!(log_file, "Creator Actions: {}", creator_actions)?;
    writeln!(log_file, "DEX Interactions: {}", dex_interactions)?;
    writeln!(log_file, "Regular Transactions: {}", regular_txs)?;
    log_file.flush()?;

    // Console summary
    info!("\n📊 Transaction Routing Complete");
    info!("   Contract Creations: {}", contract_creations);
    info!("   Creator Actions: {}", creator_actions);
    info!("   DEX Interactions: {}", dex_interactions);
    info!("   Regular Transactions: {}", regular_txs);
    info!("📁 Full log written to: {}", log_path);

    Ok(())
}