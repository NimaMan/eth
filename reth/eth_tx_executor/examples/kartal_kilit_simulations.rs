//! KARTAL_KILIT Transfer and Swap Simulations
//! 
//! This example demonstrates transfer and swap operations using the KARTAL_KILIT wallet
//! with the secure wallet implementation.

use eth_kartal::alert_processor::{Alert, ExecutionParams, Action, Priority};
use eth_kartal::tx_executor::{TransactionExecutor, ExecutorConfig};
use eth_kartal::wallet::read_password;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

// KARTAL_KILIT wallet address
const KARTAL_WALLET: &str = "0xb340ad45e7729b9C54c79e744fB3708FB6fb245C";

// Common token addresses on Ethereum mainnet
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const DAI: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";

// Uniswap V2 pools
const WETH_USDC_V2: &str = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc";
const WETH_DAI_V2: &str = "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== KARTAL_KILIT Transfer & Swap Simulations ===");
    println!("Wallet Address: {}", KARTAL_WALLET);
    println!();
    
    // Check for keystore
    let keystore_path = std::env::var("ETH_KEYSTORE_PATH")
        .unwrap_or_else(|_| "./kartal_kilit.keystore".to_string());
    
    if !PathBuf::from(&keystore_path).exists() {
        println!("❌ Keystore not found at: {}", keystore_path);
        println!("Please create a keystore for KARTAL_KILIT first:");
        println!("  cargo run --bin keystore_manager create -o {}", keystore_path);
        return Ok(());
    }
    
    // Setup provider
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    let provider = Arc::new(provider);
    
    // Check wallet balance
    println!("📊 Checking wallet balances...");
    let eth_balance = provider.get_balance(KARTAL_WALLET.parse::<Address>()?, None).await?;
    println!("ETH Balance: {} ETH", format_units(eth_balance, "ether")?);
    
    // Create executor with secure wallet
    let config = ExecutorConfig {
        keystore_path: PathBuf::from(keystore_path),
        chain_id: 1,
        rpc_url: "http://127.0.0.1:8545".to_string(),
        flashbots_enabled: false,
        flashbots_rpc: None,
        reth_ws_url: "ws://127.0.0.1:8546".to_string(),
    };
    
    println!("\n🔐 Initializing secure transaction executor...");
    let executor = TransactionExecutor::new(config).await?;
    
    // Unlock wallet
    let password = read_password("Enter keystore password: ")?;
    executor.unlock_wallet(password).await?;
    println!("✅ Wallet unlocked successfully");
    
    // Simulation menu
    loop {
        println!("\n📋 Select Simulation:");
        println!("1. ETH Transfer");
        println!("2. Token Swap (Buy)");
        println!("3. Token Swap (Sell)");
        println!("4. Check Balances");
        println!("5. Exit");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => simulate_eth_transfer(&provider).await?,
            "2" => simulate_token_buy(&executor).await?,
            "3" => simulate_token_sell(&executor).await?,
            "4" => check_balances(&provider).await?,
            "5" => break,
            _ => println!("Invalid option"),
        }
    }
    
    // Lock wallet
    executor.lock_wallet().await;
    println!("\n🔒 Wallet locked");
    
    Ok(())
}

/// Simulate ETH transfer
async fn simulate_eth_transfer(provider: &Arc<Provider<Http>>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💸 ETH Transfer Simulation");
    
    println!("Enter recipient address (or press Enter for test address): ");
    let mut recipient = String::new();
    std::io::stdin().read_line(&mut recipient)?;
    let recipient = recipient.trim();
    let recipient = if recipient.is_empty() {
        "0x0000000000000000000000000000000000000001"
    } else {
        recipient
    };
    
    println!("Enter amount in ETH (or press Enter for 0.001): ");
    let mut amount = String::new();
    std::io::stdin().read_line(&mut amount)?;
    let amount = amount.trim();
    let amount_wei = if amount.is_empty() {
        parse_ether("0.001")?
    } else {
        parse_ether(amount)?
    };
    
    // Create transfer transaction
    let tx = TransactionRequest::new()
        .from(KARTAL_WALLET.parse::<Address>()?)
        .to(recipient.parse::<Address>()?)
        .value(amount_wei)
        .gas(21000)
        .gas_price(provider.get_gas_price().await?);
    
    println!("\n📝 Transaction Details:");
    println!("From: {}", KARTAL_WALLET);
    println!("To: {}", recipient);
    println!("Amount: {} ETH", format_units(amount_wei, "ether")?);
    println!("Gas Price: {} gwei", format_units(tx.gas_price.unwrap(), "gwei")?);
    
    println!("\n⚠️  This is a SIMULATION - transaction not sent");
    println!("Transaction would cost approximately {} ETH in gas", 
        format_units(U256::from(21000) * tx.gas_price.unwrap(), "ether")?);
    
    Ok(())
}

/// Simulate token buy (ETH -> Token)
async fn simulate_token_buy(executor: &TransactionExecutor) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🛒 Token Buy Simulation (ETH -> Token)");
    
    println!("Select token to buy:");
    println!("1. USDC");
    println!("2. DAI");
    
    let mut choice = String::new();
    std::io::stdin().read_line(&mut choice)?;
    
    let (token_address, pool_address, token_name) = match choice.trim() {
        "1" => (USDC, WETH_USDC_V2, "USDC"),
        "2" => (DAI, WETH_DAI_V2, "DAI"),
        _ => {
            println!("Invalid choice");
            return Ok(());
        }
    };
    
    println!("Enter ETH amount to spend (or press Enter for 0.1): ");
    let mut amount = String::new();
    std::io::stdin().read_line(&mut amount)?;
    let amount = amount.trim();
    let eth_amount = if amount.is_empty() {
        parse_ether("0.1")?
    } else {
        parse_ether(amount)?
    };
    
    // Create buy alert
    let buy_alert = Alert {
        id: format!("buy_simulation_{}", chrono::Utc::now().timestamp()),
        timestamp: chrono::Utc::now().timestamp() as u64,
        token_address: token_address.parse()?,
        pool_address: pool_address.parse()?,
        action: Action::Buy,
        params: ExecutionParams {
            amount: eth_amount,
            slippage: 0.01, // 1%
            priority: Priority::Normal,
            max_gas_price: Some(parse_units("100", "gwei")?),
            deadline_seconds: 300,
        },
    };
    
    println!("\n📝 Buy Order Details:");
    println!("Buying: {}", token_name);
    println!("Spending: {} ETH", format_units(eth_amount, "ether")?);
    println!("Pool: {}", pool_address);
    println!("Max Slippage: 1%");
    
    println!("\n🚀 Simulating buy execution...");
    let result = executor.execute_alert(buy_alert).await;
    
    display_execution_result(&result);
    
    Ok(())
}

/// Simulate token sell (Token -> ETH)
async fn simulate_token_sell(executor: &TransactionExecutor) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💰 Token Sell Simulation (Token -> ETH)");
    
    println!("Select token to sell:");
    println!("1. WETH");
    println!("2. USDC");
    println!("3. DAI");
    
    let mut choice = String::new();
    std::io::stdin().read_line(&mut choice)?;
    
    let (token_address, pool_address, token_name, decimals) = match choice.trim() {
        "1" => (WETH, WETH_USDC_V2, "WETH", 18),
        "2" => (USDC, WETH_USDC_V2, "USDC", 6),
        "3" => (DAI, WETH_DAI_V2, "DAI", 18),
        _ => {
            println!("Invalid choice");
            return Ok(());
        }
    };
    
    println!("Enter {} amount to sell (or press Enter for max): ", token_name);
    let mut amount = String::new();
    std::io::stdin().read_line(&mut amount)?;
    let amount = amount.trim();
    let token_amount = if amount.is_empty() {
        U256::MAX // Sell all
    } else {
        parse_units(amount, decimals)?
    };
    
    // Create sell alert
    let sell_alert = Alert {
        id: format!("sell_simulation_{}", chrono::Utc::now().timestamp()),
        timestamp: chrono::Utc::now().timestamp() as u64,
        token_address: token_address.parse()?,
        pool_address: pool_address.parse()?,
        action: Action::Sell,
        params: ExecutionParams {
            amount: token_amount,
            slippage: 0.01, // 1%
            priority: Priority::High,
            max_gas_price: Some(parse_units("100", "gwei")?),
            deadline_seconds: 300,
        },
    };
    
    println!("\n📝 Sell Order Details:");
    println!("Selling: {}", token_name);
    println!("Amount: {}", if token_amount == U256::MAX { "ALL".to_string() } else { format!("{}", amount) });
    println!("Pool: {}", pool_address);
    println!("Max Slippage: 1%");
    
    println!("\n🚀 Simulating sell execution...");
    let result = executor.execute_alert(sell_alert).await;
    
    display_execution_result(&result);
    
    Ok(())
}

/// Check token balances
async fn check_balances(provider: &Arc<Provider<Http>>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💰 Checking Balances...");
    
    let wallet = KARTAL_WALLET.parse::<Address>()?;
    
    // ETH balance
    let eth_balance = provider.get_balance(wallet, None).await?;
    println!("ETH: {} ETH", format_units(eth_balance, "ether")?);
    
    // WETH balance (using balanceOf call)
    let weth_balance = get_token_balance(provider, WETH, wallet).await?;
    println!("WETH: {} WETH", format_units(weth_balance, 18)?);
    
    // USDC balance
    let usdc_balance = get_token_balance(provider, USDC, wallet).await?;
    println!("USDC: {} USDC", format_units(usdc_balance, 6)?);
    
    // DAI balance
    let dai_balance = get_token_balance(provider, DAI, wallet).await?;
    println!("DAI: {} DAI", format_units(dai_balance, 18)?);
    
    Ok(())
}

/// Get ERC20 token balance
async fn get_token_balance(
    provider: &Arc<Provider<Http>>, 
    token: &str, 
    wallet: Address
) -> Result<U256, Box<dyn std::error::Error>> {
    // ERC20 balanceOf(address) selector
    let data = ethers::abi::encode(&[
        ethers::abi::Token::Address(wallet),
    ]);
    let mut call_data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector
    call_data.extend_from_slice(&data);
    
    let tx = TransactionRequest::new()
        .to(token.parse::<Address>()?)
        .data(call_data);
    
    let result = provider.call(&tx.into(), None).await?;
    
    Ok(U256::from_big_endian(&result))
}

/// Display execution result
fn display_execution_result(result: &eth_kartal::tx_executor::ExecutionResult) {
    println!("\n📊 Execution Result:");
    println!("Success: {}", if result.success { "✅ YES" } else { "❌ NO" });
    
    if let Some(tx_hash) = result.tx_hash {
        println!("Transaction: {:?}", tx_hash);
    }
    
    if let Some(error) = &result.error {
        println!("Error: {}", error);
    }
    
    println!("\n⏱️  Performance Metrics:");
    println!("Position Check: {}ms", result.metrics.position_check_ms);
    println!("Gas Ranking: {}ms", result.metrics.gas_ranking_ms);
    println!("Price Quote: {}ms", result.metrics.price_quote_ms);
    println!("TX Build: {}ms", result.metrics.tx_build_ms);
    println!("TX Submit: {}ms", result.metrics.tx_submit_ms);
    println!("Total: {}ms", result.metrics.total_ms);
}

// Re-export some ethers utilities
use ethers::utils::{format_units, parse_ether, parse_units};