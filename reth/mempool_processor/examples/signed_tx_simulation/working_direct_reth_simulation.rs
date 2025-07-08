/// Test Direct Reth Signed Transaction Simulator with State Changes
/// 
/// This example demonstrates simulating mempool transactions using the
/// reth_signed_tx_simulator and extracting detailed state changes.

use mempool_processor::mempool_fetcher::{
    full_transaction_ipc_client::FullTransactionIpcClient,
    FullTransaction,
};
use reth_tx_simulator::RethDirectTxSimulator;
use reth_primitives::TransactionSigned;
use alloy_rlp::Decodable;
use eyre::Result;
use tracing::{info, error, warn};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n🔥 Direct Reth Simulation with State Changes");
    println!("===========================================\n");

    // Initialize Direct Reth simulator
    let start = Instant::now();
    let simulator = RethDirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    let init_time = start.elapsed();
    info!("✅ Direct Reth simulator initialized in {:?}", init_time);

    // Connect to mempool
    info!("📡 Connecting to mempool via IPC...");
    let mempool_client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    mempool_client.start_monitoring().await?;
    info!("✅ Mempool monitoring started\n");

    // Process a few transactions to demonstrate
    let target_txs = 2;
    let mut processed = 0;

    info!("🎯 Processing {} transactions to demonstrate state changes...\n", target_txs);

    while processed < target_txs {
        // Get transactions from mempool
        let transactions = mempool_client.get_full_transactions(1).await?;
        
        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        for tx in transactions {
            processed += 1;
            
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Transaction {}/{}: {}", processed, target_txs, tx.hash);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            // Log transaction details
            let from = tx.tx_data["from"].as_str().unwrap_or("unknown");
            let to = tx.tx_data["to"].as_str().unwrap_or("contract_creation");
            let value = tx.tx_data["value"].as_str().unwrap_or("0x0");
            let input = tx.tx_data["input"].as_str().unwrap_or("0x");
            let gas = tx.tx_data["gas"].as_str().unwrap_or("unknown");
            
            println!("📋 Transaction Details:");
            println!("   From:     {}", from);
            println!("   To:       {}", to);
            println!("   Value:    {}", value);
            println!("   Gas:      {}", gas);
            println!("   Input:    {} bytes", (input.len() - 2) / 2);
            println!("   Detection latency: {:.3} µs", tx.latency_ns as f64 / 1000.0);
            
            // Get raw transaction - try using the field if available
            let raw_tx = if let Some(raw) = tx.tx_data.get("raw").and_then(|v| v.as_str()) {
                raw.to_string()
            } else {
                // Fallback: get raw tx via RPC
                match get_raw_tx(&tx.hash).await {
                    Ok(raw) => raw,
                    Err(e) => {
                        error!("Failed to get raw transaction: {}", e);
                        continue;
                    }
                }
            };
            
            // Decode transaction
            let hex_str = raw_tx.strip_prefix("0x").unwrap_or(&raw_tx);
            let raw_bytes = match hex::decode(hex_str) {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Failed to decode hex: {}", e);
                    continue;
                }
            };
            
            let signed_tx = match TransactionSigned::decode(&mut raw_bytes.as_slice()) {
                Ok(tx) => tx,
                Err(e) => {
                    error!("Failed to decode transaction: {}", e);
                    continue;
                }
            };
            
            // First: Basic simulation for gas
            println!("\n⚡ Basic Simulation:");
            let sim_start = Instant::now();
            match simulator.simulate_signed_transaction(&signed_tx).await {
                Ok(result) => {
                    let sim_time = sim_start.elapsed();
                    println!("   ✅ Success in {:?}", sim_time);
                    println!("   Gas used: {} ({:.1}% of limit)", 
                        result.gas_used,
                        if let Ok(gas_limit) = u64::from_str_radix(gas.strip_prefix("0x").unwrap_or(gas), 16) {
                            (result.gas_used as f64 / gas_limit as f64) * 100.0
                        } else {
                            0.0
                        }
                    );
                    println!("   Success: {}", result.success);
                    if let Some(reason) = &result.revert_reason {
                        println!("   Revert reason: {}", reason);
                    }
                }
                Err(e) => {
                    error!("   ❌ Basic simulation failed: {}", e);
                }
            }
            
            // Second: State change simulation
            println!("\n🔍 State Change Analysis:");
            let state_start = Instant::now();
            match simulator.simulate_with_state_changes(&signed_tx).await {
                Ok(state_changes) => {
                    let state_time = state_start.elapsed();
                    println!("   ✅ State extraction in {:?}", state_time);
                    
                    // The state changes are in prestateTracer format with diffMode
                    // Format: { "pre": {...}, "post": {...} } or just addresses if not diffMode
                    
                    if let Some(obj) = state_changes.as_object() {
                        // Check if it's diffMode format
                        if let (Some(pre), Some(post)) = (obj.get("pre"), obj.get("post")) {
                            // Diff mode - shows pre and post states
                            if let Some(pre_obj) = pre.as_object() {
                                println!("\n   📊 Pre-State: {} accounts", pre_obj.len());
                            }
                            
                            if let Some(post_obj) = post.as_object() {
                                println!("   📊 Post-State: {} accounts", post_obj.len());
                                
                                // Show detailed changes for first few accounts
                                for (i, (addr, account)) in post_obj.iter().enumerate() {
                                    if i < 3 {
                                        println!("\n      Account {}: {}", i + 1, addr);
                                        
                                        if let Some(account_obj) = account.as_object() {
                                            // Check balance
                                            if let Some(balance) = account_obj.get("balance").and_then(|v| v.as_str()) {
                                                println!("        Balance: {}", balance);
                                            }
                                            
                                            // Check nonce
                                            if let Some(nonce) = account_obj.get("nonce").and_then(|v| v.as_str()) {
                                                println!("        Nonce: {}", nonce);
                                            }
                                            
                                            // Check storage
                                            if let Some(storage) = account_obj.get("storage").and_then(|v| v.as_object()) {
                                                if !storage.is_empty() {
                                                    println!("        Storage slots changed: {}", storage.len());
                                                    for (j, (slot, value)) in storage.iter().enumerate() {
                                                        if j < 2 {
                                                            println!("          {}: {}", slot, value);
                                                        }
                                                    }
                                                    if storage.len() > 2 {
                                                        println!("          ... and {} more slots", storage.len() - 2);
                                                    }
                                                }
                                            }
                                            
                                            // Check code
                                            if let Some(code) = account_obj.get("code").and_then(|v| v.as_str()) {
                                                if code != "0x" && !code.is_empty() {
                                                    println!("        Contract code: {} bytes", (code.len() - 2) / 2);
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                if post_obj.len() > 3 {
                                    println!("\n      ... and {} more accounts affected", post_obj.len() - 3);
                                }
                            }
                        } else {
                            // Non-diff mode - just shows touched accounts
                            println!("\n   📊 Touched accounts: {}", obj.len());
                            for (i, (addr, _)) in obj.iter().enumerate() {
                                if i < 5 {
                                    println!("      - {}", addr);
                                }
                            }
                            if obj.len() > 5 {
                                println!("      ... and {} more", obj.len() - 5);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("   ❌ State extraction failed: {}", e);
                }
            }
            
            println!("\n");
            
            if processed >= target_txs {
                break;
            }
        }
    }

    // Get stats from mempool client
    let stats = mempool_client.get_stats().await;
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Session Statistics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Transactions processed: {}", processed);
    println!("Mempool stats:");
    println!("  Total fetched: {}", stats.total_transactions);
    println!("  Sub-1ms detections: {}", stats.sub_1ms_count);
    println!("  Average latency: {} ns", stats.avg_latency_ns);
    
    println!("\n✅ Simulation complete!");
    println!("\n💡 State changes show:");
    println!("   - Which addresses were affected");
    println!("   - Balance changes for each address");
    println!("   - Storage slot modifications");
    println!("   - Contract deployments");
    println!("   - Nonce changes");
    println!("   - All extracted directly from Reth's database!");
    
    Ok(())
}

/// Helper to get raw transaction via RPC
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