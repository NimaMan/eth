//! ETH Kartal - Main executable
//!
//! Automated trading protection system that responds to scam alerts

use clap::Parser;
use eth_kartal::{
    alert_processor::{AlertReceiver, ReceiverConfig},
    strategy::{DecisionEngine, DecisionConfig, TradingDecision},
    tx_executor::TransactionBuilder,
    wallet::PositionTracker,
};
use ethers::prelude::*;
use ethers::abi::Token;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "eth_kartal")]
#[command(about = "Automated trading protection system")]
struct Args {
    /// Ethereum RPC URL
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    rpc_url: String,
    
    /// ZMQ endpoint for alerts
    #[arg(long, env = "ALERT_ZMQ_ENDPOINT", default_value = "tcp://localhost:5559")]
    alert_endpoint: String,
    
    /// Wallet address to protect
    #[arg(long, env = "WALLET_ADDRESS")]
    wallet_address: String,
    
    /// Router address for swaps
    #[arg(long, env = "ROUTER_ADDRESS", default_value = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D")]
    router_address: String,
    
    /// Slippage tolerance (0.05 = 5%)
    #[arg(long, env = "SLIPPAGE_TOLERANCE", default_value = "0.05")]
    slippage: f64,
    
    /// Emergency threshold (% drain)
    #[arg(long, default_value = "80")]
    emergency_threshold: f64,
    
    /// Partial sell threshold (% drain)
    #[arg(long, default_value = "50")]
    partial_threshold: f64,
    
    /// Test mode (no real trades)
    #[arg(long)]
    test_mode: bool,
    
    /// Log level
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&args.log_level))
        .init();
    
    info!("🦅 ETH Kartal starting...");
    info!("Wallet: {}", args.wallet_address);
    info!("RPC: {}", args.rpc_url);
    info!("Alerts: {}", args.alert_endpoint);
    info!("Test mode: {}", args.test_mode);
    
    // Parse addresses
    let wallet_address: Address = args.wallet_address.parse()?;
    let router_address: Address = args.router_address.parse()?;
    
    // Create provider
    let provider = Arc::new(Provider::<Http>::try_from(&args.rpc_url)?);
    
    // Create position tracker
    let position_tracker = Arc::new(PositionTracker::new(
        provider.clone(),
        wallet_address,
    )?);
    
    // Check ETH balance
    let eth_balance = position_tracker.update_eth_balance().await?;
    if eth_balance == U256::zero() {
        warn!("⚠️  No ETH balance for gas!");
    }
    
    // Create decision engine
    let decision_config = DecisionConfig {
        emergency_threshold: args.emergency_threshold,
        partial_threshold: args.partial_threshold,
        test_mode: args.test_mode,
        ..Default::default()
    };
    let decision_engine = Arc::new(DecisionEngine::new(
        decision_config,
        position_tracker.clone(),
    ));
    
    // Create transaction builder
    let tx_builder = Arc::new(TransactionBuilder::new(
        provider.clone(),
        router_address,
        wallet_address,
        args.slippage,
    ));
    
    // Create alert channel
    let (alert_tx, mut alert_rx) = mpsc::channel(100);
    
    // Create and start alert receiver
    let receiver_config = ReceiverConfig {
        endpoint: args.alert_endpoint,
        ..Default::default()
    };
    let mut alert_receiver = AlertReceiver::new(receiver_config, alert_tx);
    alert_receiver.start()?;
    
    info!("✅ System initialized, waiting for alerts...");
    
    // Main alert processing loop
    while let Some(alert) = alert_rx.recv().await {
        // Process alert in separate task to avoid blocking
        let decision_engine = decision_engine.clone();
        let tx_builder = tx_builder.clone();
        let test_mode = args.test_mode;
        
        tokio::spawn(async move {
            match process_alert(alert, decision_engine, tx_builder, test_mode).await {
                Ok(_) => {},
                Err(e) => error!("Failed to process alert: {}", e),
            }
        });
    }
    
    warn!("Alert channel closed, shutting down");
    Ok(())
}

/// Process a single alert
async fn process_alert(
    alert: eth_kartal::alert_processor::ScamAlert,
    decision_engine: Arc<DecisionEngine>,
    tx_builder: Arc<TransactionBuilder>,
    test_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Make decision
    let decision = decision_engine.process_alert(&alert).await?;
    
    // Execute decision
    match decision {
        TradingDecision::EmergencySell { token, amount, reason } => {
            error!("🚨 EMERGENCY SELL: {} - {}", token, reason);
            
            if test_mode {
                warn!("TEST MODE: Would sell {} tokens", amount);
            } else {
                // Execute emergency sell
                execute_trade(tx_builder, token, amount, true).await?;
            }
        }
        TradingDecision::PartialSell { token, amount, percentage, reason } => {
            warn!("⚠️  PARTIAL SELL: {} ({:.1}%) - {}", token, percentage, reason);
            
            if test_mode {
                warn!("TEST MODE: Would sell {} tokens", amount);
            } else {
                // Execute partial sell
                execute_trade(tx_builder, token, amount, false).await?;
            }
        }
        TradingDecision::Monitor { token, reason } => {
            info!("👁️  MONITORING: {} - {}", token, reason);
        }
        TradingDecision::Skip { token, reason } => {
            info!("⏭️  SKIPPING: {} - {}", token, reason);
        }
    }
    
    Ok(())
}

/// Execute a trade (emergency or partial sell)
async fn execute_trade(
    tx_builder: Arc<TransactionBuilder>,
    token: Address,
    amount: U256,
    is_emergency: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = std::time::Instant::now();
    
    // Step 1: Check token approval
    info!("Checking token approval for {}", token);
    let provider = tx_builder.provider().clone();
    let router = tx_builder.router_address();
    let wallet = tx_builder.wallet_address();
    
    // Get current allowance (simplified - in production use proper ABI)
    let allowance = get_token_allowance(&provider, token, wallet, router).await?;
    
    if allowance < amount {
        info!("Setting token approval for {} tokens", amount);
        let approval_tx = tx_builder.build_token_approval(token, router, U256::MAX)?;
        
        // Send approval transaction
        let pending_tx = provider.send_transaction(approval_tx, None).await?;
        let receipt = pending_tx.await?;
        
        if let Some(receipt) = receipt {
            info!("Approval tx confirmed: {:?}", receipt.transaction_hash);
        } else {
            return Err("Approval transaction failed".into());
        }
    }
    
    // Step 2: Get expected output and calculate minimum
    let expected_eth = estimate_token_to_eth(&provider, router, token, amount).await?;
    let min_eth_out = tx_builder.calculate_min_output(expected_eth);
    
    info!(
        "Expected ETH output: {}, minimum: {}", 
        ethers::utils::format_ether(expected_eth),
        ethers::utils::format_ether(min_eth_out)
    );
    
    // Step 3: Build swap transaction
    let swap_tx = if is_emergency {
        tx_builder.build_emergency_sell(token, amount, min_eth_out).await?
    } else {
        tx_builder.build_partial_sell(token, amount, min_eth_out, None).await?
    };
    
    // Step 4: Submit with appropriate priority
    let priority_fee = if is_emergency {
        U256::from(10_000_000_000u64) // 10 gwei for emergency
    } else {
        U256::from(2_000_000_000u64) // 2 gwei for normal
    };
    
    // For TypedTransaction, we need to set gas fees differently based on transaction type
    let swap_tx = match swap_tx {
        TypedTransaction::Eip1559(mut tx) => {
            tx.max_priority_fee_per_gas = Some(priority_fee);
            tx.max_fee_per_gas = Some(priority_fee * 2);
            TypedTransaction::Eip1559(tx)
        }
        TypedTransaction::Legacy(mut tx) => {
            tx.gas_price = Some(priority_fee + U256::from(20_000_000_000u64)); // Base fee + priority
            TypedTransaction::Legacy(tx)
        }
        _ => swap_tx, // Other types unchanged
    };
    
    info!("Submitting swap transaction with {} gwei priority", priority_fee / 1_000_000_000);
    let pending_tx = provider.send_transaction(swap_tx, None).await?;
    
    // Step 5: Monitor execution
    let tx_hash = pending_tx.tx_hash();
    info!("Swap transaction submitted: {:?}", tx_hash);
    
    match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        pending_tx
    ).await {
        Ok(Ok(Some(receipt))) => {
            let elapsed = start_time.elapsed();
            info!(
                "✅ Trade executed successfully in {:.2}s - Gas used: {}", 
                elapsed.as_secs_f64(),
                receipt.gas_used.unwrap_or_default()
            );
            
            if receipt.status == Some(U64::from(0)) {
                error!("❌ Transaction reverted!");
                return Err("Transaction reverted".into());
            }
        }
        Ok(Ok(None)) => {
            error!("❌ Transaction disappeared");
            return Err("Transaction disappeared".into());
        }
        Ok(Err(e)) => {
            error!("❌ Transaction failed: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            error!("❌ Transaction timeout after 60 seconds");
            return Err("Transaction timeout".into());
        }
    }
    
    Ok(())
}

/// Get token allowance (simplified version)
async fn get_token_allowance(
    provider: &Provider<Http>,
    token: Address,
    owner: Address,
    spender: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    // ERC20 allowance function
    let data = encode_function_data(
        "allowance(address,address)",
        &[Token::Address(owner), Token::Address(spender)]
    )?;
    
    let tx = TransactionRequest::new()
        .to(token)
        .data(data);
    
    let result = provider.call(&tx.into(), None).await?;
    
    // Decode uint256 result
    if result.len() >= 32 {
        Ok(U256::from_big_endian(&result[..32]))
    } else {
        Ok(U256::zero())
    }
}

/// Estimate token to ETH swap output
async fn estimate_token_to_eth(
    provider: &Provider<Http>,
    router: Address,
    token: Address,
    amount: U256,
) -> Result<U256, Box<dyn std::error::Error>> {
    use eth_kartal::tx_executor::routers;
    
    // getAmountsOut function
    let path = vec![token, *routers::WETH];
    let data = encode_function_data(
        "getAmountsOut(uint256,address[])",
        &[
            Token::Uint(amount),
            Token::Array(path.iter().map(|&a| Token::Address(a)).collect()),
        ]
    )?;
    
    let tx = TransactionRequest::new()
        .to(router)
        .data(data);
    
    let result = provider.call(&tx.into(), None).await?;
    
    // Decode uint256[] result - ETH amount is second element
    if result.len() >= 64 {
        Ok(U256::from_big_endian(&result[32..64]))
    } else {
        Err("Invalid getAmountsOut response".into())
    }
}

/// Encode function call data
fn encode_function_data(
    signature: &str,
    params: &[Token],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let selector = &ethers::utils::keccak256(signature.as_bytes())[0..4];
    let encoded_params = ethers::abi::encode(params);
    
    let mut data = selector.to_vec();
    data.extend_from_slice(&encoded_params);
    
    Ok(data)
}