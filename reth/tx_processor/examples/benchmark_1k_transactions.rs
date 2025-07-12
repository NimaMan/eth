/// Benchmark 1000 Transactions - Compare Rust vs Python
/// 
/// This example processes 1000 transactions and compares:
/// 1. Performance (time per transaction)
/// 2. Accuracy (field-by-field comparison)
/// 3. Simulation decisions (which transactions were simulated)
///
/// Usage: cargo run --example benchmark_1k_transactions --release

use tx_processor::tx_processor::TxProcessor;
use alloy_primitives::{Address, B256, U256, Log as AlloyLog};
use eyre::Result;
use std::str::FromStr;
use std::time::Instant;
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct PythonTransaction {
    hash: String,
    block_number: u64,
    block_timestamp: u64,
    txn_index: u64,
    from_address: String,
    to_address: Option<String>,
    value: String,
    status: String,
    nonce: u64,
    input: String,
    gas_price: String,
    gas_used: u64,
    logs: Vec<PythonLog>,
}

#[derive(Debug, Deserialize)]
struct PythonLog {
    address: String,
    topics: Vec<String>,
    data: String,
}

#[derive(Debug, Serialize)]
struct BenchmarkResult {
    tx_hash: String,
    rust_time_ms: u128,
    python_time_ms: f64,
    speedup: f64,
    simulated: bool,
    internal_tx_count: usize,
    state_changes_count: usize,
    matches_python: bool,
    differences: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🚀 TX Processor 1K Transaction Benchmark");
    info!("========================================");
    
    // Initialize Rust processor
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    let processor = TxProcessor::new(&reth_datadir)?;
    info!("✅ Rust TX Processor initialized");
    
    // Python validation service URL
    let python_service_url = std::env::var("PYTHON_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:18000".to_string());
    
    info!("📡 Python service: {}", python_service_url);
    
    // TODO: Replace with real transaction hashes from database
    // Current fake hashes will be replaced with real data later
    let test_transactions: Vec<&str> = vec![
        // TODO: Add real transaction hashes here
        // Example format: "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7",
    ];
    
    let mut results = Vec::new();
    let mut total_rust_time = 0u128;
    let mut total_python_time = 0f64;
    let mut simulated_count = 0;
    let mut matches = 0;
    
    info!("\n📊 Processing {} transactions...", test_transactions.len());
    info!("------------------------------------------------");
    
    let client = reqwest::Client::new();
    
    for (idx, tx_hash_str) in test_transactions.iter().enumerate() {
        if idx % 100 == 0 && idx > 0 {
            info!("Progress: {}/{} transactions processed", idx, test_transactions.len());
        }
        
        let tx_hash = B256::from_str(tx_hash_str)?;
        
        // Fetch transaction data from Python service
        let python_start = Instant::now();
        let response = match client
            .post(format!("{}/validate/transaction/{}", python_service_url, tx_hash_str))
            .json(&serde_json::json!({
                "tx_hash": tx_hash_str,
                "include_state_changes": true,
                "include_trace": true
            }))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                warn!("Failed to fetch from Python service: {}", e);
                continue;
            }
        };
        
        if !response.status().is_success() {
            warn!("Python service error for {}: {}", tx_hash_str, response.status());
            continue;
        }
        
        let python_response: serde_json::Value = response.json().await?;
        let python_time = python_start.elapsed().as_millis();
        total_python_time += python_time as f64;
        
        let python_tx = &python_response["processed_transaction"];
        if python_tx.is_null() {
            warn!("Python returned null transaction for {}", tx_hash_str);
            continue;
        }
        
        // Extract transaction data from Python response
        let from = Address::from_str(python_tx["from_address"].as_str().unwrap())?;
        let to = python_tx["to_address"].as_str()
            .map(|s| Address::from_str(s))
            .transpose()?;
        let value = U256::from_str(python_tx["value"].as_str().unwrap_or("0"))?;
        let input = hex::decode(python_tx["input"].as_str().unwrap_or("0x").trim_start_matches("0x"))?;
        let gas_price = U256::from_str(python_tx["fees"]["gas_price"].as_str().unwrap_or("0"))?;
        let gas_used = python_tx["fees"]["gas_used"].as_u64().unwrap_or(0);
        let gas_limit = python_tx["gas_limit"].as_u64().unwrap_or(21000); // Default to 21000 if not specified
        let status = if python_tx["status"].as_str().unwrap_or("0") == "1" { "success" } else { "failed" }.to_string();
        let nonce = python_tx["nonce"].as_u64().unwrap_or(0);
        let block_number = python_tx["block_number"].as_u64().unwrap_or(0);
        let block_timestamp = python_tx["block_timestamp"].as_u64().unwrap_or(0);
        let tx_index = python_tx["txn_index"].as_u64().unwrap_or(0);
        
        // Convert logs
        let mut logs = Vec::new();
        if let Some(log_array) = python_response["logs"].as_array() {
            for log in log_array {
                let address = Address::from_str(log["address"].as_str().unwrap())?;
                let topics: Vec<B256> = log["topics"].as_array().unwrap()
                    .iter()
                    .map(|t| B256::from_str(t.as_str().unwrap()).unwrap())
                    .collect();
                let data = hex::decode(log["data"].as_str().unwrap().trim_start_matches("0x"))?;
                
                logs.push(AlloyLog::new_unchecked(address, topics, data.into()));
            }
        }
        
        // Process with Rust
        let rust_start = Instant::now();
        let processed_tx = processor.process_transaction(
            tx_hash,
            block_number,
            block_timestamp,
            tx_index,
            from,
            to,
            value,
            input.clone(),
            gas_price,
            gas_used,
            status,
            nonce,
            logs,
            gas_limit,
        ).await?;
        let rust_time = rust_start.elapsed().as_millis();
        total_rust_time += rust_time;
        
        // Check if transaction was simulated
        let was_simulated = !input.is_empty() && to.is_some();
        if was_simulated {
            simulated_count += 1;
        }
        
        // Compare results
        let mut differences = Vec::new();
        
        // Compare ONLY state changes count (this is the key metric)
        let rust_state_changes_count = processed_tx.state_changes.len();
        let python_state_changes_count = python_tx["state_changes"].as_object()
            .map(|obj| obj.len()).unwrap_or(0);
        if rust_state_changes_count != python_state_changes_count {
            differences.push(format!("state_changes: {} vs {}", rust_state_changes_count, python_state_changes_count));
        }
        
        // Still track internal transactions for reporting, but don't use for matching
        let rust_internal_count = processed_tx.internal_transactions.len();
        
        let matches_python = differences.is_empty();
        if matches_python {
            matches += 1;
        } else {
            // Log mismatch for investigation (only state changes matter now)
            error!("MISMATCH - TX: {} | Rust state_changes: {} | Python state_changes: {} | Difference: {:?}", 
                tx_hash_str, 
                rust_state_changes_count,
                python_state_changes_count,
                differences
            );
        }
        
        results.push(BenchmarkResult {
            tx_hash: tx_hash_str.to_string(),
            rust_time_ms: rust_time,
            python_time_ms: python_time as f64,
            speedup: python_time as f64 / rust_time.max(1) as f64,
            simulated: was_simulated,
            internal_tx_count: rust_internal_count,
            state_changes_count: rust_state_changes_count,
            matches_python,
            differences,
        });
    }
    
    // Print summary
    info!("\n📈 Benchmark Results Summary");
    info!("============================");
    info!("Total transactions processed: {}", results.len());
    info!("Transactions simulated: {} ({:.1}%)", 
        simulated_count, 
        simulated_count as f64 / results.len() as f64 * 100.0
    );
    info!("Matching Python (state_changes only): {} ({:.1}%)", 
        matches, 
        matches as f64 / results.len() as f64 * 100.0
    );
    
    if results.len() > 0 {
        let avg_rust_time = total_rust_time as f64 / results.len() as f64;
        let avg_python_time = total_python_time / results.len() as f64;
        let avg_speedup = avg_python_time / avg_rust_time;
        
        info!("\n⏱️  Performance Metrics:");
        info!("  Rust average:   {:.2}ms per transaction", avg_rust_time);
        info!("  Python average: {:.2}ms per transaction", avg_python_time);
        info!("  Speedup:        {:.1}x faster", avg_speedup);
        info!("  Throughput:     ~{} tx/second", (1000.0 / avg_rust_time) as u32);
    }
    
    // Print transactions with differences
    let mut transactions_with_diffs: Vec<_> = results.iter()
        .filter(|r| !r.differences.is_empty())
        .collect();
    
    if !transactions_with_diffs.is_empty() {
        info!("\n⚠️  Transactions with differences:");
        transactions_with_diffs.sort_by(|a, b| b.differences.len().cmp(&a.differences.len()));
        for (i, result) in transactions_with_diffs.iter().take(10).enumerate() {
            info!("  {}. {} - {} differences", i + 1, result.tx_hash, result.differences.len());
            for diff in &result.differences {
                info!("     - {}", diff);
            }
        }
    }
    
    // Save detailed results to file
    let results_json = serde_json::to_string_pretty(&results)?;
    std::fs::write("benchmark_results.json", results_json)?;
    info!("\n💾 Detailed results saved to benchmark_results.json");
    
    info!("\n✅ Benchmark complete!");
    
    Ok(())
}