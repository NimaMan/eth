/*
 * State Cache Test Utility
 * 
 * This utility tests the in-memory state cache by:
 * 1. Connecting to an Ethereum node
 * 2. Fetching and simulating transactions from the mempool
 * 3. Storing state changes in the state cache
 * 4. Displaying aggregated state changes by address
 */

use clap::Parser;
use ethers::prelude::*;
use eyre::Result;
use std::sync::Arc;
use tracing::{info, error, debug, Level};
use mempool_processor::tx_simulator::{StateDiffTracker, StateCache};
use mempool_processor::mempool_processor::types::TransactionView;
use hex::encode as hex_encode;

#[derive(Parser, Debug)]
struct Args {
    /// HTTP RPC URL
    #[arg(long, env = "HTTP_RPC_URL", default_value = "http://localhost:8545")]
    http_rpc_url: String,
    
    /// Number of transactions to simulate
    #[arg(long, default_value = "10")]
    tx_count: usize,
    
    /// Minimum ETH value change to display (in ETH)
    #[arg(long, default_value = "0.01")]
    min_eth_change: f64,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Configure logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("State Cache Test Utility");
    info!("Connecting to Ethereum node: {}", args.http_rpc_url);
    
    // Connect to provider
    let provider = Provider::<Http>::try_from(args.http_rpc_url.clone())?;
    let provider = Arc::new(provider);
    
    // Check connection
    let block_number = provider.get_block_number().await?;
    let chain_id = provider.get_chainid().await?;
    info!("Connected to Ethereum chain ID: {}", chain_id);
    info!("Current block number: {}", block_number);
    
    // Initialize state diff tracker and cache
    info!("Initializing state diff tracker and state cache");
    let mut tracker = StateDiffTracker::new(provider.clone(), None);
    let mut state_cache = StateCache::new();
    
    // Get mempool transactions
    info!("Fetching transactions from the mempool...");
    let transactions = get_transactions(&provider).await?;
    info!("Found {} transactions", transactions.len());
    
    // Simulate transactions and cache results
    let max_txs = std::cmp::min(args.tx_count, transactions.len());
    info!("Simulating {} transactions and caching state changes...", max_txs);
    
    let mut successful_simulations = 0;
    for (i, tx) in transactions.iter().take(max_txs).enumerate() {
        let tx_hash_hex = hex_encode(&tx.hash);
        debug!("Simulating transaction {}/{}: {}", i + 1, max_txs, tx_hash_hex);
        
        match tracker.simulate_transaction(tx).await {
            Ok(Some(changes)) => {
                successful_simulations += 1;
                
                // Convert transaction hash to H256
                let mut hash_bytes = [0u8; 32];
                if tx.hash.len() == 32 {
                    hash_bytes.copy_from_slice(&tx.hash);
                    let tx_hash = H256::from(hash_bytes);
                    
                    // Add to state cache
                    state_cache.add_transaction(tx_hash, tx.clone(), changes.clone());
                    
                    debug!("Added transaction {} to cache with {} state changes", 
                           tx_hash_hex, changes.len());
                }
            },
            Ok(None) => {
                debug!("No state changes detected for transaction {}", tx_hash_hex);
            },
            Err(e) => {
                error!("Failed to simulate transaction {}: {}", tx_hash_hex, e);
            }
        }
    }
    
    info!("Successfully simulated {}/{} transactions", successful_simulations, max_txs);
    info!("State cache contains {} transactions and {} addresses", 
          state_cache.transaction_count(), state_cache.address_count());
    
    // Display aggregated state changes
    let min_eth_change = args.min_eth_change;
    info!("Addresses with ETH value changes > {} ETH:", min_eth_change);
    
    let high_value_addresses = state_cache.get_high_value_addresses(min_eth_change);
    if high_value_addresses.is_empty() {
        info!("No addresses with significant ETH value changes found");
    } else {
        for (i, (address, state)) in high_value_addresses.iter().enumerate() {
            let address_hex = hex_encode(address.as_bytes());
            info!("Address #{}: 0x{}", i + 1, address_hex);
            info!("  Balance: {} ETH", wei_to_eth(state.balance));
            info!("  Value Change: {:+.6} ETH", state.eth_value_change);
            info!("  Transaction Count: {}", state.transaction_count);
            info!("  Storage Changes: {} slots", state.storage.len());
        }
    }
    
    Ok(())
}

/// Get transactions from the mempool or latest block
async fn get_transactions(provider: &Provider<Http>) -> Result<Vec<TransactionView>> {
    let mut transactions = Vec::new();
    
    // Try to get transactions from the mempool using txpool_content
    match provider.request::<_, serde_json::Value>("txpool_content", ()).await {
        Ok(response) => {
            debug!("Got txpool_content response");
            // Process pending transactions
            if let Some(pending) = response.get("pending").and_then(|p| p.as_object()) {
                for (_address, nonce_map) in pending {
                    if let Some(nonce_obj) = nonce_map.as_object() {
                        for (_, tx_value) in nonce_obj {
                            if let Some(tx) = parse_transaction(tx_value) {
                                transactions.push(tx);
                            }
                        }
                    }
                }
            }
        },
        Err(e) => {
            // Fallback to latest block transactions if txpool not available
            debug!("Failed to get txpool_content: {}", e);
            debug!("Falling back to latest block transactions");
            
            if let Ok(Some(block)) = provider.get_block_with_txs(BlockNumber::Latest).await {
                for tx in block.transactions {
                    let tx_view = convert_transaction(tx);
                    transactions.push(tx_view);
                }
            }
        }
    }
    
    Ok(transactions)
}

/// Parse a mempool transaction from JSON format
fn parse_transaction(tx_json: &serde_json::Value) -> Option<TransactionView> {
    let hash = tx_json.get("hash")?.as_str()?;
    let from = tx_json.get("from")?.as_str()?;
    let to = tx_json.get("to").and_then(|v| v.as_str());
    let value_str = tx_json.get("value")?.as_str()?;
    let gas_str = tx_json.get("gas").and_then(|v| v.as_str());
    let gas_price_str = tx_json.get("gasPrice").and_then(|v| v.as_str());
    let input_str = tx_json.get("input").and_then(|v| v.as_str());
    let nonce_str = tx_json.get("nonce").and_then(|v| v.as_str());
    
    // Parse hash (removing 0x prefix)
    let hash = hash.strip_prefix("0x").unwrap_or(hash);
    let hash_bytes = hex::decode(hash).ok()?;
    
    // Parse addresses (removing 0x prefix)
    let from = from.strip_prefix("0x").unwrap_or(from);
    let from_bytes = hex::decode(from).ok()?;
    
    let to_bytes = if let Some(to) = to {
        let to = to.strip_prefix("0x").unwrap_or(to);
        Some(hex::decode(to).ok()?)
    } else {
        None
    };
    
    // Parse value
    let value_str = value_str.strip_prefix("0x").unwrap_or(value_str);
    let value = U256::from_str_radix(value_str, 16).ok()?;

    // Parse gas limit
    let gas_limit = if let Some(gas_str) = gas_str {
        let gas_str = gas_str.strip_prefix("0x").unwrap_or(gas_str);
        Some(U256::from_str_radix(gas_str, 16).ok()?)
    } else {
        None
    };

    // Parse gas price
    let gas_price = if let Some(gas_price_str) = gas_price_str {
        let gas_price_str = gas_price_str.strip_prefix("0x").unwrap_or(gas_price_str);
        Some(U256::from_str_radix(gas_price_str, 16).ok()?)
    } else {
        None
    };

    // Parse nonce
    let nonce = if let Some(nonce_str) = nonce_str {
        let nonce_str = nonce_str.strip_prefix("0x").unwrap_or(nonce_str);
        Some(U256::from_str_radix(nonce_str, 16).ok()?)
    } else {
        None
    };

    // Parse input data
    let input_data = if let Some(input) = input_str {
        let input = input.strip_prefix("0x").unwrap_or(input);
        if input.is_empty() {
            None
        } else {
            Some(hex::decode(input).ok()?)
        }
    } else {
        None
    };
    
    Some(TransactionView {
        hash: hash_bytes,
        from: from_bytes,
        to: to_bytes,
        value,
        gas_limit,
        gas_price,
        nonce,
        input_data,
    })
}

/// Convert an ethers Transaction to TransactionView
fn convert_transaction(tx: Transaction) -> TransactionView {
    TransactionView {
        hash: tx.hash.as_bytes().to_vec(),
        from: tx.from.as_bytes().to_vec(),
        to: tx.to.map(|addr| addr.as_bytes().to_vec()),
        value: tx.value,
        gas_limit: Some(tx.gas),
        gas_price: tx.gas_price,
        nonce: Some(tx.nonce),
        input_data: Some(tx.input.to_vec()),
    }
}

/// Helper to convert wei to ETH
fn wei_to_eth(wei: U256) -> f64 {
    wei.as_u128() as f64 / 1_000_000_000_000_000_000f64
} 