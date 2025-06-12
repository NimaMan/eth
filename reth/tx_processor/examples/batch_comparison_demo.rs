//! Batch Comparison Demo - 100 Transaction Test
//! 
//! Demonstrates batch comparison functionality with detailed failure logging

use std::collections::HashMap;
use std::time::Instant;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone)]
struct FailedTransaction {
    tx_hash: String,
    rust_calculation: JsonValue,
    python_calculation: JsonValue,
    differences: Vec<String>,
    rust_time_ms: f64,
    python_time_ms: f64,
    error_type: String,
}

#[derive(Debug)]
struct BatchResults {
    total_transactions: usize,
    successful_matches: usize,
    failed_matches: usize,
    failed_transactions: Vec<FailedTransaction>,
    total_rust_time: f64,
    total_python_time: f64,
    total_wall_time: f64,
}

impl BatchResults {
    fn success_rate(&self) -> f64 {
        if self.total_transactions == 0 { 0.0 } 
        else { (self.successful_matches as f64 / self.total_transactions as f64) * 100.0 }
    }
    
    fn avg_rust_time(&self) -> f64 {
        if self.total_transactions == 0 { 0.0 } 
        else { self.total_rust_time / self.total_transactions as f64 }
    }
    
    fn avg_python_time(&self) -> f64 {
        if self.total_transactions == 0 { 0.0 } 
        else { self.total_python_time / self.total_transactions as f64 }
    }
    
    fn speedup_factor(&self) -> f64 {
        if self.avg_rust_time() == 0.0 { 0.0 } 
        else { self.avg_python_time() / self.avg_rust_time() }
    }
}

// Generate 100 test transaction hashes
fn generate_100_test_transactions() -> Vec<String> {
    let mut transactions = Vec::new();
    
    // Start with known mainnet transactions
    transactions.push("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060".to_string());
    transactions.push("0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b".to_string());
    
    // Generate 98 more test transactions
    for i in 2..100 {
        let tx_hash = format!("0x{:064x}", 0x1000000000000000u64 + i as u64);
        transactions.push(tx_hash);
    }
    
    transactions
}

fn simulate_transaction_comparison(tx_hash: &str, index: usize) -> Result<(f64, f64), FailedTransaction> {
    // Simulate processing times (Rust faster than Python)
    let rust_time = 2.0 + (index as f64 * 0.02) + (rand::random::<f64>() * 3.0);
    let python_time = 15.0 + (index as f64 * 0.05) + (rand::random::<f64>() * 8.0);
    
    // Simulate failures for certain patterns (15% failure rate)
    let should_fail = index % 7 == 0 || index % 13 == 0 || index % 17 == 0;
    
    if should_fail {
        // Create realistic failure scenarios
        let failed_tx = FailedTransaction {
            tx_hash: tx_hash.to_string(),
            rust_calculation: serde_json::json!({
                "state_changes": {
                    format!("0x{:040x}", index): {
                        "eth_net": format!("{:.6}", 1.5 + (index as f64 * 0.001)),
                        "token_net": {
                            "USDC": format!("{:.2}", 1000.0 + (index as f64 * 10.0)),
                            "WETH": "0.5"
                        }
                    }
                },
                "metadata": {
                    "tx_hash": tx_hash,
                    "processing_time_ms": rust_time,
                    "addresses_affected": 1,
                    "tokens_involved": 2
                }
            }),
            python_calculation: serde_json::json!({
                "state_changes": {
                    format!("0x{:040x}", index): {
                        "eth_net": format!("{:.6}", if index % 7 == 0 { 1.6 + (index as f64 * 0.001) } else { 1.5 + (index as f64 * 0.001) }),
                        "token_net": {
                            "USDC": format!("{:.2}", if index % 13 == 0 { 999.0 + (index as f64 * 10.0) } else { 1000.0 + (index as f64 * 10.0) }),
                            "WETH": if index % 17 == 0 { "0.51" } else { "0.5" }
                        }
                    }
                }
            }),
            differences: {
                let mut diffs = Vec::new();
                if index % 7 == 0 {
                    diffs.push(format!("0x{:040x}.eth_net: ETH net change mismatch (Rust: {:.6}, Python: {:.6})", 
                        index, 1.5 + (index as f64 * 0.001), 1.6 + (index as f64 * 0.001)));
                }
                if index % 13 == 0 {
                    diffs.push(format!("0x{:040x}.token_net.USDC: Token amount mismatch (Rust: {:.2}, Python: {:.2})", 
                        index, 1000.0 + (index as f64 * 10.0), 999.0 + (index as f64 * 10.0)));
                }
                if index % 17 == 0 {
                    diffs.push(format!("0x{:040x}.token_net.WETH: Token amount mismatch (Rust: 0.5, Python: 0.51)", index));
                }
                diffs
            },
            rust_time_ms: rust_time,
            python_time_ms: python_time,
            error_type: {
                if index % 7 == 0 { "eth_mismatch" }
                else if index % 13 == 0 { "token_amount_mismatch" }
                else { "precision_error" }
            }.to_string(),
        };
        
        Err(failed_tx)
    } else {
        Ok((rust_time, python_time))
    }
}

fn simulate_batch_comparison() -> BatchResults {
    println!("🔍 Starting batch comparison of 100 transactions...");
    println!("📊 Rust RPC: http://127.0.0.1:8545");
    println!("🐍 Python Service: http://127.0.0.1:18000");
    println!();
    
    let tx_hashes = generate_100_test_transactions();
    let start_time = Instant::now();
    
    let mut results = BatchResults {
        total_transactions: 100,
        successful_matches: 0,
        failed_matches: 0,
        failed_transactions: Vec::new(),
        total_rust_time: 0.0,
        total_python_time: 0.0,
        total_wall_time: 0.0,
    };
    
    for (i, tx_hash) in tx_hashes.iter().enumerate() {
        if i % 10 == 0 {
            println!("Processing transaction {} of 100...", i + 1);
        }
        
        match simulate_transaction_comparison(tx_hash, i) {
            Ok((rust_time, python_time)) => {
                results.successful_matches += 1;
                results.total_rust_time += rust_time;
                results.total_python_time += python_time;
            }
            Err(failed_tx) => {
                results.failed_matches += 1;
                results.total_rust_time += failed_tx.rust_time_ms;
                results.total_python_time += failed_tx.python_time_ms;
                results.failed_transactions.push(failed_tx);
            }
        }
        
        // Simulate processing delay
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    
    results.total_wall_time = start_time.elapsed().as_secs_f64() * 1000.0;
    results
}

fn print_summary(results: &BatchResults) {
    println!("📊 Batch Comparison Results for 100 Transactions");
    println!("═══════════════════════════════════════════════");
    println!();
    
    println!("📈 Match Results:");
    println!("   Total transactions: {}", results.total_transactions);
    println!("   ✅ Successful matches: {}", results.successful_matches);
    println!("   ❌ Failed matches: {}", results.failed_matches);
    println!("   📊 Success rate: {:.1}%", results.success_rate());
    println!();
    
    println!("⚡ Performance Results:");
    println!("   🦀 Total Rust time: {:.1}ms", results.total_rust_time);
    println!("   🐍 Total Python time: {:.1}ms", results.total_python_time);
    println!("   🦀 Average Rust time: {:.1}ms", results.avg_rust_time());
    println!("   🐍 Average Python time: {:.1}ms", results.avg_python_time());
    println!("   🚀 Speedup factor: {:.1}x", results.speedup_factor());
    println!("   ⏱️  Total wall time: {:.1}ms", results.total_wall_time);
    println!();
    
    if !results.failed_transactions.is_empty() {
        println!("❌ Failed Transaction Details:");
        println!("═══════════════════════════════");
        for (i, failed) in results.failed_transactions.iter().enumerate() {
            println!("{}. Transaction: {}", i + 1, failed.tx_hash);
            println!("   Error Type: {}", failed.error_type);
            println!("   Rust Time: {:.1}ms | Python Time: {:.1}ms", failed.rust_time_ms, failed.python_time_ms);
            println!("   Differences:");
            for diff in &failed.differences {
                println!("     • {}", diff);
            }
            println!("   Rust Calculation:");
            println!("     {}", serde_json::to_string_pretty(&failed.rust_calculation).unwrap_or("Error".to_string()));
            println!("   Python Calculation:");
            println!("     {}", serde_json::to_string_pretty(&failed.python_calculation).unwrap_or("Error".to_string()));
            println!("   ─────────────────────────────────────");
            
            if i >= 4 {  // Show only first 5 for brevity
                if results.failed_transactions.len() > 5 {
                    println!("   ... and {} more failures (similar patterns)", results.failed_transactions.len() - 5);
                }
                break;
            }
        }
    }
}

fn save_failure_log(results: &BatchResults) -> Result<String, Box<dyn std::error::Error>> {
    let timestamp: DateTime<Utc> = Utc::now();
    let filename = format!("failed_transactions_{}.json", timestamp.format("%Y%m%d_%H%M%S"));
    
    let log_content = serde_json::json!({
        "metadata": {
            "timestamp": timestamp,
            "total_transactions": results.total_transactions,
            "successful_matches": results.successful_matches,
            "failed_matches": results.failed_matches,
            "success_rate_percent": results.success_rate(),
            "avg_rust_time_ms": results.avg_rust_time(),
            "avg_python_time_ms": results.avg_python_time(),
            "speedup_factor": results.speedup_factor(),
            "total_wall_time_ms": results.total_wall_time
        },
        "failed_transactions": results.failed_transactions.iter().map(|failed| {
            serde_json::json!({
                "tx_hash": failed.tx_hash,
                "error_type": failed.error_type,
                "rust_processing_time_ms": failed.rust_time_ms,
                "python_processing_time_ms": failed.python_time_ms,
                "differences": failed.differences,
                "rust_state_changes": failed.rust_calculation,
                "python_state_changes": failed.python_calculation
            })
        }).collect::<Vec<_>>(),
        "error_analysis": {
            let mut eth_errors = 0;
            let mut token_errors = 0;
            let mut precision_errors = 0;
            
            for failed in &results.failed_transactions {
                match failed.error_type.as_str() {
                    "eth_mismatch" => eth_errors += 1,
                    "token_amount_mismatch" => token_errors += 1,
                    "precision_error" => precision_errors += 1,
                    _ => {}
                }
            }
            
            let most_common = if eth_errors > token_errors && eth_errors > precision_errors { 
                "eth_mismatch" 
            } else if token_errors > precision_errors { 
                "token_amount_mismatch" 
            } else { 
                "precision_error" 
            };
            
            serde_json::json!({
                "error_types": {
                    "eth_mismatch": eth_errors,
                    "token_amount_mismatch": token_errors,
                    "precision_error": precision_errors
                },
                "most_common_error": most_common
            })
        },
        "recommendations": [
            "Investigate ETH decimal precision handling",
            "Review token amount calculation differences", 
            "Check floating point precision in comparisons",
            "Implement tolerance-based comparison for edge cases"
        ]
    });
    
    std::fs::write(&filename, serde_json::to_string_pretty(&log_content)?)?;
    Ok(filename)
}

fn print_api_usage_example() {
    println!("🔧 Real API Usage Example");
    println!("=========================");
    println!();
    println!("```rust");
    println!("use revm_tx_simulator_lib::process_tx::batch_compare_with_python;");
    println!();
    println!("// Compare 100 transactions");
    println!("let tx_hashes: Vec<String> = get_100_transaction_hashes();");
    println!("let results = batch_compare_with_python(");
    println!("    tx_hashes,");
    println!("    \"http://127.0.0.1:8545\",        // Reth RPC URL");
    println!("    Some(\"http://127.0.0.1:18000\")  // Python service URL");
    println!(").await?;");
    println!();
    println!("// Analyze results");
    println!("let successful = results.iter().filter(|r| r.matches).count();");
    println!("let total = results.len();");
    println!("println!(\"Success rate: {{:.1}}%\", (successful as f64 / total as f64) * 100.0);");
    println!();
    println!("// Log failures");
    println!("for result in &results {{");
    println!("    if !result.matches {{");
    println!("        println!(\"Failed: {{}}\", result.tx_hash);");
    println!("        for diff in &result.differences {{");
    println!("            println!(\"  • {{}}: {{}}\", diff.field, diff.description);");
    println!("        }}");
    println!("    }}");
    println!("}}");
    println!("```");
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Batch Transaction Comparison Demo");
    println!("====================================");
    println!("Demonstrating comparison of 100 transactions with failure logging");
    println!();
    
    // Show API usage example
    print_api_usage_example();
    
    // Run the simulation
    let results = simulate_batch_comparison();
    
    // Print results
    print_summary(&results);
    
    // Save failure log if there are failures
    if !results.failed_transactions.is_empty() {
        match save_failure_log(&results) {
            Ok(filename) => {
                println!();
                println!("📝 Detailed failure log saved to: {}", filename);
                println!("   This JSON file contains complete state change calculations");
                println!("   and detailed analysis for all {} failed transactions", results.failed_transactions.len());
            }
            Err(e) => {
                println!("❌ Failed to save log file: {}", e);
            }
        }
    }
    
    println!();
    println!("🎯 Summary Analysis:");
    if results.success_rate() >= 85.0 {
        println!("   ✅ Excellent success rate ({:.1}%) - system validation is strong", results.success_rate());
    } else if results.success_rate() >= 70.0 {
        println!("   ⚠️  Good success rate ({:.1}%) - minor issues to investigate", results.success_rate());
    } else {
        println!("   ❌ Low success rate ({:.1}%) - significant validation issues", results.success_rate());
    }
    
    if results.speedup_factor() >= 3.0 {
        println!("   🚀 Excellent performance: Rust is {:.1}x faster than Python", results.speedup_factor());
    } else if results.speedup_factor() >= 2.0 {
        println!("   ⚡ Good performance: Rust is {:.1}x faster than Python", results.speedup_factor());
    } else {
        println!("   🐌 Performance acceptable: {:.1}x speedup", results.speedup_factor());
    }
    
    println!();
    println!("📋 To run with real services:");
    println!("   1. Start Reth node: reth node");  
    println!("   2. Start Python service: python validation_service.py");
    println!("   3. Run: cargo run --bin python_comparison -- --batch [100_TX_HASHES]");
    
    Ok(())
}