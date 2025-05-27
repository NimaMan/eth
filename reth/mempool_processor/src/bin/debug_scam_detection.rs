/*
 * Focused Debug Scam Detection Binary
 * 
 * Algorithm:
 * 1. Initialize mempool fetcher and pool subscriber
 * 2. Load pool cache from ZeroMQ (Python service)
 * 3. For each mempool batch:
 *    - Only process transactions that affect tracked pools
 *    - Log detailed pool state changes for relevant transactions
 *    - Compare with actual scam detection results
 * 4. Focus on ETH level changes and scam threshold violations
 */

use mempool_processor::mempool_processor::fetcher::MempoolFetcher;
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::mempool_processor::TransactionSource;
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::scam_detection::{ScamDetectionService, ScamDetectionConfig};
use mempool_processor::mempool_processor::db_logger::DbLogger;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use hex;
use ethers::prelude::*;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("🔍 Starting focused debug scam detection...");
    
    // Initialize pool subscriber
    let pool_subscriber = PoolSubscriber::with_endpoint(0.4, "tcp://localhost:5557");
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    tokio::spawn({
        let pool_subscriber_clone = pool_subscriber;
        async move {
            if let Err(e) = pool_subscriber_clone.start_listening().await {
                error!("Pool subscriber failed: {}", e);
            }
        }
    });
    
    // Give pool cache time to populate
    sleep(Duration::from_secs(3)).await;
    
    // Get initial pool count
    let pool_count = pool_cache.get_pool_count();
    info!("📊 Loaded {} pools for monitoring", pool_count);
    
    if pool_count == 0 {
        error!("❌ No pools loaded! Check ZeroMQ connection to Python service");
        return Ok(());
    }
    
    // Log some sample pools
    info!("🎯 Sample tracked pools:");
    let pools = pool_cache.get_all_pools();
    for (i, (addr, state)) in pools.iter().take(3).enumerate() {
        info!("   {}: {} ({:.6} ETH)", i+1, addr, state.eth_reserve);
    }
    
    // Initialize fetcher
    let fetcher = MempoolFetcher::new("http://localhost:8545")?;
    
    // Initialize scam detection service
    let db_logger = Arc::new(DbLogger::new(
        "postgres", "postgres", "localhost", 5432, "eth_db"
    ).await?);
    
    let scam_config = ScamDetectionConfig {
        eth_threshold: 0.4,
        percentage_threshold: 0.5,
    };
    
    let service = ScamDetectionService::new(
        pool_cache.clone(),
        db_logger,
        scam_config,
    );
    
    let mut total_processed = 0;
    let mut relevant_transactions = 0;
    let mut scams_detected = 0;
    
    loop {
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                total_processed += transactions.len();
                
                // Filter for transactions affecting tracked pools only
                let mut pool_affecting_txs = Vec::new();
                
                for tx in &transactions {
                    if transaction_affects_tracked_pools(tx, &pool_cache) {
                        pool_affecting_txs.push(tx);
                    }
                }
                
                if !pool_affecting_txs.is_empty() {
                    relevant_transactions += pool_affecting_txs.len();
                    info!("🎯 Found {} transactions affecting tracked pools (out of {})", 
                          pool_affecting_txs.len(), transactions.len());
                    
                    for tx in &pool_affecting_txs {
                        debug_transaction_pool_effects(tx, &pool_cache);
                    }
                }
                
                // Log summary every 1000 transactions
                if total_processed % 1000 == 0 {
                    info!("📈 Summary: {} total processed, {} relevant, {} scams detected", 
                          total_processed, relevant_transactions, scams_detected);
                }
            }
            Err(e) => {
                warn!("⚠️ Error fetching transactions: {}", e);
            }
        }
        
        sleep(Duration::from_millis(2000)).await;
    }
}

// Check if transaction affects any tracked pools
fn transaction_affects_tracked_pools(
    tx: &TransactionView, 
    pool_cache: &Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>
) -> bool {
    // Check if transaction involves any tracked pool addresses
    if let Some(to_bytes) = &tx.to {
        let to_str = format!("0x{}", hex::encode(to_bytes));
        if pool_cache.get_pool(&to_str).is_some() {
            return true;
        }
    }
    
    // For now, return false - in real implementation would check state changes
    // This is where you'd integrate with state simulation to see actual pool effects
    false
}

// Debug transaction effects on pools
fn debug_transaction_pool_effects(
    tx: &TransactionView, 
    pool_cache: &Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>
) {
    let tx_hash = hex::encode(&tx.hash);
    info!("🔍 === ANALYZING TX: {} ===", tx_hash);
    
    if let Some(to_bytes) = &tx.to {
        let to_str = format!("0x{}", hex::encode(to_bytes));
        if let Some(pool_state) = pool_cache.get_pool(&to_str) {
            info!("  📍 Target Pool: {}", to_str);
            info!("  💰 Current ETH Level: {:.6}", pool_state.eth_reserve);
            
            // Convert U256 to f64 properly
            let value_eth = tx.value.as_u128() as f64 / 1e18;
            info!("  💸 Transaction Value: {:.6} ETH", value_eth);
            
            // This is where you'd add state simulation to get the actual effect
            warn!("  ⚠️ Need state simulation to determine actual pool effect");
        }
    }
}