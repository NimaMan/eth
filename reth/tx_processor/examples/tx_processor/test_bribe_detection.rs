/// Test that bribes to known fee recipients (validators/builders) are properly detected
/// 
/// This example tests that ETH transfers to known MEV builders and validators
/// are properly identified as bribes by the address balance change calculator.

use tx_processor::processed_tx_provider::ProcessedTxProvider;
use alloy_primitives::{B256, U256};
use eyre::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Testing Bribe Detection with FEE_RECIPIENTS");
    println!("==============================================\n");
    
    // Initialize the ProcessedTxProvider
    let provider = ProcessedTxProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Test with the specified transaction
    let tx_hash = B256::from_str("0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e")?;
    
    println!("Processing transaction: {}", tx_hash);
    println!("Looking for potential bribe payments...\n");
    
    match provider.process_transaction_by_hash(tx_hash).await {
        Ok(tx) => {
            // Check internal transactions for ETH transfers to fee recipients
            let mut bribe_count = 0;
            
            println!("Transaction Details:");
            println!("  From: {}", tx.from_address);
            if let Some(to) = tx.to_address {
                println!("  To: {}", to);
            }
            println!("  Value: {} wei", tx.value);
            println!("  Block: {}", tx.block_number);
            println!("\n");
            
            // Check internal transactions
            println!("Internal Transactions: {} found", tx.internal_transactions.len());
            for internal in &tx.internal_transactions {
                // Check if this is an ETH transfer (non-zero value)
                if internal.value > alloy_primitives::U256::ZERO {
                    // Check if recipient is a known fee recipient
                    if reth_chain_query::FEE_RECIPIENTS.contains(&internal.to_address) {
                        println!("✅ BRIBE DETECTED!");
                        println!("  From: {}", internal.from_address);
                        println!("  To: {} (known fee recipient)", internal.to_address);
                        println!("  Value: {} wei", internal.value);
                        println!("  Type: {}", internal.trace_type);
                        
                        // Get the name of the fee recipient if available
                        if let Some(name) = reth_chain_query::common_addresses::validators::get_fee_recipient_name(internal.to_address) {
                            println!("  Recipient Name: {}", name);
                        }
                        println!();
                        bribe_count += 1;
                    } else if internal.value > U256::from(0) && internal.depth == 0 {
                        // Show significant ETH transfers even if not to fee recipients
                        println!("ETH Transfer (depth {}): {} → {}: {} wei", 
                            internal.depth,
                            internal.from_address, 
                            internal.to_address, 
                            internal.value
                        );
                    }
                }
            }
            
            // Check address balance changes for bribe detection
            println!("\nAddress Balance Changes Analysis:");
            println!("---------------------------------");
            
            let mut fee_recipient_found = false;
            for (address, changes) in &tx.address_balance_changes {
                // Check if this address is a fee recipient
                if reth_chain_query::FEE_RECIPIENTS.contains(address) {
                    fee_recipient_found = true;
                    println!("\n🎯 Fee Recipient Address Found: {}", address);
                    
                    if let Some(name) = reth_chain_query::common_addresses::validators::get_fee_recipient_name(*address) {
                        println!("  Name: {}", name);
                    }
                    
                    // Check currency_net for ETH changes
                    if let Some(eth_change) = changes.currency_net.get("ETH") {
                        if *eth_change > U256::ZERO {
                            // Convert from wei to ETH for display
                            let eth_val = eth_change.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                            println!("  ✅ Received ETH bribe: {} ETH", eth_val);
                            bribe_count += 1;
                        }
                    }
                } else {
                    // Show addresses with significant ETH changes
                    if let Some(eth_change) = changes.currency_net.get("ETH") {
                        // Convert from wei to ETH for display and filter by magnitude
                        let eth_val = eth_change.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                        if eth_val.abs() > 0.001 {
                            println!("Address {}: ETH change: {:+.6} ETH", address, eth_val);
                        }
                    }
                }
            }
            
            if !fee_recipient_found {
                println!("No fee recipient addresses found in balance changes.");
            }
            
            println!("\n📊 Summary:");
            println!("  Transaction Hash: {}", tx.hash);
            println!("  Total internal transactions: {}", tx.internal_transactions.len());
            println!("  Total addresses with balance changes: {}", tx.address_balance_changes.len());
            println!("  Bribe payments detected: {}", bribe_count);
            
            if bribe_count == 0 {
                println!("\n  ℹ️ No bribes detected in this transaction.");
                println!("  This appears to be a regular transaction without payments to validators/builders.");
            }
            
            // Show stats about fee recipients
            println!("\n📈 Fee Recipients Database:");
            let stats = reth_chain_query::common_addresses::validators::fee_recipient_stats();
            for (category, count) in stats {
                println!("  {}: {}", category, count);
            }
        }
        Err(e) => {
            println!("❌ Error processing transaction: {}", e);
        }
    }
    
    Ok(())
}
