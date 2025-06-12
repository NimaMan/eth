//! Example: Simulate from raw signed transaction bytes
//! 
//! This example demonstrates simulating a transaction from raw RLP-encoded bytes
//! without fetching the transaction via RPC (only block info is fetched)
//!
//! To run as a binary:
//!   cargo run --bin simulate_raw_bytes

use anyhow::Result;

// Import the main simulation function
use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx_bytes;

const RPC_URL: &str = "http://127.0.0.1:8545";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔧 Raw Transaction Bytes Simulation Example");
    println!("============================================\n");
    
    // This is a real signed transaction in raw RLP bytes format
    // Transaction hash: 0x21aeb75078488051edcdce4b55e4a9bc6c1fd217bb19c6654921b2a4aed8e3db (simple ETH transfer)
    // Raw bytes obtained from: cast tx --raw 0x21aeb75078488051edcdce4b55e4a9bc6c1fd217bb19c6654921b2a4aed8e3db
    let raw_tx_hex = "02f87401808501193a82298501193a82298252089431385527fe03ca642f043a9e8a1c8bacbd9addac8805e19dbc7b6438a480c001a0c42862561ee35aa615dfa57228a64100bfda08752bbf8eccfe3a36f4a088371aa003cdff0088d91e73933656ecceb122337956a7c30eab96fa4642217d1f81be21";
    
    let raw_bytes = hex::decode(raw_tx_hex.trim_start_matches("0x"))?;
    
    // Block number when this transaction was mined  
    let block_number = 22687107u64;
    
    println!("📋 Simulating from raw transaction bytes:");
    println!("  • Transaction size: {} bytes", raw_bytes.len());
    println!("  • Block number: {}", block_number);
    println!("  • RPC endpoint: {}", RPC_URL);
    println!();
    
    println!("⏳ Simulating transaction from raw bytes...\n");
    
    // Simulate the transaction from raw bytes
    let output = simulate_signed_tx_bytes(&raw_bytes, block_number, RPC_URL).await?;
    
    println!("📊 Simulation Results:");
    println!("  • Status: {:?}", output.result_type);
    println!("  • Gas Used: {}", output.gas_used);
    println!("  • Gas Refunded: {}", output.gas_refunded);
    println!("  • Output Data: {} bytes", output.output_data.len());
    println!("  • Logs: {}", output.logs.len());
    println!("  • Internal Transfers: {}", output.internal_transfers.len());
    
    if !output.internal_transfers.is_empty() {
        println!("\n💸 Internal ETH Transfers:");
        for (i, transfer) in output.internal_transfers.iter().enumerate() {
            let eth_amount = transfer.value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
            println!("  {}. {:.6} ETH", i + 1, eth_amount);
            println!("     From: 0x{:x}", transfer.from);
            println!("     To: 0x{:x}", transfer.to);
            println!("     Type: {:?}", transfer.call_type);
            println!("     Depth: {}", transfer.depth);
        }
    }
    
    // Show some event logs
    if !output.logs.is_empty() {
        println!("\n📜 Event Logs (first 3):");
        for (i, log) in output.logs.iter().take(3).enumerate() {
            println!("  {}. Contract: 0x{:x}", i + 1, log.address);
            println!("     Topics: {}", log.topics().len());
            if let Some(first_topic) = log.topics().first() {
                println!("     Event: 0x{:x}...", first_topic);
            }
        }
        if output.logs.len() > 3 {
            println!("  ... and {} more logs", output.logs.len() - 3);
        }
    }
    
    println!("\n✅ Successfully simulated transaction from raw bytes!");
    println!("   Only fetched block info via RPC - transaction was processed from raw bytes");
    
    Ok(())
}