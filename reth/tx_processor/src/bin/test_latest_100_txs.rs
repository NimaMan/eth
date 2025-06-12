//! Test Latest 100 Transactions
//! 
//! Fetches the latest 100 transactions from the blockchain and compares
//! Rust simulation results with Python validation service

use anyhow::Result;
use ethers_providers::{Provider as EthersProvider, Http, Middleware};
use ethers_core::types::H256;
use std::time::Instant;
use std::collections::HashMap;
use revm_tx_simulator_lib::{
    process_tx::{compare_with_python, ValidationResult},
};

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";
const DEFAULT_PYTHON_URL: &str = "http://127.0.0.1:18000";

#[derive(Debug)]
struct TransactionInfo {
    hash: H256,
    block_number: u64,
    tx_index: u64,
    from: String,
    to: Option<String>,
    value: String,
}

#[derive(Debug)]
struct BatchSummary {
    total_transactions: usize,
    successful_matches: usize,
    failed_matches: usize,
    skipped_transactions: usize,
    success_rate: f64,
    total_rust_time: f64,
    avg_rust_time: f64,
    high_index_txs: usize,
    replay_needed_txs: usize,
    errors_by_type: HashMap<String, usize>,
}

async fn fetch_latest_transactions(rpc_url: &str, count: usize) -> Result<Vec<TransactionInfo>> {
    let provider = EthersProvider::<Http>::try_from(rpc_url)?;
    let mut transactions = Vec::new();
    
    // Get the latest block
    let latest_block_number = provider.get_block_number().await?;
    println!("📊 Latest block number: {}", latest_block_number);
    
    let mut current_block = latest_block_number.as_u64();
    
    // Fetch transactions from recent blocks until we have enough
    while transactions.len() < count && current_block > 0 {
        if let Ok(Some(block)) = provider.get_block_with_txs(current_block).await {
            println!("📦 Checking block {}: {} transactions", current_block, block.transactions.len());
            
            for (tx_index, tx) in block.transactions.iter().enumerate() {
                if transactions.len() >= count {
                    break;
                }
                
                transactions.push(TransactionInfo {
                    hash: tx.hash,
                    block_number: current_block,
                    tx_index: tx_index as u64,
                    from: format!("{:?}", tx.from),
                    to: tx.to.map(|addr| format!("{:?}", addr)),
                    value: tx.value.to_string(),
                });
            }
        }
        
        current_block = current_block.saturating_sub(1);
    }
    
    println!("✅ Fetched {} transactions from {} blocks", transactions.len(), latest_block_number.as_u64() - current_block);
    Ok(transactions)
}

async fn test_transaction(
    tx_info: &TransactionInfo,
    rust_rpc_url: &str,
    python_url: &str,
) -> Result<ValidationResult> {
    let tx_hash_str = format!("{:?}", tx_info.hash);
    
    println!("\n🔍 Testing transaction {}", tx_hash_str);
    println!("   Block: {}, Index: {}", tx_info.block_number, tx_info.tx_index);
    println!("   From: {}, To: {:?}", tx_info.from, tx_info.to);
    
    // Compare with Python (this function does both extraction and comparison)
    match compare_with_python(
        &tx_hash_str,
        rust_rpc_url,
        Some(python_url),
    ).await {
        Ok(validation) => {
            if validation.matches {
                println!("   ✅ MATCH - Processing time: Rust {:.2}ms, Python {:.2}ms", 
                         validation.rust_processing_time_ms, validation.python_processing_time_ms);
            } else {
                println!("   ❌ MISMATCH - {} differences found", validation.differences.len());
                for diff in validation.differences.iter().take(3) {
                    println!("      - {}: {}", diff.field, diff.description);
                }
                if validation.differences.len() > 3 {
                    println!("      ... and {} more differences", validation.differences.len() - 3);
                }
            }
            Ok(validation)
        }
        Err(e) => {
            println!("   ❌ Comparison failed: {}", e);
            // Return a failed validation result
            Ok(ValidationResult {
                tx_hash: tx_hash_str,
                matches: false,
                differences: vec![],
                rust_processing_time_ms: 0.0,
                python_processing_time_ms: 0.0,
                rust_state_changes: None,
                python_state_changes: None,
            })
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let rust_rpc_url = std::env::var("RUST_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
    let python_url = std::env::var("PYTHON_URL").unwrap_or_else(|_| DEFAULT_PYTHON_URL.to_string());
    
    println!("🚀 Testing Latest 100 Transactions");
    println!("📊 Rust RPC: {}", rust_rpc_url);
    println!("🐍 Python Service: {}", python_url);
    println!("{}", "=".repeat(80));
    
    // Fetch latest transactions
    let transactions = fetch_latest_transactions(&rust_rpc_url, 100).await?;
    println!("\n📋 Testing {} transactions...", transactions.len());
    
    let start_time = Instant::now();
    let mut results = Vec::new();
    let mut errors_by_type: HashMap<String, usize> = HashMap::new();
    let mut high_index_count = 0;
    let mut replay_needed_count = 0;
    
    // Test each transaction
    for (i, tx_info) in transactions.iter().enumerate() {
        println!("\n[{}/{}] Transaction: {:?}", i + 1, transactions.len(), tx_info.hash);
        
        // Track high-index transactions
        if tx_info.tx_index > 50 {
            high_index_count += 1;
        }
        if tx_info.tx_index > 0 {
            replay_needed_count += 1;
        }
        
        match test_transaction(tx_info, &rust_rpc_url, &python_url).await {
            Ok(result) => {
                if !result.matches {
                    // Categorize errors
                    for diff in &result.differences {
                        let error_type = if diff.description.contains("ETH net change") {
                            "ETH Amount Mismatch"
                        } else if diff.description.contains("Token") {
                            "Token Amount Mismatch"
                        } else {
                            "Other Mismatch"
                        };
                        *errors_by_type.entry(error_type.to_string()).or_insert(0) += 1;
                    }
                }
                results.push(result);
            }
            Err(e) => {
                println!("   ⚠️  Error testing transaction: {}", e);
                *errors_by_type.entry("Test Error".to_string()).or_insert(0) += 1;
            }
        }
    }
    
    let total_time = start_time.elapsed();
    
    // Calculate summary statistics
    let total = results.len();
    let successful = results.iter().filter(|r| r.matches).count();
    let failed = total - successful;
    let skipped = transactions.len() - total;
    let success_rate = if total > 0 { (successful as f64 / total as f64) * 100.0 } else { 0.0 };
    
    let total_rust_time: f64 = results.iter().map(|r| r.rust_processing_time_ms).sum();
    let avg_rust_time = if total > 0 { total_rust_time / total as f64 } else { 0.0 };
    
    let summary = BatchSummary {
        total_transactions: transactions.len(),
        successful_matches: successful,
        failed_matches: failed,
        skipped_transactions: skipped,
        success_rate,
        total_rust_time,
        avg_rust_time,
        high_index_txs: high_index_count,
        replay_needed_txs: replay_needed_count,
        errors_by_type,
    };
    
    // Print summary
    println!("\n{}", "=".repeat(80));
    println!("📊 BATCH TEST SUMMARY");
    println!("{}", "=".repeat(80));
    println!("Total Transactions:    {}", summary.total_transactions);
    println!("Successful Matches:    {} ({:.1}%)", summary.successful_matches, summary.success_rate);
    println!("Failed Matches:        {}", summary.failed_matches);
    println!("Skipped Transactions:  {}", summary.skipped_transactions);
    println!("\nTransaction Index Analysis:");
    println!("  High Index (>50):    {} ({:.1}%)", summary.high_index_txs, 
             (summary.high_index_txs as f64 / summary.total_transactions as f64) * 100.0);
    println!("  Replay Needed (>0):  {} ({:.1}%)", summary.replay_needed_txs,
             (summary.replay_needed_txs as f64 / summary.total_transactions as f64) * 100.0);
    
    if !summary.errors_by_type.is_empty() {
        println!("\nError Breakdown:");
        for (error_type, count) in &summary.errors_by_type {
            println!("  {}: {}", error_type, count);
        }
    }
    
    println!("\nPerformance:");
    println!("  Total Time:          {:.2}s", total_time.as_secs_f64());
    println!("  Avg Rust Time:       {:.2}ms per transaction", summary.avg_rust_time);
    println!("  Total Rust Time:     {:.2}s", summary.total_rust_time / 1000.0);
    println!("{}", "=".repeat(80));
    
    if summary.success_rate < 100.0 {
        println!("\n⚠️  Some transactions failed validation. This may be due to:");
        println!("   - Token formatting differences (raw amounts vs decimals)");
        println!("   - Complex DeFi transactions with high gas usage");
        println!("   - Transactions that failed on-chain");
    }
    
    Ok(())
}