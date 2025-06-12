//! Test ONE real transaction comparison between Rust and Python

use revm_tx_simulator_lib::process_tx::{
    extract_state_changes_python_format,
    PythonCompatibleStateChanges,
    ProcessTxError,
    python_validator::{
        compare_state_changes,
        PythonProcessedTransaction,
        PythonEventCounts,
        ComparisonResult,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct PythonResponse {
    success: bool,
    processed_transaction: Option<serde_json::Value>,
}

async fn get_rust_state_changes(tx_hash: &str) -> Result<PythonCompatibleStateChanges, ProcessTxError> {
    println!("🦀 Calculating Rust state changes for: {}", tx_hash);
    let result = extract_state_changes_python_format(
        tx_hash.to_string(),
        "http://127.0.0.1:8545"
    ).await?;
    println!("✅ Rust calculation complete");
    Ok(result)
}

async fn get_python_response(tx_hash: &str) -> Result<Option<PythonProcessedTransaction>, Box<dyn std::error::Error>> {
    println!("🐍 Getting Python state changes for: {}", tx_hash);
    
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "tx_hash": tx_hash,
        "include_state_changes": true
    });
    
    let response = client
        .post(&format!("http://127.0.0.1:18000/validate/transaction/{}", tx_hash))
        .json(&request_body)
        .send()
        .await?;
    
    let status = response.status();
    let response_text = response.text().await?;
    
    if !status.is_success() {
        return Err(format!("Python service error ({}): {}", status, response_text).into());
    }
    
    // Parse the response
    let python_response: PythonResponse = serde_json::from_str(&response_text)?;
    
    if !python_response.success {
        return Err("Python processing failed".into());
    }
    
    // For now, we'll extract just the state changes since that's what we need for comparison
    let processed_tx = match python_response.processed_transaction {
        Some(tx_value) => {
            // Create a minimal PythonProcessedTransaction with just the state changes
            if let Some(state_changes_value) = tx_value.get("state_changes") {
                let state_changes: std::collections::HashMap<String, serde_json::Value> = 
                    serde_json::from_value(state_changes_value.clone())?;
                
                Some(PythonProcessedTransaction {
                    hash: tx_hash.to_string(),
                    block_number: 0,
                    block_timestamp: 0,
                    txn_index: 0,
                    from_address: String::new(),
                    to_address: None,
                    contract_address: None,
                    value: 0.0,
                    status: 1,
                    nonce: 0,
                    input: String::new(),
                    txn_type: String::new(),
                    actions: vec![],
                    fees: None,
                    bribe_amount: None,
                    unique_addresses: vec![],
                    erc20_contracts: vec![],
                    event_counts: PythonEventCounts {
                        erc20_transfers: 0,
                        erc721_transfers: 0,
                        erc1155_transfers: 0,
                        internal_transactions: 0,
                        uniswap_v2_swaps: 0,
                        uniswap_v2_syncs: 0,
                        uniswap_v3_swaps: 0,
                        uniswap_v4_swaps: 0,
                        approvals: 0,
                        mints: 0,
                        burns: 0,
                        deposits: 0,
                        withdraws: 0,
                        permit2_events: 0,
                        trading_enabled_events: 0,
                        trading_disabled_events: 0,
                    },
                    erc20_transfers: vec![],
                    internal_transactions: vec![],
                    uniswap_v2_swaps: vec![],
                    uniswap_v4_swaps: vec![],
                    state_changes,
                })
            } else {
                None
            }
        }
        None => None,
    };
    
    println!("✅ Python response received");
    Ok(processed_tx)
}

fn display_comparison_result(
    tx_hash: &str,
    comparison_result: &ComparisonResult,
    rust_result: &PythonCompatibleStateChanges,
    python_processed_tx: &Option<PythonProcessedTransaction>,
) {
    println!("\n📊 COMPARISON RESULTS FOR: {}\n", tx_hash);
    
    if comparison_result.matches {
        println!("✅ All state changes match!");
    } else {
        println!("❌ Found {} differences:", comparison_result.differences.len());
        println!();
        
        for diff in &comparison_result.differences {
            println!("  Field: {}", diff.field);
            println!("    Rust:   {}", diff.rust_value);
            println!("    Python: {}", diff.python_value);
            println!("    Note:   {}", diff.description);
            println!();
        }
    }
    
    // Display address counts
    let rust_addresses = rust_result.state_changes.len();
    let python_addresses = if let Some(ptx) = python_processed_tx {
        ptx.state_changes.len()
    } else {
        0
    };
    
    println!("📊 SUMMARY:");
    println!("   Rust addresses: {}", rust_addresses);
    println!("   Python addresses: {}", python_addresses);
    println!("   Match status: {}", if comparison_result.matches { "✅ PASS" } else { "❌ FAIL" });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <transaction_hash>", args[0]);
        eprintln!("Example: {} 0xc2ee34725dd0db8df65144fa70252e4a25db891e7597b4144fa3e0599174bce8", args[0]);
        std::process::exit(1);
    }
    
    let tx_hash = &args[1];
    
    println!("🔍 Testing transaction: {}", tx_hash);
    println!("📊 Rust RPC: http://127.0.0.1:8545");
    println!("🐍 Python Service: http://127.0.0.1:18000");
    println!();
    
    // Get Rust state changes
    let rust_result = match get_rust_state_changes(tx_hash).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("❌ Rust calculation failed: {}", e);
            return Err(e.into());
        }
    };
    
    // Get Python response
    let python_processed_tx = match get_python_response(tx_hash).await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Python service failed: {}", e);
            return Err(e);
        }
    };
    
    // Log detailed state changes for manual inspection
    println!("\n🦀 RUST STATE CHANGES:");
    println!("{}", serde_json::to_string_pretty(&rust_result.state_changes).unwrap_or_default());
    
    if let Some(ref python_tx) = python_processed_tx {
        println!("\n🐍 PYTHON STATE CHANGES:");
        println!("{}", serde_json::to_string_pretty(&python_tx.state_changes).unwrap_or_default());
    }
    
    // Save to files for easier comparison
    let rust_file = format!("rust_state_changes_{}.json", &tx_hash[2..10]);
    let python_file = format!("python_state_changes_{}.json", &tx_hash[2..10]);
    
    if let Ok(rust_json) = serde_json::to_string_pretty(&rust_result.state_changes) {
        std::fs::write(&rust_file, rust_json).ok();
        println!("\n💾 Rust state changes saved to: {}", rust_file);
    }
    
    if let Some(ref python_tx) = python_processed_tx {
        if let Ok(python_json) = serde_json::to_string_pretty(&python_tx.state_changes) {
            std::fs::write(&python_file, python_json).ok();
            println!("💾 Python state changes saved to: {}", python_file);
        }
    }
    
    // Use the compare_state_changes function from python_validator module
    let comparison_result = compare_state_changes(&rust_result, &python_processed_tx);
    
    // Display results
    display_comparison_result(tx_hash, &comparison_result, &rust_result, &python_processed_tx);
    
    // Log metadata
    println!("\n📈 PERFORMANCE:");
    println!("   Rust processing time: {:.2}ms", rust_result.metadata.processing_time_ms);
    println!("   Addresses affected: {}", rust_result.metadata.addresses_affected);
    println!("   Tokens involved: {}", rust_result.metadata.tokens_involved);
    
    Ok(())
}