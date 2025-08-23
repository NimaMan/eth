/// Test the trading viability module directly from tx_processor
/// 
/// This verifies that the module works correctly in its new location

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
    println!("Testing ERC20 Token Trading Viability from tx_processor");
    println!("========================================================");
    
    // Get reth datadir
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    // Create chain query and get simulator
    let chain_query = Arc::new(ChainQuery::new(&reth_datadir)?);
    let simulator = chain_query.get_simulator();
    
    // Create tx processor
    let tx_processor = Arc::new(TxProcessor::new(&reth_datadir)?);
    
    // Example: PEPE token and its Uniswap V2 pool
    let token_address: Address = "0x6982508145454Ce325dDbE47a25d4ec3d2311933".parse()?;
    let pool_address: Address = "0xA43fe16908251ee70EF74718545e4FE6C5cCEc9f".parse()?;
    
    // Create configuration
    let config = PoolViabilityConfig::new(
        token_address,
        pool_address,
        PoolType::UniswapV2,
    )
    .with_test_amount(U256::from(10_000_000_000_000_000u64)); // 0.01 ETH
    
    println!("Configuration:");
    println!("  Token: {:?}", token_address);
    println!("  Pool: {:?}", pool_address);
    println!("  Type: {:?}", config.pool_type);
    println!("  Test Amount: {} wei (0.01 ETH)", config.test_amount);
    println!();
    
    // Run the analysis
    println!("Running pool viability analysis...");
    match analyze_pool_viability(simulator, tx_processor, config).await {
        Ok(result) => {
            println!("\nAnalysis Results:");
            println!("=================");
            println!("Is Tradeable: {}", result.is_tradeable);
            println!("Block Number: {}", result.block_number);
            
            if result.is_tradeable {
                println!("\nTax Analysis:");
                println!("  Buy Tax: {:.2}%", result.buy_tax_percent);
                println!("  Sell Tax: {:.2}%", result.sell_tax_percent);
                
                println!("\nTrade Details:");
                println!("  Tokens Received: {}", result.tokens_received);
                println!("  ETH Received Back: {} wei", result.eth_received);
                
                let loss = result.eth_spent.saturating_sub(result.eth_received);
                println!("  Net Loss: {} wei", loss);
            } else {
                println!("\nTrading failed!");
                if let Some(reason) = &result.failure_reason {
                    println!("Reason: {}", reason);
                }
            }
        }
        Err(e) => {
            println!("Analysis failed: {}", e);
        }
    }
    
    Ok(())
}