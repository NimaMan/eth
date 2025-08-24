/// Test USDC on Uniswap V2
/// 
/// This tests USDC/WETH trading on Uniswap V2 to verify buy/sell works

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
    println!("USDC/WETH Uniswap V2 Pool Analysis");
    println!("===================================");
    
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    // Create tx processor (contains shared chain_query)
    let tx_processor = Arc::new(TxProcessor::new(&reth_datadir)?);
    
    // Use shared chain_query from tx_processor (no separate DB connection)
    let chain_query = tx_processor.chain_query.clone();
    let simulator = chain_query.get_simulator();
    
    // USDC address
    let usdc_address: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
    
    // USDC/WETH V2 pool address
    let v2_pool_address: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".parse()?;
    
    // Configure for V2
    let config = PoolViabilityConfig::new(
        usdc_address,
        v2_pool_address,
        PoolType::UniswapV2,
    )
    .with_test_amount(U256::from(10_000_000_000_000_000u64)); // 0.01 ETH
    
    println!("Configuration:");
    println!("  Token: USDC (0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48)");
    println!("  Pool: Uniswap V2 USDC/WETH (0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc)");
    println!("  Type: UniswapV2");
    println!("  Test Amount: 0.01 ETH");
    println!();
    
    println!("Analyzing V2 pool...");
    
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