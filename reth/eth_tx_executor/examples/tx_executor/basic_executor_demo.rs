//! Basic Transaction Executor Demo
//! 
//! This example demonstrates how to use ETH Kartal's transaction executor
//! to perform high-speed token trades on Ethereum mainnet.
//!
//! WARNING: This example uses REAL transactions. Use --dry-run flag for testing.
//!
//! Usage:
//!   cargo run --example basic_executor_demo -- --help
//!   cargo run --example basic_executor_demo -- --dry-run --action sell --token USDC --amount 100
//!   cargo run --example basic_executor_demo -- --keystore ./keystore.json --action buy --token UNI --eth-amount 0.1

use clap::Parser;
use eth_kartal::{
    alert_processor::{Alert, Action, ExecutionParams, Priority},
    tx_executor::{TransactionExecutor, ExecutorConfig, ExecutionResult},
    risk::RiskConfig,
    wallet::read_password,
};
use ethers::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, error, warn};
use tracing_subscriber::EnvFilter;

/// Common token addresses on Ethereum mainnet
mod tokens {
    pub const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    pub const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    pub const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
    pub const DAI: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
    pub const UNI: &str = "0x1f9840a85d5aF5bf1D1762F925BDADdc4201F984";
    pub const LINK: &str = "0x514910771AF9Ca656af840dff83E8264EcF986CA";
}

/// Common Uniswap V2 pools
mod pools {
    pub const USDC_ETH: &str = "0xB4e16d0168e52d35CaCd2c6185b44281Ec28C9Dc";
    pub const USDT_ETH: &str = "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852";
    pub const DAI_ETH: &str = "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11";
    pub const UNI_ETH: &str = "0xd3d2E2692501A5c9Ca623199D38826e513033a17";
    pub const LINK_ETH: &str = "0xa2107FA5B38d9bbd2C461D6EDf11B11A50F6b974";
}

#[derive(Parser, Debug)]
#[command(author, version, about = "ETH Kartal Transaction Executor Demo", long_about = None)]
struct Args {
    /// Path to keystore file
    #[arg(long, default_value = "./keystore.json")]
    keystore: PathBuf,
    
    /// Ethereum RPC endpoint
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    rpc_url: String,
    
    /// Action to perform (buy or sell)
    #[arg(long, value_enum)]
    action: TradeAction,
    
    /// Token symbol to trade
    #[arg(long)]
    token: TokenSymbol,
    
    /// Amount of tokens to sell (for sell action)
    #[arg(long, conflicts_with = "eth_amount")]
    amount: Option<f64>,
    
    /// Amount of ETH to spend (for buy action)
    #[arg(long, conflicts_with = "amount")]
    eth_amount: Option<f64>,
    
    /// Slippage tolerance (default: 3%)
    #[arg(long, default_value = "3.0")]
    slippage: f64,
    
    /// Transaction priority
    #[arg(long, default_value = "normal")]
    priority: TxPriority,
    
    /// Enable Flashbots for MEV protection
    #[arg(long)]
    flashbots: bool,
    
    /// Dry run - simulate without executing
    #[arg(long)]
    dry_run: bool,
    
    /// Maximum gas price in gwei
    #[arg(long, default_value = "100")]
    max_gas_gwei: u64,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum TradeAction {
    Buy,
    Sell,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum TokenSymbol {
    #[value(name = "USDC")]
    Usdc,
    #[value(name = "USDT")]
    Usdt,
    #[value(name = "DAI")]
    Dai,
    #[value(name = "UNI")]
    Uni,
    #[value(name = "LINK")]
    Link,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum TxPriority {
    Critical,
    High,
    Normal,
}

impl TokenSymbol {
    fn address(&self) -> Address {
        let addr = match self {
            TokenSymbol::Usdc => tokens::USDC,
            TokenSymbol::Usdt => tokens::USDT,
            TokenSymbol::Dai => tokens::DAI,
            TokenSymbol::Uni => tokens::UNI,
            TokenSymbol::Link => tokens::LINK,
        };
        addr.parse().expect("Invalid token address")
    }
    
    fn pool_address(&self) -> Address {
        let addr = match self {
            TokenSymbol::Usdc => pools::USDC_ETH,
            TokenSymbol::Usdt => pools::USDT_ETH,
            TokenSymbol::Dai => pools::DAI_ETH,
            TokenSymbol::Uni => pools::UNI_ETH,
            TokenSymbol::Link => pools::LINK_ETH,
        };
        addr.parse().expect("Invalid pool address")
    }
    
    fn decimals(&self) -> u8 {
        match self {
            TokenSymbol::Usdc | TokenSymbol::Usdt => 6,
            TokenSymbol::Dai | TokenSymbol::Uni | TokenSymbol::Link => 18,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("eth_kartal=info".parse()?)
            .add_directive("basic_executor_demo=info".parse()?))
        .init();
    
    let args = Args::parse();
    
    info!("🚀 ETH Kartal Transaction Executor Demo");
    info!("=====================================");
    
    // Validate arguments
    match args.action {
        TradeAction::Buy => {
            if args.eth_amount.is_none() {
                return Err("--eth-amount required for buy action".into());
            }
        }
        TradeAction::Sell => {
            if args.amount.is_none() {
                return Err("--amount required for sell action".into());
            }
        }
    }
    
    // Show configuration
    info!("Configuration:");
    info!("  Action: {:?}", args.action);
    info!("  Token: {:?} ({})", args.token, args.token.address());
    info!("  Pool: {}", args.token.pool_address());
    info!("  Slippage: {}%", args.slippage);
    info!("  Priority: {:?}", args.priority);
    info!("  Flashbots: {}", args.flashbots);
    info!("  Dry Run: {}", args.dry_run);
    info!("");
    
    if args.dry_run {
        warn!("🏃 DRY RUN MODE - No real transactions will be executed");
    } else {
        warn!("⚠️  REAL TRANSACTION MODE - This will execute actual trades!");
        warn!("Press Ctrl+C now to cancel, or wait 5 seconds to continue...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
    
    // Setup provider
    let provider = Provider::<Http>::try_from(&args.rpc_url)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    info!("Connected to chain ID: {}", chain_id);
    
    // Create executor configuration
    let config = ExecutorConfig {
        keystore_path: args.keystore.clone(),
        chain_id,
        rpc_url: args.rpc_url.clone(),
        flashbots_enabled: args.flashbots,
        flashbots_rpc: if args.flashbots {
            Some("https://relay.flashbots.net".to_string())
        } else {
            None
        },
        reth_ws_url: "ws://localhost:8546".to_string(), // Required but not used in this demo
        risk_config: RiskConfig {
            max_position_size_eth: 10.0,
            max_daily_loss_eth: 50.0,
            max_slippage_allowed: 0.2, // 20%
            min_liquidity_eth: 5.0,
            circuit_breaker_enabled: true,
            max_consecutive_failures: 3,
        },
        rabbitmq_url: None,
    };
    
    // Create executor
    info!("Initializing transaction executor...");
    let executor = Arc::new(TransactionExecutor::new(config).await?);
    
    // Get wallet password
    let password = if args.dry_run {
        // Use dummy password for dry run
        secrecy::Secret::new("test".to_string())
    } else {
        read_password("Enter keystore password: ")?
    };
    
    executor.unlock_wallet(password).await?;
    info!("✅ Wallet unlocked successfully");
    
    // Check balances
    let wallet_address = executor.wallet_address();
    info!("Wallet address: {}", wallet_address);
    
    let eth_balance = provider.get_balance(wallet_address, None).await?;
    info!("ETH balance: {} ETH", ethers::utils::format_ether(eth_balance));
    
    // Create trading alert
    let alert = create_alert(&args)?;
    info!("\n📋 Trading Alert Created:");
    info!("  ID: {}", alert.id);
    info!("  Action: {:?}", alert.action);
    info!("  Amount: {}", alert.params.amount);
    info!("  Max Gas: {:?}", alert.params.max_gas_price);
    
    if args.dry_run {
        info!("\n🔍 DRY RUN - Simulating execution...");
        simulate_execution(&alert);
    } else {
        info!("\n⚡ Executing trade...");
        match executor.execute_alert(alert).await {
            Ok(result) => {
                handle_success(result);
            }
            Err(e) => {
                handle_error(e);
            }
        }
    }
    
    Ok(())
}

fn create_alert(args: &Args) -> Result<Alert, Box<dyn std::error::Error>> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    
    let (action, amount) = match args.action {
        TradeAction::Buy => {
            let eth_amount = args.eth_amount.unwrap();
            let amount_wei = ethers::utils::parse_ether(eth_amount)?;
            (Action::Buy, amount_wei)
        }
        TradeAction::Sell => {
            let token_amount = args.amount.unwrap();
            let decimals = args.token.decimals();
            let amount_wei = ethers::utils::parse_units(token_amount, decimals)?;
            (Action::Sell, amount_wei)
        }
    };
    
    let priority = match args.priority {
        TxPriority::Critical => Priority::Critical,
        TxPriority::High => Priority::High,
        TxPriority::Normal => Priority::Normal,
    };
    
    let max_gas_price = Some(ethers::utils::parse_units(args.max_gas_gwei, "gwei")?);
    
    Ok(Alert {
        id: format!("demo_{}_{}", 
            match action {
                Action::Buy => "buy",
                Action::Sell => "sell",
                _ => "unknown",
            },
            timestamp
        ),
        timestamp,
        token_address: args.token.address(),
        pool_address: args.token.pool_address(),
        action,
        params: ExecutionParams {
            amount,
            slippage: args.slippage / 100.0, // Convert percentage to decimal
            max_gas_price,
            deadline_seconds: 300, // 5 minutes
            priority,
        },
    })
}

fn simulate_execution(alert: &Alert) {
    info!("\n📊 Simulation Results:");
    info!("  Would execute: {:?}", alert.action);
    info!("  Token: {:?}", alert.token_address);
    info!("  Pool: {:?}", alert.pool_address);
    info!("  Amount: {}", alert.params.amount);
    info!("  Slippage: {}%", alert.params.slippage * 100.0);
    
    // Simulate execution metrics
    info!("\n⏱️  Simulated Performance Metrics:");
    info!("  Alert processing: 3ms");
    info!("  Position check: 15ms");
    info!("  Gas optimization: 25ms");
    info!("  Price quote: 18ms");
    info!("  Transaction build: 8ms");
    info!("  Submission: 45ms");
    info!("  Total latency: 114ms");
    
    info!("\n✅ Simulation complete - no real transaction executed");
}

fn handle_success(result: ExecutionResult) {
    info!("\n✅ TRADE EXECUTED SUCCESSFULLY!");
    info!("=====================================");
    
    if let Some(tx_hash) = result.tx_hash {
        info!("Transaction Hash: {:?}", tx_hash);
        info!("View on Etherscan: https://etherscan.io/tx/{:?}", tx_hash);
    }
    
    info!("\n⏱️  Performance Metrics:");
    info!("  Alert → Start: {}ms", result.metrics.alert_to_start_ms);
    info!("  Position Check: {}ms", result.metrics.position_check_ms);
    info!("  Gas Optimization: {}ms", result.metrics.gas_ranking_ms);
    info!("  Price Quote: {}ms", result.metrics.price_quote_ms);
    info!("  TX Build: {}ms", result.metrics.tx_build_ms);
    info!("  TX Submit: {}ms", result.metrics.tx_submit_ms);
    info!("  ================");
    info!("  Total Time: {}ms", result.metrics.total_ms);
    
    if result.metrics.total_ms < 200 {
        info!("\n🚀 EXCELLENT! Sub-200ms execution achieved!");
    } else if result.metrics.total_ms < 500 {
        info!("\n⚡ Good performance - under 500ms");
    } else {
        warn!("\n⚠️  Execution took longer than expected");
    }
}

fn handle_error(error: Box<dyn std::error::Error>) {
    error!("\n❌ EXECUTION FAILED!");
    error!("=====================================");
    error!("Error: {}", error);
    
    // Provide helpful suggestions based on error
    let error_str = error.to_string();
    
    if error_str.contains("No tokens to sell") {
        info!("\n💡 Suggestion: Check that you have a balance of the token you're trying to sell");
    } else if error_str.contains("Insufficient ETH") {
        info!("\n💡 Suggestion: Ensure your wallet has enough ETH for gas fees");
    } else if error_str.contains("slippage") {
        info!("\n💡 Suggestion: Try increasing the slippage tolerance with --slippage");
    } else if error_str.contains("nonce") {
        info!("\n💡 Suggestion: Wait for pending transactions to confirm or clear them");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_addresses() {
        // Verify all token addresses are valid
        assert!(tokens::WETH.parse::<Address>().is_ok());
        assert!(tokens::USDC.parse::<Address>().is_ok());
        assert!(tokens::USDT.parse::<Address>().is_ok());
        assert!(tokens::DAI.parse::<Address>().is_ok());
        assert!(tokens::UNI.parse::<Address>().is_ok());
        assert!(tokens::LINK.parse::<Address>().is_ok());
    }
    
    #[test]
    fn test_pool_addresses() {
        // Verify all pool addresses are valid
        assert!(pools::USDC_ETH.parse::<Address>().is_ok());
        assert!(pools::USDT_ETH.parse::<Address>().is_ok());
        assert!(pools::DAI_ETH.parse::<Address>().is_ok());
        assert!(pools::UNI_ETH.parse::<Address>().is_ok());
        assert!(pools::LINK_ETH.parse::<Address>().is_ok());
    }
}