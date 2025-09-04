/// Example: Analyze token trading viability across a block range
/// 
/// This example tests a token's trading viability at multiple blocks
/// to detect when trading was enabled/disabled or tax changes occurred.
///
/// Useful for:
/// - Finding when a token became tradeable
/// - Detecting tax changes over time
/// - Analyzing liquidity changes
/// - Historical analysis of token behavior

use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, U256};
use tx_processor::{TxProcessor, chain_query::ChainQuery};
use tx_processor::erc20_token_trading_viability::{
    check_can_buy_sell_pool,
    PoolViabilityConfig,
    PoolType,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Block Range Token Trading Analysis");
    println!("===================================");
    
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    // Create tx processor (contains shared chain_query)
    let tx_processor = Arc::new(TxProcessor::new(&reth_datadir)?);
    
    // Use shared chain_query from tx_processor (no separate DB connection)
    let chain_query = tx_processor.chain_query.clone();
    let simulator = chain_query.get_simulator();
    
    // Configure token to analyze - using moo token as example
    let token_address: Address = "0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2".parse()?; // moo token
    let pool_address: Address = "0xFc099D07b32D52D61d2f5Dd6De2614d26474eCf7".parse()?; // moo/WETH V2 pool
    
    // Define block range to analyze
    // Analyzing blocks around the liquidity addition at 23196199
    // Starting 10 blocks before liquidity addition (23196189) to after first swap (23196499)
    let start_block = 23196189; // 10 blocks before liquidity addition
    let end_block = 23196499;   // 10 blocks after the successful swap
    let block_step = 10; // Test every 10th block to cover the range efficiently
    
    println!("Configuration:");
    println!("  Token: moo (0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2)");
    println!("  Pool: Uniswap V2 moo/WETH");
    println!("  Block Range: {} to {}", start_block, end_block);
    println!("  Key Events:");
    println!("    - Block 23196199: Liquidity addition (1 ETH + 89B moo tokens)");
    println!("    - Block 23196488: Successful swap (0.1 ETH -> 575M moo tokens)");
    println!("  Step: Every {} blocks", block_step);
    println!("  Test Amount: 0.1 ETH");
    println!();
    
    println!("Starting block range analysis...\n");
    println!("{:<10} {:<12} {:<10} {:<10} {:<20} {:<20}", 
        "Block", "Tradeable", "Buy Tax", "Sell Tax", "Tokens Received", "ETH Received");
    println!("{}", "=".repeat(92));
    
    let mut results = Vec::new();
    let mut trading_enabled_block = None;
    let mut trading_disabled_block = None;
    let mut tax_changes = Vec::new();
    let mut last_buy_tax: Option<f64> = None;
    let mut last_sell_tax: Option<f64> = None;
    
    for block_number in (start_block..=end_block).step_by(block_step) {
        // Configure for this specific block
        let config = PoolViabilityConfig::new(
            token_address,
            pool_address,
            PoolType::UniswapV2,
        )
        .with_test_amount(U256::from(100_000_000_000_000_000u64)) // 0.1 ETH
        .with_block(block_number);
        
        match check_can_buy_sell_pool(simulator.clone(), tx_processor.clone(), config).await {
            Ok(result) => {
                let status = if result.is_tradeable { "✅ Yes" } else { "❌ No" };
                let buy_tax = if result.is_tradeable { 
                    format!("{:.2}%", result.buy_tax_percent) 
                } else { 
                    "-".to_string() 
                };
                let sell_tax = if result.is_tradeable { 
                    format!("{:.2}%", result.sell_tax_percent) 
                } else { 
                    "-".to_string() 
                };
                let tokens = if result.is_tradeable {
                    format!("{}", result.tokens_received)
                } else {
                    "-".to_string()
                };
                let eth_received = if result.is_tradeable {
                    format!("{:.6} ETH", result.eth_received.to::<u128>() as f64 / 1e18)
                } else {
                    "-".to_string()
                };
                
                println!("{:<10} {:<12} {:<10} {:<10} {:<20} {:<20}", 
                    block_number, status, buy_tax, sell_tax, tokens, eth_received);
                
                // Log failure details for non-tradeable tokens
                if !result.is_tradeable {
                    if let Some(reason) = &result.failure_reason {
                        println!("           🔍 Failure reason: {}", reason);
                    }
                    
                    // Log individual transaction statuses for debugging
                    let buy_status = if result.buy_transaction.status == "1" { "✅" } else { "❌" };
                    let approve_status = if result.approve_transaction.status == "1" { "✅" } else { "❌" };
                    let sell_status = if result.sell_transaction.status == "1" { "✅" } else { "❌" };
                    
                    println!("           📊 Transaction status: Buy {} | Approve {} | Sell {}", 
                        buy_status, approve_status, sell_status);
                }
                
                // Track state changes
                if result.is_tradeable {
                    if trading_enabled_block.is_none() {
                        trading_enabled_block = Some(block_number);
                    }
                    
                    // Check for tax changes
                    if let Some(last_buy) = last_buy_tax {
                        if (result.buy_tax_percent - last_buy).abs() > 0.01_f64 {
                            tax_changes.push((block_number, "buy", last_buy, result.buy_tax_percent));
                        }
                    }
                    if let Some(last_sell) = last_sell_tax {
                        if (result.sell_tax_percent - last_sell).abs() > 0.01_f64 {
                            tax_changes.push((block_number, "sell", last_sell, result.sell_tax_percent));
                        }
                    }
                    
                    last_buy_tax = Some(result.buy_tax_percent);
                    last_sell_tax = Some(result.sell_tax_percent);
                } else if trading_enabled_block.is_some() && trading_disabled_block.is_none() {
                    trading_disabled_block = Some(block_number);
                }
                
                results.push((block_number, result));
            }
            Err(e) => {
                println!("{:<10} ❌ Error: {}", block_number, e);
            }
        }
    }
    
    // Summary
    println!("\n📊 Analysis Summary:");
    println!("===================");
    
    let tradeable_count = results.iter().filter(|(_, r)| r.is_tradeable).count();
    let total_count = results.len();
    
    println!("  Total blocks analyzed: {}", total_count);
    println!("  Tradeable blocks: {} ({:.1}%)", 
        tradeable_count, 
        (tradeable_count as f64 / total_count as f64) * 100.0
    );
    
    if let Some(block) = trading_enabled_block {
        println!("  Trading first detected at block: {}", block);
    }
    
    if let Some(block) = trading_disabled_block {
        println!("  Trading disabled at block: {}", block);
    }
    
    if !tax_changes.is_empty() {
        println!("\n📈 Tax Changes Detected:");
        for (block, tax_type, old_tax, new_tax) in tax_changes {
            println!("  Block {}: {} tax changed from {:.2}% to {:.2}%", 
                block, tax_type, old_tax, new_tax);
        }
    }
    
    // Find best trading block (lowest combined tax)
    if tradeable_count > 0 {
        let best_block = results.iter()
            .filter(|(_, r)| r.is_tradeable)
            .min_by(|(_, a), (_, b)| {
                let a_total = a.buy_tax_percent + a.sell_tax_percent;
                let b_total = b.buy_tax_percent + b.sell_tax_percent;
                a_total.partial_cmp(&b_total).unwrap()
            });
        
        if let Some((block, result)) = best_block {
            println!("\n🏆 Best Trading Block: {}", block);
            println!("  Combined tax: {:.2}%", result.buy_tax_percent + result.sell_tax_percent);
            println!("  Buy tax: {:.2}%, Sell tax: {:.2}%", 
                result.buy_tax_percent, result.sell_tax_percent);
        }
    }
    
    Ok(())
}