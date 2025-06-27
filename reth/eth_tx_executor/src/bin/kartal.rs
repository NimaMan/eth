//! ETH Kartal - High-Performance Transaction Executor
//! 
//! Alert → Execution pipeline with sub-200ms target latency

use clap::Parser;
use eth_kartal::{
    alert_processor::{AlertReceiver, ReceiverConfig, Alert},
    tx_executor::{TransactionExecutor, ExecutorConfig},
    wallet::read_password,
};
use std::time::Instant;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, error};
use tracing_subscriber::{EnvFilter, fmt};
use secrecy::Secret;

/// CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to keystore file
    #[arg(long, env = "ETH_KEYSTORE_PATH")]
    keystore_path: PathBuf,
    
    /// RPC endpoint URL
    #[arg(long, default_value = "http://127.0.0.1:8545", env = "ETH_RPC_URL")]
    rpc_url: String,
    
    /// Chain ID (1 for mainnet)
    #[arg(long, default_value = "1", env = "ETH_CHAIN_ID")]
    chain_id: u64,
    
    /// ZMQ alert endpoint
    #[arg(long, default_value = "tcp://localhost:5559", env = "ALERT_ENDPOINT")]
    alert_endpoint: String,
    
    /// Enable flashbots for critical alerts
    #[arg(long, env = "FLASHBOTS_ENABLED")]
    flashbots: bool,
    
    /// Flashbots RPC endpoint
    #[arg(long, env = "FLASHBOTS_RPC")]
    flashbots_rpc: Option<String>,
    
    /// Reth WebSocket URL for mempool monitoring
    #[arg(long, default_value = "ws://127.0.0.1:8546", env = "RETH_WS_URL")]
    reth_ws_url: String,
    
    /// Test mode (simulates transactions)
    #[arg(long)]
    test_mode: bool,
    
    /// Log level
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&args.log_level));
    
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
    
    info!("Starting ETH Kartal Transaction Executor");
    info!("RPC: {}", args.rpc_url);
    info!("Chain ID: {}", args.chain_id);
    info!("Alert endpoint: {}", args.alert_endpoint);
    info!("Test mode: {}", args.test_mode);
    
    // Create executor
    let executor_config = ExecutorConfig {
        keystore_path: args.keystore_path.clone(),
        chain_id: args.chain_id,
        rpc_url: args.rpc_url.clone(),
        flashbots_enabled: args.flashbots,
        flashbots_rpc: args.flashbots_rpc,
        reth_ws_url: args.reth_ws_url,
    };
    
    let executor = TransactionExecutor::new(executor_config).await?;
    info!("Transaction executor initialized");
    
    // Prompt for password to unlock wallet
    let password = read_password("Enter keystore password: ")?;
    executor.unlock_wallet(password).await?;
    info!("Wallet unlocked successfully");
    
    // Create alert channel
    let (alert_tx, mut alert_rx) = mpsc::channel::<Alert>(100);
    
    // Create alert receiver
    let receiver_config = ReceiverConfig {
        endpoint: args.alert_endpoint,
        ..Default::default()
    };
    
    let receiver = AlertReceiver::new(receiver_config, alert_tx);
    
    // Start alert receiver in background
    let receiver_handle = tokio::spawn(async move {
        receiver.run().await;
    });
    
    info!("Alert receiver started");
    
    // Execution metrics
    let mut total_alerts = 0u64;
    let mut successful_executions = 0u64;
    let mut failed_executions = 0u64;
    let mut total_latency_ms = 0u64;
    
    // Main execution loop
    while let Some(alert) = alert_rx.recv().await {
        total_alerts += 1;
        let start_time = Instant::now();
        
        info!("Received alert: {} for token {}", alert.id, alert.token_address);
        
        // Execute alert
        let result = if args.test_mode {
            info!("TEST MODE: Would execute {} action for {} tokens",
                match alert.action {
                    eth_kartal::alert_processor::Action::Sell => "SELL",
                    eth_kartal::alert_processor::Action::Buy => "BUY",
                    _ => "OTHER",
                },
                alert.params.amount
            );
            
            // Simulate execution
            eth_kartal::tx_executor::ExecutionResult {
                alert_id: alert.id,
                tx_hash: Some(ethers::types::H256::random()),
                success: true,
                error: None,
                metrics: eth_kartal::tx_executor::ExecutionMetrics {
                    alert_to_start_ms: 5,
                    position_check_ms: 15,
                    gas_ranking_ms: 25,
                    price_quote_ms: 20,
                    tx_build_ms: 10,
                    tx_submit_ms: 15,
                    total_ms: 90,
                },
            }
        } else {
            executor.execute_alert(alert).await
        };
        
        let latency = start_time.elapsed().as_millis() as u64;
        total_latency_ms += latency;
        
        if result.success {
            successful_executions += 1;
            info!("✅ Execution successful: {:?} in {}ms", 
                result.tx_hash, result.metrics.total_ms);
            
            // Log detailed metrics
            info!("  Alert→Start: {}ms", result.metrics.alert_to_start_ms);
            info!("  Position check: {}ms", result.metrics.position_check_ms);
            info!("  Gas ranking: {}ms", result.metrics.gas_ranking_ms);
            info!("  Price quote: {}ms", result.metrics.price_quote_ms);
            info!("  TX build: {}ms", result.metrics.tx_build_ms);
            info!("  TX submit: {}ms", result.metrics.tx_submit_ms);
        } else {
            failed_executions += 1;
            error!("❌ Execution failed: {} in {}ms", 
                result.error.as_ref().unwrap_or(&"Unknown error".to_string()),
                latency);
        }
        
        // Print running statistics
        if total_alerts % 10 == 0 {
            let avg_latency = total_latency_ms / total_alerts;
            let success_rate = (successful_executions as f64 / total_alerts as f64) * 100.0;
            
            info!("📊 Statistics: {} alerts, {:.1}% success rate, {}ms avg latency",
                total_alerts, success_rate, avg_latency);
        }
    }
    
    // Wait for receiver to finish
    receiver_handle.await?;
    
    Ok(())
}

/// Create a test alert for development
#[allow(dead_code)]
fn create_test_alert() -> Alert {
    use eth_kartal::alert_processor::{Action, ExecutionParams, Priority};
    
    Alert {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        token_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(), // USDC
        pool_address: "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".parse().unwrap(), // USDC/WETH V2
        action: Action::Sell,
        params: ExecutionParams {
            amount: ethers::types::U256::from(1000_000_000), // 1000 USDC
            slippage: 0.05,
            max_gas_price: None,
            deadline_seconds: 300,
            priority: Priority::High,
        },
    }
}