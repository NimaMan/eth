/// Example: Test moo token from specific transaction
/// 
/// Tests the moo token that was successfully swapped in transaction:
/// 0xea83d46948ebd529ad26370a5f46341a6a84f3c2f1b07caf7cf94aa232283115
/// at block 23196488
///
/// This example simulates buy/approve/sell at the same block where we know
/// the token was tradeable to verify our simulation matches reality.

use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, U256};
use tx_processor::{TxProcessor, chain_query::ChainQuery};
use tx_processor::erc20_token_trading_viability::{
    analyze_pool_viability,
    PoolViabilityConfig,
    PoolType,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("moo Token Trading Viability Analysis");
    println!("=====================================");
    println!("Reference TX: 0xea83d46948ebd529ad26370a5f46341a6a84f3c2f1b07caf7cf94aa232283115");
    println!("Block: 23196488");
    println!("Original swap: 0.1 ETH -> 575,446,178 moo tokens");
    println!();
    
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    // Create tx processor (contains shared chain_query)
    let tx_processor = Arc::new(TxProcessor::new(&reth_datadir)?);
    
    // Use shared chain_query from tx_processor (no separate DB connection)
    let chain_query = tx_processor.chain_query.clone();
    let simulator = chain_query.get_simulator();
    
    // moo token details from the transaction
    let moo_token_address: Address = "0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2".parse()?;
    let moo_pool_address: Address = "0xFc099D07b32D52D61d2f5Dd6De2614d26474eCf7".parse()?; // V2 pool
    
    // Configure for V2 at the specific block
    let config = PoolViabilityConfig::new(
        moo_token_address,
        moo_pool_address,
        PoolType::UniswapV2,
    )
    .with_test_amount(U256::from(100_000_000_000_000_000u64)) // 0.1 ETH (same as original tx)
    .with_block(23196488); // Test at exact block of known good transaction
    
    println!("Configuration:");
    println!("  Token: moo (0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2)");
    println!("  Pool: Uniswap V2 moo/WETH (0xFc099D07b32D52D61d2f5Dd6De2614d26474eCf7)");
    println!("  Type: UniswapV2");
    println!("  Test Amount: 0.1 ETH");
    println!("  Block: 23196488 (same as reference transaction)");
    println!();
    
    println!("Analyzing moo token at block 23196488...");
    
    match analyze_pool_viability(simulator, tx_processor, config).await {
        Ok(result) => {
            println!("\nAnalysis Results:");
            println!("=================");
            println!("Block Number: {}", result.block_number);
            println!("Is Tradeable: {}", result.is_tradeable);
            
            if result.is_tradeable {
                println!("\n✅ Pool is tradeable!");
                println!("  Buy Tax: {:.2}%", result.buy_tax_percent);
                println!("  Sell Tax: {:.2}%", result.sell_tax_percent);
                println!("  Tokens Received: {}", result.tokens_received);
                println!("  ETH Spent: {} wei", result.eth_spent);
                println!("  ETH Received: {} wei", result.eth_received);
                
                // Compare with original transaction
                let original_tokens = U256::from(575_446_178_536_175_301u128);
                let simulated_tokens = result.tokens_received;
                let difference_pct = if original_tokens > U256::ZERO {
                    let diff = if original_tokens > simulated_tokens {
                        original_tokens - simulated_tokens
                    } else {
                        simulated_tokens - original_tokens
                    };
                    (diff.to::<u128>() as f64 / original_tokens.to::<u128>() as f64) * 100.0
                } else {
                    0.0
                };
                
                println!("\n📊 Comparison with original transaction:");
                println!("  Original tokens received: {}", original_tokens);
                println!("  Simulated tokens received: {}", simulated_tokens);
                println!("  Difference: {:.2}%", difference_pct);
                
                if difference_pct < 1.0 {
                    println!("  ✅ Simulation closely matches actual transaction!");
                } else {
                    println!("  ⚠️ Some difference detected (could be due to MEV, slippage, or fees)");
                }
            } else {
                println!("\n❌ Pool is not tradeable");
                if let Some(reason) = &result.failure_reason {
                    println!("  Reason: {}", reason);
                }
            }
            
            // Debug info
            println!("\nTransaction Status:");
            println!("  Buy: {} (gas: {})", 
                if result.buy_transaction.status == "1" { "Success" } else { "Failed" },
                result.buy_transaction.fees.gas_used
            );
            println!("  Approve: {} (gas: {})", 
                if result.approve_transaction.status == "1" { "Success" } else { "Failed" },
                result.approve_transaction.fees.gas_used
            );
            println!("  Sell: {} (gas: {})", 
                if result.sell_transaction.status == "1" { "Success" } else { "Failed" },
                result.sell_transaction.fees.gas_used
            );
        }
        Err(e) => {
            println!("Analysis failed: {}", e);
        }
    }
    
    Ok(())
}