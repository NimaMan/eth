//! Example: Basic usage of the transaction simulator
//! 
//! This example shows the simplest way to use the library:
//! Just provide a transaction hash and get simulation results.
//!
//! To run as a binary:
//!   cargo run --bin simulate_basic_usage

use anyhow::Result;

// That's all you need to import!
use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx;

#[tokio::main]
async fn main() -> Result<()> {
    // Recent complex DeFi transaction
    let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
        .parse()?;
    
    // Your local Reth node
    let rpc_url = "http://127.0.0.1:8545";
    
    println!("Simulating transaction...\n");
    
    // That's it! The library handles everything else
    let result = simulate_signed_tx(tx_hash, rpc_url).await?;
    
    // Use the results
    println!("Transaction simulation complete!");
    println!("- Status: {:?}", result.result_type);
    println!("- Gas used: {}", result.gas_used);
    println!("- Gas refunded: {}", result.gas_refunded);
    
    Ok(())
}