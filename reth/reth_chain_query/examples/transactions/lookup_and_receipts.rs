/// Transaction lookup and receipt queries
/// 
/// This example shows how to:
/// 1. Find transactions by hash
/// 2. Get transaction receipts
/// 3. Parse logs and events
/// 4. Check transaction status
/// 
/// Run with: cargo run --example lookup_and_receipts

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{B256, Address, U256, utils::format_ether};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Transaction Lookup and Receipts ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // === Find Transaction by Hash ===
    println!("1. Transaction Lookup");
    println!("-" .repeat(60));
    
    // Example: A recent USDC transfer transaction
    // You'll need to replace this with a real transaction hash
    let tx_hash = B256::from_str(
        "0x3feab7d74656d4f3ea4814ec450eedc9875c7e8e856270951334cf66da6e14da"
    )?;
    
    // Check if transaction exists
    if provider.transaction_exists(tx_hash).await? {
        // Get transaction data
        let tx_data = provider.get_transaction_by_hash(tx_hash).await?;
        
        println!("Transaction found!");
        println!("  Hash: 0x{}", tx_data.hash);
        println!("  Block: #{} (timestamp: {})", 
            tx_data.block_number, 
            tx_data.block_timestamp
        );
        println!("  From: 0x{}", tx_data.from);
        println!("  To: {}", 
            tx_data.to.map(|a| format!("0x{}", a))
                .unwrap_or("Contract Creation".to_string())
        );
        println!("  Value: {} ETH", format_ether(tx_data.value));
        println!("  Gas: {} @ {} gwei", 
            tx_data.gas_limit, 
            tx_data.gas_price / 1_000_000_000u64
        );
        
        // Get receipt
        let receipt = provider.get_transaction_receipt(tx_hash).await?;
        
        println!("\nReceipt:");
        println!("  Status: {}", 
            if receipt.status { "✅ Success" } else { "❌ Failed" }
        );
        println!("  Gas used: {} ({:.1}% of limit)", 
            receipt.gas_used,
            (receipt.gas_used as f64 / tx_data.gas_limit as f64) * 100.0
        );
        println!("  Logs: {} events emitted", receipt.logs.len());
        
        // Parse logs
        if !receipt.logs.is_empty() {
            println!("\nEvent Logs:");
            for (i, log) in receipt.logs.iter().enumerate() {
                println!("  Log #{}:", i);
                println!("    Contract: 0x{}", log.address);
                println!("    Topics: {}", log.topics.len());
                
                // Check for common event signatures
                if !log.topics.is_empty() {
                    let event_sig = &log.topics[0];
                    
                    // Transfer event signature
                    let transfer_sig = B256::from_str(
                        "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
                    )?;
                    
                    if *event_sig == transfer_sig {
                        println!("    Event: Transfer");
                        if log.topics.len() >= 3 {
                            println!("      From: 0x{:064x}", log.topics[1]);
                            println!("      To: 0x{:064x}", log.topics[2]);
                        }
                    }
                }
            }
        }
    } else {
        println!("Transaction not found. Using sample data for demonstration...");
        
        // Demonstrate with mock data
        println!("\nSample transaction structure:");
        println!("  Hash: 0x...");
        println!("  Block: #18000000");
        println!("  From: 0xSender");
        println!("  To: 0xReceiver");
        println!("  Value: 1.5 ETH");
        println!("  Status: Success");
    }
    
    println!();
    
    // === Build CallRequest from Transaction ===
    println!("2. Rebuild Transaction for Simulation");
    println!("-" .repeat(60));
    
    if provider.transaction_exists(tx_hash).await? {
        let call_request = provider.build_call_request(tx_hash).await?;
        
        println!("CallRequest built from transaction:");
        println!("  From: {:?}", call_request.from);
        println!("  To: {:?}", call_request.to);
        println!("  Value: {:?}", call_request.value);
        println!("  Data length: {} bytes", 
            call_request.data.as_ref().map(|d| d.len()).unwrap_or(0)
        );
        println!("  Gas: {:?}", call_request.gas);
        
        println!("\nThis CallRequest can be used to:");
        println!("  - Re-simulate the transaction");
        println!("  - Test with different parameters");
        println!("  - Debug transaction behavior");
    }
    
    println!();
    
    // === Transaction with Traces ===
    println!("3. Transaction with Traces (if RPC configured)");
    println!("-" .repeat(60));
    
    // This requires RPC endpoint to be configured
    match provider.get_transaction_with_trace(tx_hash).await {
        Ok(tx_with_trace) => {
            println!("Transaction with trace data:");
            println!("  Main call type: {:?}", 
                tx_with_trace.trace.as_ref().map(|t| &t.call_type)
            );
            
            if let Some(trace) = tx_with_trace.trace {
                let subcall_count = trace.call_frame.subcalls.len();
                println!("  Subcalls: {}", subcall_count);
                
                if subcall_count > 0 {
                    println!("\n  Internal calls:");
                    for (i, subcall) in trace.call_frame.subcalls.iter().enumerate() {
                        println!("    Call #{}: {} -> {}", 
                            i + 1,
                            subcall.from.map(|a| format!("0x{}", a))
                                .unwrap_or("N/A".to_string()),
                            subcall.to.map(|a| format!("0x{}", a))
                                .unwrap_or("N/A".to_string())
                        );
                    }
                }
            }
        }
        Err(_) => {
            println!("Trace data not available (RPC endpoint not configured)");
            println!("To enable traces, initialize provider with:");
            println!("  .with_rpc_endpoint(\"http://localhost:8545\")?");
        }
    }
    
    println!();
    
    // === Batch Transaction Queries ===
    println!("4. Batch Transaction Queries");
    println!("-" .repeat(60));
    
    // Query multiple transactions at once
    let tx_hashes = vec![
        B256::from_str("0x1111111111111111111111111111111111111111111111111111111111111111")?,
        B256::from_str("0x2222222222222222222222222222222222222222222222222222222222222222")?,
        B256::from_str("0x3333333333333333333333333333333333333333333333333333333333333333")?,
    ];
    
    println!("Checking {} transactions...", tx_hashes.len());
    
    let mut found_count = 0;
    for hash in &tx_hashes {
        if provider.transaction_exists(*hash).await? {
            found_count += 1;
            println!("  ✓ Transaction 0x{} found", hash);
        } else {
            println!("  ✗ Transaction 0x{} not found", hash);
        }
    }
    
    println!("\nFound {}/{} transactions", found_count, tx_hashes.len());
    
    println!("\n✅ Transaction queries complete!");
    
    Ok(())
}