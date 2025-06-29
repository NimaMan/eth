//! Transfer Simulator for ETH Kartal
//! 
//! Simulates ETH and token transfers without executing real transactions

use eth_kartal::wallet::SecureWallet;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use std::path::PathBuf;

// Common token addresses
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";

// ERC20 ABI for balance checking
abigen!(
    IERC20,
    r#"[
        function balanceOf(address) external view returns (uint256)
        function decimals() external view returns (uint8)
        function symbol() external view returns (string)
    ]"#
);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ETH Kartal Transfer Simulator ===\n");
    
    // Connect to local node
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    let provider = Arc::new(provider);
    
    // Get wallet address from environment or use KARTAL wallet
    let wallet_address: Address = if let Ok(addr) = std::env::var("WALLET_ADDRESS") {
        addr.parse()?
    } else {
        "0xb340ad45e7729b9C54c79e744fB3708FB6fb245C".parse()? // KARTAL wallet
    };
    
    println!("Simulating transfers from: {}", wallet_address);
    
    // Check balances
    let eth_balance = provider.get_balance(wallet_address, None).await?;
    println!("ETH Balance: {} ETH\n", ethers::utils::format_ether(eth_balance));
    
    loop {
        println!("\n📤 Transfer Simulation Options:");
        println!("1. Simulate ETH transfer");
        println!("2. Simulate USDC transfer");
        println!("3. Simulate batch transfers");
        println!("4. Simulate gas-optimized transfer");
        println!("5. Exit");
        
        println!("\nSelect option (1-5): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => simulate_eth_transfer(&provider, wallet_address, eth_balance).await?,
            "2" => simulate_token_transfer(&provider, wallet_address, USDC).await?,
            "3" => simulate_batch_transfers(&provider, wallet_address).await?,
            "4" => simulate_optimized_transfer(&provider, wallet_address).await?,
            "5" => break,
            _ => println!("Invalid option"),
        }
    }
    
    Ok(())
}

async fn simulate_eth_transfer(
    provider: &Arc<Provider<Http>>,
    from: Address,
    balance: U256,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💸 Simulating ETH Transfer");
    
    println!("Enter recipient address: ");
    let mut recipient = String::new();
    std::io::stdin().read_line(&mut recipient)?;
    let to: Address = recipient.trim().parse()?;
    
    println!("Enter amount in ETH: ");
    let mut amount_str = String::new();
    std::io::stdin().read_line(&mut amount_str)?;
    let amount = ethers::utils::parse_ether(amount_str.trim())?;
    
    // Check if sufficient balance
    if amount > balance {
        println!("❌ Insufficient balance! Have {} ETH", ethers::utils::format_ether(balance));
        return Ok(());
    }
    
    // Build transaction
    let tx = TransactionRequest::new()
        .from(from)
        .to(to)
        .value(amount);
    
    // Convert to TypedTransaction for gas estimation
    let typed_tx: TypedTransaction = tx.clone().into();
    
    // Estimate gas
    let gas_estimate = provider.estimate_gas(&typed_tx, None).await.unwrap_or(U256::from(21_000));
    let gas_price = provider.get_gas_price().await?;
    let total_cost = amount + (gas_estimate * gas_price);
    
    println!("\n📊 Transfer Simulation Results:");
    println!("From: {}", from);
    println!("To: {}", to);
    println!("Amount: {} ETH", ethers::utils::format_ether(amount));
    println!("Gas estimate: {} units", gas_estimate);
    println!("Gas price: {} gwei", ethers::utils::format_units(gas_price, 9)?);
    println!("Gas cost: {} ETH", ethers::utils::format_ether(gas_estimate * gas_price));
    println!("Total cost: {} ETH", ethers::utils::format_ether(total_cost));
    
    if total_cost > balance {
        println!("\n❌ Insufficient balance for transfer + gas!");
    } else {
        println!("\n✅ Transfer simulation successful");
        println!("Remaining balance: {} ETH", 
            ethers::utils::format_ether(balance - total_cost));
    }
    
    println!("\nNO REAL TRANSACTION SENT - SIMULATION ONLY");
    
    Ok(())
}

async fn simulate_token_transfer(
    provider: &Arc<Provider<Http>>,
    from: Address,
    token_address: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💰 Simulating Token Transfer");
    
    let token = IERC20::new(token_address.parse::<Address>()?, provider.clone());
    
    // Get token info
    let symbol = token.symbol().call().await.unwrap_or_else(|_| "UNKNOWN".to_string());
    let decimals = token.decimals().call().await.unwrap_or(18);
    let balance = token.balance_of(from).call().await?;
    
    println!("Token: {} ({})", symbol, token_address);
    println!("Balance: {} {}", 
        ethers::utils::format_units(balance, decimals as u32)?, symbol);
    
    if balance == U256::zero() {
        println!("❌ No {} balance to transfer!", symbol);
        return Ok(());
    }
    
    println!("\nEnter recipient address: ");
    let mut recipient = String::new();
    std::io::stdin().read_line(&mut recipient)?;
    let to: Address = recipient.trim().parse()?;
    
    println!("Enter amount in {}: ", symbol);
    let mut amount_str = String::new();
    std::io::stdin().read_line(&mut amount_str)?;
    let amount = ethers::utils::parse_units(amount_str.trim(), decimals as u32)?;
    let amount = match amount {
        ethers::utils::ParseUnits::U256(val) => val,
        _ => return Err("Invalid amount format".into()),
    };
    
    if amount > balance {
        println!("❌ Insufficient balance!");
        return Ok(());
    }
    
    // Build transfer calldata
    let transfer_fn = token.method::<_, bool>("transfer", (to, amount))?;
    let transfer_data = transfer_fn.calldata().unwrap();
    
    let tx = TransactionRequest::new()
        .from(from)
        .to(token_address.parse::<Address>()?)
        .data(transfer_data);
    
    // Convert to TypedTransaction for gas estimation
    let typed_tx: TypedTransaction = tx.into();
    
    // Estimate gas
    let gas_estimate = provider.estimate_gas(&typed_tx, None).await.unwrap_or(U256::from(65_000));
    let gas_price = provider.get_gas_price().await?;
    let gas_cost = gas_estimate * gas_price;
    
    println!("\n📊 Token Transfer Simulation:");
    println!("From: {}", from);
    println!("To: {}", to);
    println!("Amount: {} {}", ethers::utils::format_units(amount, decimals as u32)?, symbol);
    println!("Gas estimate: {} units", gas_estimate);
    println!("Gas cost: {} ETH", ethers::utils::format_ether(gas_cost));
    
    // Check ETH balance for gas
    let eth_balance = provider.get_balance(from, None).await?;
    if gas_cost > eth_balance {
        println!("\n❌ Insufficient ETH for gas!");
    } else {
        println!("\n✅ Token transfer simulation successful");
    }
    
    println!("\nNO REAL TRANSACTION SENT - SIMULATION ONLY");
    
    Ok(())
}

async fn simulate_batch_transfers(
    provider: &Arc<Provider<Http>>,
    _from: Address,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📦 Simulating Batch Transfers");
    println!("This simulates sending ETH to multiple recipients in one transaction");
    
    // Example recipients
    let recipients = vec![
        ("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD9E", "0.01"), // Example 1
        ("0x40B38765696e3d5d8d9d834D8AaD4bB6e418E489", "0.02"), // Example 2  
        ("0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE", "0.015"), // Example 3
    ];
    
    let mut total_amount = U256::zero();
    let mut transfers = Vec::new();
    
    println!("\nBatch transfers:");
    for (addr, amount_str) in &recipients {
        let amount = ethers::utils::parse_ether(amount_str)?;
        total_amount += amount;
        transfers.push((addr.parse::<Address>()?, amount));
        println!("  {} → {} ETH", addr, amount_str);
    }
    
    // Multicall gas estimation (approximate)
    let gas_per_transfer = U256::from(21_000);
    let overhead = U256::from(25_000); // Contract overhead
    let total_gas = overhead + (gas_per_transfer * recipients.len());
    let gas_price = provider.get_gas_price().await?;
    let gas_cost = total_gas * gas_price;
    
    println!("\n📊 Batch Transfer Analysis:");
    println!("Total ETH to send: {} ETH", ethers::utils::format_ether(total_amount));
    println!("Number of recipients: {}", recipients.len());
    println!("Estimated gas: {} units", total_gas);
    println!("Gas cost: {} ETH", ethers::utils::format_ether(gas_cost));
    println!("Total cost: {} ETH", ethers::utils::format_ether(total_amount + gas_cost));
    
    // Compare with individual transfers
    let individual_gas = gas_per_transfer * recipients.len();
    let individual_cost = individual_gas * gas_price;
    let savings = individual_cost - gas_cost;
    
    println!("\n💡 Gas Savings:");
    println!("Individual transfers: {} ETH", ethers::utils::format_ether(individual_cost));
    println!("Batch transfer: {} ETH", ethers::utils::format_ether(gas_cost));
    println!("Savings: {} ETH ({:.1}%)", 
        ethers::utils::format_ether(savings),
        (savings.as_u128() as f64 / individual_cost.as_u128() as f64) * 100.0
    );
    
    println!("\n✅ BATCH SIMULATION COMPLETE - No transactions sent");
    
    Ok(())
}

async fn simulate_optimized_transfer(
    provider: &Arc<Provider<Http>>,
    _from: Address,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ Simulating Gas-Optimized Transfer");
    
    // Get current gas prices
    let base_fee = provider.get_block(BlockNumber::Latest)
        .await?
        .unwrap()
        .base_fee_per_gas
        .unwrap_or(U256::zero());
    
    let current_gas_price = provider.get_gas_price().await?;
    
    println!("\n📊 Current Network Conditions:");
    println!("Base fee: {} gwei", ethers::utils::format_units(base_fee, 9)?);
    println!("Current gas price: {} gwei", ethers::utils::format_units(current_gas_price, 9)?);
    
    // Simulate different priority levels
    let priority_levels = vec![
        ("🐌 Slow (5 min)", base_fee + ethers::utils::parse_units("1", 9)?),
        ("⚡ Normal (30 sec)", current_gas_price),
        ("🚀 Fast (12 sec)", current_gas_price * 125 / 100),
        ("💎 Instant (next block)", current_gas_price * 150 / 100),
    ];
    
    println!("\n💰 Transfer Cost Analysis (0.1 ETH transfer):");
    let transfer_amount = ethers::utils::parse_ether("0.1")?;
    let gas_limit = U256::from(21_000);
    
    for (name, gas_price) in priority_levels {
        let gas_cost = gas_price * gas_limit;
        let total = transfer_amount + gas_cost;
        println!("\n{}", name);
        println!("  Gas price: {} gwei", ethers::utils::format_units(gas_price, 9)?);
        println!("  Gas cost: {} ETH", ethers::utils::format_ether(gas_cost));
        println!("  Total cost: {} ETH", ethers::utils::format_ether(total));
    }
    
    // Time-based optimization
    println!("\n⏰ Best Times to Transfer (based on historical data):");
    println!("  🌙 Late night (2-5 AM UTC): ~20% cheaper");
    println!("  🌅 Early morning (6-8 AM UTC): ~15% cheaper");
    println!("  📈 Peak hours (2-6 PM UTC): Most expensive");
    println!("  🌃 Weekends: Generally 10-30% cheaper");
    
    println!("\n✅ OPTIMIZATION SIMULATION COMPLETE");
    println!("Use these insights to minimize transfer costs!");
    
    Ok(())
}