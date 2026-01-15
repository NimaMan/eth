//! Position Updater Example - Updates live positions based on execution results
//! 
//! This example shows how to:
//! - Monitor execution confirmations
//! - Update position states (BUY_CONFIRMED, SELL_CONFIRMED)
//! - Calculate P&L and ROI
//! - Handle position lifecycle
//!
//! Usage:
//!   cargo run --example position_updater

use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use ethers::prelude::*;
use std::env;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug)]
struct ExecutionRecord {
    id: i32,
    signal_id: Uuid,
    wallet_id: i32,
    tx_hash: String,
    status: String,
    position_id: Option<i32>,
}

#[derive(Debug)]
struct LivePosition {
    id: i32,
    wallet_id: i32,
    pool_id: i32,
    token_address: String,
    status: String,
    entry_price: Option<f64>,
    token_amount: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ETH Kartal Position Updater");
    println!("==============================\n");
    
    // Database connection
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/live_trading_db".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    println!("✅ Connected to database");
    
    // Create provider for on-chain data
    let provider = Provider::<Http>::try_from(
        env::var("ETH_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string())
    )?;
    
    println!("✅ Connected to Ethereum RPC");
    
    // Start monitoring loop
    println!("\n📡 Starting position update loop...");
    println!("   Monitoring executions and updating positions\n");
    
    loop {
        // Process new executions
        match process_executions(&pool, &provider).await {
            Ok(count) => {
                if count > 0 {
                    println!("✅ Processed {} executions", count);
                }
            }
            Err(e) => {
                eprintln!("❌ Error processing executions: {}", e);
            }
        }
        
        // Update position snapshots
        match update_position_snapshots(&pool, &provider).await {
            Ok(count) => {
                if count > 0 {
                    println!("📸 Created {} position snapshots", count);
                }
            }
            Err(e) => {
                eprintln!("❌ Error updating snapshots: {}", e);
            }
        }
        
        sleep(Duration::from_secs(5)).await;
    }
}

async fn process_executions(
    pool: &Pool<Postgres>,
    provider: &Provider<Http>,
) -> Result<usize, Box<dyn std::error::Error>> {
    // Find unprocessed successful executions
    let query = r#"
        SELECT 
            e.id,
            e.signal_id,
            e.wallet_id,
            e.tx_hash,
            e.status,
            ts.action,
            ts.token_address,
            ts.pool_address,
            ts.amount,
            lp.id as position_id
        FROM executions e
        JOIN trade_signals ts ON e.signal_id = ts.signal_id
        LEFT JOIN live_positions lp ON lp.wallet_id = e.wallet_id 
            AND lp.token_address = ts.token_address 
            AND lp.status NOT IN ('SELL_CONFIRMED', 'CLOSED')
        WHERE e.status = 'SUCCESS' 
            AND e.position_updated = false
        LIMIT 10
    "#;
    
    let rows = sqlx::query(query).fetch_all(pool).await?;
    
    for row in &rows {
        let execution_id: i32 = row.get(0);
        let signal_id: Uuid = row.get(1);
        let wallet_id: i32 = row.get(2);
        let tx_hash: String = row.get(3);
        let action: String = row.get(5);
        let token_address: String = row.get(6);
        let pool_address: String = row.get(7);
        let amount: String = row.get(8);
        let position_id: Option<i32> = row.get(9);
        
        println!("\n🔄 Processing execution: {}", signal_id);
        println!("   Action: {}", action);
        println!("   TX: {}", tx_hash);
        
        // Get transaction receipt
        let hash = tx_hash.parse::<H256>()?;
        match provider.get_transaction_receipt(hash).await? {
            Some(receipt) => {
                if receipt.status == Some(U64::from(1)) {
                    // Transaction successful
                    match action.as_str() {
                        "BUY" => {
                            handle_buy_confirmation(
                                pool,
                                execution_id,
                                wallet_id,
                                &token_address,
                                &pool_address,
                                &amount,
                                &receipt,
                            ).await?;
                        }
                        "SELL" => {
                            if let Some(pos_id) = position_id {
                                handle_sell_confirmation(
                                    pool,
                                    execution_id,
                                    pos_id,
                                    &receipt,
                                ).await?;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Transaction failed on-chain
                    update_execution_failed(pool, execution_id).await?;
                }
            }
            None => {
                println!("   ⏳ Transaction still pending");
            }
        }
    }
    
    Ok(rows.len())
}

async fn handle_buy_confirmation(
    pool: &Pool<Postgres>,
    execution_id: i32,
    wallet_id: i32,
    token_address: &str,
    pool_address: &str,
    eth_amount: &str,
    receipt: &TransactionReceipt,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("   💰 Handling BUY confirmation");
    
    // Calculate entry price and token amount from logs
    // In real implementation, parse Uniswap swap event logs
    let gas_used = receipt.gas_used.unwrap_or_default();
    let effective_gas_price = receipt.effective_gas_price.unwrap_or_default();
    let gas_cost = gas_used * effective_gas_price;
    
    // Mock calculation (real would parse logs)
    let token_amount = "1000000000"; // Mock: 1000 tokens with 6 decimals
    let entry_price = 0.0001; // Mock: price per token in ETH
    
    // Get pool_id from eth_db
    let pool_id = get_pool_id(pool, pool_address).await?;
    
    // Create or update position
    let position_query = r#"
        INSERT INTO live_positions (
            wallet_id,
            pool_id,
            token_address,
            status,
            entry_time,
            entry_tx_hash,
            entry_price,
            entry_gas_eth,
            token_amount,
            eth_invested
        ) VALUES ($1, $2, $3, 'BUY_CONFIRMED', NOW(), $4, $5, $6, $7, $8)
        ON CONFLICT (wallet_id, token_address, pool_id) 
        WHERE status = 'INIT'
        DO UPDATE SET
            status = 'BUY_CONFIRMED',
            entry_time = NOW(),
            entry_tx_hash = $4,
            entry_price = $5,
            entry_gas_eth = $6,
            token_amount = $7,
            eth_invested = $8
        RETURNING id
    "#;
    
    let position_id: i32 = sqlx::query_scalar(position_query)
        .bind(wallet_id)
        .bind(pool_id)
        .bind(token_address.to_lowercase())
        .bind(&receipt.transaction_hash.to_string())
        .bind(entry_price)
        .bind(format!("{}", gas_cost))
        .bind(token_amount)
        .bind(eth_amount)
        .fetch_one(pool)
        .await?;
    
    println!("   ✅ Position created/updated: ID {}", position_id);
    
    // Update execution record
    let update_query = r#"
        UPDATE executions 
        SET position_updated = true,
            block_number = $1,
            gas_used = $2,
            gas_price = $3
        WHERE id = $4
    "#;
    
    sqlx::query(update_query)
        .bind(receipt.block_number.unwrap_or_default().as_u64() as i64)
        .bind(gas_used.as_u64() as i64)
        .bind(effective_gas_price.to_string())
        .bind(execution_id)
        .execute(pool)
        .await?;
    
    Ok(())
}

async fn handle_sell_confirmation(
    pool: &Pool<Postgres>,
    execution_id: i32,
    position_id: i32,
    receipt: &TransactionReceipt,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("   💸 Handling SELL confirmation");
    
    // Get position details
    let position = get_position(pool, position_id).await?;
    
    // Calculate exit metrics
    let gas_used = receipt.gas_used.unwrap_or_default();
    let effective_gas_price = receipt.effective_gas_price.unwrap_or_default();
    let gas_cost = gas_used * effective_gas_price;
    
    // Mock calculation (real would parse logs)
    let eth_received = "15000000000000000"; // 0.015 ETH
    let exit_price = 0.00015; // Price per token
    
    // Calculate P&L
    let eth_invested: f64 = position.eth_invested.unwrap_or_default().parse().unwrap_or(0.0);
    let eth_received_f64: f64 = eth_received.parse::<u128>().unwrap_or(0) as f64 / 1e18;
    let total_gas = gas_cost.as_u128() as f64 / 1e18;
    
    let pnl_eth = eth_received_f64 - eth_invested - total_gas;
    let roi = if eth_invested > 0.0 {
        (pnl_eth / eth_invested) * 100.0
    } else {
        0.0
    };
    
    // Update position
    let update_query = r#"
        UPDATE live_positions 
        SET status = 'SELL_CONFIRMED',
            exit_time = NOW(),
            exit_tx_hash = $1,
            exit_price = $2,
            exit_gas_eth = $3,
            eth_received = $4,
            pnl_eth = $5,
            roi_percent = $6
        WHERE id = $7
    "#;
    
    sqlx::query(update_query)
        .bind(receipt.transaction_hash.to_string())
        .bind(exit_price)
        .bind(format!("{}", gas_cost))
        .bind(eth_received)
        .bind(pnl_eth)
        .bind(roi)
        .bind(position_id)
        .execute(pool)
        .await?;
    
    println!("   ✅ Position closed with ROI: {:.2}%", roi);
    
    // Update execution
    sqlx::query("UPDATE executions SET position_updated = true WHERE id = $1")
        .bind(execution_id)
        .execute(pool)
        .await?;
    
    Ok(())
}

async fn update_position_snapshots(
    pool: &Pool<Postgres>,
    provider: &Provider<Http>,
) -> Result<usize, Box<dyn std::error::Error>> {
    // Get active positions that need snapshots
    let query = r#"
        SELECT 
            p.id,
            p.pool_id,
            p.token_address,
            p.token_amount,
            pl.address as pool_address
        FROM live_positions p
        JOIN eth_db.pools pl ON p.pool_id = pl.id
        WHERE p.status = 'BUY_CONFIRMED'
            AND (p.last_snapshot_time IS NULL 
                 OR p.last_snapshot_time < NOW() - INTERVAL '1 hour')
        LIMIT 10
    "#;
    
    let rows = sqlx::query(query).fetch_all(pool).await?;
    
    for row in &rows {
        let position_id: i32 = row.get(0);
        let pool_address: String = row.get(4);
        
        // Get current pool data (mock)
        let current_value_eth = 0.016; // Mock current value
        let gas_price = provider.get_gas_price().await?.as_u128() as f64 / 1e9;
        
        // Insert snapshot
        let snapshot_query = r#"
            INSERT INTO position_snapshots (
                position_id,
                value_eth,
                gas_price_gwei,
                created_at
            ) VALUES ($1, $2, $3, NOW())
        "#;
        
        sqlx::query(snapshot_query)
            .bind(position_id)
            .bind(current_value_eth)
            .bind(gas_price)
            .execute(pool)
            .await?;
        
        // Update last snapshot time
        sqlx::query("UPDATE live_positions SET last_snapshot_time = NOW() WHERE id = $1")
            .bind(position_id)
            .execute(pool)
            .await?;
    }
    
    Ok(rows.len())
}

async fn get_pool_id(pool: &Pool<Postgres>, pool_address: &str) -> Result<i32, Box<dyn std::error::Error>> {
    // Query eth_db for pool_id
    let query = "SELECT id FROM eth_db.pools WHERE address = $1";
    
    let pool_id: i32 = sqlx::query_scalar(query)
        .bind(pool_address.to_lowercase())
        .fetch_optional(pool)
        .await?
        .unwrap_or(1); // Default to 1 for demo
    
    Ok(pool_id)
}

async fn get_position(pool: &Pool<Postgres>, position_id: i32) -> Result<Position, Box<dyn std::error::Error>> {
    let query = "SELECT id, eth_invested FROM live_positions WHERE id = $1";
    
    let row = sqlx::query(query)
        .bind(position_id)
        .fetch_one(pool)
        .await?;
    
    Ok(Position {
        id: row.get(0),
        eth_invested: row.get(1),
    })
}

async fn update_execution_failed(pool: &Pool<Postgres>, execution_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let query = r#"
        UPDATE executions 
        SET status = 'FAILED',
            error_message = 'Transaction reverted on-chain',
            position_updated = true
        WHERE id = $1
    "#;
    
    sqlx::query(query)
        .bind(execution_id)
        .execute(pool)
        .await?;
    
    Ok(())
}

#[derive(Debug)]
struct Position {
    id: i32,
    eth_invested: Option<String>,
}