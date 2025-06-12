//! Python Comparison Example
//! 
//! This example demonstrates how to validate Rust transaction processing
//! results against the Python service running on port 18000.
//! 
//! Usage:
//!   cargo run --bin python_comparison -- 0x<tx_hash>
//!   cargo run --bin python_comparison -- --batch 0x<hash1> 0x<hash2> 0x<hash3>
//!   cargo run --bin python_comparison -- --health-check
//! 
//! The example will:
//! 1. Process the transaction(s) with Rust
//! 2. Send the same transaction(s) to Python service
//! 3. Compare results and show differences
//! 4. Display performance metrics

use std::env;
use std::process;
use anyhow::Result;
use serde_json;

use revm_tx_simulator_lib::process_tx::{
    PythonValidatorClient,
    compare_with_python,
    batch_compare_with_python,
    extract_state_changes_python_format,
};

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";
const DEFAULT_PYTHON_URL: &str = "http://127.0.0.1:18000";

// Known test transactions for demonstration
const TEST_TRANSACTIONS: &[(&str, &str)] = &[
    ("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060", "Simple ETH transfer (early block)"),
    ("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae", "Complex DeFi transaction with Uniswap"),
    ("0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b", "ERC20 token transfer"),
];

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "--help" | "-h" => {
            print_usage();
            return Ok(());
        }
        "--health-check" => {
            run_health_check().await?;
            return Ok(());
        }
        "--batch" => {
            if args.len() < 3 {
                eprintln!("Error: --batch requires at least one transaction hash");
                print_usage();
                process::exit(1);
            }
            let tx_hashes = args[2..].to_vec();
            run_batch_comparison(tx_hashes).await?;
            return Ok(());
        }
        "--test-examples" => {
            run_test_examples().await?;
            return Ok(());
        }
        tx_hash => {
            if !tx_hash.starts_with("0x") {
                eprintln!("Error: Transaction hash must start with 0x");
                print_usage();
                process::exit(1);
            }
            run_single_comparison(tx_hash.to_string()).await?;
        }
    }

    Ok(())
}

/// Run health check on Python service
async fn run_health_check() -> Result<()> {
    println!("🔍 Checking Python validation service health...");
    
    let client = PythonValidatorClient::new(DEFAULT_PYTHON_URL);
    
    match client.health_check().await {
        Ok(health) => {
            println!("✅ Python service is healthy!");
            println!("   Status: {}", health.status);
            println!("   Node connected: {}", health.node_connected);
            println!("   Latest block: {}", health.latest_block);
            println!("   Node version: {}", health.node_version);
        }
        Err(e) => {
            println!("❌ Python service health check failed: {}", e);
            println!("   Make sure the service is running at {}", DEFAULT_PYTHON_URL);
            println!("   You can start it with: python validation_service.py");
        }
    }
    
    Ok(())
}

/// Compare a single transaction
async fn run_single_comparison(tx_hash: String) -> Result<()> {
    println!("🔍 Comparing transaction: {}", tx_hash);
    println!("📊 Rust RPC: {}", DEFAULT_RPC_URL);
    println!("🐍 Python Service: {}", DEFAULT_PYTHON_URL);
    println!();

    match compare_with_python(&tx_hash, DEFAULT_RPC_URL, Some(DEFAULT_PYTHON_URL)).await {
        Ok(result) => {
            print_validation_result(&result);
        }
        Err(e) => {
            println!("❌ Comparison failed: {}", e);
            println!();
            println!("💡 Troubleshooting:");
            println!("   • Ensure Python service is running: python validation_service.py");
            println!("   • Ensure Reth node is running at {}", DEFAULT_RPC_URL);
            println!("   • Check that transaction exists and node is synced");
        }
    }

    Ok(())
}

/// Compare multiple transactions in batch
async fn run_batch_comparison(tx_hashes: Vec<String>) -> Result<()> {
    println!("🔍 Batch comparing {} transactions", tx_hashes.len());
    println!("📊 Rust RPC: {}", DEFAULT_RPC_URL);
    println!("🐍 Python Service: {}", DEFAULT_PYTHON_URL);
    println!();

    match batch_compare_with_python(tx_hashes.clone(), DEFAULT_RPC_URL, Some(DEFAULT_PYTHON_URL)).await {
        Ok(results) => {
            let successful = results.iter().filter(|r| r.matches).count();
            let total = results.len();
            
            println!("📊 Batch Results Summary:");
            println!("   Total transactions: {}", total);
            println!("   Successful matches: {}", successful);
            println!("   Failed matches: {}", total - successful);
            println!("   Success rate: {:.1}%", (successful as f64 / total as f64) * 100.0);
            println!();

            for (i, result) in results.iter().enumerate() {
                println!("Transaction {} of {}:", i + 1, total);
                print_validation_result(result);
                if i < results.len() - 1 {
                    println!("─────────────────────────────────────────────");
                }
            }
        }
        Err(e) => {
            println!("❌ Batch comparison failed: {}", e);
        }
    }

    Ok(())
}

/// Run comparison on test examples
async fn run_test_examples() -> Result<()> {
    println!("🧪 Running validation on test examples...");
    println!();

    for (tx_hash, description) in TEST_TRANSACTIONS {
        println!("🔍 Testing: {}", description);
        println!("   Hash: {}", tx_hash);
        
        match compare_with_python(tx_hash, DEFAULT_RPC_URL, Some(DEFAULT_PYTHON_URL)).await {
            Ok(result) => {
                let status = if result.matches { "✅ PASS" } else { "❌ FAIL" };
                println!("   Result: {}", status);
                println!("   Rust time: {:.1}ms", result.rust_processing_time_ms);
                println!("   Python time: {:.1}ms", result.python_processing_time_ms);
                
                if !result.differences.is_empty() {
                    println!("   Differences: {}", result.differences.len());
                }
            }
            Err(e) => {
                println!("   Result: ❌ ERROR - {}", e);
            }
        }
        println!();
    }

    Ok(())
}

/// Print detailed validation result
fn print_validation_result(result: &revm_tx_simulator_lib::process_tx::ValidationResult) {
    let status_icon = if result.matches { "✅" } else { "❌" };
    
    println!("{} Transaction: {}", status_icon, result.tx_hash);
    println!("   Match Status: {}", if result.matches { "PASS" } else { "FAIL" });
    println!("   Rust Processing: {:.1}ms", result.rust_processing_time_ms);
    println!("   Python Processing: {:.1}ms", result.python_processing_time_ms);
    
    let speed_ratio = result.rust_processing_time_ms / result.python_processing_time_ms;
    if speed_ratio < 1.0 {
        println!("   Performance: Rust is {:.1}x faster", 1.0 / speed_ratio);
    } else {
        println!("   Performance: Python is {:.1}x faster", speed_ratio);
    }

    if !result.differences.is_empty() {
        println!("   Differences Found: {}", result.differences.len());
        for (i, diff) in result.differences.iter().enumerate() {
            println!("     {}. {}: {}", i + 1, diff.field, diff.description);
        }
    }

    // Display state changes summary
    if let Some(ref rust_changes) = result.rust_state_changes {
        println!("   Rust State Changes:");
        println!("     Addresses affected: {}", rust_changes.metadata.addresses_affected);
        println!("     Tokens involved: {}", rust_changes.metadata.tokens_involved);
        
        // Show sample of state changes
        let sample_count = rust_changes.state_changes.len().min(3);
        for (addr, changes) in rust_changes.state_changes.iter().take(sample_count) {
            println!("     {}: ETH: {}, Tokens: {}", 
                &addr[..10], 
                changes.eth_net,
                changes.token_net.len()
            );
        }
        
        if rust_changes.state_changes.len() > sample_count {
            println!("     ... and {} more addresses", 
                rust_changes.state_changes.len() - sample_count);
        }
    }

    println!();
}

/// Print usage information
fn print_usage() {
    println!("Python Comparison Tool");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin python_comparison -- [OPTIONS] <TRANSACTION_HASH>");
    println!();
    println!("OPTIONS:");
    println!("    --health-check              Check Python service health");
    println!("    --batch <TX1> <TX2> ...     Compare multiple transactions");
    println!("    --test-examples             Run validation on built-in test examples");
    println!("    --help, -h                  Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    # Compare single transaction");
    println!("    cargo run --bin python_comparison -- 0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060");
    println!();
    println!("    # Compare multiple transactions");
    println!("    cargo run --bin python_comparison -- --batch \\");
    println!("        0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060 \\");
    println!("        0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae");
    println!();
    println!("    # Check service health");
    println!("    cargo run --bin python_comparison -- --health-check");
    println!();
    println!("    # Test built-in examples");
    println!("    cargo run --bin python_comparison -- --test-examples");
    println!();
    println!("REQUIREMENTS:");
    println!("    • Python validation service running at http://127.0.0.1:18000");
    println!("    • Reth node running at http://127.0.0.1:8545");
    println!("    • Node synced to required block heights");
    println!();
    println!("SETUP:");
    println!("    # Start Python service");
    println!("    cd /path/to/validation/service");
    println!("    python validation_service.py");
    println!();
    println!("    # Verify service is running");
    println!("    curl http://127.0.0.1:18000/health");
}