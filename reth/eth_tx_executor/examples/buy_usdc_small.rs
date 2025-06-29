//! Buy USDC with 0.05 ETH - Small amount that should succeed
//! 
//! Demonstrates buying tokens with ETH using the implemented swapExactETHForTokens

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
    println!("=== Buy USDC with 0.05 ETH (Should Succeed) ===");
    println!("Wallet: {}", KARTAL_WALLET);
    println!("Mode: SIMULATION ONLY (no real transactions)\n");
    
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
    
    // Amount to spend
    let eth_to_spend = parse_ether("0.05")?;
    println!("\n🔄 Planning to buy USDC with {} ETH", format_units(eth_to_spend, "ether")?);
    
    // Find WETH/USDC pool
    let pool = pool_factory.find_best_pool(
        WETH.parse()?,
        USDC.parse()?,
    ).await?;
    
    println!("\n📊 Pool Information:");
    println!("Pool Address: {}", pool.address());
    println!("Protocol: {}", pool.protocol());
    
    // Get reserves to check liquidity
    let (reserve0, reserve1) = pool.get_reserves().await?;
    println!("Pool Reserves: {} / {}", reserve0, reserve1);
    
    // Get quote for ETH -> USDC
    let usdc_out = pool.get_amount_out(eth_to_spend, WETH.parse()?).await?;
    let usdc_out_human = format_units(usdc_out, 6)?;
    
    println!("\n💱 Swap Quote:");
    println!("Input: {} ETH", format_units(eth_to_spend, "ether")?);
    println!("Expected Output: {} USDC", usdc_out_human);
    
    // Calculate implied price
    let eth_amount_f64 = 0.05;
    let usdc_amount_f64 = usdc_out_human.parse::<f64>()?;
    let implied_eth_price = usdc_amount_f64 / eth_amount_f64;
    println!("Implied ETH Price: ${:.2} USDC", implied_eth_price);
    
    // Apply slippage
    let slippage = 0.01; // 1%
    let min_amount_out = apply_slippage(usdc_out, slippage);
    println!("\n🛡️  Slippage Protection:");
    println!("Slippage Tolerance: {}%", slippage * 100.0);
    println!("Minimum USDC Out: {} USDC", format_units(min_amount_out, 6)?);
    
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
    println!("\n🔨 Building Transaction...");
    let tx = pool.build_swap_tx(swap_params).await?;
    
    // Verify transaction details
    println!("\n📝 Transaction Details:");
    println!("To: {:?} (Uniswap V2 Router)", tx.to().unwrap());
    println!("Value: {} ETH", format_units(*tx.value().unwrap_or(&U256::zero()), "ether")?);
    println!("Data Length: {} bytes", tx.data().map(|d| d.len()).unwrap_or(0));
    
    // Decode function selector to verify it's swapExactETHForTokens
    if let Some(data) = tx.data() {
        let selector = &data[0..4];
        let expected_selector = &ethers::utils::keccak256("swapExactETHForTokens(uint256,address[],address,uint256)")[0..4];
        if selector == expected_selector {
            println!("✅ Function: swapExactETHForTokens (correct for ETH->token swap)");
        } else {
            println!("❌ Function selector mismatch!");
        }
    }
    
    // Estimate gas
    println!("\n⛽ Gas Estimation:");
    match provider.estimate_gas(&tx, None).await {
        Ok(gas_estimate) => {
            println!("Estimated Gas: {}", gas_estimate);
            let gas_price = provider.get_gas_price().await?;
            let gas_cost = gas_price * gas_estimate;
            println!("Gas Price: {} gwei", format_units(gas_price, "gwei")?);
            println!("Total Gas Cost: {} ETH", format_units(gas_cost, "ether")?);
            
            // Check if we have enough ETH
            let total_cost = eth_to_spend + gas_cost;
            println!("\n💸 Total Cost Analysis:");
            println!("ETH for swap: {} ETH", format_units(eth_to_spend, "ether")?);
            println!("Gas cost: {} ETH", format_units(gas_cost, "ether")?);
            println!("Total needed: {} ETH", format_units(total_cost, "ether")?);
            
            if total_cost <= eth_balance {
                println!("✅ Sufficient balance! Transaction would succeed.");
            } else {
                println!("❌ Insufficient balance! Need {} more ETH", 
                    format_units(total_cost - eth_balance, "ether")?);
            }
        }
        Err(e) => {
            println!("❌ Gas estimation failed: {}", e);
            println!("This might indicate the transaction would revert");
        }
    }
    
    println!("\n✅ SIMULATION COMPLETE");
    println!("In a real execution, you would receive approximately {} USDC for {} ETH", 
        usdc_out_human, format_units(eth_to_spend, "ether")?);
    
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