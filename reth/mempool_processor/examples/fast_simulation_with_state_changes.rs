/// Fast RPC Simulation with State Changes
/// 
/// Fetches transactions from mempool, simulates with Fast RPC,
/// and calculates address state changes
///
/// Run with: cargo run --example fast_simulation_with_state_changes

use mempool_processor::mempool_fetcher::{WebSocketClient, TransactionView};
use mempool_processor::tx_simulator::SimulatorWrapper;
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use std::time::{Duration, Instant};
use tracing::{info, error, warn};
use eyre::Result;
use revm_context::BlockEnv;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,fast_simulation=info")
        .init();

    info!("🚀 Fast RPC Transaction Simulation with State Changes");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    
    // Connect to provider
    info!("Connecting to Ethereum node...");
    let http_provider = Provider::<Http>::try_from(http_url)?;
    
    // Get current block
    let block_number = http_provider.get_block_number().await?;
    let latest_block = http_provider.get_block(ethers::types::BlockNumber::Latest).await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    info!("Connected! Current block: {}", block_number);
    
    // Create block environment for simulation
    let block_env = BlockEnv {
        number: revm_primitives::U256::from(latest_block.number.unwrap_or_default().as_u64()),
        timestamp: revm_primitives::U256::from(latest_block.timestamp.as_u64()),
        gas_limit: latest_block.gas_limit.as_u64(),
        basefee: latest_block.base_fee_per_gas.unwrap_or_default().as_u64(),
        difficulty: revm_primitives::U256::from(latest_block.difficulty.as_u64()),
        prevrandao: Some(revm_primitives::B256::from_slice(latest_block.mix_hash.unwrap_or_default().as_bytes())),
        beneficiary: revm_primitives::Address::from_slice(latest_block.author.unwrap_or_default().as_bytes()),
        ..Default::default()
    };
    
    // Initialize Fast RPC simulator
    info!("Initializing Fast RPC simulator...");
    let simulator = SimulatorWrapper::new_fast_rpc(http_url).await?;
    
    // Connect to mempool
    info!("Connecting to mempool via WebSocket...");
    let ws_client = WebSocketClient::new(ws_url, http_url)?;
    ws_client.start_monitoring().await?;
    
    info!("Starting simulation...\n");
    
    let start_time = Instant::now();
    let mut total_simulated = 0;
    let mut successful_simulations = 0;
    let mut total_state_changes = 0;
    let mut simulation_times = Vec::new();
    
    // Run for 30 seconds
    while start_time.elapsed() < Duration::from_secs(30) {
        match tokio::time::timeout(
            Duration::from_millis(100),
            ws_client.get_transactions(5)
        ).await {
            Ok(Ok(transactions)) => {
                for ws_tx in transactions {
                    let tx_hash = ws_tx.hash.clone();
                    
                    // Fetch full transaction
                    let tx_hash_h256 = match tx_hash.parse::<H256>() {
                        Ok(h) => h,
                        Err(_) => continue,
                    };
                    
                    match http_provider.get_transaction(tx_hash_h256).await {
                        Ok(Some(tx)) => {
                            // Convert to TransactionView
                            let tx_view = TransactionView {
                                hash: tx.hash.as_bytes().to_vec(),
                                from: tx.from.as_bytes().to_vec(),
                                to: tx.to.map(|addr| addr.as_bytes().to_vec()),
                                value: tx.value,
                                gas_price: tx.gas_price,
                                gas_limit: Some(tx.gas),
                                nonce: Some(U256::from(tx.nonce.as_u64())),
                                input_data: Some(tx.input.to_vec()),
                            };
                            
                            // Simulate transaction
                            let sim_start = Instant::now();
                            match simulator.process_transaction(&tx_view, &block_env).await {
                                Ok(Some(state_changes)) => {
                                    let sim_time = sim_start.elapsed();
                                    simulation_times.push(sim_time.as_millis() as f64);
                                    
                                    total_simulated += 1;
                                    successful_simulations += 1;
                                    
                                    // Count state changes
                                    let num_addresses = state_changes.len();
                                    total_state_changes += num_addresses;
                                    
                                    info!("TX {}: {}", total_simulated, &tx_hash[..10]);
                                    info!("  Simulation time: {:.2}ms", sim_time.as_millis());
                                    info!("  Addresses affected: {}", num_addresses);
                                    
                                    // Show first few address changes
                                    for (i, (address, changes)) in state_changes.iter().enumerate() {
                                        if i >= 3 { 
                                            if num_addresses > 3 {
                                                info!("  ... and {} more addresses", num_addresses - 3);
                                            }
                                            break; 
                                        }
                                        
                                        let addr_str = format!("0x{}", hex::encode(address.as_ref()));
                                        info!("  Address {}: {}", i+1, &addr_str[..10]);
                                        
                                        // Show ETH balance change if any
                                        let eth_change = &changes.eth_net_change;
                                        if !eth_change.absolute_value.is_zero() {
                                            let abs_val = eth_change.absolute_value;
                                            let change_f64 = if !eth_change.is_negative {
                                                // Convert U256 to f64
                                                let bytes = abs_val.to_be_bytes::<32>();
                                                let lower_u128 = u128::from_be_bytes(bytes[16..32].try_into().unwrap());
                                                lower_u128 as f64 / 1e18
                                            } else {
                                                let bytes = abs_val.to_be_bytes::<32>();
                                                let lower_u128 = u128::from_be_bytes(bytes[16..32].try_into().unwrap());
                                                -(lower_u128 as f64 / 1e18)
                                            };
                                            info!("    ETH change: {:+.6}", change_f64);
                                        }
                                        
                                        // Count token changes
                                        let token_changes = changes.token_net_changes.len();
                                        if token_changes > 0 {
                                            info!("    Token changes: {} tokens", token_changes);
                                        }
                                    }
                                    
                                    info!("");
                                }
                                Ok(None) => {
                                    total_simulated += 1;
                                    info!("TX {}: {} - No state changes", total_simulated, &tx_hash[..10]);
                                }
                                Err(e) => {
                                    total_simulated += 1;
                                    warn!("TX {}: {} - Simulation failed: {}", total_simulated, &tx_hash[..10], e);
                                }
                            }
                        }
                        Ok(None) => {
                            warn!("Transaction {} not found", tx_hash);
                        }
                        Err(e) => {
                            error!("Error fetching transaction: {}", e);
                        }
                    }
                }
            }
            _ => {} // Timeout, continue
        }
        
        // Stop if we have enough data
        if total_simulated >= 20 {
            break;
        }
    }
    
    // Calculate statistics
    info!("\n\n📊 SIMULATION RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Total transactions simulated: {}", total_simulated);
    info!("Successful simulations: {} ({:.1}%)", 
          successful_simulations, 
          successful_simulations as f64 / total_simulated as f64 * 100.0);
    info!("Total address state changes: {}", total_state_changes);
    
    if !simulation_times.is_empty() {
        let avg_time = simulation_times.iter().sum::<f64>() / simulation_times.len() as f64;
        let min_time = simulation_times.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let max_time = simulation_times.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        
        info!("\n⏱️  SIMULATION PERFORMANCE:");
        info!("Average simulation time: {:.2}ms", avg_time);
        info!("Min simulation time: {:.2}ms", min_time);
        info!("Max simulation time: {:.2}ms", max_time);
        
        if successful_simulations > 0 {
            info!("Average addresses per tx: {:.1}", 
                  total_state_changes as f64 / successful_simulations as f64);
        }
    }
    
    Ok(())
}