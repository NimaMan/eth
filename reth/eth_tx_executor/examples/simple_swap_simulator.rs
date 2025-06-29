//! Simple Swap Simulator - Working version
//! 
//! Demonstrates swap simulation without real transactions

use ethers::prelude::*;
use std::sync::Arc;

// Token addresses
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

// Uniswap V2 WETH/USDC pool
const WETH_USDC_POOL: &str = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple ETH → USDC Swap Simulator ===\n");
    
    // Connect to local node
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    let provider = Arc::new(provider);
    
    // Amount to simulate: 0.05 ETH
    let eth_amount = ethers::utils::parse_ether("0.05")?;
    println!("Simulating swap of 0.05 ETH to USDC...\n");
    
    // Get current block for context
    let block = provider.get_block_number().await?;
    println!("Current block: {}", block);
    
    // Simulate calling getAmountOut on pool
    // In real implementation, this would use the pool contract
    println!("\n📊 Swap Simulation Results:");
    println!("═══════════════════════════");
    
    // Approximate calculation (for demo)
    let eth_price = 2500.0; // Approximate ETH price in USD
    let expected_usdc = 0.05 * eth_price;
    
    println!("Input:    0.05 ETH");
    println!("Output:   ~{:.2} USDC", expected_usdc);
    println!("Pool:     Uniswap V2 WETH/USDC");
    println!("Address:  {}", WETH_USDC_POOL);
    
    // Gas estimation
    println!("\n⛽ Gas Estimation:");
    println!("══════════════════");
    let gas_price = provider.get_gas_price().await?;
    let gas_limit = U256::from(200_000); // Typical swap gas
    let gas_cost = gas_price * gas_limit;
    
    println!("Gas Price: {} gwei", format_gwei(gas_price));
    println!("Gas Limit: {}", gas_limit);
    println!("Total Cost: {} ETH", format_ether(gas_cost));
    
    // Slippage calculation
    println!("\n💧 Slippage Protection:");
    println!("═══════════════════════");
    let slippage = 0.01; // 1%
    let min_output = expected_usdc * (1.0 - slippage);
    println!("Slippage:    1%");
    println!("Min Output:  {:.2} USDC", min_output);
    
    // Summary
    println!("\n✅ SIMULATION COMPLETE");
    println!("══════════════════════");
    println!("This was a simulation only - no transaction was sent!");
    println!("No gas was spent, no funds were moved.");
    
    // Show what would happen in real execution
    println!("\n📝 What would happen in real execution:");
    println!("1. Check ETH balance >= 0.05 ETH");
    println!("2. Approve router if needed");
    println!("3. Call swapExactETHForTokens on Uniswap Router");
    println!("4. Receive ~{:.2} USDC (minus slippage)", expected_usdc);
    println!("5. Pay ~{} ETH in gas fees", format_ether(gas_cost));
    
    Ok(())
}

// Helper to format wei as gwei
fn format_gwei(wei: U256) -> String {
    let gwei = wei.as_u128() as f64 / 1e9;
    format!("{:.2}", gwei)
}

// Helper to format wei as ether
fn format_ether(wei: U256) -> String {
    let ether = wei.as_u128() as f64 / 1e18;
    format!("{:.6}", ether)
}