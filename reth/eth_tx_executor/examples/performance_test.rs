//! Performance test for eth_kartal execution system
//! 
//! Tests the complete alert→execution pipeline with real network calls
//! to validate sub-200ms response times.

use eth_kartal::alert_processor::{Alert, ExecutionParams, Action, Priority};
use eth_kartal::tx_executor::{TransactionExecutor, ExecutorConfig};
use ethers::types::{Address, U256};
use std::time::Instant;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("🚀 eth_kartal Performance Test");
    println!("Testing complete alert→execution pipeline");
    println!("Target: <200ms total latency\n");
    
    // Configuration for executor
    let config = ExecutorConfig {
        private_key: std::env::var("PRIVATE_KEY")
            .unwrap_or_else(|_| "0x1111111111111111111111111111111111111111111111111111111111111111".to_string()),
        chain_id: 1,
        rpc_url: "http://127.0.0.1:8545".to_string(),
        flashbots_enabled: false,
        flashbots_rpc: None,
        reth_ws_url: "ws://127.0.0.1:8546".to_string(),
    };
    
    // Test 1: Basic system initialization
    let init_start = Instant::now();
    println!("⏱️  Initializing TransactionExecutor...");
    
    let executor = match TransactionExecutor::new(config).await {
        Ok(exec) => {
            let init_time = init_start.elapsed().as_millis();
            println!("✅ Executor initialized in {}ms", init_time);
            exec
        }
        Err(e) => {
            println!("❌ Failed to initialize executor: {}", e);
            println!("💡 Make sure Reth node is running at 127.0.0.1:8545 and 127.0.0.1:8546");
            return Ok(());
        }
    };
    
    // Test 2: Create a sample alert (WETH sell for testing)
    let weth_address = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    let weth_usdc_pool = Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")?; // Uniswap V3 WETH/USDC
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    
    let test_alert = Alert {
        id: "test_001".to_string(),
        timestamp: current_time,
        token_address: weth_address,
        pool_address: weth_usdc_pool,
        action: Action::Sell,
        params: ExecutionParams {
            amount: U256::from(1_000_000_000_000_000u64), // 0.001 ETH
            slippage: 0.01, // 1%
            priority: Priority::High,
            max_gas_price: Some(U256::from(100_000_000_000u64)), // 100 gwei
            deadline_seconds: 300, // 5 minutes
        },
    };
    
    println!("📝 Created test alert: {}", test_alert.id);
    println!("   Token: {}", test_alert.token_address);
    println!("   Action: {:?}", test_alert.action);
    println!("   Amount: {} wei", test_alert.params.amount);
    println!("   Priority: {:?}\n", test_alert.params.priority);
    
    // Test 3: Execute alert and measure performance
    println!("🔥 Executing alert...");
    let execution_start = Instant::now();
    
    let result = executor.execute_alert(test_alert.clone()).await;
    let total_time = execution_start.elapsed().as_millis();
    
    // Analyze results
    println!("\n📊 PERFORMANCE RESULTS");
    println!("======================");
    println!("Total execution time: {}ms", total_time);
    
    match result.success {
        true => {
            println!("✅ Execution: SUCCESS");
            if let Some(tx_hash) = result.tx_hash {
                println!("📜 Transaction: {:?}", tx_hash);
            }
        }
        false => {
            println!("❌ Execution: FAILED");
            if let Some(error) = result.error {
                println!("💥 Error: {}", error);
            }
        }
    }
    
    // Detailed metrics breakdown
    println!("\n⚡ DETAILED METRICS");
    println!("==================");
    println!("Alert→Start:     {}ms", result.metrics.alert_to_start_ms);
    println!("Position Check:  {}ms", result.metrics.position_check_ms);
    println!("Gas Ranking:     {}ms", result.metrics.gas_ranking_ms);
    println!("Price Quote:     {}ms", result.metrics.price_quote_ms);
    println!("TX Build:        {}ms", result.metrics.tx_build_ms);
    println!("TX Submit:       {}ms", result.metrics.tx_submit_ms);
    println!("TOTAL:          {}ms", result.metrics.total_ms);
    
    // Performance analysis
    println!("\n🎯 PERFORMANCE ANALYSIS");
    println!("=======================");
    
    if total_time <= 200 {
        println!("🏆 EXCELLENT: Sub-200ms target achieved!");
    } else if total_time <= 500 {
        println!("✅ GOOD: Fast execution (<500ms)");
    } else if total_time <= 1000 {
        println!("⚠️  ACCEPTABLE: Under 1 second");
    } else {
        println!("🐌 SLOW: Execution took over 1 second");
    }
    
    // Identify bottlenecks
    let metrics_array = [
        ("Position Check", result.metrics.position_check_ms),
        ("Gas Ranking", result.metrics.gas_ranking_ms),
        ("Price Quote", result.metrics.price_quote_ms),
        ("TX Build", result.metrics.tx_build_ms),
        ("TX Submit", result.metrics.tx_submit_ms),
    ];
    let bottleneck = metrics_array.iter().max_by_key(|(_, time)| *time);
    
    if let Some((stage, time)) = bottleneck {
        println!("🎯 Slowest stage: {} ({}ms)", stage, time);
    }
    
    // Test 4: Multiple rapid executions to test consistency
    println!("\n🔄 CONSISTENCY TEST");
    println!("===================");
    println!("Running 5 rapid executions...");
    
    let mut execution_times = Vec::new();
    
    for i in 1..=5 {
        let quick_alert = Alert {
            id: format!("rapid_test_{:03}", i),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            token_address: test_alert.token_address,
            pool_address: test_alert.pool_address,
            action: test_alert.action.clone(),
            params: test_alert.params.clone(),
        };
        
        let start = Instant::now();
        let quick_result = executor.execute_alert(quick_alert).await;
        let exec_time = start.elapsed().as_millis();
        
        execution_times.push(exec_time);
        println!("  Test {}: {}ms ({})", i, exec_time, 
                if quick_result.success { "✅" } else { "❌" });
    }
    
    // Statistics
    let avg_time = execution_times.iter().sum::<u128>() / execution_times.len() as u128;
    let min_time = execution_times.iter().min().unwrap();
    let max_time = execution_times.iter().max().unwrap();
    
    println!("\n📈 CONSISTENCY STATS");
    println!("====================");
    println!("Average: {}ms", avg_time);
    println!("Minimum: {}ms", min_time);
    println!("Maximum: {}ms", max_time);
    println!("Range:   {}ms", max_time - min_time);
    
    if avg_time <= 200 {
        println!("🏆 CONSISTENT: Average execution under target!");
    } else {
        println!("⚠️  VARIABLE: Performance inconsistent");
    }
    
    println!("\n🏁 Performance test completed!");
    
    Ok(())
}