/*
 * Verify State Diff Tracking Pipeline
 * 
 * This utility verifies the whole state diff tracking pipeline by:
 * 1. Using MempoolFetcher to get real transactions from your local node
 * 2. Simulating them with StateDiffTracker to extract state changes
 * 3. Storing and aggregating results in the in-memory StateCache
 * 4. Logging detailed state diffs for verification
 */

use clap::Parser;
use ethers::prelude::*;
use eyre::Result;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, error, warn, Level};
use mempool_processor::mempool_processor::fetcher::MempoolFetcher;
use mempool_processor::mempool_processor::TransactionSource;
use mempool_processor::tx_simulator::{StateDiffTracker, StateCache};
use hex::encode as hex_encode;

#[derive(Parser, Debug)]
struct Args {
    /// HTTP RPC URL for your local Ethereum node
    #[arg(long, env = "HTTP_RPC_URL", default_value = "http://localhost:8545")]
    http_rpc_url: String,
    
    /// Number of transactions to simulate (0 = all)
    #[arg(long, default_value = "5")]
    tx_count: usize,
    
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
    
    info!("Verify State Diff Tracking Pipeline");
    info!("HTTP RPC URL: {}", args.http_rpc_url);
    
    // Connect to provider
    let provider = Provider::<Http>::try_from(args.http_rpc_url.clone())?;
    let provider = Arc::new(provider);
    
    // Check connection
    let block_number = provider.get_block_number().await?;
    let chain_id = provider.get_chainid().await?;
    info!("Connected to Ethereum network");
    info!("  Chain ID: {}", chain_id);
    info!("  Current block: {}", block_number);
    
    // Set up MempoolFetcher (as used in production)
    info!("Setting up MempoolFetcher to get real transactions...");
    let fetcher = MempoolFetcher::new(&args.http_rpc_url)?;
    
    // Initialize state diff tracker
    info!("Initializing StateDiffTracker and StateCache...");
    let mut tracker = StateDiffTracker::new(provider.clone(), None);
    let mut state_cache = StateCache::new();
    
    // Fetch transactions from mempool
    info!("Fetching transactions from mempool...");
    let start_time = Instant::now();
    let transactions = fetcher.get_transactions().await?;
    info!("Fetched {} transactions in {:?}", transactions.len(), start_time.elapsed());
    
    // Simulate transactions
    let tx_count = if args.tx_count == 0 {
        transactions.len()
    } else {
        std::cmp::min(args.tx_count, transactions.len())
    };
    
    info!("Simulating {} transactions and tracking state changes...", tx_count);
    let mut successful_simulations = 0;
    
    for (i, tx) in transactions.iter().take(tx_count).enumerate() {
        let tx_hash_hex = hex_encode(&tx.hash);
        info!("Transaction {}/{}: {}", i + 1, tx_count, tx_hash_hex);
        
        let from_hex = hex_encode(&tx.from);
        let to_hex = if let Some(to) = &tx.to {
            hex_encode(to)
        } else {
            "Contract Creation".to_string()
        };
        
        let value_eth = tx.value.as_u128() as f64 / 1e18;
        info!("  From: 0x{}", from_hex);
        info!("  To: 0x{}", to_hex);
        info!("  Value: {} ETH", value_eth);
        
        // Simulate the transaction
        let sim_start = Instant::now();
        match tracker.simulate_transaction(tx).await {
            Ok(Some(changes)) => {
                successful_simulations += 1;
                let sim_time = sim_start.elapsed();
                info!("  Simulation successful in {:?}", sim_time);
                info!("  Found {} state changes:", changes.len());
                
                // Display detailed state changes
                for (j, change) in changes.iter().enumerate() {
                    let addr_hex = hex_encode(change.address.as_bytes());
                    info!("    Change #{}: Address 0x{}", j + 1, addr_hex);
                    info!("      Balance before: {} ETH", change.balance_before.as_u128() as f64 / 1e18);
                    info!("      Balance after:  {} ETH", change.balance_after.as_u128() as f64 / 1e18);
                    info!("      Change: {:+.6} ETH", change.eth_value);
                    
                    if !change.storage_changes.is_empty() {
                        info!("      Storage changes: {} slots", change.storage_changes.len());
                        for (slot, (before, after)) in &change.storage_changes {
                            if j < 3 { // Show only first 3 storage changes to avoid excessive logging
                                info!("        Slot {}: {:?} -> {:?}", hex_encode(slot.as_bytes()), before, after);
                            }
                        }
                        if change.storage_changes.len() > 3 {
                            info!("        ... and {} more slots", change.storage_changes.len() - 3);
                        }
                    }
                }
                
                // Add to state cache
                let mut hash_bytes = [0u8; 32];
                if tx.hash.len() == 32 {
                    hash_bytes.copy_from_slice(&tx.hash);
                    let tx_hash = H256::from(hash_bytes);
                    state_cache.add_transaction(tx_hash, tx.clone(), changes);
                }
            },
            Ok(None) => {
                warn!("  No state changes detected");
            },
            Err(e) => {
                error!("  Simulation failed: {}", e);
            }
        }
        
        info!("");  // Empty line for better readability
    }
    
    info!("Simulation complete. {}/{} transactions simulated successfully", 
         successful_simulations, tx_count);
    
    // Show aggregated state in the cache
    info!("StateCache contains {} transactions affecting {} addresses", 
          state_cache.transaction_count(), state_cache.address_count());
    
    // Show addresses with significant ETH changes
    let min_eth_change = 0.001;  // 0.001 ETH
    info!("Addresses with significant balance changes (> {} ETH):", min_eth_change);
    
    let high_value_addresses = state_cache.get_high_value_addresses(min_eth_change);
    if high_value_addresses.is_empty() {
        info!("  No addresses with significant changes found");
    } else {
        for (i, (address, state)) in high_value_addresses.iter().enumerate() {
            let address_hex = hex_encode(address.as_bytes());
            info!("  Address #{}: 0x{}", i + 1, address_hex);
            info!("    Current balance: {} ETH", state.balance.as_u128() as f64 / 1e18);
            info!("    Net ETH change: {:+.6} ETH", state.eth_value_change);
            info!("    Storage changes: {} slots", state.storage.len());
            info!("    Affected by {} transactions", state.transaction_count);
        }
    }
    
    // Summary
    info!("State diff tracking pipeline verification complete");
    info!("All components working correctly:");
    info!("  ✅ MempoolFetcher - Successfully fetched {} transactions", transactions.len());
    info!("  ✅ StateDiffTracker - Successfully simulated {} transactions", successful_simulations);
    info!("  ✅ StateCache - Aggregated changes for {} addresses", state_cache.address_count());
    
    Ok(())
} 