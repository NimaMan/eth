//! Signal Generator Example - Simulates trading signals in the database
//!
//! This example shows how to generate BUY and SELL signals that ETH Kartal
//! will process. Signals are written to the live_trading_db database.
//!
//! Usage:
//!   cargo run --example signal_generator -- --buy --eth-amount 0.015
//!   cargo run --example signal_generator -- --sell --percentage 50

use chrono::Utc;
use clap::Parser;
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate trading signals for ETH Kartal")]
struct Args {
    /// Generate a BUY signal
    #[arg(long, group = "action")]
    buy: bool,

    /// Generate a SELL signal
    #[arg(long, group = "action")]
    sell: bool,

    /// Amount of ETH to spend (for BUY signals)
    #[arg(long, requires = "buy")]
    eth_amount: Option<f64>,

    /// Percentage of position to sell (for SELL signals)
    #[arg(long, requires = "sell", value_parser = validate_percentage)]
    percentage: Option<f64>,

    /// Token address to trade
    #[arg(long, default_value = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")]
    token: String,

    /// Pool address
    #[arg(long, default_value = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc")]
    pool: String,

    /// Slippage tolerance (%)
    #[arg(long, default_value = "3.0")]
    slippage: f64,

    /// Priority level (normal, high, critical)
    #[arg(long, default_value = "high")]
    priority: String,

    /// Wallet ID to use
    #[arg(long, default_value = "1")]
    wallet_id: i32,
}

fn validate_percentage(s: &str) -> Result<f64, String> {
    let val: f64 = s.parse().map_err(|_| "Invalid percentage")?;
    if val <= 0.0 || val > 100.0 {
        Err("Percentage must be between 0 and 100".to_string())
    } else {
        Ok(val)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Database connection
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:password@localhost:5432/live_trading_db".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("🚀 ETH Kartal Signal Generator");
    println!("==============================");

    // Validate action
    if !args.buy && !args.sell {
        return Err("Must specify either --buy or --sell".into());
    }

    // Generate signal
    let signal_id = Uuid::new_v4();
    let action = if args.buy { "BUY" } else { "SELL" };

    let (amount_wei, signal_value) = if args.buy {
        let eth = args.eth_amount.unwrap();
        let wei = (eth * 1e18) as i64;
        (
            wei.to_string(),
            json!({
                "eth_amount": eth,
                "eth_amount_wei": wei.to_string()
            }),
        )
    } else {
        let pct = args.percentage.unwrap();
        (
            "0".to_string(),
            json!({
                "percentage": pct,
                "position_percentage": true
            }),
        )
    };

    // Prepare signal data
    let signal_data = json!({
        "slippage": args.slippage / 100.0, // Convert to decimal
        "priority": args.priority,
        "max_gas_price": "100000000000", // 100 gwei
        "deadline_seconds": 300
    });

    println!("\n📋 Creating {} Signal:", action);
    println!("  ID: {}", signal_id);
    println!("  Token: {}", args.token);
    println!("  Pool: {}", args.pool);
    if args.buy {
        println!("  ETH Amount: {} ETH", args.eth_amount.unwrap());
    } else {
        println!("  Sell Percentage: {}%", args.percentage.unwrap());
    }
    println!("  Slippage: {}%", args.slippage);
    println!("  Priority: {}", args.priority);

    // Insert signal into database
    let result = insert_signal(
        &pool,
        signal_id,
        args.wallet_id,
        &args.token,
        &args.pool,
        action,
        &amount_wei,
        signal_value,
        signal_data,
    )
    .await?;

    println!("\n✅ Signal created successfully!");
    println!("  Database ID: {}", result.id);
    println!("  Status: {}", result.status);
    println!("  Created at: {}", result.created_at);

    // Show how to monitor the signal
    println!("\n📡 Monitor signal execution:");
    println!(
        "  SELECT * FROM trade_signals WHERE signal_id = '{}';",
        signal_id
    );
    println!(
        "  SELECT * FROM executions WHERE signal_id = '{}';",
        signal_id
    );

    // Explain signal lifecycle
    println!("\n🔄 Signal Lifecycle:");
    println!("  1. PENDING - Signal created, waiting for ETH Kartal");
    println!("  2. SENT - ETH Kartal received the signal");
    println!("  3. EXECUTING - Transaction in progress");
    println!("  4. CONFIRMED - Transaction confirmed on-chain");
    println!("  5. FAILED - Transaction failed (check execution table for details)");

    Ok(())
}

#[derive(Debug)]
struct SignalResult {
    id: i32,
    status: String,
    created_at: chrono::DateTime<Utc>,
}

async fn insert_signal(
    pool: &Pool<Postgres>,
    signal_id: Uuid,
    wallet_id: i32,
    token_address: &str,
    pool_address: &str,
    action: &str,
    amount: &str,
    signal_value: serde_json::Value,
    signal_data: serde_json::Value,
) -> Result<SignalResult, Box<dyn std::error::Error>> {
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
            status,
            created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'PENDING', NOW())
        RETURNING id, status, created_at
    "#;

    let row = sqlx::query_as::<_, (i32, String, chrono::DateTime<Utc>)>(query)
        .bind(signal_id)
        .bind(wallet_id)
        .bind(token_address.to_lowercase())
        .bind(pool_address.to_lowercase())
        .bind(action)
        .bind(amount)
        .bind(signal_value)
        .bind(signal_data)
        .fetch_one(pool)
        .await?;

    Ok(SignalResult {
        id: row.0,
        status: row.1,
        created_at: row.2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_validation() {
        assert!(validate_percentage("50").is_ok());
        assert!(validate_percentage("100").is_ok());
        assert!(validate_percentage("0").is_err());
        assert!(validate_percentage("101").is_err());
        assert!(validate_percentage("-10").is_err());
    }
}
