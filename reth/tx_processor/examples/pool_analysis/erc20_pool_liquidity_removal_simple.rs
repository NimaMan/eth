/// Simple Example: Detect Liquidity Removal from ERC20 Pools
/// 
/// Shows how to check if a pool lost liquidity after a transaction

use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, B256};
use tx_processor::processed_tx_provider::ProcessedTxProvider;

#[tokio::main]
async fn main() -> Result<()> {
    println!("ERC20 Pool Liquidity Removal Detection");
    println!("======================================\n");
    
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    let provider = Arc::new(ProcessedTxProvider::new(&reth_datadir)?);
    
    // Example: Liquidity removal transaction hash (edit to test another TX)
    let removal_tx: B256 = "0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966".parse()?;
    
    println!("Analyzing transaction for liquidity removal...");
    println!("TX: {}\n", removal_tx);
    
    // Process the transaction
    match provider.process_transaction_by_hash(removal_tx).await {
        Ok(tx) => {
            println!("Transaction details:");
            println!("  Block: {}", tx.block_number);
            println!("  From: {}", tx.from_address);
            println!("  To: {:?}", tx.to_address);
            println!("  Status: {}", if tx.status == "1" { "Success" } else { "Failed" });
            
            // Check for liquidity removal indicators
            println!("\nLiquidity removal indicators:");
            
            // Check function selector for common liquidity removal methods
            if tx.input.len() >= 4 {
                let selector = &tx.input[0..4];
                let is_remove = match selector {
                    [0x02, 0x75, 0x1c, 0xec] => Some("removeLiquidity"),
                    [0xba, 0xa2, 0xab, 0xde] => Some("removeLiquidityETH"),
                    [0x2e, 0x6d, 0x6e, 0xbf] => Some("removeLiquidityETHWithPermit"),
                    [0x5b, 0x0d, 0x59, 0x84] => Some("removeLiquidityWithPermit"),
                    _ => None,
                };
                
                if let Some(method) = is_remove {
                    println!("  🚨 LIQUIDITY REMOVAL DETECTED!");
                    println!("  Method: {}", method);
                } else {
                    println!("  Function selector: 0x{}", hex::encode(selector));
                }
            }
            
            // Check events
            println!("\nEvent summary:");
            println!("  ERC20 transfers: {}", tx.erc20_transfers.len());
            println!("  Internal transactions: {}", tx.internal_transactions.len());
            println!("  Uniswap V2 events: {}", tx.uniswap_v2_swaps.len());
            
            // Look for large token movements
            if !tx.erc20_transfers.is_empty() {
                println!("\nToken movements detected:");
                for (i, transfer) in tx.erc20_transfers.iter().take(5).enumerate() {
                    println!("  Transfer {}: {} -> {}", 
                        i + 1,
                        transfer.from_address,
                        transfer.to_address
                    );
                    println!("    Amount: {}", transfer.amount);
                }
                if tx.erc20_transfers.len() > 5 {
                    println!("  ... and {} more transfers", tx.erc20_transfers.len() - 5);
                }
            }
            
            println!("\n⚠️  Note: For complete liquidity analysis, check:");
            println!("  1. Pool reserves before and after");
            println!("  2. LP token burns");
            println!("  3. Trading volume changes");
        }
        Err(e) => {
            println!("Failed to process transaction: {}", e);
        }
    }
    
    Ok(())
}
