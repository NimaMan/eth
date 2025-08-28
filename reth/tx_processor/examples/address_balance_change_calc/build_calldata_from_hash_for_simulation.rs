/// Debug CallRequest Creation
/// 
/// This example tests the CallRequest creation step by step to identify where
/// the balance change calculation is failing.

use tx_processor::TxProcessor;
use alloy_primitives::B256;
use eyre::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Debug CallRequest Creation");
    println!("=============================\n");
    
    // Initialize processor
    let processor = TxProcessor::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Test transaction that should have balance changes but shows 0
    let tx_hash = B256::from_str("0x6c85e9d68a79ad31ec397d50e53f7736f2e4e02d3594d5165fe0a4b534cc0c4d")?;
    
    println!("Testing transaction: {}", tx_hash);
    println!("Expected from Etherscan:");
    println!("  Block: 23209434");
    println!("  From: 0x84200f7ffF124d4218d02098E1874AA1e9562a12");
    println!("  To: 0x1fFF6b56aaE528daB78d6D99F8F98a2B9E6dC0E1");
    println!("  Value: 0 ETH");
    println!("  Internal: 1 ETH transfer");
    println!("  ERC-20: 4 token transfers\n");
    
    // Step 1: Test raw database data loading
    println!("📊 Step 1: Loading Raw Transaction Data from Database");
    println!("{}", "=".repeat(60));
    
    // We need to access the transaction loader directly
    // Let's check if we can get transaction data first
    match processor.fetch_historical_transaction_as_calldata(tx_hash).await {
        Ok(call_request) => {
            println!("✅ CallRequest created successfully!");
            println!("\n📋 CallRequest Details:");
            println!("  From: {:?}", call_request.from);
            println!("  To: {:?}", call_request.to);
            println!("  Value: {:?}", call_request.value);
            println!("  Gas: {:?}", call_request.gas);
            println!("  Gas Price: {:?}", call_request.gas_price);
            println!("  Max Fee Per Gas: {:?}", call_request.max_fee_per_gas);
            println!("  Max Priority Fee Per Gas: {:?}", call_request.max_priority_fee_per_gas);
            println!("  Nonce: {:?}", call_request.nonce);
            
            // Show input data
            if let Some(data) = &call_request.data {
                println!("  Input Data Length: {} bytes", data.len());
                println!("  Input Data (first 32 bytes): 0x{}", hex::encode(&data[..std::cmp::min(32, data.len())]));
                if data.len() > 32 {
                    println!("  Input Data (truncated): ... ({} more bytes)", data.len() - 32);
                }
            } else {
                println!("  Input Data: None");
            }
            
            // Validate against expected values
            println!("\n🔍 Validation:");
            
            let expected_from = alloy_primitives::Address::from_str("0x84200f7ffF124d4218d02098E1874AA1e9562a12")?;
            let expected_to = alloy_primitives::Address::from_str("0x1fFF6b56aaE528daB78d6D99F8F98a2B9E6dC0E1")?;
            
            if call_request.from == Some(expected_from) {
                println!("  ✅ From address matches Etherscan");
            } else {
                println!("  ❌ From address mismatch! Expected: {}, Got: {:?}", expected_from, call_request.from);
            }
            
            if call_request.to == Some(expected_to) {
                println!("  ✅ To address matches Etherscan");
            } else {
                println!("  ❌ To address mismatch! Expected: {}, Got: {:?}", expected_to, call_request.to);
            }
            
            if call_request.value == Some(alloy_primitives::U256::ZERO) {
                println!("  ✅ Value matches Etherscan (0 ETH)");
            } else {
                println!("  ❌ Value mismatch! Expected: 0, Got: {:?}", call_request.value);
            }
            
            if call_request.nonce.is_none() {
                println!("  ✅ Nonce is None (correct for simulation)");
            } else {
                println!("  ❌ Nonce should be None for simulation, got: {:?}", call_request.nonce);
            }
            
            // Now test the simulation with this CallRequest
            println!("\n📊 Step 2: Testing Simulation with CallRequest");
            println!("{}", "=".repeat(60));
            
            // Simulate at block N-1 (23209433 if tx was in 23209434)
            let simulation_block = 23209433; // One block before the mined transaction
            
            match processor.simulate_transaction_from_calldata_with_balance_changes(call_request, Some(simulation_block)).await {
                Ok(processed_tx) => {
                    println!("✅ Simulation completed successfully!");
                    println!("\n📋 Simulation Results:");
                    println!("  Block Number: {}", processed_tx.block_number);
                    println!("  Transaction Type: {}", processed_tx.txn_type);
                    println!("  Status: {}", processed_tx.status);
                    println!("  Gas Used: {}", processed_tx.fees.gas_used);
                    println!("  ERC20 Transfers: {}", processed_tx.erc20_transfers.len());
                    println!("  Internal Transactions: {}", processed_tx.internal_transactions.len());
                    println!("  Address Balance Changes: {}", processed_tx.address_balance_changes.len());
                    
                    // This is the key test - do we have balance changes?
                    if processed_tx.address_balance_changes.len() > 0 {
                        println!("\n✅ ADDRESS BALANCE CHANGES FOUND!");
                        println!("Balance changes for {} addresses:", processed_tx.address_balance_changes.len());
                        
                        for (addr, changes) in &processed_tx.address_balance_changes {
                            println!("\n  Address: {}", addr);
                            println!("  Changes: {}", serde_json::to_string_pretty(changes)?);
                        }
                    } else {
                        println!("\n❌ NO ADDRESS BALANCE CHANGES FOUND!");
                        println!("This indicates the simulation is not calculating balance changes properly.");
                        
                        // Let's check other transaction data that should exist
                        if processed_tx.erc20_transfers.len() > 0 {
                            println!("  But we do have {} ERC20 transfers - simulation is working partially", processed_tx.erc20_transfers.len());
                        }
                        
                        if processed_tx.internal_transactions.len() > 0 {
                            println!("  But we do have {} internal transactions - simulation is working partially", processed_tx.internal_transactions.len());
                        }
                        
                        println!("  This suggests balance change calculation is broken in the simulation");
                    }
                }
                Err(e) => {
                    println!("❌ Simulation failed: {}", e);
                    println!("This means the CallRequest is invalid or the simulation has issues");
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create CallRequest: {}", e);
            println!("This indicates the transaction data loading from database is broken");
        }
    }
    
    Ok(())
}