use std::collections::HashMap;
use std::process::Command;
use serde_json::Value;
use tokio;

/// Tests to ensure REVM simulator accuracy matches Python implementation
/// These tests compare state changes between Rust REVM and Python for validation

#[tokio::test]
async fn test_revm_python_accuracy_single_transaction() {
    // Test a single transaction to ensure basic accuracy
    let test_result = run_single_transaction_comparison().await;
    assert!(test_result.is_ok(), "Single transaction comparison failed: {:?}", test_result.err());
}

#[tokio::test]
async fn test_revm_python_accuracy_batch() {
    // Test multiple transactions to ensure consistent accuracy
    let test_result = run_batch_comparison(10).await;
    assert!(test_result.success_rate >= 0.95, "Batch accuracy below 95%: {:.2}%", test_result.success_rate * 100.0);
}

#[tokio::test]
async fn test_revm_python_accuracy_complex_transactions() {
    // Test complex DeFi transactions specifically
    let test_result = run_complex_transaction_test().await;
    assert!(test_result.is_ok(), "Complex transaction test failed: {:?}", test_result.err());
}

#[tokio::test]
async fn test_revm_python_accuracy_performance() {
    // Ensure accuracy is maintained even under performance pressure
    let test_result = run_performance_accuracy_test().await;
    assert!(test_result.accuracy >= 0.95, "Performance test accuracy below 95%: {:.2}%", test_result.accuracy * 100.0);
    assert!(test_result.avg_processing_time < 1.0, "Processing time too slow: {:.3}s", test_result.avg_processing_time);
}

// Helper structs for test results
#[derive(Debug)]
struct BatchTestResult {
    total_transactions: usize,
    successful_matches: usize,
    success_rate: f64,
    significant_differences: Vec<String>,
}

#[derive(Debug)]
struct PerformanceTestResult {
    accuracy: f64,
    avg_processing_time: f64,
    transactions_tested: usize,
}

// Test implementation functions
async fn run_single_transaction_comparison() -> Result<(), Box<dyn std::error::Error>> {
    // Get latest block and pick a transaction
    let latest_transactions = get_latest_transactions(1).await?;
    
    if latest_transactions.is_empty() {
        return Err("No transactions found".into());
    }
    
    let tx_hash = &latest_transactions[0];
    
    // Run Python validation
    let python_result = run_python_validation(tx_hash).await?;
    
    // Run Rust REVM validation
    let rust_result = run_rust_validation(tx_hash).await?;
    
    // Compare results
    let comparison = compare_state_changes(&python_result, &rust_result)?;
    
    if !comparison.exact_match {
        if comparison.significant_differences.is_empty() {
            // Minor differences below threshold are acceptable
            println!("✅ Minor differences below 0.005 ETH threshold");
            return Ok(());
        } else {
            return Err(format!("Significant differences found: {:?}", comparison.significant_differences).into());
        }
    }
    
    println!("✅ Perfect match between Python and Rust implementations");
    Ok(())
}

async fn run_batch_comparison(num_transactions: usize) -> BatchTestResult {
    let mut total = 0;
    let mut successful = 0;
    let mut significant_diffs = Vec::new();
    
    // Get recent transactions for testing
    match get_latest_transactions(num_transactions).await {
        Ok(transactions) => {
            for tx_hash in transactions {
                total += 1;
                
                // Compare Python vs Rust for this transaction
                match run_transaction_comparison(&tx_hash).await {
                    Ok(comparison) => {
                        if comparison.exact_match || comparison.significant_differences.is_empty() {
                            successful += 1;
                        } else {
                            significant_diffs.extend(comparison.significant_differences);
                        }
                    }
                    Err(e) => {
                        significant_diffs.push(format!("Transaction {} failed: {}", tx_hash, e));
                    }
                }
            }
        }
        Err(e) => {
            significant_diffs.push(format!("Failed to get transactions: {}", e));
        }
    }
    
    BatchTestResult {
        total_transactions: total,
        successful_matches: successful,
        success_rate: if total > 0 { successful as f64 / total as f64 } else { 0.0 },
        significant_differences: significant_diffs,
    }
}

async fn run_complex_transaction_test() -> Result<(), Box<dyn std::error::Error>> {
    // Focus on complex DeFi transactions (multi-hop swaps, liquidity provision, etc.)
    let complex_transactions = get_complex_transactions(5).await?;
    
    for tx_hash in complex_transactions {
        let comparison = run_transaction_comparison(&tx_hash).await?;
        
        if !comparison.exact_match && !comparison.significant_differences.is_empty() {
            return Err(format!("Complex transaction {} failed validation: {:?}", tx_hash, comparison.significant_differences).into());
        }
    }
    
    Ok(())
}

async fn run_performance_accuracy_test() -> PerformanceTestResult {
    let start_time = std::time::Instant::now();
    let num_transactions = 50;
    
    let batch_result = run_batch_comparison(num_transactions).await;
    
    let elapsed = start_time.elapsed().as_secs_f64();
    
    PerformanceTestResult {
        accuracy: batch_result.success_rate,
        avg_processing_time: elapsed / num_transactions as f64,
        transactions_tested: batch_result.total_transactions,
    }
}

// Helper functions for running comparisons
async fn get_latest_transactions(count: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Use the existing json_state_validator_no_rpc.rs to get recent transactions
    let output = Command::new("cargo")
        .args(&["run", "--example", "json_state_validator_no_rpc", "--", "--count", &count.to_string()])
        .current_dir("/home/nima/code/crypto/rust/revm_tx_simulator")
        .output()?;
    
    if !output.status.success() {
        return Err(format!("Failed to get transactions: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    
    // Parse output to extract transaction hashes
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut transactions = Vec::new();
    
    for line in output_str.lines() {
        if line.contains("Processing transaction:") {
            if let Some(hash) = extract_transaction_hash(line) {
                transactions.push(hash);
            }
        }
    }
    
    Ok(transactions)
}

async fn get_complex_transactions(count: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Get transactions and filter for complex ones (multiple state changes)
    let all_transactions = get_latest_transactions(count * 3).await?;
    
    let mut complex_transactions = Vec::new();
    
    for tx_hash in all_transactions {
        // Check if transaction has multiple state changes (indicating complexity)
        match run_rust_validation(&tx_hash).await {
            Ok(result) => {
                if count_state_changes(&result) >= 3 {
                    complex_transactions.push(tx_hash);
                    if complex_transactions.len() >= count {
                        break;
                    }
                }
            }
            Err(_) => continue,
        }
    }
    
    Ok(complex_transactions)
}

async fn run_python_validation(tx_hash: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let output = Command::new("python3")
        .args(&[
            "/home/nima/code/crypto/rust/mempool_processor/python/core/validate_state_changes.py",
            tx_hash,
            "--json-output"
        ])
        .current_dir("/home/nima/code/crypto/rust/mempool_processor/python")
        .output()?;
    
    if !output.status.success() {
        return Err(format!("Python validation failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    
    let json_str = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(&json_str)?;
    Ok(result)
}

async fn run_rust_validation(tx_hash: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(&["run", "--example", "json_state_validator_no_rpc", "--", "--tx-hash", tx_hash])
        .current_dir("/home/nima/code/crypto/rust/revm_tx_simulator")
        .output()?;
    
    if !output.status.success() {
        return Err(format!("Rust validation failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Extract JSON from output
    for line in output_str.lines() {
        if line.trim().starts_with('{') && line.trim().ends_with('}') {
            let result: Value = serde_json::from_str(line.trim())?;
            return Ok(result);
        }
    }
    
    Err("No JSON output found in Rust validation".into())
}

async fn run_transaction_comparison(tx_hash: &str) -> Result<ComparisonResult, Box<dyn std::error::Error>> {
    let python_result = run_python_validation(tx_hash).await?;
    let rust_result = run_rust_validation(tx_hash).await?;
    compare_state_changes(&python_result, &rust_result)
}

#[derive(Debug)]
struct ComparisonResult {
    exact_match: bool,
    significant_differences: Vec<String>,
    minor_differences: usize,
}

fn compare_state_changes(python_result: &Value, rust_result: &Value) -> Result<ComparisonResult, Box<dyn std::error::Error>> {
    let mut exact_match = true;
    let mut significant_differences = Vec::new();
    let mut minor_differences = 0;
    
    // Extract state changes from both results
    let python_changes = extract_state_changes(python_result)?;
    let rust_changes = extract_state_changes(rust_result)?;
    
    // Compare each address
    let all_addresses: std::collections::HashSet<_> = python_changes.keys()
        .chain(rust_changes.keys())
        .collect();
    
    for address in all_addresses {
        let python_change = python_changes.get(address).cloned().unwrap_or_default();
        let rust_change = rust_changes.get(address).cloned().unwrap_or_default();
        
        let difference = (python_change - rust_change).abs();
        
        if difference > 0.005 {
            // Significant difference above threshold
            significant_differences.push(format!(
                "Address {}: Python={:.6} ETH, Rust={:.6} ETH, Diff={:.6} ETH",
                address, python_change, rust_change, difference
            ));
            exact_match = false;
        } else if difference > 0.0 {
            // Minor difference below threshold
            minor_differences += 1;
            exact_match = false;
        }
    }
    
    Ok(ComparisonResult {
        exact_match,
        significant_differences,
        minor_differences,
    })
}

// Utility functions
fn extract_transaction_hash(line: &str) -> Option<String> {
    // Extract transaction hash from log line
    if let Some(start) = line.find("0x") {
        let hash_part = &line[start..];
        if let Some(end) = hash_part.find(' ') {
            Some(hash_part[..end].to_string())
        } else {
            Some(hash_part.trim().to_string())
        }
    } else {
        None
    }
}

fn extract_state_changes(result: &Value) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
    let mut changes = HashMap::new();
    
    if let Some(state_changes) = result.get("state_changes") {
        if let Some(obj) = state_changes.as_object() {
            for (address, change_data) in obj {
                if let Some(eth_change) = change_data.get("eth_change") {
                    if let Some(eth_value) = eth_change.as_f64() {
                        changes.insert(address.clone(), eth_value);
                    }
                }
            }
        }
    }
    
    Ok(changes)
}

fn count_state_changes(result: &Value) -> usize {
    if let Some(state_changes) = result.get("state_changes") {
        if let Some(obj) = state_changes.as_object() {
            obj.len()
        } else {
            0
        }
    } else {
        0
    }
}