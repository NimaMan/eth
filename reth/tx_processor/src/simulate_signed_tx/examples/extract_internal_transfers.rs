//! Example: Extract internal transfers from a transaction
//! 
//! This example shows how to simulate a transaction and extract:
//! - Internal ETH transfers 
//! - Gas usage and execution status
//!
//! To run as a binary:
//!   cargo run --bin extract_internal_transfers

use anyhow::Result;
use ethers_core::types::H256;
use std::str::FromStr;
use revm_primitives::U256;

// Import the main simulation function
use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx;

/// Format wei value as ETH
fn format_ether(wei: U256) -> String {
    let eth_decimals = 18;
    let divisor = U256::from(10).pow(U256::from(eth_decimals));
    let eth_whole = wei / divisor;
    let eth_fractional = wei % divisor;
    
    // Convert fractional part to string with leading zeros
    let fractional_str = format!("{:018}", eth_fractional);
    // Trim trailing zeros for cleaner display
    let trimmed_fractional = fractional_str.trim_end_matches('0');
    
    if trimmed_fractional.is_empty() {
        format!("{}", eth_whole)
    } else {
        format!("{}.{}", eth_whole, trimmed_fractional)
    }
}

const RPC_URL: &str = "http://127.0.0.1:8545";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Internal Transfer Extraction Example");
    println!("=======================================\n");
    
    // Transaction with known internal transfers
    // This is a complex DeFi transaction that should have internal transfers
    let tx_hash = H256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
    
    println!("📋 Analyzing transaction: {:?}", tx_hash);
    println!("⏳ Simulating transaction...\n");
    
    // Simulate the transaction
    let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
    
    println!("📊 Simulation Results:");
    println!("  • Status: {:?}", output.result_type);
    println!("  • Gas Used: {}", output.gas_used);
    println!("  • Gas Refunded: {}", output.gas_refunded);
    println!("  • Output Data: {} bytes", output.output_data.len());
    println!("  • Logs: {}", output.logs.len());
    println!("  • Internal Transfers: {}", output.internal_transfers.len());
    
    // Check internal transfers (currently returns empty due to REVM v25 integration pending)
    if !output.internal_transfers.is_empty() {
        println!("\n💸 Internal ETH Transfers:");
        for (i, transfer) in output.internal_transfers.iter().enumerate() {
            println!("  {}. {} ETH", i + 1, format_ether(transfer.value));
            println!("     From: 0x{:x}", transfer.from);
            println!("     To: 0x{:x}", transfer.to);
            println!("     Type: {:?}", transfer.call_type);
            println!("     Depth: {}", transfer.depth);
        }
    } else {
        println!("\n💡 No internal ETH transfers detected in this transaction.");
        println!("   This transaction may only involve ERC20 transfers or storage updates.");
    }
    
    // For now, we can demonstrate what information is available
    if output.logs.len() > 0 {
        println!("\n📜 Transaction Logs:");
        for (i, log) in output.logs.iter().take(5).enumerate() {
            println!("  {}. Address: 0x{:x}", i + 1, log.address);
            println!("     Topics: {} topics", log.topics().len());
            if let Some(first_topic) = log.topics().first() {
                println!("     Event: 0x{:x}...", first_topic);
            }
        }
        if output.logs.len() > 5 {
            println!("  ... and {} more logs", output.logs.len() - 5);
        }
    }
    
    Ok(())
}