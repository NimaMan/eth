/// State Change Extraction from Mempool Transactions
/// 
/// This example shows how to extract detailed state changes from mempool
/// transactions using Direct Reth's implementation of debug_traceCall.

use mempool_processor::mempool_fetcher::{
    full_transaction_ipc_client::FullTransactionIpcClient,
};
use reth_tx_simulator::{DirectTxSimulator, ipc_to_call_request};
use reth_primitives::TransactionSigned;
use alloy_rlp::Decodable;
use eyre::Result;
use tracing::{info, error, debug};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n🔍 State Change Extraction from Mempool");
    println!("======================================\n");

    // Initialize
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    let mempool_client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    mempool_client.start_monitoring().await?;
    
    info!("✅ Systems initialized\n");

    // Process a few transactions to show state changes
    let mut processed = 0;
    let target = 3;

    while processed < target {
        let transactions = mempool_client.get_full_transactions(1).await?;
        
        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        for tx in transactions {
            processed += 1;
            
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Transaction {}/{}: {}", processed, target, tx.hash);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            // Get and decode transaction
            let raw_tx = get_raw_tx(&tx.hash).await?;
            let _signed_tx = decode_transaction(&raw_tx)?;

            // Extract transaction info
            let from = tx.tx_data["from"].as_str().unwrap_or("unknown");
            let to = tx.tx_data["to"].as_str().unwrap_or("contract_creation");
            let value = tx.tx_data["value"].as_str().unwrap_or("0x0");
            let input = tx.tx_data["input"].as_str().unwrap_or("0x");
            
            println!("\n📋 Transaction Info:");
            println!("  From:  {}", from);
            println!("  To:    {}", to);
            println!("  Value: {}", value);
            println!("  Data:  {} bytes", (input.len() - 2) / 2);

            // Convert tx data to CallRequest using existing function
            let call_request = match ipc_to_call_request(&tx.tx_data) {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to convert transaction to CallRequest: {}", e);
                    continue;
                }
            };
            
            // First try basic simulation with nonce adaptation for performance
            println!("\n🔍 Attempting simulation with automatic nonce adaptation...");
            let start = Instant::now();
            
            match simulator.simulate_unsigned_transaction(&call_request).await {
                Ok(basic_result) => {
                    // If basic simulation succeeds, run detailed simulation
                    debug!("Basic simulation succeeded, running detailed analysis");
                    match simulator.simulate_transaction_detailed(call_request, None).await {
                        Ok(detailed_result) => {
                            let elapsed = start.elapsed();
                            println!("\n✅ State extraction completed in {:?}", elapsed);
                            
                            // Analyze the state changes
                            analyze_detailed_state_changes(&detailed_result);
                        }
                        Err(e) => {
                            // This shouldn't happen if basic simulation succeeded
                            error!("❌ Detailed simulation failed after basic success: {}", e);
                        }
                    }
                }
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("nonce") && error_str.contains("too high") {
                        println!("\n⚠️  Skipping: Nonce too high (transaction depends on pending txs)");
                        println!("    Expected nonce from error: {}", error_str);
                    } else if error_str.contains("nonce") && error_str.contains("too low") {
                        // This should have been auto-corrected, but log it anyway
                        println!("\n⚠️  Nonce too low error not auto-corrected: {}", error_str);
                    } else {
                        error!("❌ Failed to simulate: {}", e);
                    }
                }
            }
            
            println!();
            
            if processed >= target {
                break;
            }
        }
    }

    println!("✅ Analysis complete!");
    Ok(())
}

fn analyze_detailed_state_changes(result: &reth_tx_simulator::DetailedSimulationResult) {
    println!("\n📊 Detailed State Changes:");
    println!("  Transaction success: {}", result.success);
    println!("  Gas used: {}", result.gas_used);
    
    if let Some(reason) = &result.revert_reason {
        println!("  Revert reason: {}", reason);
    }
    
    println!("  Affected addresses: {}", result.state_changes.len());
    
    // Show detailed changes for first few addresses
    for (i, (addr, changes)) in result.state_changes.iter().enumerate() {
        if i < 3 {
            println!("\n  📍 Address {}: 0x{:x}", i + 1, addr);
            println!("     ETH change: {:.6} ETH", changes.eth_net);
            
            if !changes.token_net.is_empty() {
                println!("     Token changes: {}", changes.token_net.len());
                for (j, (token, amount)) in changes.token_net.iter().enumerate() {
                    if j < 3 {
                        println!("       {}: {}", token, amount);
                    }
                }
                if changes.token_net.len() > 3 {
                    println!("       ... and {} more tokens", changes.token_net.len() - 3);
                }
            }
        }
    }
    
    if result.state_changes.len() > 3 {
        println!("\n  ... and {} more addresses affected", result.state_changes.len() - 3);
    }
    
    // Summary statistics
    let total_eth_moved: f64 = result.state_changes.values()
        .map(|changes| changes.eth_net.abs())
        .sum();
    let total_tokens_affected: usize = result.state_changes.values()
        .map(|changes| changes.token_net.len())
        .sum();
    
    println!("\n  📈 Summary:");
    println!("     Total ETH movement: {:.6} ETH", total_eth_moved / 2.0); // Divide by 2 since we count both sender and receiver
    println!("     Total token interactions: {}", total_tokens_affected);
}

fn decode_transaction(raw_tx: &str) -> Result<TransactionSigned> {
    let hex_str = raw_tx.strip_prefix("0x").unwrap_or(raw_tx);
    let raw_bytes = hex::decode(hex_str)?;
    Ok(TransactionSigned::decode(&mut raw_bytes.as_slice())?)
}


async fn get_raw_tx(hash: &str) -> Result<String> {
    use jsonrpsee::http_client::{HttpClientBuilder, HttpClient};
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::rpc_params;
    
    let client: HttpClient = HttpClientBuilder::default()
        .build("http://127.0.0.1:8545")?;
    
    let raw_tx: String = client.request(
        "eth_getRawTransactionByHash",
        rpc_params![hash]
    ).await?;
    
    Ok(raw_tx)
}