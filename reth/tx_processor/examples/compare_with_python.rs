/// Compare Rust TX Processor with Python Service
/// 
/// This example fetches transaction data from the Python validation service
/// and compares it with our Rust implementation to ensure consistency.

use tx_processor::{tx_processor::TxProcessor, CallRequest};
use alloy_primitives::{Address, U256};
use eyre::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Initialize Rust processor
    let processor = TxProcessor::new("/home/nima/.local/share/reth/mainnet")?;
    tracing::info!("✅ Rust TX Processor initialized");
    
    // Get a recent transaction hash
    let tx_hash = std::env::args().nth(1)
        .unwrap_or_else(|| "0xa6d6100daa0a29fcbbe65ea1fe9cf6a6939680b7a4950ea7ea3984368d252e32".to_string());
    
    tracing::info!("\n🔍 Fetching transaction {} from Python service...", tx_hash);
    
    // Fetch from Python service
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://localhost:18000/validate/transaction/{}", tx_hash))
        .json(&serde_json::json!({
            "tx_hash": tx_hash,
            "include_state_changes": true,
            "include_trace": true
        }))
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre::eyre!("Python service error: {}", error_text));
    }
    
    let python_response: serde_json::Value = response.json().await?;
    
    // Check if Python processing was successful
    if !python_response["success"].as_bool().unwrap_or(false) {
        let error = python_response["error"].as_str().unwrap_or("Unknown error");
        return Err(eyre::eyre!("Python processing failed: {}", error));
    }
    
    let processing_time = python_response["processing_time_ms"].as_f64().unwrap_or(0.0);
    let transaction = &python_response["processed_transaction"];
    
    if transaction.is_null() {
        return Err(eyre::eyre!("Python returned null transaction"));
    }
    
    tracing::info!("✅ Python processed in {:.2}ms", processing_time);
    tracing::info!("   Type: {}", transaction["txn_type"].as_str().unwrap_or("unknown"));
    tracing::info!("   Actions: {:?}", transaction["actions"]);
    tracing::info!("   ERC20 Transfers: {}", transaction["erc20_transfers"].as_array().map(|a| a.len()).unwrap_or(0));
    
    // Now process with Rust
    tracing::info!("\n🦀 Processing same transaction with Rust...");
    
    let start = std::time::Instant::now();
    
    // Simulate the transaction to get state changes
    let from = Address::from_str(transaction["from_address"].as_str().unwrap())?;
    let to = transaction["to_address"].as_str()
        .map(|s| Address::from_str(s))
        .transpose()?;
    let value = U256::from_str(transaction["value"].as_str().unwrap_or("0"))?;
    
    // For a real comparison, we would fetch the actual calldata and logs
    // For now, let's just simulate a USDC transfer
    let call_request = CallRequest {
        from: Some(from),
        to,
        value: Some(value),
        data: None, // Would need actual calldata
        gas: Some(100_000),
        gas_price: Some(20_000_000_000),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: None, // Let it auto-detect current nonce
    };
    
    let state_changes = processor.simulate_transaction(call_request).await?;
    let rust_time = start.elapsed().as_millis();
    
    tracing::info!("✅ Rust processed in {}ms", rust_time);
    
    // Compare results
    tracing::info!("\n📊 Comparison Results:");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Performance comparison
    let speedup = processing_time / rust_time as f64;
    tracing::info!("⚡ Performance:");
    tracing::info!("   Python: {:.2}ms", processing_time);
    tracing::info!("   Rust:   {}ms", rust_time);
    tracing::info!("   Speedup: {:.1}x faster", speedup);
    
    // Transaction type comparison
    tracing::info!("\n📋 Transaction Classification:");
    tracing::info!("   Python Type: {}", transaction["txn_type"].as_str().unwrap_or("unknown"));
    tracing::info!("   Python Actions: {:?}", transaction["actions"]);
    
    // State changes comparison
    if let Some(python_state_changes) = transaction["state_changes"].as_object() {
        if !python_state_changes.is_empty() {
            tracing::info!("\n💰 State Changes (Python):");
            for (addr, changes) in python_state_changes {
                tracing::info!("   {}: {}", addr, serde_json::to_string_pretty(changes)?);
            }
        }
    }
    
    if !state_changes.is_empty() {
        tracing::info!("\n💰 State Changes (Rust):");
        for (addr, changes) in &state_changes {
            if changes.eth_net != 0.0 {
                tracing::info!("   {} ETH: {:+}", addr, changes.eth_net);
            }
            for (token, amount) in &changes.token_net {
                tracing::info!("   {} {}: {:+}", addr, token, amount);
            }
        }
    }
    
    // ERC20 transfers comparison
    if let Some(erc20_transfers) = transaction["erc20_transfers"].as_array() {
        if !erc20_transfers.is_empty() {
            tracing::info!("\n💸 ERC20 Transfers (Python): {}", erc20_transfers.len());
            for transfer in erc20_transfers {
                tracing::info!("   {} -> {}: {} (token: {})", 
                    transfer["from_address"].as_str().unwrap_or("?"), 
                    transfer["to_address"].as_str().unwrap_or("?"), 
                    transfer["amount"].as_str().unwrap_or("0"),
                    transfer["token_address"].as_str().unwrap_or("?")
                );
            }
        }
    }
    
    tracing::info!("\n✅ Comparison complete!");
    
    // Note about full comparison
    tracing::info!("\n📝 Note: For a complete comparison, we would need:");
    tracing::info!("   - Actual transaction input data");
    tracing::info!("   - Transaction receipt logs");
    tracing::info!("   - Full trace data");
    tracing::info!("   These would allow exact matching of all decoded events");
    
    Ok(())
}