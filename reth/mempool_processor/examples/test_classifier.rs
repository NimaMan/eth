/// Test the transaction classifier
/// 
/// This example tests the classifier with sample transactions

use mempool_processor::mempool_fetcher::{NonBlockingIpcClient, SimpleFunctionDetector};
use mempool_processor::signal_engine::classifier::TransactionClassifier;
use mempool_processor::token_tracking::TokenTrackingCache;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use eyre::Result;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting classifier test");

    // Initialize components
    let ipc_client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start().await?;
    info!("✅ Connected to IPC");

    let function_detector = SimpleFunctionDetector::new();
    
    // Create token cache (optional, but helps with classification)
    let token_cache = None; // In production, this would connect to ZMQ
    
    // Create classifier
    let classifier = TransactionClassifier::new(token_cache);
    info!("✅ Classifier initialized");

    // Process 50 transactions
    let mut total_processed = 0u64;
    let target_count = 50u64;
    
    let mut contract_creations = 0;
    let mut creator_txs = 0;
    let mut dex_interactions = 0;
    let mut regular_txs = 0;

    while total_processed < target_count {
        // Get batch of transactions
        let mut new_txs = ipc_client.get_transactions(50).await?;
        
        if new_txs.is_empty() {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            continue;
        }

        // Detect functions
        let transactions_with_functions = function_detector.detect_batch(new_txs);
        
        // Classify each transaction
        for tx in &transactions_with_functions {
            let result = classifier.classify(tx).await;
            
            match &result.category {
                mempool_processor::signal_engine::classifier::TransactionCategory::ContractCreation { .. } => {
                    contract_creations += 1;
                    info!("📝 Contract Creation - Priority: {:?}", result.priority);
                }
                mempool_processor::signal_engine::classifier::TransactionCategory::CreatorTransaction { .. } => {
                    creator_txs += 1;
                    info!("👤 Creator Transaction - Priority: {:?}", result.priority);
                }
                mempool_processor::signal_engine::classifier::TransactionCategory::DexInteraction { dex_type, action, .. } => {
                    dex_interactions += 1;
                    info!("💱 DEX Interaction: {:?} {:?} - Priority: {:?}", dex_type, action, result.priority);
                }
                mempool_processor::signal_engine::classifier::TransactionCategory::Regular { is_transfer, is_approval } => {
                    regular_txs += 1;
                    if *is_transfer || *is_approval {
                        info!("💸 Regular: transfer={}, approval={}", is_transfer, is_approval);
                    }
                }
            }
            
            total_processed += 1;
            if total_processed >= target_count {
                break;
            }
        }
    }
    
    // Final statistics
    info!("\n📊 === Classification Results ===");
    info!("✅ Total transactions: {}", total_processed);
    info!("📝 Contract creations: {}", contract_creations);
    info!("👤 Creator transactions: {}", creator_txs);
    info!("💱 DEX interactions: {}", dex_interactions);
    info!("💸 Regular transactions: {}", regular_txs);
    
    Ok(())
}