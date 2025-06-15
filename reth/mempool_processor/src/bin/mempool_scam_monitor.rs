/*
 * Mempool Scam Monitor
 * 
 * Real-time scam detection service that monitors mempool transactions
 * for potential scams on liquidity pools using state change analysis.
 */

use std::sync::Arc;
use std::time::Duration;
use eyre::Result;
use tracing::{info, warn, error};

// Mempool processor imports
use mempool_processor::mempool_fetcher::WebSocketClient;
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::mempool_scam_detector::{MempoolScamDetector, PoolInfo};

// Ethers imports
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber, H256};

const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mempool_scam_monitor=info,mempool_processor=info")
        .init();
    
    info!("🚀 Starting Mempool Scam Monitor");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    let zmq_url = "tcp://localhost:5557";
    let eth_threshold = 0.01;
    
    // Initialize HTTP provider
    info!("📡 Connecting to Ethereum node...");
    let http_provider = Arc::new(Provider::<Http>::try_from(http_url)?);
    
    // Get current block
    let latest_block = http_provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    info!("📦 Current block: #{}", latest_block.number.unwrap_or_default());
    
    // Initialize WebSocket client
    info!("🔌 Initializing WebSocket client...");
    let ws_client = Arc::new(WebSocketClient::new(ws_url, http_url)?);
    
    // Start monitoring in background
    let ws_client_clone = ws_client.clone();
    tokio::spawn(async move {
        if let Err(e) = ws_client_clone.start_monitoring().await {
            error!("WebSocket monitoring failed: {}", e);
        }
    });
    
    // Give it time to connect
    tokio::time::sleep(Duration::from_secs(1)).await;
    info!("✅ WebSocket monitoring started");
    
    // Initialize pool subscriber
    info!("🏊 Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(eth_threshold, zmq_url);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        info!("Starting pool subscriber listener...");
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber failed: {}", e);
        }
    });
    
    // Give pool subscriber time to initialize
    tokio::time::sleep(Duration::from_secs(2)).await;
    let pool_count = pool_cache_clone.get_pool_count();
    info!("📊 Monitoring {} pools", pool_count);
    
    // Initialize scam detector
    let mut scam_detector = MempoolScamDetector::new(http_url, 1).await?;
    
    // Add some pools to watch (from the pool cache)
    let pools = pool_cache_clone.get_all_pools();
    let mut watched_count = 0;
    for (pool_address, pool_state) in pools.iter().take(10) { // Watch first 10 pools
        if pool_state.eth_reserve > eth_threshold {
            let pool_info = PoolInfo {
                token0: pool_state.token_address.parse()?,
                token1: WETH_ADDRESS.parse()?,
                liquidity_eth: pool_state.eth_reserve,
                is_honeypot: false,
            };
            scam_detector.add_watched_pool(pool_address.parse()?, pool_info);
            watched_count += 1;
        }
    }
    
    info!("👀 Watching {} pools for scams", watched_count);
    
    let mut total_processed = 0u64;
    let mut total_scams = 0u64;
    
    // Main processing loop
    info!("🔄 Starting main processing loop...");
    
    loop {
        // Get new transactions from WebSocket
        let new_txs = ws_client.get_transactions(100).await?;
        
        if new_txs.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        for ws_tx in new_txs {
            // Parse transaction hash
            let tx_hash = match ws_tx.hash.parse::<H256>() {
                Ok(hash) => hash,
                Err(e) => {
                    warn!("Invalid transaction hash {}: {}", ws_tx.hash, e);
                    continue;
                }
            };
            
            // Fetch full transaction
            match http_provider.get_transaction(tx_hash).await {
                Ok(Some(tx)) => {
                    // Analyze for scams
                    match scam_detector.analyze_transaction(tx).await {
                        Ok(Some(alert)) => {
                            total_scams += 1;
                            error!("🚨 SCAM DETECTED!");
                            error!("   Transaction: {:?}", alert.tx_hash);
                            error!("   Type: {:?}", alert.scam_type);
                            error!("   Severity: {:?}", alert.severity);
                            error!("   Pool: {:?}", alert.affected_pool);
                            error!("   Drain: {:.2} ETH ({:.1}%)", 
                                alert.drain_amount_eth, alert.drain_percentage);
                            error!("   Details: {}", alert.details);
                        }
                        Ok(None) => {
                            // No scam detected
                        }
                        Err(e) => {
                            warn!("Failed to analyze transaction {}: {}", tx_hash, e);
                        }
                    }
                    
                    total_processed += 1;
                    
                    if total_processed % 100 == 0 {
                        info!("📊 Processed {} transactions, found {} scams", 
                            total_processed, total_scams);
                    }
                }
                Ok(None) => {
                    warn!("Transaction {} not found", tx_hash);
                }
                Err(e) => {
                    warn!("Failed to fetch transaction {}: {}", tx_hash, e);
                }
            }
        }
    }
}