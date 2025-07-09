/// Exact Comparison Test - Simulates at the same block as original transaction
/// 
/// This test fetches a transaction, simulates it at the exact block it was executed,
/// and compares the results with Python's validation service.

use tx_processor::{tx_processor::TxProcessor, CallRequest};
use alloy_primitives::{Address, U256, Bytes};
use eyre::Result;
use std::str::FromStr;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Initialize processor
    let processor = TxProcessor::new("/home/nima/.local/share/reth/mainnet")?;
    tracing::info!("✅ Rust TX Processor initialized");
    
    // Test transaction - use one that's old enough to be in the database
    let tx_hash = std::env::args().nth(1)
        .unwrap_or_else(|| "0x9e63085271890a141297039b3b711913699f1ee4db1acb667ad7ce304772036b".to_string());
    
    tracing::info!("\n🔍 Testing transaction: {}", tx_hash);
    
    // First, fetch transaction data and receipt from RPC
    let client = reqwest::Client::new();
    
    // Get transaction
    let tx_response = client
        .post("http://localhost:8545")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionByHash",
            "params": [tx_hash],
            "id": 1
        }))
        .send()
        .await?;
    
    let tx_json: Value = tx_response.json().await?;
    let tx_data = &tx_json["result"];
    
    if tx_data.is_null() {
        return Err(eyre::eyre!("Transaction not found"));
    }
    
    // Get receipt to find the actual block
    let receipt_response = client
        .post("http://localhost:8545")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        }))
        .send()
        .await?;
    
    let receipt_json: Value = receipt_response.json().await?;
    let receipt = &receipt_json["result"];
    
    // Extract transaction details
    let from = Address::from_str(tx_data["from"].as_str().unwrap())?;
    let to = tx_data["to"].as_str()
        .map(|s| Address::from_str(s))
        .transpose()?;
    let value = U256::from_str(tx_data["value"].as_str().unwrap_or("0x0"))?;
    let input = Bytes::from_str(tx_data["input"].as_str().unwrap_or("0x"))?;
    let gas = u64::from_str_radix(tx_data["gas"].as_str().unwrap().trim_start_matches("0x"), 16)?;
    let gas_price = U256::from_str(tx_data["gasPrice"].as_str().unwrap_or("0x0"))?;
    
    // Get the block number from receipt
    let block_number = u64::from_str_radix(
        receipt["blockNumber"].as_str().unwrap().trim_start_matches("0x"), 
        16
    )?;
    
    tracing::info!("📋 Transaction Details:");
    tracing::info!("   Block: {}", block_number);
    tracing::info!("   From: {}", from);
    tracing::info!("   To: {:?}", to);
    tracing::info!("   Value: {} ETH", format_ether(value));
    tracing::info!("   Input: {} bytes", input.len());
    tracing::info!("   Gas: {}", gas);
    
    // Fetch from Python service
    tracing::info!("\n🐍 Fetching from Python service...");
    let python_start = std::time::Instant::now();
    
    let python_response = client
        .post(format!("http://localhost:18000/validate/transaction/{}", tx_hash))
        .json(&serde_json::json!({
            "tx_hash": tx_hash,
            "include_state_changes": true,
            "include_trace": true
        }))
        .send()
        .await?;
    
    if !python_response.status().is_success() {
        let error_text = python_response.text().await?;
        return Err(eyre::eyre!("Python service error: {}", error_text));
    }
    
    let python_json: Value = python_response.json().await?;
    let python_time = python_start.elapsed().as_millis();
    
    if !python_json["success"].as_bool().unwrap_or(false) {
        return Err(eyre::eyre!("Python processing failed: {}", 
            python_json["error"].as_str().unwrap_or("Unknown")));
    }
    
    let python_tx = &python_json["processed_transaction"];
    
    tracing::info!("✅ Python processed in {}ms", python_time);
    tracing::info!("   Type: {}", python_tx["txn_type"]);
    tracing::info!("   Actions: {:?}", python_tx["actions"]);
    tracing::info!("   ERC20 Transfers: {}", 
        python_tx["erc20_transfers"].as_array().map(|a| a.len()).unwrap_or(0));
    
    // Now simulate with Rust AT THE EXACT SAME BLOCK
    tracing::info!("\n🦀 Simulating with Rust at block {}...", block_number);
    let rust_start = std::time::Instant::now();
    
    // Create call request with actual data
    let call_request = CallRequest {
        from: Some(from),
        to,
        value: Some(value),
        data: Some(input),
        gas: Some(gas),
        gas_price: Some(gas_price.try_into().unwrap_or(0)),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: None, // Will use the nonce from that block
    };
    
    // Simulate at the exact block
    let detailed_result = processor.simulate_transaction_detailed(
        call_request,
        Some(block_number)
    ).await?;
    
    let rust_time = rust_start.elapsed().as_millis();
    
    tracing::info!("✅ Rust processed in {}ms", rust_time);
    tracing::info!("   Success: {}", detailed_result.success);
    tracing::info!("   Gas Used: {}", detailed_result.gas_used);
    tracing::info!("   Logs: {}", detailed_result.logs.len());
    tracing::info!("   State Changes: {}", detailed_result.state_changes.len());
    
    // Compare results
    tracing::info!("\n📊 EXACT Comparison Results:");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Performance
    let speedup = if rust_time > 0 {
        python_time as f64 / rust_time as f64
    } else {
        python_time as f64
    };
    tracing::info!("⚡ Performance:");
    tracing::info!("   Python: {}ms", python_time);
    tracing::info!("   Rust:   {}ms", rust_time);
    tracing::info!("   Speedup: {:.1}x faster", speedup);
    
    // Logs comparison
    tracing::info!("\n📜 Event Logs:");
    let py_logs_count = python_tx["event_counts"]["total_logs"].as_u64().unwrap_or(0);
    tracing::info!("   Python detected: {} logs", py_logs_count);
    tracing::info!("   Rust detected: {} logs", detailed_result.logs.len());
    
    // Decode logs with Rust
    if !detailed_result.logs.is_empty() {
        tracing::info!("   Rust log details:");
        for (i, log) in detailed_result.logs.iter().enumerate() {
            tracing::info!("     Log {}: address={}, topics={}", 
                i, log.address, log.topics().len());
            
            // Try to decode as ERC20 transfer
            if log.topics().len() == 3 && 
               log.topics()[0] == alloy_primitives::keccak256(b"Transfer(address,address,uint256)") {
                let from_bytes: &[u8] = log.topics()[1].as_ref();
                let to_bytes: &[u8] = log.topics()[2].as_ref();
                let from = Address::from_slice(&from_bytes[12..]);
                let to = Address::from_slice(&to_bytes[12..]);
                let amount = U256::from_be_slice(&log.data.data);
                tracing::info!("       ERC20 Transfer: {} -> {}, amount: {}", from, to, amount);
            }
        }
    }
    
    // State changes comparison
    tracing::info!("\n💰 State Changes:");
    if let Some(py_changes) = python_tx["state_changes"].as_object() {
        tracing::info!("   Python: {} addresses", py_changes.len());
    }
    tracing::info!("   Rust: {} addresses", detailed_result.state_changes.len());
    
    // Compare specific addresses
    if let Some(py_changes) = python_tx["state_changes"].as_object() {
        for (addr_str, py_change) in py_changes.iter().take(2) {
            tracing::info!("\n   Address {}:", addr_str);
            
            // Python values
            if let Some(eth_net) = py_change["eth_net"].as_str() {
                let eth_val: f64 = eth_net.parse().unwrap_or(0.0);
                if eth_val.abs() > 0.000001 {
                    tracing::info!("     Python ETH: {:+}", eth_val);
                }
            }
            
            // Rust values
            if let Ok(addr) = Address::from_str(addr_str) {
                if let Some(rust_change) = detailed_result.state_changes.get(&addr) {
                    if rust_change.eth_net.abs() > 0.000001 {
                        tracing::info!("     Rust ETH: {:+}", rust_change.eth_net);
                    }
                    for (token, amount) in &rust_change.token_net {
                        tracing::info!("     Rust {}: {:+}", token, amount);
                    }
                }
            }
        }
    }
    
    // Transaction type and actions
    tracing::info!("\n📋 Classification:");
    tracing::info!("   Python Type: {}", python_tx["txn_type"]);
    tracing::info!("   Python Actions: {:?}", python_tx["actions"]);
    
    // Summary
    let logs_match = detailed_result.logs.len() as u64 == py_logs_count;
    let state_match = detailed_result.state_changes.len() == 
        python_tx["state_changes"].as_object().map(|o| o.len()).unwrap_or(0);
    
    tracing::info!("\n✅ Exact Comparison Summary:");
    tracing::info!("   Logs Match: {} (Rust: {}, Python: {})", 
        if logs_match { "✓" } else { "✗" },
        detailed_result.logs.len(),
        py_logs_count
    );
    tracing::info!("   State Changes Match: {} (Rust: {}, Python: {})", 
        if state_match { "✓" } else { "✗" },
        detailed_result.state_changes.len(),
        python_tx["state_changes"].as_object().map(|o| o.len()).unwrap_or(0)
    );
    tracing::info!("   Performance: Rust is {:.1}x faster", speedup);
    
    Ok(())
}

fn format_ether(value: U256) -> String {
    let eth = value.to_string();
    if eth.len() > 18 {
        let (whole, decimal) = eth.split_at(eth.len() - 18);
        format!("{}.{}", whole, &decimal[..6])
    } else {
        format!("0.{:0>18}", eth)
    }
}