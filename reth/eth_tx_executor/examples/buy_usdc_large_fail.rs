//! Buy USDC with 1 ETH - Large amount that should fail due to insufficient balance
//! 
//! Demonstrates error handling when trying to buy more than available balance

use eth_kartal::pools::{PoolFactory, SwapParams};
use ethers::prelude::*;
use ethers::utils::{format_units, parse_ether};
use std::sync::Arc;

// Token addresses on Ethereum mainnet
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

// KARTAL_KILIT wallet address
const KARTAL_WALLET: &str = "0xb340ad45e7729b9C54c79e744fB3708FB6fb245C";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Buy USDC with 1 ETH (Should Fail - Insufficient Balance) ===");
    println!("Wallet: {}", KARTAL_WALLET);
    println!("Mode: SIMULATION ONLY (testing insufficient balance scenario)\n");
    
    // Connect to local node
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    let provider = Arc::new(provider);
    
    // Create pool factory
    let pool_factory = PoolFactory::new(provider.clone());
    
    // Check current balances
    let eth_balance = provider.get_balance(KARTAL_WALLET.parse::<Address>()?, None).await?;
    println!("💰 Current Balances:");
    println!("ETH: {} ETH", format_units(eth_balance, "ether")?);
    
    // Check USDC balance
    let usdc_contract = IERC20::new(USDC.parse::<Address>()?, provider.clone());
    let usdc_balance = usdc_contract.balance_of(KARTAL_WALLET.parse::<Address>()?).call().await?;
    println!("USDC: {} USDC", format_units(usdc_balance, 6)?);
    
    // Amount to spend (1 ETH - likely more than wallet balance)
    let eth_to_spend = parse_ether("1.0")?;
    println!("\n🔄 Attempting to buy USDC with {} ETH", format_units(eth_to_spend, "ether")?);
    
    // Check if we even have this much ETH
    if eth_to_spend > eth_balance {
        println!("\n⚠️  WARNING: Attempting to spend more ETH than available!");
        println!("Requested: {} ETH", format_units(eth_to_spend, "ether")?);
        println!("Available: {} ETH", format_units(eth_balance, "ether")?);
        println!("Shortfall: {} ETH", format_units(eth_to_spend - eth_balance, "ether")?);
    }
    
    // Find WETH/USDC pool
    let pool = pool_factory.find_best_pool(
        WETH.parse()?,
        USDC.parse()?,
    ).await?;
    
    println!("\n📊 Pool Information:");
    println!("Pool Address: {}", pool.address());
    println!("Protocol: {}", pool.protocol());
    
    // Get quote for ETH -> USDC (even though we can't afford it)
    let usdc_out = pool.get_amount_out(eth_to_spend, WETH.parse()?).await?;
    let usdc_out_human = format_units(usdc_out, 6)?;
    
    println!("\n💱 Theoretical Swap Quote (if we had enough ETH):");
    println!("Input: {} ETH", format_units(eth_to_spend, "ether")?);
    println!("Expected Output: {} USDC", usdc_out_human);
    
    // Calculate implied price
    let eth_amount_f64 = 1.0;
    let usdc_amount_f64 = usdc_out_human.parse::<f64>()?;
    let implied_eth_price = usdc_amount_f64 / eth_amount_f64;
    println!("Implied ETH Price: ${:.2} USDC", implied_eth_price);
    
    // Apply slippage
    let slippage = 0.01; // 1%
    let min_amount_out = apply_slippage(usdc_out, slippage);
    
    // Build swap parameters
    let swap_params = SwapParams {
        token_in: WETH.parse()?,
        token_out: USDC.parse()?,
        amount_in: eth_to_spend,
        amount_out_min: min_amount_out,
        recipient: KARTAL_WALLET.parse()?,
        deadline: current_timestamp() + 300, // 5 minutes
    };
    
    // Build the transaction
    println!("\n🔨 Building Transaction (even though it will fail)...");
    let tx = pool.build_swap_tx(swap_params).await?;
    
    // Verify transaction details
    println!("\n📝 Transaction Details:");
    println!("To: {:?} (Uniswap V2 Router)", tx.to().unwrap());
    println!("Value: {} ETH (THIS IS THE PROBLEM!)", format_units(*tx.value().unwrap_or(&U256::zero()), "ether")?);
    
    // Estimate gas (this should fail)
    println!("\n⛽ Attempting Gas Estimation (should fail):");
    match provider.estimate_gas(&tx, None).await {
        Ok(gas_estimate) => {
            println!("❓ Unexpected: Gas estimation succeeded with: {}", gas_estimate);
            
            let gas_price = provider.get_gas_price().await?;
            let gas_cost = gas_price * gas_estimate;
            let total_cost = eth_to_spend + gas_cost;
            
            println!("\n💸 Total Cost Analysis:");
            println!("ETH for swap: {} ETH", format_units(eth_to_spend, "ether")?);
            println!("Gas cost: {} ETH", format_units(gas_cost, "ether")?);
            println!("Total needed: {} ETH", format_units(total_cost, "ether")?);
            println!("Available: {} ETH", format_units(eth_balance, "ether")?);
            println!("❌ SHORTFALL: {} ETH", format_units(total_cost.saturating_sub(eth_balance), "ether")?);
        }
        Err(e) => {
            println!("✅ Expected failure: {}", e);
            println!("Reason: Insufficient ETH balance for transaction value + gas");
        }
    }
    
    // Simulate what would happen in the executor
    println!("\n🤖 Simulating Executor Behavior:");
    println!("1. Check ETH balance: {} ETH", format_units(eth_balance, "ether")?);
    println!("2. Amount to spend: {} ETH", format_units(eth_to_spend, "ether")?);
    
    // Executor logic simulation
    let executor_eth_to_spend = if eth_to_spend == U256::MAX {
        // If MAX, use 90% of balance
        eth_balance * 90 / 100
    } else {
        eth_to_spend.min(eth_balance)
    };
    
    println!("3. Executor would adjust to: {} ETH", format_units(executor_eth_to_spend, "ether")?);
    
    // But we need to leave room for gas
    let estimated_gas_cost = parse_ether("0.01")?; // Rough estimate
    let safe_eth_to_spend = executor_eth_to_spend.saturating_sub(estimated_gas_cost);
    
    println!("4. After reserving for gas: {} ETH", format_units(safe_eth_to_spend, "ether")?);
    
    if safe_eth_to_spend.is_zero() {
        println!("5. ❌ RESULT: Insufficient ETH for purchase after gas costs");
    } else {
        println!("5. ℹ️  With adjustment, could buy with: {} ETH", format_units(safe_eth_to_spend, "ether")?);
    }
    
    println!("\n✅ SIMULATION COMPLETE");
    println!("This example demonstrates how the system handles insufficient balance scenarios.");
    println!("In production, the executor would either:");
    println!("  1. Fail early with 'Insufficient ETH for purchase' error");
    println!("  2. Adjust the amount down to available balance (if using MAX amount)");
    
    Ok(())
}

/// Apply slippage to amount
fn apply_slippage(amount: U256, slippage: f64) -> U256 {
    let factor = 1.0 - slippage;
    let adjusted = amount.as_u128() as f64 * factor;
    U256::from(adjusted as u128)
}

/// Get current timestamp
fn current_timestamp() -> U256 {
    U256::from(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs())
}

// IERC20 interface for balance checking
use ethers::contract::abigen;
abigen!(
    IERC20,
    r#"[
        function balanceOf(address) external view returns (uint256)
    ]"#
);