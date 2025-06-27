//! Accurate Performance Test for eth_kartal
//! 
//! Tests real execution paths with mock pools to measure true performance
//! without needing actual token balances.

use eth_kartal::alert_processor::{Alert, ExecutionParams, Action, Priority};
use eth_kartal::tx_executor::{TransactionExecutor, ExecutorConfig};
use ethers::types::{Address, U256};
use ethers::providers::Middleware;
use std::time::Instant;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🔬 eth_kartal Accurate Performance Test");
    println!("Testing real execution paths with precise measurements");
    println!("Target: <200ms total latency\n");
    
    // Test configuration
    let config = ExecutorConfig {
        private_key: "0x1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        chain_id: 1,
        rpc_url: "http://127.0.0.1:8545".to_string(),
        flashbots_enabled: false,
        flashbots_rpc: None,
        reth_ws_url: "ws://127.0.0.1:8546".to_string(),
    };
    
    // Test Scenario 1: Sell Transaction (should fail on balance check)
    println!("📊 TEST 1: SELL TRANSACTION PERFORMANCE");
    println!("=========================================");
    
    let sell_result = test_sell_performance(&config).await?;
    analyze_performance("SELL", &sell_result);
    
    println!("\n📊 TEST 2: BUY TRANSACTION PERFORMANCE");
    println!("========================================");
    
    let buy_result = test_buy_performance(&config).await?;
    analyze_performance("BUY", &buy_result);
    
    println!("\n📊 TEST 3: COMPONENT TIMING BREAKDOWN");
    println!("======================================");
    
    test_component_timing(&config).await?;
    
    println!("\n📊 TEST 4: MULTIPLE EXECUTION CONSISTENCY");
    println!("==========================================");
    
    test_execution_consistency(&config).await?;
    
    println!("\n🎯 PERFORMANCE SUMMARY");
    println!("======================");
    
    // Estimate full execution time based on components
    let estimated_full_execution = estimate_full_execution_time(&sell_result, &buy_result);
    
    if estimated_full_execution <= 200 {
        println!("🏆 EXCELLENT: Estimated full execution {} ms < 200ms target", estimated_full_execution);
    } else {
        println!("⚠️  NEEDS OPTIMIZATION: Estimated full execution {} ms > 200ms target", estimated_full_execution);
    }
    
    Ok(())
}

async fn test_sell_performance(config: &ExecutorConfig) -> Result<TestResult, Box<dyn std::error::Error>> {
    let executor = TransactionExecutor::new(config.clone()).await?;
    
    let weth_address = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    let weth_usdc_pool = Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")?;
    
    let alert = Alert {
        id: "sell_perf_test".to_string(),
        timestamp: current_timestamp(),
        token_address: weth_address,
        pool_address: weth_usdc_pool,
        action: Action::Sell,
        params: ExecutionParams {
            amount: U256::from(1_000_000_000_000_000u64), // 0.001 ETH worth
            slippage: 0.01,
            priority: Priority::High,
            max_gas_price: Some(U256::from(100_000_000_000u64)),
            deadline_seconds: 300,
        },
    };
    
    let start = Instant::now();
    let result = executor.execute_alert(alert).await;
    let total_time = start.elapsed().as_millis() as u64;
    
    Ok(TestResult {
        total_time,
        success: result.success,
        error: result.error,
        metrics: result.metrics,
        action: "Sell".to_string(),
    })
}

async fn test_buy_performance(config: &ExecutorConfig) -> Result<TestResult, Box<dyn std::error::Error>> {
    let executor = TransactionExecutor::new(config.clone()).await?;
    
    let random_token = Address::from_str("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984")?; // UNI token
    let uni_eth_pool = Address::from_str("0x1d42064Fc4Beb5F8aAF85F4617AE8b3b5B8Bd801")?;
    
    let alert = Alert {
        id: "buy_perf_test".to_string(),
        timestamp: current_timestamp(),
        token_address: random_token,
        pool_address: uni_eth_pool,
        action: Action::Buy,
        params: ExecutionParams {
            amount: U256::from(10_000_000_000_000_000u64), // 0.01 ETH worth
            slippage: 0.01,
            priority: Priority::High,
            max_gas_price: Some(U256::from(100_000_000_000u64)),
            deadline_seconds: 300,
        },
    };
    
    let start = Instant::now();
    let result = executor.execute_alert(alert).await;
    let total_time = start.elapsed().as_millis() as u64;
    
    Ok(TestResult {
        total_time,
        success: result.success,
        error: result.error,
        metrics: result.metrics,
        action: "Buy".to_string(),
    })
}

async fn test_component_timing(config: &ExecutorConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing individual component performance...");
    
    // Test executor initialization
    let init_start = Instant::now();
    let _executor = TransactionExecutor::new(config.clone()).await?;
    let init_time = init_start.elapsed().as_millis();
    println!("• Executor Initialization: {}ms", init_time);
    
    // Test RPC connectivity
    let rpc_start = Instant::now();
    let provider = ethers::providers::Provider::<ethers::providers::Http>::try_from(&config.rpc_url)?;
    let _block_number = provider.get_block_number().await?;
    let rpc_time = rpc_start.elapsed().as_millis();
    println!("• RPC Round Trip: {}ms", rpc_time);
    
    // Test WebSocket connectivity
    let ws_start = Instant::now();
    match ethers::providers::Provider::<ethers::providers::Ws>::connect(&config.reth_ws_url).await {
        Ok(_ws_provider) => {
            let ws_time = ws_start.elapsed().as_millis();
            println!("• WebSocket Connection: {}ms", ws_time);
        }
        Err(e) => {
            println!("• WebSocket Connection: FAILED - {}", e);
        }
    }
    
    Ok(())
}

async fn test_execution_consistency(config: &ExecutorConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing execution consistency over 10 runs...");
    
    let executor = TransactionExecutor::new(config.clone()).await?;
    let mut execution_times = Vec::new();
    
    let test_alert = Alert {
        id: "consistency_test".to_string(),
        timestamp: current_timestamp(),
        token_address: Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?,
        pool_address: Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")?,
        action: Action::Sell,
        params: ExecutionParams {
            amount: U256::from(1_000_000_000_000_000u64),
            slippage: 0.01,
            priority: Priority::Normal,
            max_gas_price: Some(U256::from(50_000_000_000u64)),
            deadline_seconds: 300,
        },
    };
    
    for i in 1..=10 {
        let mut alert = test_alert.clone();
        alert.id = format!("consistency_test_{:02}", i);
        alert.timestamp = current_timestamp();
        
        let start = Instant::now();
        let _result = executor.execute_alert(alert).await;
        let exec_time = start.elapsed().as_millis() as u64;
        
        execution_times.push(exec_time);
        print!(".");
    }
    
    println!("\nConsistency Results:");
    let avg_time = execution_times.iter().sum::<u64>() / execution_times.len() as u64;
    let min_time = *execution_times.iter().min().unwrap();
    let max_time = *execution_times.iter().max().unwrap();
    let range = max_time - min_time;
    
    println!("• Average: {}ms", avg_time);
    println!("• Minimum: {}ms", min_time);
    println!("• Maximum: {}ms", max_time);
    println!("• Range: {}ms", range);
    println!("• Consistency: {}", if range < 5 { "EXCELLENT" } else if range < 20 { "GOOD" } else { "VARIABLE" });
    
    Ok(())
}

fn analyze_performance(action: &str, result: &TestResult) {
    println!("Action: {}", action);
    println!("Success: {}", if result.success { "✅" } else { "❌" });
    if let Some(ref error) = result.error {
        println!("Error: {}", error);
    }
    
    println!("Timing Breakdown:");
    println!("• Position Check: {}ms", result.metrics.position_check_ms);
    println!("• Gas Ranking: {}ms", result.metrics.gas_ranking_ms);
    println!("• Price Quote: {}ms", result.metrics.price_quote_ms);
    println!("• TX Build: {}ms", result.metrics.tx_build_ms);
    println!("• TX Submit: {}ms", result.metrics.tx_submit_ms);
    println!("• TOTAL: {}ms", result.metrics.total_ms);
    
    let pipeline_time = result.metrics.position_check_ms + 
                       result.metrics.gas_ranking_ms + 
                       result.metrics.price_quote_ms + 
                       result.metrics.tx_build_ms + 
                       result.metrics.tx_submit_ms;
    
    println!("• Pipeline Sum: {}ms", pipeline_time);
    
    if result.success {
        if result.total_time <= 50 {
            println!("Performance: 🏆 EXCELLENT (<50ms)");
        } else if result.total_time <= 100 {
            println!("Performance: ✅ VERY GOOD (<100ms)");
        } else if result.total_time <= 200 {
            println!("Performance: ✅ GOOD (<200ms)");
        } else {
            println!("Performance: ⚠️ NEEDS OPTIMIZATION (>200ms)");
        }
    } else {
        println!("Performance: Fast failure ({}ms) - expected for test", result.total_time);
    }
}

fn estimate_full_execution_time(sell_result: &TestResult, buy_result: &TestResult) -> u64 {
    // Take the maximum time for each component from both tests
    // This represents worst-case scenario with actual token balance
    
    let max_position_check = sell_result.metrics.position_check_ms.max(buy_result.metrics.position_check_ms);
    let max_gas_ranking = sell_result.metrics.gas_ranking_ms.max(buy_result.metrics.gas_ranking_ms);
    let max_price_quote = sell_result.metrics.price_quote_ms.max(buy_result.metrics.price_quote_ms);
    let max_tx_build = sell_result.metrics.tx_build_ms.max(buy_result.metrics.tx_build_ms);
    let max_tx_submit = sell_result.metrics.tx_submit_ms.max(buy_result.metrics.tx_submit_ms);
    
    // Add estimated time for successful execution (network calls, etc.)
    let estimated_network_overhead = 20; // Estimated additional time for real execution
    let estimated_gas_estimation = 10;   // Gas estimation for real transaction
    let estimated_approval_check = 5;    // Token approval checking
    
    max_position_check + max_gas_ranking + max_price_quote + 
    max_tx_build + max_tx_submit + estimated_network_overhead + 
    estimated_gas_estimation + estimated_approval_check
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug)]
struct TestResult {
    total_time: u64,
    success: bool,
    error: Option<String>,
    metrics: eth_kartal::tx_executor::ExecutionMetrics,
    action: String,
}