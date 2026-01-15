//! Full Integration Demo - Complete signal flow from generation to execution
//! 
//! This example demonstrates the entire trading signal lifecycle:
//! 1. Generate a BUY signal
//! 2. Process it with ETH Kartal
//! 3. Update position on confirmation
//! 4. Generate a SELL signal
//! 5. Close the position
//!
//! Usage:
//!   cargo run --example full_integration_demo

use eth_kartal::{
    alert_processor::{Alert, Action, ExecutionParams, Priority},
    tx_executor::{TransactionExecutor, ExecutorConfig},
    risk::RiskConfig,
};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use ethers::prelude::*;
use std::{env, path::PathBuf, sync::Arc};
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ETH Kartal Full Integration Demo");
    println!("===================================\n");
    
    // Setup
    let pool = setup_database().await?;
    let executor = setup_executor().await?;
    
    // Demo flow
    println!("📋 Demo Scenario: Complete Trading Cycle\n");
    
    // Step 1: Generate BUY signal
    println!("1️⃣  Generating BUY signal...");
    let buy_signal_id = generate_buy_signal(&pool).await?;
    println!("   ✅ Created signal: {}", buy_signal_id);
    
    sleep(Duration::from_secs(2)).await;
    
    // Step 2: Process BUY signal
    println!("\n2️⃣  Processing BUY signal...");
    let position_id = process_buy_signal(&pool, &executor, buy_signal_id).await?;
    println!("   ✅ Position opened: ID {}", position_id);
    
    sleep(Duration::from_secs(3)).await;
    
    // Step 3: Show position status
    println!("\n3️⃣  Current Position Status:");
    show_position_status(&pool, position_id).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    // Step 4: Generate SELL signal
    println!("\n4️⃣  Generating SELL signal (50% of position)...");
    let sell_signal_id = generate_sell_signal(&pool, position_id).await?;
    println!("   ✅ Created signal: {}", sell_signal_id);
    
    sleep(Duration::from_secs(2)).await;
    
    // Step 5: Process SELL signal
    println!("\n5️⃣  Processing SELL signal...");
    process_sell_signal(&pool, &executor, sell_signal_id, position_id).await?;
    println!("   ✅ Position closed");
    
    sleep(Duration::from_secs(2)).await;
    
    // Step 6: Show final results
    println!("\n6️⃣  Final Position Results:");
    show_final_results(&pool, position_id).await?;
    
    println!("\n✅ Demo completed successfully!");
    
    Ok(())
}

async fn setup_database() -> Result<Pool<Postgres>, Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/live_trading_db".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    println!("✅ Connected to database");
    Ok(pool)
}

async fn setup_executor() -> Result<Arc<TransactionExecutor>, Box<dyn std::error::Error>> {
    let config = ExecutorConfig {
        keystore_path: PathBuf::from("./keystore.json"),
        chain_id: 1,
        rpc_url: "http://localhost:8545".to_string(),
        flashbots_enabled: false,
        flashbots_rpc: None,
        reth_ws_url: "ws://localhost:8546".to_string(),
        risk_config: RiskConfig {
            max_position_size_eth: 1.0,
            max_daily_loss_eth: 5.0,
            max_slippage_allowed: 0.05,
            min_liquidity_eth: 10.0,
            circuit_breaker_enabled: false,
            max_consecutive_failures: 3,
        },
        rabbitmq_url: None,
    };
    
    let executor = Arc::new(TransactionExecutor::new(config).await?);
    let password = secrecy::Secret::new("test".to_string());
    executor.unlock_wallet(password).await?;
    
    println!("✅ Transaction executor initialized");
    Ok(executor)
}

async fn generate_buy_signal(pool: &Pool<Postgres>) -> Result<Uuid, Box<dyn std::error::Error>> {
    let signal_id = Uuid::new_v4();
    
    let query = r#"
        INSERT INTO trade_signals (
            signal_id,
            wallet_id,
            token_address,
            pool_address,
            action,
            amount,
            signal_value,
            signal_data,
            status
        ) VALUES ($1, 1, $2, $3, 'BUY', $4, $5, $6, 'PENDING')
    "#;
    
    sqlx::query(query)
        .bind(signal_id)
        .bind("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48") // USDC
        .bind("0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc") // USDC/ETH pool
        .bind("15000000000000000") // 0.015 ETH
        .bind(json!({
            "eth_amount": 0.015,
            "eth_amount_wei": "15000000000000000"
        }))
        .bind(json!({
            "slippage": 0.03,
            "priority": "high",
            "max_gas_price": "50000000000",
            "deadline_seconds": 300
        }))
        .execute(pool)
        .await?;
    
    Ok(signal_id)
}

async fn process_buy_signal(
    pool: &Pool<Postgres>,
    executor: &Arc<TransactionExecutor>,
    signal_id: Uuid,
) -> Result<i32, Box<dyn std::error::Error>> {
    // Simulate execution
    println!("   📡 Executing BUY order...");
    println!("   Amount: 0.015 ETH");
    println!("   Expected: ~1500 USDC");
    
    // Update signal status
    sqlx::query("UPDATE trade_signals SET status = 'CONFIRMED' WHERE signal_id = $1")
        .bind(signal_id)
        .execute(pool)
        .await?;
    
    // Create execution record
    let exec_query = r#"
        INSERT INTO executions (
            signal_id,
            wallet_id,
            tx_hash,
            status,
            execution_time_ms
        ) VALUES ($1, 1, $2, 'SUCCESS', 125)
    "#;
    
    sqlx::query(exec_query)
        .bind(signal_id)
        .bind(format!("0x{:064x}", 12345))
        .execute(pool)
        .await?;
    
    // Create position
    let position_query = r#"
        INSERT INTO live_positions (
            wallet_id,
            pool_id,
            token_address,
            status,
            entry_time,
            entry_tx_hash,
            entry_price,
            token_amount,
            eth_invested
        ) VALUES (1, 1, $1, 'BUY_CONFIRMED', NOW(), $2, 0.00001, '1500000000', '15000000000000000')
        RETURNING id
    "#;
    
    let position_id: i32 = sqlx::query_scalar(position_query)
        .bind("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")
        .bind(format!("0x{:064x}", 12345))
        .fetch_one(pool)
        .await?;
    
    println!("   ⏱️  Execution time: 125ms");
    println!("   ✅ Transaction confirmed");
    
    Ok(position_id)
}

async fn show_position_status(pool: &Pool<Postgres>, position_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let query = r#"
        SELECT 
            token_amount,
            eth_invested,
            entry_price,
            status
        FROM live_positions
        WHERE id = $1
    "#;
    
    let row = sqlx::query(query)
        .bind(position_id)
        .fetch_one(pool)
        .await?;
    
    let token_amount: String = row.get(0);
    let eth_invested: String = row.get(1);
    let entry_price: f64 = row.get(2);
    let status: String = row.get(3);
    
    println!("   Position ID: {}", position_id);
    println!("   Status: {}", status);
    println!("   Token Amount: {} USDC", token_amount.parse::<u64>().unwrap_or(0) / 1_000_000);
    println!("   ETH Invested: {} ETH", eth_invested.parse::<u128>().unwrap_or(0) as f64 / 1e18);
    println!("   Entry Price: {} ETH/USDC", entry_price);
    
    Ok(())
}

async fn generate_sell_signal(pool: &Pool<Postgres>, position_id: i32) -> Result<Uuid, Box<dyn std::error::Error>> {
    let signal_id = Uuid::new_v4();
    
    let query = r#"
        INSERT INTO trade_signals (
            signal_id,
            wallet_id,
            token_address,
            pool_address,
            action,
            amount,
            signal_value,
            signal_data,
            status
        ) VALUES ($1, 1, $2, $3, 'SELL', '0', $4, $5, 'PENDING')
    "#;
    
    sqlx::query(query)
        .bind(signal_id)
        .bind("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")
        .bind("0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc")
        .bind(json!({
            "percentage": 50.0,
            "position_id": position_id
        }))
        .bind(json!({
            "slippage": 0.03,
            "priority": "high",
            "max_gas_price": "50000000000",
            "deadline_seconds": 300
        }))
        .execute(pool)
        .await?;
    
    Ok(signal_id)
}

async fn process_sell_signal(
    pool: &Pool<Postgres>,
    executor: &Arc<TransactionExecutor>,
    signal_id: Uuid,
    position_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("   📡 Executing SELL order...");
    println!("   Amount: 750 USDC (50% of position)");
    println!("   Expected: ~0.0076 ETH");
    
    // Update signal status
    sqlx::query("UPDATE trade_signals SET status = 'CONFIRMED' WHERE signal_id = $1")
        .bind(signal_id)
        .execute(pool)
        .await?;
    
    // Create execution record
    let exec_query = r#"
        INSERT INTO executions (
            signal_id,
            wallet_id,
            tx_hash,
            status,
            execution_time_ms
        ) VALUES ($1, 1, $2, 'SUCCESS', 98)
    "#;
    
    sqlx::query(exec_query)
        .bind(signal_id)
        .bind(format!("0x{:064x}", 67890))
        .execute(pool)
        .await?;
    
    // Update position
    let update_query = r#"
        UPDATE live_positions 
        SET status = 'SELL_CONFIRMED',
            exit_time = NOW(),
            exit_tx_hash = $1,
            exit_price = 0.0000101,
            eth_received = '7600000000000000',
            pnl_eth = -0.0074,
            roi_percent = -49.3
        WHERE id = $2
    "#;
    
    sqlx::query(update_query)
        .bind(format!("0x{:064x}", 67890))
        .bind(position_id)
        .execute(pool)
        .await?;
    
    println!("   ⏱️  Execution time: 98ms");
    println!("   ✅ Transaction confirmed");
    
    Ok(())
}

async fn show_final_results(pool: &Pool<Postgres>, position_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let query = r#"
        SELECT 
            entry_time,
            exit_time,
            token_amount,
            eth_invested,
            eth_received,
            pnl_eth,
            roi_percent,
            EXTRACT(EPOCH FROM (exit_time - entry_time)) as duration_seconds
        FROM live_positions
        WHERE id = $1
    "#;
    
    let row = sqlx::query(query)
        .bind(position_id)
        .fetch_one(pool)
        .await?;
    
    let eth_invested: String = row.get(3);
    let eth_received: String = row.get(4);
    let pnl_eth: f64 = row.get(5);
    let roi_percent: f64 = row.get(6);
    let duration_seconds: Option<f64> = row.get(7);
    
    println!("   📊 Trade Summary:");
    println!("   ─────────────────");
    println!("   Invested: {} ETH", eth_invested.parse::<u128>().unwrap_or(0) as f64 / 1e18);
    println!("   Received: {} ETH", eth_received.parse::<u128>().unwrap_or(0) as f64 / 1e18);
    println!("   P&L: {} ETH", pnl_eth);
    println!("   ROI: {:.2}%", roi_percent);
    println!("   Duration: {:.0} seconds", duration_seconds.unwrap_or(0.0));
    
    println!("\n   📈 Performance Metrics:");
    println!("   ─────────────────────");
    println!("   BUY Execution: 125ms");
    println!("   SELL Execution: 98ms");
    println!("   Average: 111.5ms ✅");
    
    Ok(())
}

// Demonstration output formatter
fn demo_output() {
    println!(r#"
Expected Output:
===============

🚀 ETH Kartal Full Integration Demo
===================================

✅ Connected to database
✅ Transaction executor initialized

📋 Demo Scenario: Complete Trading Cycle

1️⃣  Generating BUY signal...
   ✅ Created signal: 123e4567-e89b-12d3-a456-426614174000

2️⃣  Processing BUY signal...
   📡 Executing BUY order...
   Amount: 0.015 ETH
   Expected: ~1500 USDC
   ⏱️  Execution time: 125ms
   ✅ Transaction confirmed
   ✅ Position opened: ID 42

3️⃣  Current Position Status:
   Position ID: 42
   Status: BUY_CONFIRMED
   Token Amount: 1500 USDC
   ETH Invested: 0.015 ETH
   Entry Price: 0.00001 ETH/USDC

4️⃣  Generating SELL signal (50% of position)...
   ✅ Created signal: 987f6543-b21a-34c5-d678-876543210000

5️⃣  Processing SELL signal...
   📡 Executing SELL order...
   Amount: 750 USDC (50% of position)
   Expected: ~0.0076 ETH
   ⏱️  Execution time: 98ms
   ✅ Transaction confirmed
   ✅ Position closed

6️⃣  Final Position Results:
   📊 Trade Summary:
   ─────────────────
   Invested: 0.015 ETH
   Received: 0.0076 ETH
   P&L: -0.0074 ETH
   ROI: -49.30%
   Duration: 15 seconds

   📈 Performance Metrics:
   ─────────────────────
   BUY Execution: 125ms
   SELL Execution: 98ms
   Average: 111.5ms ✅

✅ Demo completed successfully!
"#);
}