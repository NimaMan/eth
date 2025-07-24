/// Process a transaction by hash and display all decoded information
/// 
/// This example shows how to fetch and process a transaction from the Reth database
/// using only its hash. It demonstrates the full processing pipeline including:
/// - Event decoding (ERC20 transfers, swaps, etc.)
/// - Internal transaction extraction
/// - Transaction classification

use tx_processor::tx_processor::TxProcessor;
use alloy_primitives::B256;
use eyre::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Process Transaction by Hash Example");
    println!("=====================================\n");
    
    // Initialize processor
    let processor = TxProcessor::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Test transaction that should have 5 ERC20 transfers and 2 internal transfers
    let tx_hash = B256::from_str("0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7")?;
    
    println!("Fetching transaction: {}", tx_hash);
    println!("Expected: 5 ERC20 transfers, 2 internal transfers\n");
    
    match processor.process_transaction_by_hash(tx_hash).await {
        Ok(tx) => {
            println!("✅ Transaction Fetched Successfully!");
            println!("===================================");
            
            // Basic info
            println!("\nBasic Information:");
            println!("  Block: {}", tx.block_number);
            println!("  From: {}", tx.from_address);
            println!("  To: {:?}", tx.to_address);
            println!("  Value: {} wei", tx.value);
            println!("  Status: {}", tx.status);
            println!("  Type: {}", tx.txn_type);
            println!("  Input data length: {} bytes", tx.input.len());
            
            // ERC20 Transfers
            println!("\n📊 ERC20 Transfers: {} found", tx.erc20_transfers.len());
            for (i, transfer) in tx.erc20_transfers.iter().enumerate() {
                println!("\n  Transfer #{}:", i + 1);
                println!("    Token: {}", transfer.token_address);
                println!("    From: {}", transfer.from_address);
                println!("    To: {}", transfer.to_address);
                println!("    Amount: {}", transfer.amount);
                println!("    Log Index: {}", transfer.log_index);
            }
            
            // Internal Transactions
            println!("\n💰 Internal Transactions: {} found", tx.internal_transactions.len());
            for (i, internal) in tx.internal_transactions.iter().enumerate() {
                println!("\n  Internal Tx #{}:", i + 1);
                println!("    From: {}", internal.from_address);
                println!("    To: {}", internal.to_address);
                println!("    Value: {} wei", internal.value);
                println!("    Depth: {}", internal.depth);
                println!("    Type: {}", internal.trace_type);
                if let Some(call_type) = &internal.call_type {
                    println!("    Call Type: {}", call_type);
                }
            }
            
            // Other events
            println!("\n📋 Other Decoded Events:");
            println!("  Uniswap V2 Swaps: {}", tx.uniswap_v2_swaps.len());
            println!("  Uniswap V3 Swaps: {}", tx.uniswap_v3_swaps.len());
            println!("  Approvals: {}", tx.approvals.len());
            
            // Summary
            println!("\n📊 Summary:");
            println!("  ✅ ERC20 Transfers: {} (expected 5)", tx.erc20_transfers.len());
            println!("  ✅ Internal Transactions: {} (expected 2)", tx.internal_transactions.len());
            println!("  ✅ Unique Addresses: {}", tx.unique_addresses.len());
            
            // How we got this data
            println!("\n🔧 How We Got This Data:");
            println!("1. TransactionLoader fetched from Reth DB:");
            println!("   - Transaction data (from, to, value, input)");
            println!("   - Receipt with logs (contains ERC20 Transfer events)");
            println!("   - Block header (for timestamp)");
            println!("2. process_transaction() decoded the logs:");
            println!("   - Each log checked against known event signatures");
            println!("   - ERC20 Transfer events decoded into transfers");
            println!("3. Simulation for internal transactions:");
            println!("   - Contract interaction detected (has input data)");
            println!("   - Transaction simulated to extract internal ETH transfers");
            
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            println!("\nThis transaction might not be in your local Reth database.");
            println!("The Reth node needs to have synced past the block containing this transaction.");
        }
    }
    
    Ok(())
}