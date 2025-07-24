/// Compare Rust TX Processor with Python ProcessedTransaction
/// 
/// This example fetches transactions from recent blocks and compares
/// the Rust implementation with Python's eth_block_processor.txn module.
///
/// Usage: cargo run --example compare_with_python -- [num_transactions]
/// Default: 5 transactions

use tx_processor::TxProcessor;
use alloy_primitives::{B256};
use eyre::Result;
use std::str::FromStr;
use std::time::Instant;
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct ComparisonResult {
    tx_hash: String,
    block_number: u64,
    rust_time_ms: u128,
    python_time_ms: Option<u128>,
    matches: bool,
    differences: Vec<String>,
    rust_state_changes: usize,
    python_state_changes: Option<usize>,
    rust_internal_txs: usize,
    python_internal_txs: Option<usize>,
    rust_erc20_transfers: usize,
    python_erc20_transfers: Option<usize>,
}

/// Fetch transaction hashes from recent blocks that exist in Reth DB
async fn fetch_recent_transactions(processor: &TxProcessor, num_transactions: usize) -> Result<Vec<(B256, u64)>> {
    // Get latest block from Reth DB
    let latest_db_block = processor.get_latest_block()?;
    info!("Latest block in Reth DB: {}", latest_db_block);
    
    // Use blocks from 1000 blocks before to ensure transactions are in the DB
    let start_block = latest_db_block.saturating_sub(1000);
    
    let eth_rpc_url = std::env::var("ETH_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    info!("Fetching {} transactions from blocks around {} via {}", num_transactions, start_block, eth_rpc_url);
    
    let client = reqwest::Client::new();
    let mut transactions = Vec::new();
    
    // Fetch transactions from recent blocks
    let mut block_num = start_block;
    while transactions.len() < num_transactions && block_num > start_block.saturating_sub(100) {
        // Fetch block by number
        let response = client
            .post(&eth_rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getBlockByNumber",
                "params": [format!("0x{:x}", block_num), true],
                "id": 1
            }))
            .send()
            .await?;
        
        let result: Value = response.json().await?;
        if let Some(block) = result["result"].as_object() {
            if let Some(txs) = block["transactions"].as_array() {
                if !txs.is_empty() {
                    info!("Block {} has {} transactions", block_num, txs.len());
                }
                
                for tx in txs.iter().take(num_transactions - transactions.len()) {
                    if let Some(hash_str) = tx["hash"].as_str() {
                        let hash = B256::from_str(hash_str)?;
                        transactions.push((hash, block_num));
                    }
                }
            }
        }
        
        block_num = block_num.saturating_sub(1);
    }
    
    info!("Fetched {} transaction hashes", transactions.len());
    Ok(transactions)
}

/// Compare Rust ProcessedTransaction with Python result
fn compare_processed_transactions(
    rust_tx: &tx_processor::ProcessedTransaction,
    python_tx: &Value,
) -> (bool, Vec<String>) {
    let mut differences = Vec::new();
    
    // Compare basic fields
    if let Some(py_from) = python_tx["from_address"].as_str() {
        if rust_tx.from_address.to_string().to_lowercase() != py_from.to_lowercase() {
            differences.push(format!("from_address: {} vs {}", rust_tx.from_address, py_from));
        }
    }
    
    // Compare transaction type
    if let Some(py_type) = python_tx["txn_type"].as_str() {
        if rust_tx.txn_type != py_type {
            differences.push(format!("txn_type: {} vs {}", rust_tx.txn_type, py_type));
        }
    }
    
    // Compare state changes count
    let rust_state_changes = rust_tx.state_changes.len();
    let python_state_changes = python_tx["state_changes"].as_object()
        .map(|obj| obj.len())
        .unwrap_or(0);
    
    if rust_state_changes != python_state_changes {
        differences.push(format!("state_changes count: {} vs {}", rust_state_changes, python_state_changes));
    }
    
    // Compare internal transactions count
    let rust_internal_txs = rust_tx.internal_transactions.len();
    let python_internal_txs = python_tx["internal_transactions"].as_array()
        .map(|arr| arr.len())
        .unwrap_or(0);
    
    if rust_internal_txs != python_internal_txs {
        differences.push(format!("internal_transactions count: {} vs {}", rust_internal_txs, python_internal_txs));
    }
    
    // Compare ERC20 transfers count
    let rust_erc20_count = rust_tx.erc20_transfers.len();
    let python_erc20_count = python_tx["erc20_transfers"].as_array()
        .map(|arr| arr.len())
        .unwrap_or(0);
    
    if rust_erc20_count != python_erc20_count {
        differences.push(format!("erc20_transfers count: {} vs {}", rust_erc20_count, python_erc20_count));
    }
    
    (differences.is_empty(), differences)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let num_transactions = if args.len() > 1 {
        args[1].parse::<usize>().unwrap_or(5)
    } else {
        5
    };
    
    info!("🔄 TX Processor Comparison: Rust vs Python");
    info!("=========================================");
    
    // Initialize Rust processor
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    let processor = TxProcessor::new(&reth_datadir)?;
    info!("✅ Rust TX Processor initialized");
    
    // Python service URL (optional)
    let python_service_url = std::env::var("PYTHON_SERVICE_URL").ok();
    let has_python = python_service_url.is_some();
    
    if let Some(ref url) = python_service_url {
        info!("📡 Python service: {}", url);
    } else {
        info!("⚠️  No Python service URL provided, running Rust-only benchmark");
    }
    
    // Fetch recent transactions
    let transactions = fetch_recent_transactions(&processor, num_transactions).await?;
    
    if transactions.is_empty() {
        error!("No transactions found!");
        return Ok(());
    }
    
    let mut results = Vec::new();
    let client = reqwest::Client::new();
    
    info!("\n📊 Processing {} transactions...", transactions.len());
    info!("------------------------------------------------");
    
    for (tx_hash, block_number) in transactions {
        info!("\nProcessing: {} (block {})", tx_hash, block_number);
        
        // Process with Rust
        let rust_start = Instant::now();
        let rust_result = match processor.process_transaction_by_hash(tx_hash).await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Rust processing failed: {}", e);
                continue;
            }
        };
        let rust_time = rust_start.elapsed().as_millis();
        
        info!("  ✅ Rust processed in {}ms", rust_time);
        info!("     - Type: {}", rust_result.txn_type);
        info!("     - State changes: {}", rust_result.state_changes.len());
        info!("     - Internal txs: {}", rust_result.internal_transactions.len());
        info!("     - ERC20 transfers: {}", rust_result.erc20_transfers.len());
        
        let mut comparison = ComparisonResult {
            tx_hash: tx_hash.to_string(),
            block_number,
            rust_time_ms: rust_time,
            python_time_ms: None,
            matches: true,
            differences: vec![],
            rust_state_changes: rust_result.state_changes.len(),
            python_state_changes: None,
            rust_internal_txs: rust_result.internal_transactions.len(),
            python_internal_txs: None,
            rust_erc20_transfers: rust_result.erc20_transfers.len(),
            python_erc20_transfers: None,
        };
        
        // Compare with Python if available
        if let Some(ref url) = python_service_url {
            let python_start = Instant::now();
            
            match client
                .post(format!("{}/process_transaction", url))
                .json(&serde_json::json!({
                    "tx_hash": tx_hash.to_string(),
                }))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    let python_time = python_start.elapsed().as_millis();
                    comparison.python_time_ms = Some(python_time);
                    
                    if let Ok(python_result) = response.json::<Value>().await {
                        if let Some(python_tx) = python_result["processed_transaction"].as_object() {
                            let (matches, differences) = compare_processed_transactions(&rust_result, &python_result["processed_transaction"]);
                            comparison.matches = matches;
                            comparison.differences = differences;
                            comparison.python_state_changes = python_tx["state_changes"].as_object().map(|o| o.len());
                            comparison.python_internal_txs = python_tx["internal_transactions"].as_array().map(|a| a.len());
                            comparison.python_erc20_transfers = python_tx["erc20_transfers"].as_array().map(|a| a.len());
                            
                            info!("  📊 Python processed in {}ms", python_time);
                            if matches {
                                info!("  ✅ Results match!");
                            } else {
                                warn!("  ⚠️  Differences found:");
                                for diff in &comparison.differences {
                                    warn!("     - {}", diff);
                                }
                            }
                        }
                    }
                }
                Ok(response) => {
                    warn!("  ❌ Python service error: {}", response.status());
                }
                Err(e) => {
                    warn!("  ❌ Failed to reach Python service: {}", e);
                }
            }
        }
        
        results.push(comparison);
    }
    
    // Print summary
    info!("\n📈 Comparison Summary");
    info!("====================");
    info!("Total transactions: {}", results.len());
    
    let successful_rust = results.iter().filter(|r| r.rust_time_ms > 0).count();
    info!("Rust successful: {}/{}", successful_rust, results.len());
    
    if has_python {
        let successful_python = results.iter().filter(|r| r.python_time_ms.is_some()).count();
        let matching = results.iter().filter(|r| r.matches).count();
        
        info!("Python successful: {}/{}", successful_python, results.len());
        info!("Matching results: {}/{} ({:.1}%)", 
            matching, 
            successful_python,
            if successful_python > 0 { matching as f64 / successful_python as f64 * 100.0 } else { 0.0 }
        );
        
        // Performance comparison
        let rust_times: Vec<u128> = results.iter()
            .filter_map(|r| if r.rust_time_ms > 0 { Some(r.rust_time_ms) } else { None })
            .collect();
        let python_times: Vec<u128> = results.iter()
            .filter_map(|r| r.python_time_ms)
            .collect();
        
        if !rust_times.is_empty() && !python_times.is_empty() {
            let avg_rust = rust_times.iter().sum::<u128>() as f64 / rust_times.len() as f64;
            let avg_python = python_times.iter().sum::<u128>() as f64 / python_times.len() as f64;
            
            info!("\n⏱️  Performance:");
            info!("  Rust avg: {:.2}ms", avg_rust);
            info!("  Python avg: {:.2}ms", avg_python);
            info!("  Speedup: {:.1}x", avg_python / avg_rust);
        }
    } else {
        // Rust-only performance
        let rust_times: Vec<u128> = results.iter()
            .map(|r| r.rust_time_ms)
            .collect();
        
        if !rust_times.is_empty() {
            let avg_rust = rust_times.iter().sum::<u128>() as f64 / rust_times.len() as f64;
            let total_time = rust_times.iter().sum::<u128>();
            
            info!("\n⏱️  Rust Performance:");
            info!("  Average: {:.2}ms per transaction", avg_rust);
            info!("  Total: {}ms for {} transactions", total_time, rust_times.len());
            info!("  Throughput: ~{} tx/second", (1000.0 / avg_rust) as u32);
        }
        
        // Data extracted summary
        let total_state_changes: usize = results.iter().map(|r| r.rust_state_changes).sum();
        let total_internal_txs: usize = results.iter().map(|r| r.rust_internal_txs).sum();
        let total_erc20_transfers: usize = results.iter().map(|r| r.rust_erc20_transfers).sum();
        
        info!("\n📊 Data Extracted:");
        info!("  Total state changes: {}", total_state_changes);
        info!("  Total internal txs: {}", total_internal_txs);
        info!("  Total ERC20 transfers: {}", total_erc20_transfers);
    }
    
    // Save results
    let results_json = serde_json::to_string_pretty(&results)?;
    std::fs::write("comparison_results.json", results_json)?;
    info!("\n💾 Results saved to comparison_results.json");
    
    info!("\n✅ Comparison complete!");
    
    Ok(())
}