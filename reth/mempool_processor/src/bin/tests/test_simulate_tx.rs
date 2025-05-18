/*
 * Ethereum Mempool Transaction Simulator
 * 
 * This tool:
 * 1. Connects to an Ethereum node and monitors the mempool
 * 2. Tracks addresses with balances >1 ETH
 * 3. Simulates mempool transactions involving high-value addresses
 * 4. Reports potential suspicious activity
 */

use mempool_processor::mempool_processor::*;
use mempool_processor::tx_simulator::simulator::*;
use ethers::prelude::*;
use eyre::Result;
use std::time::{Duration, Instant};
use tracing::{info, warn, debug, error};
use std::sync::Arc;
use clap::Parser;
use std::collections::HashSet;

#[derive(Parser, Debug)]
struct Args {
    /// HTTP RPC endpoint
    #[arg(long, env = "HTTP_RPC_URL", default_value = "http://localhost:8545")]
    http_rpc_url: String,
    
    /// WebSocket endpoint for real-time updates
    #[arg(long, env = "WS_RPC_URL", default_value = "ws://localhost:8546")]
    ws_rpc_url: String,
    
    /// Minimum ETH balance to track (in ETH, not wei)
    #[arg(long, default_value = "1.0")]
    min_eth_balance: f64,
    
    /// Poll interval for mempool in milliseconds
    #[arg(long, default_value = "2000")]
    poll_interval_ms: u64,
    
    /// Verbose logging mode
    #[arg(short, long)]
    verbose: bool,
}

fn format_eth(wei: U256) -> String {
    let eth_value = wei.as_u128() as f64 / 1_000_000_000_000_000_000f64;
    format!("{:.6} ETH", eth_value)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Configure tracing based on verbosity
    let env_filter = if args.verbose {
        "mempool_processor=debug,info"
    } else {
        "mempool_processor=info"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(env_filter))
        .init();

    info!("======== Ethereum Mempool Transaction Simulator ========");
    info!("Starting with configuration:");
    info!("  HTTP RPC URL: {}", args.http_rpc_url);
    info!("  WebSocket URL: {}", args.ws_rpc_url);
    info!("  Min ETH balance: {} ETH", args.min_eth_balance);
    info!("  Poll interval: {}ms", args.poll_interval_ms);
    info!("  Verbose mode: {}", args.verbose);
    info!("========================================================");
    
    // Connect to Ethereum node
    info!("Connecting to Ethereum node at {}", args.http_rpc_url);
    let http_provider = Provider::<Http>::try_from(args.http_rpc_url.clone())?;
    let http_provider = Arc::new(http_provider);
    
    // Get basic network info
    let network_id = http_provider.get_net_version().await?;
    let block_number = http_provider.get_block_number().await?;
    let chain_id = http_provider.get_chainid().await?;
    
    info!("Connected to Ethereum network");
    info!("  Chain ID: {}", chain_id);
    info!("  Network ID: {}", network_id);
    info!("  Current block: {}", block_number);
    
    // Set up transaction simulator
    let mut simulator = TransactionSimulator::new(
        Arc::clone(&http_provider),
        args.min_eth_balance
    );
    
    // Setup mempool fetcher
    let fetcher = MempoolFetcher::new(&args.http_rpc_url)?;
    
    // Setup WebSocket provider for subscriptions
    info!("Connecting to WebSocket endpoint at {}", args.ws_rpc_url);
    let ws = Ws::connect(&args.ws_rpc_url).await?;
    let ws_provider = Provider::new(ws);
    
    // Store seen transaction hashes to avoid duplicates
    let mut seen_hashes = HashSet::new();
    
    // Try to use WebSocket subscription first, fall back to polling
    match ws_provider.subscribe_pending_txs().await {
        Ok(mut stream) => {
            info!("Successfully subscribed to pending transactions via WebSocket!");
            
            // Setup interval for stats logging
            let stats_interval = Duration::from_secs(30);
            let mut stats_timer = tokio::time::interval(stats_interval);
            
            info!("Watching for transactions...");
            let mut tx_count = 0;
            let mut high_value_tx_count = 0;
            
            // Main processing loop
            loop {
                tokio::select! {
                    // New transaction from subscription
                    tx_hash_opt = stream.next() => {
                        if let Some(tx_hash) = tx_hash_opt {
                            // Skip if already seen
                            if seen_hashes.contains(&tx_hash) {
                                continue;
                            }
                            
                            seen_hashes.insert(tx_hash);
                            tx_count += 1;
                            
                            // Get full transaction details
                            match http_provider.get_transaction(tx_hash).await {
                                Ok(Some(tx)) => {
                                    // Convert to our format
                                    let from = tx.from;
                                    let hash_bytes = tx.hash.as_bytes().to_vec();
                                    let from_bytes = from.as_bytes().to_vec();
                                    let to_bytes = tx.to.map(|to| to.as_bytes().to_vec());
                                    
                                    let tx_view = TransactionView {
                                        hash: hash_bytes,
                                        from: from_bytes,
                                        to: to_bytes,
                                        value: tx.value,
                                        gas_price: Some(tx.gas_price.unwrap_or_default()),
                                        gas_limit: Some(tx.gas),
                                        nonce: Some(tx.nonce),
                                        input_data: Some(tx.input.to_vec()),
                                    };
                                    
                                    // Process with simulator
                                    if let Ok(Some(result)) = simulator.process_transaction(&tx_view).await {
                                        high_value_tx_count += 1;
                                        
                                        info!("HIGH-VALUE TX #{}: {} ETH from {} to {}", 
                                            high_value_tx_count,
                                            format_eth(result.value),
                                            format!("0x{}", hex::encode(result.from.as_bytes())),
                                            format!("0x{}", hex::encode(result.to.as_bytes())));
                                        
                                        info!("  From balance: {}", format_eth(result.from_balance_before));
                                        info!("  To balance: {}", format_eth(result.to_balance_before));
                                    }
                                },
                                Ok(None) => {
                                    debug!("Transaction not found: {}", tx_hash);
                                },
                                Err(e) => {
                                    warn!("Error fetching transaction {}: {}", tx_hash, e);
                                }
                            }
                        } else {
                            // Stream ended, try to reconnect
                            warn!("WebSocket subscription ended, attempting to reconnect...");
                            break;
                        }
                    }
                    
                    // Log stats periodically
                    _ = stats_timer.tick() => {
                        info!("STATUS: Processed {} transactions, {} high-value", tx_count, high_value_tx_count);
                        
                        // Show high-value accounts being tracked
                        let high_value_accounts = simulator.get_high_value_accounts();
                        info!("Tracking {} high-value accounts", high_value_accounts.len());
                        
                        for (i, (addr, balance)) in high_value_accounts.iter().enumerate().take(10) {
                            info!("  Account #{}: 0x{} - {}", 
                                 i+1, 
                                 hex::encode(addr.as_bytes()),
                                 format_eth(*balance));
                        }
                        
                        if high_value_accounts.len() > 10 {
                            info!("  ... and {} more", high_value_accounts.len() - 10);
                        }
                        
                        // Prune old hashes if needed
                        if seen_hashes.len() > 10_000 {
                            info!("Pruning seen transaction set: {} entries", seen_hashes.len());
                            seen_hashes.clear();
                            info!("Cleared transaction hash cache");
                        }
                    }
                }
            }
        },
        Err(e) => {
            warn!("Failed to subscribe to pending transactions: {}", e);
            warn!("Falling back to polling approach...");
            
            // Fallback to polling approach
            let poll_interval = Duration::from_millis(args.poll_interval_ms);
            info!("Starting mempool polling with interval of {}ms", args.poll_interval_ms);
            
            let mut tx_count = 0;
            let mut high_value_tx_count = 0;
            let mut poll_count = 0;
            
            // Set of transaction hashes we've already seen
            let mut known_txs = HashSet::new();
            
            loop {
                poll_count += 1;
                
                // Log status every 20 polls
                if poll_count % 20 == 0 {
                    info!("STATUS: {} polls, {} transactions processed, {} high-value", 
                          poll_count, tx_count, high_value_tx_count);
                    
                    // Show high-value accounts
                    let high_value_accounts = simulator.get_high_value_accounts();
                    info!("Tracking {} high-value accounts", high_value_accounts.len());
                    
                    for (i, (addr, balance)) in high_value_accounts.iter().enumerate().take(10) {
                        info!("  Account #{}: 0x{} - {}", 
                             i+1, 
                             hex::encode(addr.as_bytes()),
                             format_eth(*balance));
                    }
                    
                    if high_value_accounts.len() > 10 {
                        info!("  ... and {} more", high_value_accounts.len() - 10);
                    }
                    
                    // Prune old tx hashes if needed
                    if known_txs.len() > 20_000 {
                        info!("Pruning seen transaction set: {} entries", known_txs.len());
                        known_txs.clear();
                        info!("Cleared transaction hash cache");
                    }
                }
                
                // Fetch transactions
                let start_time = Instant::now();
                match fetcher.get_transactions().await {
                    Ok(transactions) => {
                        debug!("Retrieved {} transactions in {:?}", 
                              transactions.len(), start_time.elapsed());
                        
                        // Process new transactions
                        let mut new_txs = 0;
                        let mut high_value = 0;
                        
                        for tx in transactions {
                            let hash_hex = hex::encode(&tx.hash);
                            
                            // Skip if already seen
                            if known_txs.contains(&hash_hex) {
                                continue;
                            }
                            
                            known_txs.insert(hash_hex);
                            new_txs += 1;
                            tx_count += 1;
                            
                            // Process with simulator
                            if let Ok(Some(result)) = simulator.process_transaction(&tx).await {
                                high_value += 1;
                                high_value_tx_count += 1;
                                
                                info!("HIGH-VALUE TX #{}: {} ETH from {} to {}", 
                                    high_value_tx_count,
                                    format_eth(result.value),
                                    format!("0x{}", hex::encode(result.from.as_bytes())),
                                    format!("0x{}", hex::encode(result.to.as_bytes())));
                                
                                info!("  From balance: {}", format_eth(result.from_balance_before));
                                info!("  To balance: {}", format_eth(result.to_balance_before));
                            }
                        }
                        
                        if new_txs > 0 {
                            debug!("Processed {} new transactions ({} high-value)", new_txs, high_value);
                        }
                    },
                    Err(e) => {
                        error!("Failed to get mempool transactions: {}", e);
                    }
                }
                
                // Sleep until next poll
                let poll_time = start_time.elapsed();
                let sleep_time = if poll_time < poll_interval {
                    poll_interval - poll_time
                } else {
                    Duration::from_millis(100) // Minimum sleep
                };
                
                tokio::time::sleep(sleep_time).await;
            }
        }
    }
    
    Ok(())
} 