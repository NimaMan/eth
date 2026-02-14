//! Trade Logger Demo - Demonstrates database writing functionality
//!
//! Shows how TradeLogger writes trading activity to the database

use chrono;
use eth_kartal::{
    alert_processor::{Action, Alert, ExecutionParams, Priority},
    db_writers::{TradeEvent, TradeLogger},
    risk::RiskDecision,
    tx_executor::{ExecutionMetrics, ExecutionResult},
};
use ethers::prelude::*;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("=== Trade Logger Demo ===\n");

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL").ok();

    if database_url.is_none() {
        println!("⚠️  No DATABASE_URL set, running in log-only mode");
        println!("   Set DATABASE_URL to enable database writes\n");
    } else {
        println!("✅ Database URL configured");
    }

    // Create wallet address for demo
    let wallet_address = "0xb340c40dB8d07d6751172B07ECfB0aEe8bF7245c".parse::<Address>()?;

    // Create trade logger
    let logger = TradeLogger::new(database_url.as_deref(), wallet_address).await?;
    println!(
        "✅ Trade logger initialized for wallet: {}\n",
        wallet_address
    );

    // Demo 1: Log alert received
    println!("📋 Demo 1: Alert Received");
    let alert = create_demo_alert();
    let signal_id = logger.log_alert_received(&alert).await;
    println!("   Signal ID: {}", signal_id);

    // Demo 2: Log risk decision - Allow
    println!("\n📋 Demo 2: Risk Decision - Allow");
    let risk_allow = RiskDecision::Allow;
    let trade_amount = U256::from((0.01 * 1e18) as u128);
    logger
        .log_risk_decision(signal_id, &alert.id, &risk_allow, trade_amount)
        .await;

    // Demo 3: Log risk decision - Block
    println!("\n📋 Demo 3: Risk Decision - Block");
    let risk_block = RiskDecision::Block {
        reason: "Gas cost too high: 15% of trade value".to_string(),
    };
    let signal_id_2 = Uuid::new_v4();
    logger
        .log_risk_decision(signal_id_2, "high-gas-alert", &risk_block, trade_amount)
        .await;

    // Demo 4: Log transaction submission
    println!("\n📋 Demo 4: Transaction Submitted");
    let tx_hash = H256::random();
    let nonce = U256::from(42);
    let gas_price = U256::from(50_000_000_000u128); // 50 gwei
    logger
        .log_tx_submitted(signal_id, &alert.id, tx_hash, nonce, gas_price, "Public")
        .await;
    println!("   TX Hash: {:?}", tx_hash);

    // Demo 5: Log successful execution
    println!("\n📋 Demo 5: Execution Success");
    let result = ExecutionResult {
        alert_id: alert.id.clone(),
        tx_hash: Some(tx_hash),
        success: true,
        error: None,
        metrics: ExecutionMetrics {
            alert_to_start_ms: 5,
            position_check_ms: 12,
            gas_ranking_ms: 25,
            price_quote_ms: 18,
            tx_build_ms: 8,
            tx_submit_ms: 35,
            total_ms: 103,
        },
    };
    logger
        .log_execution_result(signal_id, &alert.id, &result)
        .await;

    // Demo 6: Log failed execution
    println!("\n📋 Demo 6: Execution Failure");
    let failed_result = ExecutionResult {
        alert_id: "failed-alert".to_string(),
        tx_hash: None,
        success: false,
        error: Some("Slippage exceeded: expected 1000 tokens, got 850".to_string()),
        metrics: ExecutionMetrics {
            alert_to_start_ms: 5,
            position_check_ms: 10,
            gas_ranking_ms: 20,
            price_quote_ms: 15,
            tx_build_ms: 0,
            tx_submit_ms: 0,
            total_ms: 50,
        },
    };
    let signal_id_3 = Uuid::new_v4();
    logger
        .log_execution_result(signal_id_3, "failed-alert", &failed_result)
        .await;

    // Demo 7: Get wallet statistics
    println!("\n📋 Demo 7: Wallet Statistics");
    match logger.get_wallet_stats().await {
        Ok(stats) => {
            println!("   Stats: {}", serde_json::to_string_pretty(&stats)?);
        }
        Err(e) => {
            println!("   Failed to get stats: {}", e);
        }
    }

    // Demo 8: Custom events
    println!("\n📋 Demo 8: Custom Trade Events");

    // Transaction confirmed event
    let confirmed_event = TradeEvent::TxConfirmed {
        alert_id: alert.id.clone(),
        tx_hash,
        block_number: 18500000,
        gas_used: U256::from(150000),
    };
    logger.log_event(confirmed_event).await;

    // Transaction failed event
    let failed_event = TradeEvent::TxFailed {
        alert_id: "timeout-alert".to_string(),
        tx_hash: Some(H256::random()),
        error: "Transaction timed out in mempool".to_string(),
        revert_reason: None,
    };
    logger.log_event(failed_event).await;

    println!("\n✅ Trade logger demo complete!");

    if database_url.is_some() {
        println!("\n📊 Check your database for the logged entries:");
        println!("   - trade_signals table for alerts");
        println!("   - executions table for results");
    }

    Ok(())
}

fn create_demo_alert() -> Alert {
    Alert {
        id: "demo-alert-001".to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        token_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse()
            .unwrap(), // USDC
        pool_address: "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
            .parse()
            .unwrap(), // USDC/WETH V2
        action: Action::Buy,
        params: ExecutionParams {
            amount: U256::from((0.01 * 1e18) as u128), // 0.01 ETH
            slippage: 0.02,                            // 2%
            max_gas_price: Some(U256::from(100_000_000_000u128)), // 100 gwei
            deadline_seconds: 300,                     // 5 minutes
            priority: Priority::Normal,
        },
    }
}
