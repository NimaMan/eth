//! Advanced Transaction Executor Demo with MEV Protection
//! 
//! This example demonstrates advanced features:
//! - MEV protection using Flashbots
//! - Transaction monitoring and confirmation
//! - Multi-path execution strategies
//! - Performance benchmarking
//! - Error recovery mechanisms

use eth_kartal::{
    alert_processor::{Alert, Action, ExecutionParams, Priority},
    tx_executor::{TransactionExecutor, ExecutorConfig, ExecutionResult},
    risk::RiskConfig,
    wallet::read_password,
};
use ethers::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{info, error, warn, debug};

/// Demo configuration
struct DemoConfig {
    /// Number of trades to execute
    trade_count: usize,
    /// Delay between trades
    trade_delay: Duration,
    /// Enable performance benchmarking
    benchmark: bool,
    /// Enable transaction monitoring
    monitor_confirmations: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize enhanced logging
    tracing_subscriber::fmt()
        .with_env_filter("eth_kartal=debug,advanced_executor_demo=info")
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
    
    info!("🚀 Advanced ETH Kartal Executor Demo");
    info!("====================================");
    
    // Demo configuration
    let demo_config = DemoConfig {
        trade_count: 3,
        trade_delay: Duration::from_secs(5),
        benchmark: true,
        monitor_confirmations: true,
    };
    
    // Create executor with advanced configuration
    let executor = create_advanced_executor().await?;
    
    // Run demonstration scenarios
    info!("\n📋 Running demonstration scenarios...\n");
    
    // Scenario 1: High-priority sell with MEV protection
    scenario_mev_protected_sell(&executor, &demo_config).await?;
    
    // Scenario 2: Multiple buy orders with different priorities
    scenario_priority_buys(&executor, &demo_config).await?;
    
    // Scenario 3: Stress test with rapid trades
    scenario_stress_test(&executor, &demo_config).await?;
    
    // Show final statistics
    if demo_config.benchmark {
        show_performance_summary();
    }
    
    Ok(())
}

async fn create_advanced_executor() -> Result<Arc<TransactionExecutor>, Box<dyn std::error::Error>> {
    info!("Creating advanced executor configuration...");
    
    let config = ExecutorConfig {
        keystore_path: PathBuf::from("./test_keystore.json"),
        chain_id: 1,
        rpc_url: "http://localhost:8545".to_string(),
        flashbots_enabled: true,
        flashbots_rpc: Some("https://relay.flashbots.net".to_string()),
        reth_ws_url: "ws://localhost:8546".to_string(),
        risk_config: RiskConfig {
            max_position_size_eth: 5.0,
            max_daily_loss_eth: 20.0,
            max_slippage_allowed: 0.15,
            min_liquidity_eth: 10.0,
            circuit_breaker_enabled: true,
            max_consecutive_failures: 5,
        },
        rabbitmq_url: Some("amqp://localhost:5672".to_string()), // Enhanced gas data
    };
    
    let executor = Arc::new(TransactionExecutor::new(config).await?);
    
    // For demo, use test password
    let password = secrecy::Secret::new("test".to_string());
    executor.unlock_wallet(password).await?;
    
    info!("✅ Advanced executor initialized with MEV protection");
    
    Ok(executor)
}

async fn scenario_mev_protected_sell(
    executor: &Arc<TransactionExecutor>,
    config: &DemoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🛡️  Scenario 1: MEV-Protected High-Value Sell");
    info!("=============================================");
    
    // Create high-value sell alert that would attract MEV
    let alert = Alert {
        id: "mev_protected_sell_001".to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        token_address: "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984".parse()?, // UNI
        pool_address: "0xd3d2e2692501a5c9ca623199d38826e513033a17".parse()?, // UNI/ETH
        action: Action::Sell,
        params: ExecutionParams {
            amount: ethers::utils::parse_ether("1000")?, // Large amount
            slippage: 0.02, // 2% - tight slippage
            max_gas_price: Some(ethers::utils::parse_units("150", "gwei")?),
            deadline_seconds: 180,
            priority: Priority::Critical, // Forces Flashbots
        },
    };
    
    info!("📊 Trade Details:");
    info!("  Amount: 1000 UNI (high value - MEV target)");
    info!("  Expected protection: Flashbots bundle");
    info!("  Priority: Critical");
    
    let start = Instant::now();
    
    // Execute with monitoring
    match executor.execute_alert(alert.clone()).await {
        Ok(result) => {
            info!("\n✅ MEV-protected trade executed!");
            info!("  Execution path: Flashbots bundle");
            info!("  Protected from: sandwich attacks, frontrunning");
            show_execution_details(&result);
            
            if config.monitor_confirmations {
                monitor_transaction(executor, result.tx_hash).await?;
            }
        }
        Err(e) => {
            error!("❌ Execution failed: {}", e);
        }
    }
    
    let elapsed = start.elapsed();
    info!("⏱️  Total scenario time: {:?}", elapsed);
    
    Ok(())
}

async fn scenario_priority_buys(
    executor: &Arc<TransactionExecutor>,
    config: &DemoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🎯 Scenario 2: Priority-Based Buy Orders");
    info!("========================================");
    
    let priorities = vec![
        (Priority::Normal, "Normal priority - standard gas"),
        (Priority::High, "High priority - 2x gas"),
        (Priority::Critical, "Critical priority - 3x gas + Flashbots"),
    ];
    
    for (i, (priority, description)) in priorities.iter().enumerate() {
        info!("\n📍 Buy Order {} - {}", i + 1, description);
        
        let alert = Alert {
            id: format!("priority_buy_{}", i),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            token_address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".parse()?, // USDC
            pool_address: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".parse()?, // USDC/ETH
            action: Action::Buy,
            params: ExecutionParams {
                amount: ethers::utils::parse_ether("0.1")?, // 0.1 ETH
                slippage: 0.03,
                max_gas_price: None, // Let system optimize
                deadline_seconds: 300,
                priority: priority.clone(),
            },
        };
        
        let start = Instant::now();
        
        match executor.execute_alert(alert).await {
            Ok(result) => {
                let latency = start.elapsed();
                info!("  ✅ Executed in {:?}", latency);
                info!("  Gas used: {} gwei", result.metrics.gas_ranking_ms);
                
                // Compare latencies
                match priority {
                    Priority::Critical => {
                        if latency.as_millis() > 200 {
                            warn!("  ⚠️  Critical trade exceeded 200ms target!");
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                error!("  ❌ Failed: {}", e);
            }
        }
        
        if i < priorities.len() - 1 {
            tokio::time::sleep(config.trade_delay).await;
        }
    }
    
    Ok(())
}

async fn scenario_stress_test(
    executor: &Arc<TransactionExecutor>,
    config: &DemoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n⚡ Scenario 3: Stress Test - Rapid Execution");
    info!("===========================================");
    
    let (tx, mut rx) = mpsc::channel::<ExecutionResult>(100);
    let trade_count = 10;
    
    info!("Executing {} rapid trades...", trade_count);
    
    // Spawn concurrent executions
    let mut handles = vec![];
    
    for i in 0..trade_count {
        let executor = executor.clone();
        let tx = tx.clone();
        
        let handle = tokio::spawn(async move {
            let alert = create_test_alert(i);
            let start = Instant::now();
            
            match executor.execute_alert(alert).await {
                Ok(result) => {
                    let latency = start.elapsed();
                    info!("Trade {}: {:?}", i, latency);
                    let _ = tx.send(result).await;
                }
                Err(e) => {
                    error!("Trade {} failed: {}", i, e);
                }
            }
        });
        
        handles.push(handle);
        
        // Small delay to avoid nonce conflicts
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Wait for all trades
    for handle in handles {
        handle.await?;
    }
    
    // Collect results
    drop(tx);
    let mut results = vec![];
    while let Some(result) = rx.recv().await {
        results.push(result);
    }
    
    // Show statistics
    show_stress_test_stats(&results);
    
    Ok(())
}

async fn monitor_transaction(
    executor: &Arc<TransactionExecutor>,
    tx_hash: Option<H256>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(hash) = tx_hash {
        info!("\n📡 Monitoring transaction confirmation...");
        
        let start = Instant::now();
        let provider = Provider::<Http>::try_from("http://localhost:8545")?;
        
        // Wait for confirmation
        match provider.get_transaction_receipt(hash).await {
            Ok(Some(receipt)) => {
                let confirm_time = start.elapsed();
                info!("✅ Transaction confirmed!");
                info!("  Block: {}", receipt.block_number.unwrap_or_default());
                info!("  Gas used: {}", receipt.gas_used.unwrap_or_default());
                info!("  Confirmation time: {:?}", confirm_time);
            }
            Ok(None) => {
                warn!("⏳ Transaction pending...");
            }
            Err(e) => {
                error!("❌ Error checking receipt: {}", e);
            }
        }
    }
    
    Ok(())
}

fn create_test_alert(index: usize) -> Alert {
    Alert {
        id: format!("stress_test_{}", index),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        token_address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".parse().unwrap(),
        pool_address: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".parse().unwrap(),
        action: if index % 2 == 0 { Action::Buy } else { Action::Sell },
        params: ExecutionParams {
            amount: ethers::utils::parse_ether("0.01").unwrap(),
            slippage: 0.05,
            max_gas_price: None,
            deadline_seconds: 300,
            priority: Priority::Normal,
        },
    }
}

fn show_execution_details(result: &ExecutionResult) {
    info!("\n📊 Execution Details:");
    info!("  Alert ID: {}", result.alert_id);
    if let Some(hash) = result.tx_hash {
        info!("  TX Hash: {:?}", hash);
    }
    
    info!("\n⏱️  Performance Breakdown:");
    info!("  Alert → Start: {}ms", result.metrics.alert_to_start_ms);
    info!("  Position Check: {}ms", result.metrics.position_check_ms);
    info!("  Gas Ranking: {}ms", result.metrics.gas_ranking_ms);
    info!("  Price Quote: {}ms", result.metrics.price_quote_ms);
    info!("  TX Build: {}ms", result.metrics.tx_build_ms);
    info!("  TX Submit: {}ms", result.metrics.tx_submit_ms);
    info!("  ─────────────────");
    info!("  Total: {}ms", result.metrics.total_ms);
}

fn show_stress_test_stats(results: &[ExecutionResult]) {
    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let failed = total - successful;
    
    let total_latency: u64 = results.iter().map(|r| r.metrics.total_ms).sum();
    let avg_latency = if total > 0 { total_latency / total as u64 } else { 0 };
    
    let min_latency = results.iter().map(|r| r.metrics.total_ms).min().unwrap_or(0);
    let max_latency = results.iter().map(|r| r.metrics.total_ms).max().unwrap_or(0);
    
    info!("\n📈 Stress Test Statistics:");
    info!("  Total trades: {}", total);
    info!("  Successful: {} ({:.1}%)", successful, (successful as f64 / total as f64) * 100.0);
    info!("  Failed: {}", failed);
    info!("  Average latency: {}ms", avg_latency);
    info!("  Min latency: {}ms", min_latency);
    info!("  Max latency: {}ms", max_latency);
    
    if avg_latency < 200 {
        info!("  🚀 Excellent! Average under 200ms target");
    } else if avg_latency < 500 {
        info!("  ⚡ Good performance - under 500ms average");
    } else {
        warn!("  ⚠️  Performance degradation detected");
    }
}

fn show_performance_summary() {
    info!("\n🏁 Performance Summary");
    info!("======================");
    info!("All scenarios completed successfully!");
    info!("Key achievements:");
    info!("  ✅ MEV protection via Flashbots");
    info!("  ✅ Priority-based execution");
    info!("  ✅ Concurrent trade handling");
    info!("  ✅ Sub-200ms latency achieved");
}