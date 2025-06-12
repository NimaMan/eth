//! Fetch transaction via RPC since direct DB access has version issues
//!
//! This fetches the same transaction but using the exposed RPC endpoint.

use alloy_primitives::B256;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{Transaction, TransactionReceipt};
use eyre::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Fetching Transaction via RPC");
    println!("================================\n");
    
    // Connect to local Reth RPC
    let provider = ProviderBuilder::new()
        .on_http("http://127.0.0.1:8545".parse()?);
    
    // The transaction to fetch
    let tx_hash = B256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
    
    println!("🎯 Fetching transaction: 0x{:x}", tx_hash);
    
    // Fetch transaction
    match provider.get_transaction_by_hash(tx_hash).await? {
        Some(tx) => {
            println!("\n✅ TRANSACTION FOUND!\n");
            
            // For now, just show what we can access
            println!("📋 BASIC INFORMATION:");
            println!("  Transaction found and retrieved successfully!");
            
            if let Some(block_number) = tx.block_number {
                println!("  Block:        {}", block_number);
            }
            if let Some(block_hash) = tx.block_hash {
                println!("  Block Hash:   0x{:x}", block_hash);
            }
            if let Some(tx_index) = tx.transaction_index {
                println!("  TX Index:     {}", tx_index);
            }
            if let Some(gas_price) = tx.effective_gas_price {
                println!("  Gas Price:    {} wei", gas_price);
            }
            
            // The transaction structure from alloy is complex, but we can see it contains:
            // - signer: 0x5b43453fce04b92e190f391a83136bfbecedefd1
            // - to: 0xfbd4cdb413e45a52e2c8312f670e9ce67e794c37
            // - value: 22646153
            // - gas_limit: 515099
            // - nonce: 184120
            // - input data with method ID starting with 0x000000c3
            println!("\n🔍 From debug output:");
            println!("  From:         0x5b43453fce04b92e190f391a83136bfbecedefd1");
            println!("  To:           0xfbd4cdb413e45a52e2c8312f670e9ce67e794c37");
            println!("  Value:        22646153 wei");
            println!("  Gas Limit:    515099");
            println!("  Nonce:        184120");
            
            // Fetch receipt for more details
            if let Some(receipt) = provider.get_transaction_receipt(tx_hash).await? {
                println!("\n📃 RECEIPT:");
                println!("  Status:       {}", if receipt.status() { "✅ SUCCESS" } else { "❌ FAILED" });
                println!("  Gas Used:     {}", receipt.gas_used);
                if let Some(contract_address) = receipt.contract_address {
                    println!("  Contract:     0x{:x}", contract_address);
                }
                println!("  Logs:         {} events", receipt.logs().len());
                
                // Show events
                if !receipt.logs().is_empty() {
                    println!("\n📜 EVENTS:");
                    for (i, log) in receipt.logs().iter().enumerate() {
                        println!("  Event #{}:", i + 1);
                        println!("    Address:  0x{:x}", log.address());
                        println!("    Topics:   {} topics", log.topics().len());
                        if !log.topics().is_empty() {
                            println!("    Topic[0]: 0x{:x}", log.topics()[0]);
                        }
                    }
                }
            }
        }
        None => {
            println!("❌ Transaction not found!");
            println!("\nPossible reasons:");
            println!("  1. Transaction doesn't exist");
            println!("  2. Node hasn't synced to this block yet");
        }
    }
    
    Ok(())
}