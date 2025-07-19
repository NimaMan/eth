//! Risk Integrated Trading Demo
//!
//! Shows how risk checks are integrated into actual trading flow

use eth_kartal::{
    alert_processor::{Alert, Action, ExecutionParams, Priority},
    risk::{RiskManager, RiskConfig, RiskDecision},
    pools::PoolFactory,
    common::validate_slippage,
};
use ethers::{
    prelude::*,
    utils::{format_units, parse_ether},
};
use std::sync::Arc;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("=== Risk Integrated Trading Demo ===");

    // Connect to local node
    let provider = Arc::new(Provider::<Http>::try_from("http://127.0.0.1:8545")?);
    
    // Setup wallet from KARTAL_KILIT
    let private_key = std::env::var("KARTAL_KILIT")
        .expect("Set KARTAL_KILIT environment variable");
    let wallet = private_key.parse::<LocalWallet>()?
        .with_chain_id(1u64);
    let wallet_address = wallet.address();
    
    info!("Wallet address: {}", wallet_address);

    // Get current balances
    let eth_balance = provider.get_balance(wallet_address, None).await?;
    info!("ETH Balance: {} ETH", format_units(eth_balance, "ether")?);

    // Create risk manager
    let risk_config = RiskConfig {
        max_gas_cost_percent: 5.0,    // 5% max gas cost
        max_slippage_percent: 3.0,    // 3% max slippage
        min_eth_balance: 0.01,        // Keep 0.01 ETH buffer
    };
    let risk_manager = RiskManager::new(risk_config);

    // Create pool factory
    let pool_factory = PoolFactory::new(provider.clone());

    // Test token addresses
    let usdc_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse::<Address>()?;
    let usdt_address = "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse::<Address>()?;

    // Scenario 1: Small trade that should pass all checks
    info!("\n📊 Scenario 1: Small ETH → USDC trade (should pass)");
    simulate_trade(
        &risk_manager,
        &pool_factory,
        &provider,
        wallet_address,
        usdc_address,
        0.01,  // 0.01 ETH
        eth_balance,
    ).await?;

    // Scenario 2: Trade with insufficient funds
    info!("\n📊 Scenario 2: Large ETH → USDT trade (insufficient funds)");
    simulate_trade(
        &risk_manager,
        &pool_factory,
        &provider,
        wallet_address,
        usdt_address,
        10.0,  // 10 ETH (more than we have)
        eth_balance,
    ).await?;

    // Scenario 3: Very small trade with high gas
    info!("\n📊 Scenario 3: Tiny ETH → USDC trade (high gas percentage)");
    simulate_trade(
        &risk_manager,
        &pool_factory,
        &provider,
        wallet_address,
        usdc_address,
        0.0001,  // 0.0001 ETH (gas will be high percentage)
        eth_balance,
    ).await?;

    Ok(())
}

async fn simulate_trade(
    risk_manager: &RiskManager,
    pool_factory: &PoolFactory,
    provider: &Arc<Provider<Http>>,
    wallet_address: Address,
    token_address: Address,
    trade_amount_eth: f64,
    eth_balance: U256,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Attempting to trade {} ETH for tokens", trade_amount_eth);
    
    // Convert to wei
    let trade_amount_wei = parse_ether(trade_amount_eth)?;
    
    // Get current gas price
    let gas_price = provider.get_gas_price().await?;
    let estimated_gas = U256::from(200_000); // Typical swap gas
    let gas_cost_wei = gas_price * estimated_gas;
    let gas_cost_eth = format_units(gas_cost_wei, "ether")?.parse::<f64>()?;
    
    info!("Current gas price: {} gwei", format_units(gas_price, "gwei")?);
    info!("Estimated gas cost: {} ETH", gas_cost_eth);
    
    // Risk Check 1: Sufficient funds
    info!("\n  ➤ Check 1: Sufficient funds");
    match risk_manager.check_sufficient_funds(eth_balance, trade_amount_wei, gas_cost_eth) {
        RiskDecision::Allow => {
            info!("  ✅ Sufficient funds check PASSED");
        }
        RiskDecision::Block { reason } => {
            error!("  ❌ BLOCKED: {}", reason);
            return Ok(());
        }
    }
    
    // Risk Check 2: Gas cost percentage
    info!("\n  ➤ Check 2: Gas cost percentage");
    match risk_manager.check_gas_cost(trade_amount_eth, gas_cost_eth) {
        RiskDecision::Allow => {
            info!("  ✅ Gas cost check PASSED ({:.1}% of trade)", 
                (gas_cost_eth / trade_amount_eth) * 100.0);
        }
        RiskDecision::Block { reason } => {
            error!("  ❌ BLOCKED: {}", reason);
            return Ok(());
        }
    }
    
    // Get pool and quote
    let weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<Address>()?;
    
    match pool_factory.find_best_pool(weth_address, token_address).await {
        Ok(pool) => {
            info!("\n  ➤ Getting price quote from pool");
            
            // Get expected output
            match pool.get_amount_out(trade_amount_wei, weth_address).await {
                Ok(expected_tokens) => {
                    info!("  Expected output: {} tokens", 
                        format_units(expected_tokens, 6)?); // Assuming 6 decimals
                    
                    // Simulate some slippage
                    let actual_tokens = expected_tokens * 98 / 100; // 2% slippage
                    
                    // Risk Check 3: Slippage
                    info!("\n  ➤ Check 3: Slippage tolerance");
                    match risk_manager.check_slippage(expected_tokens, actual_tokens) {
                        RiskDecision::Allow => {
                            info!("  ✅ Slippage check PASSED (2% slippage)");
                            info!("\n✅ All risk checks passed! Trade would be executed.");
                        }
                        RiskDecision::Block { reason } => {
                            error!("  ❌ BLOCKED: {}", reason);
                        }
                    }
                }
                Err(e) => {
                    error!("  Failed to get quote: {}", e);
                }
            }
        }
        Err(e) => {
            error!("  No pool found: {}", e);
        }
    }
    
    Ok(())
}