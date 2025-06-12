//! Example: Simulate a transaction by its hash
//! 
//! This example demonstrates how to use the simulate_signed_tx module to:
//! 1. Fetch a transaction from the network
//! 2. Simulate it using REVM
//! 3. Extract gas usage, status, and other results
//!
//! To run as a binary:
//!   cargo run --bin simulate_transaction
//!   cargo run --bin simulate_transaction -- 0x<your_tx_hash>

use anyhow::Result;
use ethers_core::types::H256;
use std::str::FromStr;

// Import from our crate
use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Transaction Simulation Example");
    println!("=================================\n");

    // Get transaction hash from command line or use default
    let args: Vec<String> = std::env::args().collect();
    let tx_hash_str = if args.len() > 1 {
        &args[1]
    } else {
        // Default: Complex DeFi transaction with internal transfers
        "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
    };
    
    let tx_hash = H256::from_str(tx_hash_str)?;
    let rpc_url = "http://127.0.0.1:8545";
    
    println!("📋 Transaction: {:?}", tx_hash);
    println!("🌐 RPC URL: {}\n", rpc_url);
    
    println!("⏳ Simulating transaction...");
    let start = std::time::Instant::now();
    
    // The module handles everything internally
    let output = simulate_signed_tx(tx_hash, rpc_url).await?;
    
    let elapsed = start.elapsed();
    
    // Display results
    println!("\n✅ Simulation Complete!");
    println!("⏱️  Time: {:.2}ms\n", elapsed.as_millis());
    
    println!("📊 Results:");
    println!("  • Status: {:?}", output.result_type);
    println!("  • Gas Used: {}", output.gas_used);
    println!("  • Gas Refunded: {}", output.gas_refunded);
    println!("  • Output Data: {} bytes", output.output_data.len());
    println!("  • Logs: {}", output.logs.len());
    
    // For the known complex transaction, verify gas usage
    if tx_hash_str == "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae" {
        assert_eq!(output.gas_used, 315099, "Gas usage should match expected");
        assert!(matches!(output.result_type, revm_tx_simulator_lib::ExecutionResultType::Success(_)), "Transaction should succeed");
        println!("\n✅ Verification passed!");
    }
    
    Ok(())
}