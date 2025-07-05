//! Secure Wallet Swap Simulator
//! 
//! Demonstrates swap simulation using the new secure wallet implementation
//! with encrypted keystore support

use eth_kartal::{
    wallet::{SecureWallet, SecureWalletConfig, read_password},
    pools::PoolFactory,
};
use ethers::prelude::*;
use std::sync::Arc;
use std::path::PathBuf;

// Token addresses on Ethereum mainnet
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Secure Wallet Swap Simulator ===\n");
    
    // Get keystore path
    let keystore_path = if let Ok(path) = std::env::var("ETH_KEYSTORE_PATH") {
        PathBuf::from(path)
    } else {
        // Use default location
        PathBuf::from("/home/nima/.eth-kartal/keystore.json")
    };
    
    if !keystore_path.exists() {
        println!("❌ Keystore not found at: {:?}", keystore_path);
        println!("\nTo create a keystore from KARTAL_KILIT:");
        println!("cargo run --bin keystore_manager -- create");
        return Ok(());
    }
    
    // Connect to local node
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    let provider = Arc::new(provider);
    
    // Load secure wallet
    println!("Loading secure wallet from keystore...");
    
    let wallet_config = SecureWalletConfig {
        keystore_path: keystore_path.clone(),
        chain_id: 1,
        auto_lock_timeout: Some(std::time::Duration::from_secs(300)), // 5 minutes
    };
    
    let wallet = SecureWallet::from_keystore(wallet_config).await?;
    
    // Unlock wallet
    let password = read_password("Enter keystore password: ")?;
    wallet.unlock(password).await?;
    
    println!("✅ Wallet loaded: {}\n", wallet.address());
    
    // Check ETH balance
    let balance = provider.get_balance(wallet.address(), None).await?;
    println!("Current ETH Balance: {} ETH\n", ethers::utils::format_ether(balance));
    
    // Create pool factory
    let pool_factory = PoolFactory::new(provider.clone());
    
    // Simulate swaps
    loop {
        println!("\n📊 Simulation Options:");
        println!("1. Simulate 0.05 ETH → USDC swap");
        println!("2. Simulate 0.1 ETH → USDT swap");
        println!("3. Simulate emergency sell (all ETH → USDC)");
        println!("4. Simulate arbitrage opportunity");
        println!("5. Exit");
        
        println!("\nSelect option (1-5): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => simulate_eth_to_usdc(&wallet, &pool_factory, &provider).await?,
            "2" => simulate_eth_to_usdt(&wallet, &pool_factory, &provider).await?,
            "3" => simulate_emergency_sell(&wallet, &pool_factory, &provider, balance).await?,
            "4" => simulate_arbitrage(&wallet, &pool_factory, &provider).await?,
            "5" => {
                println!("\n🔒 Locking wallet...");
                wallet.lock().await;
                break;
            }
            _ => println!("Invalid option"),
        }
    }
    
    Ok(())
}

async fn simulate_eth_to_usdc(
    wallet: &SecureWallet,
    pool_factory: &PoolFactory,
    provider: &Arc<Provider<Http>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Simulating: 0.05 ETH → USDC");
    println!("From wallet: {}", wallet.address());
    
    let eth_amount = ethers::utils::parse_ether("0.05")?;
    
    // Find best pool
    let pool = pool_factory.find_best_pool(
        WETH.parse()?,
        USDC.parse()?,
    ).await?;
    
    println!("Best pool: {} ({})", pool.address(), pool.protocol());
    
    // Get quote
    let usdc_out = pool.get_amount_out(eth_amount, WETH.parse()?).await?;
    let usdc_formatted = ethers::utils::format_units(usdc_out, 6)?;
    
    println!("\n📈 Simulation Results:");
    println!("Input: 0.05 ETH");
    println!("Expected output: {} USDC", usdc_formatted);
    println!("Exchange rate: 1 ETH = {:.2} USDC", 
        usdc_formatted.parse::<f64>()? / 0.05);
    
    // Simulate transaction signing (without sending)
    println!("\n🔐 Simulating transaction signing...");
    
    // Build a dummy transaction
    let _tx = TransactionRequest::new()
        .to(pool.address())
        .value(eth_amount)
        .data(vec![0x00]); // Dummy calldata
    
    // Estimate gas
    let gas_price = provider.get_gas_price().await?;
    let gas_limit = U256::from(200_000);
    let gas_cost = gas_price * gas_limit;
    
    println!("Gas price: {} gwei", ethers::utils::format_units(gas_price, 9)?);
    println!("Gas limit: {}", gas_limit);
    println!("Total gas cost: {} ETH", ethers::utils::format_units(gas_cost, 18)?);
    
    println!("\n✅ SIMULATION COMPLETE");
    println!("Wallet can sign transactions: {}", wallet.is_unlocked().await);
    println!("No real transaction was sent!");
    
    Ok(())
}

async fn simulate_eth_to_usdt(
    _wallet: &SecureWallet,
    pool_factory: &PoolFactory,
    _provider: &Arc<Provider<Http>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Simulating: 0.1 ETH → USDT");
    println!("From wallet: {}", _wallet.address());
    
    let eth_amount = ethers::utils::parse_ether("0.1")?;
    
    // Find best pool
    let pool = pool_factory.find_best_pool(
        WETH.parse()?,
        USDT.parse()?,
    ).await?;
    
    // Get quote
    let usdt_out = pool.get_amount_out(eth_amount, WETH.parse()?).await?;
    let usdt_formatted = ethers::utils::format_units(usdt_out, 6)?;
    
    println!("\n📈 Simulation Results:");
    println!("Input: 0.1 ETH");
    println!("Expected output: {} USDT", usdt_formatted);
    
    // Calculate slippage scenarios
    let slippages = vec![0.001, 0.005, 0.01]; // 0.1%, 0.5%, 1%
    println!("\n💧 Slippage Analysis:");
    for slippage in slippages {
        let min_out = usdt_out.as_u128() as f64 * (1.0 - slippage);
        println!("{:.1}% slippage → Min output: {:.2} USDT", 
            slippage * 100.0, min_out / 1e6);
    }
    
    println!("\n✅ SIMULATION COMPLETE - No transaction sent");
    
    Ok(())
}

async fn simulate_emergency_sell(
    wallet: &SecureWallet,
    pool_factory: &PoolFactory,
    provider: &Arc<Provider<Http>>,
    balance: U256,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚨 Simulating Emergency Sell (All ETH → USDC)");
    println!("From wallet: {}", wallet.address());
    
    // Reserve gas for transaction
    let gas_reserve = ethers::utils::parse_ether("0.01")?; // Reserve 0.01 ETH for gas
    let sell_amount = balance.saturating_sub(gas_reserve);
    
    if sell_amount == U256::zero() {
        println!("❌ Insufficient balance for emergency sell");
        return Ok(());
    }
    
    let sell_amount_eth = ethers::utils::format_ether(sell_amount);
    println!("Amount to sell: {} ETH (keeping 0.01 ETH for gas)", sell_amount_eth);
    
    // Find pool and get quote
    let pool = pool_factory.find_best_pool(
        WETH.parse()?,
        USDC.parse()?,
    ).await?;
    
    let usdc_out = pool.get_amount_out(sell_amount, WETH.parse()?).await?;
    let usdc_formatted = ethers::utils::format_units(usdc_out, 6)?;
    
    println!("\n📈 Emergency Sell Results:");
    println!("Selling: {} ETH", sell_amount_eth);
    println!("Expected USDC: {}", usdc_formatted);
    println!("Pool: {} ({})", pool.address(), pool.protocol());
    
    // Simulate high-priority gas
    let priority_gas = provider.get_gas_price().await? * 150 / 100; // 1.5x normal
    println!("\n⚡ Priority Execution:");
    println!("Priority gas: {} gwei (1.5x normal)", 
        ethers::utils::format_units(priority_gas, 9)?);
    println!("Estimated time to inclusion: ~12 seconds (next block)");
    
    println!("\n✅ EMERGENCY SELL SIMULATION COMPLETE");
    println!("In real emergency: Transaction would be sent with high priority");
    
    Ok(())
}

async fn simulate_arbitrage(
    _wallet: &SecureWallet,
    pool_factory: &PoolFactory,
    provider: &Arc<Provider<Http>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💎 Simulating Arbitrage Opportunity");
    println!("Strategy: ETH → USDC → USDT → ETH");
    
    let initial_eth = ethers::utils::parse_ether("1.0")?;
    println!("Starting with: 1.0 ETH");
    
    // Step 1: ETH → USDC
    let pool1 = pool_factory.find_best_pool(WETH.parse()?, USDC.parse()?).await?;
    let usdc_amount = pool1.get_amount_out(initial_eth, WETH.parse()?).await?;
    println!("\nStep 1: ETH → USDC");
    println!("  Output: {} USDC", ethers::utils::format_units(usdc_amount, 6)?);
    
    // Step 2: USDC → USDT
    let pool2 = pool_factory.find_best_pool(USDC.parse()?, USDT.parse()?).await?;
    let usdt_amount = pool2.get_amount_out(usdc_amount, USDC.parse()?).await?;
    println!("\nStep 2: USDC → USDT");
    println!("  Output: {} USDT", ethers::utils::format_units(usdt_amount, 6)?);
    
    // Step 3: USDT → ETH
    let pool3 = pool_factory.find_best_pool(USDT.parse()?, WETH.parse()?).await?;
    let final_eth = pool3.get_amount_out(usdt_amount, USDT.parse()?).await?;
    println!("\nStep 3: USDT → ETH");
    println!("  Output: {} ETH", ethers::utils::format_ether(final_eth));
    
    // Calculate profit
    let profit = if final_eth > initial_eth {
        final_eth - initial_eth
    } else {
        U256::zero()
    };
    
    let profit_eth = ethers::utils::format_ether(profit);
    let profit_pct = (final_eth.as_u128() as f64 / initial_eth.as_u128() as f64 - 1.0) * 100.0;
    
    println!("\n📊 Arbitrage Analysis:");
    println!("Initial: 1.0 ETH");
    println!("Final: {} ETH", ethers::utils::format_ether(final_eth));
    println!("Profit: {} ETH ({:.3}%)", profit_eth, profit_pct);
    
    // Gas cost analysis
    let gas_per_swap = U256::from(180_000);
    let total_gas = gas_per_swap * 3; // 3 swaps
    let gas_price = provider.get_gas_price().await?;
    let gas_cost = gas_price * total_gas;
    
    println!("\n⛽ Gas Cost Analysis:");
    println!("Gas per swap: {}", gas_per_swap);
    println!("Total gas: {}", total_gas);
    println!("Gas cost: {} ETH", ethers::utils::format_ether(gas_cost));
    
    let net_profit = profit.saturating_sub(gas_cost);
    if net_profit > U256::zero() {
        println!("\n✅ PROFITABLE ARBITRAGE FOUND!");
        println!("Net profit: {} ETH", ethers::utils::format_ether(net_profit));
    } else {
        println!("\n❌ Not profitable after gas costs");
    }
    
    println!("\n✅ ARBITRAGE SIMULATION COMPLETE");
    println!("Secure wallet ready to execute if profitable");
    
    Ok(())
}