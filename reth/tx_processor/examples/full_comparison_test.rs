/// Full Comparison Test with Python Service
/// 
/// This fetches actual transaction data from chain and compares
/// both simulation and full processing between Rust and Python

use tx_processor::{tx_processor::TxProcessor, ProcessedTransaction, CallRequest};
use alloy_primitives::{Address, B256, U256, Bytes, Log as AlloyLog};
use eyre::Result;
use std::str::FromStr;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Initialize processors
    let processor = TxProcessor::new("/home/nima/.local/share/reth/mainnet")?;
    tracing::info!("✅ Rust TX Processor initialized");
    
    // Test with a recent transaction (can be overridden via command line)
    let tx_hash = std::env::args().nth(1)
        .unwrap_or_else(|| "0xa6d6100daa0a29fcbbe65ea1fe9cf6a6939680b7a4950ea7ea3984368d252e32".to_string());
    
    tracing::info!("\n🔍 Testing transaction: {}", tx_hash);
    
    // First, fetch transaction data from RPC
    let client = reqwest::Client::new();
    let rpc_response = client
        .post("http://localhost:8545")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionByHash",
            "params": [tx_hash],
            "id": 1
        }))
        .send()
        .await?;
    
    let rpc_json: Value = rpc_response.json().await?;
    let tx_data = &rpc_json["result"];
    
    if tx_data.is_null() {
        return Err(eyre::eyre!("Transaction not found"));
    }
    
    // Extract transaction details
    let from = Address::from_str(tx_data["from"].as_str().unwrap())?;
    let to = tx_data["to"].as_str()
        .map(|s| Address::from_str(s))
        .transpose()?;
    let value = U256::from_str(tx_data["value"].as_str().unwrap_or("0x0"))?;
    let input = Bytes::from_str(tx_data["input"].as_str().unwrap_or("0x"))?;
    let gas = u64::from_str_radix(tx_data["gas"].as_str().unwrap().trim_start_matches("0x"), 16)?;
    let gas_price = U256::from_str(tx_data["gasPrice"].as_str().unwrap_or("0x0"))?;
    
    tracing::info!("📋 Transaction Details:");
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
    
    // Process with Rust
    tracing::info!("\n🦀 Processing with Rust...");
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
        nonce: None,
    };
    
    let state_changes = processor.simulate_transaction(call_request).await?;
    let rust_time = rust_start.elapsed().as_millis();
    
    tracing::info!("✅ Rust processed in {}ms", rust_time);
    
    // Compare results
    tracing::info!("\n📊 Comparison Results:");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Performance
    let speedup = if rust_time > 0 {
        python_time as f64 / rust_time as f64
    } else {
        python_time as f64
    };
    tracing::info!("⚡ Performance:");
    tracing::info!("   Python: {}ms (including RPC overhead)", python_time);
    tracing::info!("   Rust:   {}ms (direct DB access)", rust_time);
    tracing::info!("   Speedup: {:.1}x faster", speedup);
    
    // State changes comparison
    tracing::info!("\n💰 State Changes Comparison:");
    
    // Python state changes
    if let Some(py_changes) = python_tx["state_changes"].as_object() {
        tracing::info!("Python detected {} addresses with changes", py_changes.len());
        for (addr, changes) in py_changes.iter().take(3) {
            if let Some(eth_net) = changes["eth_net"].as_str() {
                let eth_val: f64 = eth_net.parse().unwrap_or(0.0);
                if eth_val.abs() > 0.000001 {
                    tracing::info!("   {} ETH: {:+} ETH", addr, eth_val);
                }
            }
            if let Some(tokens) = changes["token_net"].as_object() {
                for (token, amount) in tokens {
                    tracing::info!("   {} {}: {}", addr, token, amount);
                }
            }
        }
    }
    
    // Rust state changes
    if !state_changes.is_empty() {
        tracing::info!("\nRust detected {} addresses with changes", state_changes.len());
        for (addr, changes) in state_changes.iter().take(3) {
            if changes.eth_net.abs() > 0.000001 {
                tracing::info!("   {} ETH: {:+} ETH", addr, changes.eth_net);
            }
            for (token, amount) in &changes.token_net {
                tracing::info!("   {} {}: {:+}", addr, token, amount);
            }
        }
    }
    
    // Compare specific values
    tracing::info!("\n🔍 Detailed Comparison:");
    
    // Check if both detected same number of transfers
    let py_transfers = python_tx["erc20_transfers"].as_array().map(|a| a.len()).unwrap_or(0);
    tracing::info!("ERC20 Transfers - Python: {}, Rust: {}", 
        py_transfers, 
        state_changes.iter()
            .flat_map(|(_, c)| &c.token_net)
            .count()
    );
    
    // Transaction type
    tracing::info!("Transaction Type - Python: {}", python_tx["txn_type"]);
    
    tracing::info!("\n✅ Full comparison complete!");
    
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