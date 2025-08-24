/// Example: Analyze Uniswap V3 Pool
/// 
/// This example shows how to analyze a Uniswap V3 pool with fee tiers

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
    println!("Uniswap V3 Pool Trading Viability Analysis");
    println!("==========================================");
    
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    // Create tx processor (contains shared chain_query)
    let tx_processor = Arc::new(TxProcessor::new(&reth_datadir)?);
    
    // Use shared chain_query from tx_processor (no separate DB connection)
    let chain_query = tx_processor.chain_query.clone();
    let simulator = chain_query.get_simulator();
    
    // Example: USDC/WETH 0.05% pool on Uniswap V3
    let usdc_address: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
    let pool_address: Address = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".parse()?; // V3 pool
    
    // V3 pools have fee tiers (500 = 0.05%, 3000 = 0.30%, 10000 = 1%)
    let config = PoolViabilityConfig::new(
        usdc_address,
        pool_address,
        PoolType::UniswapV3 { fee_tier: 500 }, // 0.05% fee tier
    )
    .with_test_amount(U256::from(100_000_000_000_000_000u64)); // 0.1 ETH
    
    println!("Configuration:");
    println!("  Token: USDC");
    println!("  Pool: Uniswap V3 USDC/WETH 0.05%");
    println!("  Fee Tier: 0.05%");
    println!("  Test Amount: 0.1 ETH");
    println!();
    
    println!("Analyzing V3 pool...");
    
    match analyze_pool_viability(simulator, tx_processor, config).await {
        Ok(result) => {
            println!("\nAnalysis Results:");
            println!("=================");
            
            if result.is_tradeable {
                println!("✓ Pool is tradeable");
                println!();
                println!("Swap Costs:");
                println!("  Buy Impact: {:.3}%", result.buy_tax_percent);
                println!("  Sell Impact: {:.3}%", result.sell_tax_percent);
                println!("  Total Round-trip Cost: {:.3}%", 
                    result.buy_tax_percent + result.sell_tax_percent);
                
                println!();
                println!("Token Flow:");
                println!("  USDC Received: {}", result.tokens_received);
                println!("  ETH Recovered: {} wei", result.eth_received);
                
                // Calculate slippage
                let expected_loss = result.eth_spent * U256::from(10) / U256::from(10000); // 0.1% expected
                let actual_loss = result.eth_spent.saturating_sub(result.eth_received);
                
                println!();
                println!("Slippage Analysis:");
                println!("  Expected Loss (0.1% fees): {} wei", expected_loss);
                println!("  Actual Loss: {} wei", actual_loss);
                
                if actual_loss > expected_loss {
                    println!("  ⚠ Higher than expected - likely price impact");
                } else {
                    println!("  ✓ Within expected range");
                }
            } else {
                println!("✗ Pool is not tradeable");
                if let Some(reason) = result.failure_reason {
                    println!("  Reason: {}", reason);
                }
            }
        }
        Err(e) => {
            println!("Analysis failed: {}", e);
            println!("Note: V3 pools require concentrated liquidity at current price");
        }
    }
    
    Ok(())
}