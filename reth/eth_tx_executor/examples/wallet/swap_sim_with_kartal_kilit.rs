//! Swap Simulator using KARTAL_KILIT directly
//!
//! This example demonstrates swap simulation using the private key from environment

use eth_kartal::pools::PoolFactory;
use ethers::prelude::*;
use ethers::signers::Signer;
use std::sync::Arc;

// Token addresses on Ethereum mainnet
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Swap Simulator with KARTAL_KILIT ===\n");

    // Get private key from environment
    let private_key =
        std::env::var("KARTAL_KILIT").expect("KARTAL_KILIT environment variable not set");

    // Create wallet from private key
    let wallet = private_key.parse::<LocalWallet>()?.with_chain_id(1u64); // mainnet

    let wallet_address = wallet.address();
    println!("Wallet address: {}\n", wallet_address);

    // Connect to local node
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    let provider = Arc::new(provider);

    // Check ETH balance
    let balance = provider.get_balance(wallet_address, None).await?;
    println!(
        "Current ETH Balance: {} ETH\n",
        ethers::utils::format_ether(balance)
    );

    // Create pool factory
    let pool_factory = PoolFactory::new(provider.clone());

    // Simulate 0.05 ETH → USDC swap
    println!("🔄 Simulating: 0.05 ETH → USDC");
    let eth_amount = ethers::utils::parse_ether("0.05")?;

    // Find best pool
    let pool = pool_factory
        .find_best_pool(WETH.parse()?, USDC.parse()?)
        .await?;

    println!("Best pool: {} ({})", pool.address(), pool.protocol());

    // Get quote
    let usdc_out = pool.get_amount_out(eth_amount, WETH.parse()?).await?;
    let usdc_formatted = ethers::utils::format_units(usdc_out, 6)?;

    println!("\n📈 Simulation Results:");
    println!("Input: 0.05 ETH");
    println!("Expected output: {} USDC", usdc_formatted);
    println!(
        "Exchange rate: 1 ETH = {:.2} USDC",
        usdc_formatted.parse::<f64>()? / 0.05
    );

    // Gas estimation
    let gas_price = provider.get_gas_price().await?;
    let gas_limit = U256::from(200_000);
    let gas_cost = gas_price * gas_limit;

    println!("\n⛽ Gas Analysis:");
    println!(
        "Gas price: {} gwei",
        ethers::utils::format_units(gas_price, 9)?
    );
    println!("Gas limit: {}", gas_limit);
    println!(
        "Total gas cost: {} ETH",
        ethers::utils::format_units(gas_cost, 18)?
    );

    // Calculate slippage scenarios
    let slippages = vec![0.001, 0.005, 0.01]; // 0.1%, 0.5%, 1%
    println!("\n💧 Slippage Analysis:");
    for slippage in slippages {
        let min_out = usdc_out.as_u128() as f64 * (1.0 - slippage);
        println!(
            "{:.1}% slippage → Min output: {:.2} USDC",
            slippage * 100.0,
            min_out / 1e6
        );
    }

    // Simulate 0.1 ETH → USDT swap
    println!("\n\n🔄 Simulating: 0.1 ETH → USDT");
    let eth_amount_2 = ethers::utils::parse_ether("0.1")?;

    let pool2 = pool_factory
        .find_best_pool(WETH.parse()?, USDT.parse()?)
        .await?;

    let usdt_out = pool2.get_amount_out(eth_amount_2, WETH.parse()?).await?;
    let usdt_formatted = ethers::utils::format_units(usdt_out, 6)?;

    println!("\n📈 Simulation Results:");
    println!("Input: 0.1 ETH");
    println!("Expected output: {} USDT", usdt_formatted);
    println!(
        "Exchange rate: 1 ETH = {:.2} USDT",
        usdt_formatted.parse::<f64>()? / 0.1
    );

    // Block and timestamp info
    let block = provider.get_block(BlockNumber::Latest).await?.unwrap();
    println!("\n📦 Current Block Info:");
    println!("Block number: {}", block.number.unwrap());
    println!("Block timestamp: {}", block.timestamp);
    println!(
        "Base fee: {} gwei",
        ethers::utils::format_units(block.base_fee_per_gas.unwrap_or_default(), 9)?
    );

    println!("\n✅ SIMULATION COMPLETE");
    println!("Ready for real transaction execution with KARTAL_KILIT wallet");

    Ok(())
}
