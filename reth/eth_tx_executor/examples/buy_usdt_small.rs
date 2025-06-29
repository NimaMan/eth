//! Buy USDT with 0.05 ETH - Small amount that should succeed
//! 
//! Demonstrates buying USDT tokens with ETH using the implemented swapExactETHForTokens

use eth_kartal::pools::{PoolFactory, SwapParams};
use ethers::prelude::*;
use ethers::utils::{format_units, parse_ether};
use std::sync::Arc;

// Token addresses on Ethereum mainnet
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";

// KARTAL_KILIT wallet address
const KARTAL_WALLET: &str = "0xb340ad45e7729b9C54c79e744fB3708FB6fb245C";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Buy USDT with 0.05 ETH (Should Succeed) ===");
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
    
    // Check USDT balance
    let usdt_contract = IERC20::new(USDT.parse::<Address>()?, provider.clone());
    let usdt_balance = usdt_contract.balance_of(KARTAL_WALLET.parse::<Address>()?).call().await?;
    println!("USDT: {} USDT", format_units(usdt_balance, 6)?);
    
    // Amount to spend
    let eth_to_spend = parse_ether("0.05")?;
    println!("\n🔄 Planning to buy USDT with {} ETH", format_units(eth_to_spend, "ether")?);
    
    // Find WETH/USDT pool
    let pool = pool_factory.find_best_pool(
        WETH.parse()?,
        USDT.parse()?,
    ).await?;
    
    println!("\n📊 Pool Information:");
    println!("Pool Address: {}", pool.address());
    println!("Protocol: {}", pool.protocol());
    
    // Get reserves to check liquidity
    let (reserve0, reserve1) = pool.get_reserves().await?;
    println!("Pool Reserves: {} / {}", reserve0, reserve1);
    
    // Get quote for ETH -> USDT
    let usdt_out = pool.get_amount_out(eth_to_spend, WETH.parse()?).await?;
    let usdt_out_human = format_units(usdt_out, 6)?;
    println!("\n💱 Swap Quote:");
    println!("Input: {} ETH", format_units(eth_to_spend, "ether")?);
    println!("Expected Output: {} USDT", usdt_out_human);
    
    // Calculate implied ETH price
    let eth_price = usdt_out.as_u128() as f64 / 1_000_000.0 / 0.05;
    println!("Implied ETH Price: ${:.2} USDT", eth_price);
    
    // Calculate slippage
    let slippage = 0.01; // 1%
    let min_out = usdt_out * (100 - (slippage * 100.0) as u64) / 100;
    println!("\n🛡️  Slippage Protection:");
    println!("Slippage Tolerance: {}%", slippage * 100.0);
    println!("Minimum USDT Out: {} USDT", format_units(min_out, 6)?);
    
    // Build swap parameters
    let deadline = chrono::Utc::now().timestamp() as u64 + 300; // 5 minutes
    let recipient = KARTAL_WALLET.parse::<Address>()?;
    
    let swap_params = SwapParams {
        token_in: WETH.parse()?,
        token_out: USDT.parse()?,
        amount_in: eth_to_spend,
        amount_out_min: min_out,
        recipient,
        deadline: U256::from(deadline),
    };
    
    println!("\n🔨 Building Transaction...");
    
    // Build the transaction
    let tx = pool.build_swap_tx(swap_params).await?;
    
    println!("\n📝 Transaction Details:");
    println!("To: {:?} (Uniswap V2 Router)", tx.to().unwrap());
    println!("Value: {} ETH", format_units(tx.value().unwrap_or(&U256::zero()), "ether")?);
    println!("Data Length: {} bytes", tx.data().unwrap().len());
    
    // Verify it's using the correct function
    let data = tx.data().unwrap();
    if data.len() >= 4 && &data[0..4] == &[0x7f, 0xf3, 0x6a, 0xb5] {
        println!("✅ Function: swapExactETHForTokens (correct for ETH->token swap)");
    }
    
    // Estimate gas
    println!("\n⛽ Gas Estimation:");
    let gas_estimate = U256::from(156_000); // Conservative estimate
    let gas_price = provider.get_gas_price().await?;
    let gas_cost = gas_estimate * gas_price;
    
    println!("Estimated Gas: {}", gas_estimate);
    println!("Gas Price: {} gwei", format_units(gas_price, 9)?);
    println!("Total Gas Cost: {} ETH", format_units(gas_cost, "ether")?);
    
    // Check if wallet has enough ETH
    let total_needed = eth_to_spend + gas_cost;
    println!("\n💸 Total Cost Analysis:");
    println!("ETH for swap: {} ETH", format_units(eth_to_spend, "ether")?);
    println!("Gas cost: {} ETH", format_units(gas_cost, "ether")?);
    println!("Total needed: {} ETH", format_units(total_needed, "ether")?);
    
    if total_needed > eth_balance {
        println!("❌ Insufficient balance! Need {} more ETH", 
            format_units(total_needed - eth_balance, "ether")?);
    } else {
        println!("✅ Sufficient balance! Transaction would succeed.");
    }
    
    println!("\n✅ SIMULATION COMPLETE");
    println!("In a real execution, you would receive approximately {} USDT for {} ETH",
        usdt_out_human,
        format_units(eth_to_spend, "ether")?
    );
    
    Ok(())
}

// ABI for IERC20
abigen!(
    IERC20,
    r#"[
        function balanceOf(address) external view returns (uint256)
    ]"#
);