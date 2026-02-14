//! Signal Processor Example - Shows how ETH Kartal processes database signals
//!
//! This example demonstrates:
//! - Polling the database for new trading signals
//! - Processing BUY and SELL signals with proper amounts
//! - Updating signal status and writing execution results
//! - Handling position percentage calculations
//!
//! Usage:
//!   cargo run --example signal_processor

use eth_kartal::{
    alert_processor::{Action, Alert, ExecutionParams, Priority},
    risk::RiskConfig,
    tx_executor::{ExecutionResult, ExecutorConfig, TransactionExecutor},
};
use ethers::prelude::*;
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::{env, path::PathBuf, sync::Arc};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Database signal structure
#[derive(Debug, Clone)]
struct TradeSignal {
    id: i32,
    signal_id: Uuid,
    wallet_id: i32,
    token_address: String,
    pool_address: String,
    action: String,
    amount: String,
    signal_value: Value,
    signal_data: Value,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("eth_kartal=info,signal_processor=info")
        .init();

    println!("🚀 ETH Kartal Signal Processor");
    println!("===============================\n");

    // Database connection
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:password@localhost:5432/live_trading_db".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✅ Connected to database");

    // Create executor
    let executor = create_executor().await?;
    println!("✅ Transaction executor initialized");

    // Start processing loop
    println!("\n📡 Starting signal processing loop...");
    println!("   Polling every 1 second for new signals\n");

    loop {
        match process_pending_signals(&pool, &executor).await {
            Ok(count) => {
                if count > 0 {
                    println!("✅ Processed {} signals", count);
                }
            }
            Err(e) => {
                eprintln!("❌ Error processing signals: {}", e);
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

async fn create_executor() -> Result<Arc<TransactionExecutor>, Box<dyn std::error::Error>> {
    let config = ExecutorConfig {
        keystore_path: PathBuf::from("./keystore.json"),
        chain_id: 1,
        rpc_url: env::var("ETH_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string()),
        flashbots_enabled: true,
        flashbots_rpc: Some("https://relay.flashbots.net".to_string()),
        reth_ws_url: "ws://localhost:8546".to_string(),
        risk_config: RiskConfig::default(),
        rabbitmq_url: None,
    };

    let executor = Arc::new(TransactionExecutor::new(config).await?);

    // For demo, use test password
    let password = secrecy::Secret::new("test".to_string());
    executor.unlock_wallet(password).await?;

    Ok(executor)
}

async fn process_pending_signals(
    pool: &Pool<Postgres>,
    executor: &Arc<TransactionExecutor>,
) -> Result<usize, Box<dyn std::error::Error>> {
    // Fetch pending signals
    let signals = fetch_pending_signals(pool).await?;

    if signals.is_empty() {
        return Ok(0);
    }

    println!("📋 Found {} pending signals", signals.len());

    for signal in signals {
        println!("\n🔄 Processing signal: {}", signal.signal_id);
        println!("   Action: {} {}", signal.action, signal.token_address);

        // Update status to SENT
        update_signal_status(pool, signal.id, "SENT").await?;

        // Convert to Alert
        match convert_signal_to_alert(&signal, executor.as_ref()).await {
            Ok(alert) => {
                println!("   Amount: {}", alert.params.amount);
                println!("   Slippage: {}%", alert.params.slippage * 100.0);

                // Execute the trade
                match executor.execute_alert(alert).await {
                    Ok(result) => {
                        handle_execution_success(pool, &signal, result).await?;
                    }
                    Err(e) => {
                        handle_execution_failure(pool, &signal, e.to_string()).await?;
                    }
                }
            }
            Err(e) => {
                eprintln!("   ❌ Failed to convert signal: {}", e);
                update_signal_status(pool, signal.id, "FAILED").await?;
            }
        }
    }

    Ok(signals.len())
}

async fn fetch_pending_signals(
    pool: &Pool<Postgres>,
) -> Result<Vec<TradeSignal>, Box<dyn std::error::Error>> {
    let query = r#"
        SELECT 
            id,
            signal_id,
            wallet_id,
            token_address,
            pool_address,
            action,
            amount,
            signal_value,
            signal_data
        FROM trade_signals
        WHERE status = 'PENDING'
        ORDER BY created_at ASC
        LIMIT 10
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

    let signals = rows
        .into_iter()
        .map(|row| TradeSignal {
            id: row.get(0),
            signal_id: row.get(1),
            wallet_id: row.get(2),
            token_address: row.get(3),
            pool_address: row.get(4),
            action: row.get(5),
            amount: row.get(6),
            signal_value: row.get(7),
            signal_data: row.get(8),
        })
        .collect();

    Ok(signals)
}

async fn convert_signal_to_alert(
    signal: &TradeSignal,
    executor: &TransactionExecutor,
) -> Result<Alert, Box<dyn std::error::Error>> {
    let token_address: Address = signal.token_address.parse()?;
    let pool_address: Address = signal.pool_address.parse()?;

    // Determine action
    let action = match signal.action.as_str() {
        "BUY" => Action::Buy,
        "SELL" => Action::Sell,
        _ => return Err("Invalid action".into()),
    };

    // Calculate amount
    let amount = if signal.action == "BUY" {
        // For BUY, amount is ETH to spend (already in wei)
        U256::from_dec_str(&signal.amount)?
    } else {
        // For SELL, need to calculate based on percentage
        if let Some(pct) = signal.signal_value["percentage"].as_f64() {
            calculate_sell_amount(executor, token_address, pct).await?
        } else {
            // Direct amount specified
            U256::from_dec_str(&signal.amount)?
        }
    };

    // Extract parameters from signal_data
    let slippage = signal.signal_data["slippage"].as_f64().unwrap_or(0.03);

    let priority = match signal.signal_data["priority"].as_str() {
        Some("critical") => Priority::Critical,
        Some("high") => Priority::High,
        _ => Priority::Normal,
    };

    let max_gas_price = signal.signal_data["max_gas_price"]
        .as_str()
        .and_then(|s| U256::from_dec_str(s).ok());

    let deadline_seconds = signal.signal_data["deadline_seconds"]
        .as_u64()
        .unwrap_or(300);

    Ok(Alert {
        id: signal.signal_id.to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        token_address,
        pool_address,
        action,
        params: ExecutionParams {
            amount,
            slippage,
            max_gas_price,
            deadline_seconds,
            priority,
        },
    })
}

async fn calculate_sell_amount(
    executor: &TransactionExecutor,
    token_address: Address,
    percentage: f64,
) -> Result<U256, Box<dyn std::error::Error>> {
    // Get current token balance
    let provider = executor.get_provider();
    let wallet_address = executor.wallet_address();

    // ERC20 balanceOf call
    let balance_data = provider
        .call(
            &TransactionRequest::new()
                .to(token_address)
                .data(ethers::abi::encode(&[
                    ethers::abi::Token::FixedBytes(
                        ethers::utils::keccak256("balanceOf(address)")[0..4].to_vec(),
                    ),
                    ethers::abi::Token::Address(wallet_address),
                ])),
            None,
        )
        .await?;

    let balance = U256::from_big_endian(&balance_data);

    // Calculate percentage
    let sell_amount = balance * U256::from((percentage * 100.0) as u64) / U256::from(10000);

    println!("   Current balance: {}", balance);
    println!("   Selling {}%: {}", percentage, sell_amount);

    Ok(sell_amount)
}

async fn update_signal_status(
    pool: &Pool<Postgres>,
    signal_id: i32,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = "UPDATE trade_signals SET status = $1, updated_at = NOW() WHERE id = $2";

    sqlx::query(query)
        .bind(status)
        .bind(signal_id)
        .execute(pool)
        .await?;

    Ok(())
}

async fn handle_execution_success(
    pool: &Pool<Postgres>,
    signal: &TradeSignal,
    result: ExecutionResult,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("   ✅ Execution successful!");
    println!("   TX Hash: {:?}", result.tx_hash);
    println!("   Total time: {}ms", result.metrics.total_ms);

    // Update signal status
    update_signal_status(pool, signal.id, "CONFIRMED").await?;

    // Insert execution record
    let query = r#"
        INSERT INTO executions (
            signal_id,
            wallet_id,
            tx_hash,
            block_number,
            gas_used,
            gas_price,
            status,
            execution_time_ms,
            error_message,
            created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, 'SUCCESS', $7, NULL, NOW())
    "#;

    let tx_hash = result.tx_hash.map(|h| format!("{:?}", h));

    sqlx::query(query)
        .bind(signal.signal_id)
        .bind(signal.wallet_id)
        .bind(tx_hash)
        .bind(0i64) // Block number - would be filled by confirmation monitor
        .bind(150000i64) // Estimated gas used
        .bind("40000000000") // Gas price in wei
        .bind(result.metrics.total_ms as i32)
        .execute(pool)
        .await?;

    Ok(())
}

async fn handle_execution_failure(
    pool: &Pool<Postgres>,
    signal: &TradeSignal,
    error: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("   ❌ Execution failed: {}", error);

    // Update signal status
    update_signal_status(pool, signal.id, "FAILED").await?;

    // Insert failed execution record
    let query = r#"
        INSERT INTO executions (
            signal_id,
            wallet_id,
            status,
            error_message,
            created_at
        ) VALUES ($1, $2, 'FAILED', $3, NOW())
    "#;

    sqlx::query(query)
        .bind(signal.signal_id)
        .bind(signal.wallet_id)
        .bind(error)
        .execute(pool)
        .await?;

    Ok(())
}

// Mock provider extension for demo
trait MockProvider {
    fn get_provider(&self) -> Provider<Http>;
}

impl MockProvider for TransactionExecutor {
    fn get_provider(&self) -> Provider<Http> {
        // In real implementation, this would return the internal provider
        Provider::<Http>::try_from("http://localhost:8545").unwrap()
    }
}
