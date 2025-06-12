//! Example: Simulate a signed transaction by its hash
//! 
//! This demonstrates the module's ability to:
//! 1. Take a transaction hash as input
//! 2. Fetch the transaction from the network
//! 3. Convert it to REVM types internally
//! 4. Simulate and return results

use anyhow::Result;
use ethers_core::types::H256;
use std::str::FromStr;

// Import from our crate
use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx;

#[tokio::main]
async fn main() -> Result<()> {
    // Get transaction hash from args or use a recent simple transfer
    let args: Vec<String> = std::env::args().collect();
    let tx_hash_str = if args.len() > 1 {
        &args[1]
    } else {
        // Simple ETH transfer with available state
        "0x21aeb75078488051edcdce4b55e4a9bc6c1fd217bb19c6654921b2a4aed8e3db"
    };
    
    let tx_hash = H256::from_str(tx_hash_str)?;
    let rpc_url = "http://127.0.0.1:8545";
    
    println!("Simulating transaction: {:?}", tx_hash);
    println!("Using RPC: {}", rpc_url);
    
    // The module handles everything internally:
    // 1. Fetches transaction data
    // 2. Fetches block data
    // 3. Converts to REVM types
    // 4. Sets up database
    // 5. Runs simulation
    let output = simulate_signed_tx(tx_hash, rpc_url).await?;
    
    // Display results
    println!("\nSimulation Results:");
    println!("==================");
    println!("Gas Used: {}", output.gas_used);
    println!("Status: {:?}", output.result_type);
    println!("Output Data Length: {}", output.output_data.len());
    
    // Verify transaction succeeded
    assert!(matches!(output.result_type, revm_tx_simulator_lib::ExecutionResultType::Success(_)), "Transaction should succeed");
    
    // For the known complex transaction, verify gas usage
    if tx_hash_str == "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae" {
        assert_eq!(output.gas_used, 315099, "Gas usage should match expected");
    }
    
    println!("\n✅ Module successfully simulated the signed transaction!");
    
    Ok(())
}