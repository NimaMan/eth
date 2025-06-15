/// Performance test for transaction simulator
/// 
/// This example tests and compares the performance of Fast RPC vs REVM simulators
/// to verify the timing claims in the documentation.
///
/// Run with: cargo run --example test_tx_simulator_performance

use mempool_processor::tx_simulator::{SimulatorWrapper, FastRpcSimulator};
use mempool_processor::mempool_fetcher::TransactionView;
use ethers::providers::{Http, Provider, Middleware};
use ethers::types::U256;
use std::time::Instant;
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber;
use revm_primitives::hardfork::SpecId;
use revm_context::BlockEnv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("test_tx_simulator_performance=info")
        .init();
    
    info!("🏃 Transaction Simulator Performance Test");
    info!("========================================\n");
    
    let rpc_url = std::env::var("ETH_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    
    info!("Using RPC URL: {}", rpc_url);
    
    // Check RPC connectivity first
    let provider = Arc::new(Provider::<Http>::try_from(&rpc_url)?);
    match provider.get_block_number().await {
        Ok(block) => info!("Connected to RPC, current block: {}", block),
        Err(e) => {
            error!("Failed to connect to RPC: {}", e);
            error!("Please ensure your Ethereum node is running with debug API enabled");
            return Err(e.into());
        }
    }
    
    // Get current block for simulation context
    let latest_block = provider.get_block(ethers::types::BlockNumber::Latest).await?
        .ok_or("Failed to get latest block")?;
    
    let block_env = BlockEnv {
        number: revm_primitives::U256::from(latest_block.number.unwrap_or_default().as_u64()),
        timestamp: revm_primitives::U256::from(latest_block.timestamp.as_u64()),
        gas_limit: latest_block.gas_limit.as_u64(),
        basefee: latest_block.base_fee_per_gas.unwrap_or_default().as_u64(),
        difficulty: revm_primitives::U256::from(latest_block.difficulty.as_u64()),
        prevrandao: Some(revm_primitives::B256::from_slice(latest_block.mix_hash.unwrap_or_default().as_bytes())),
        beneficiary: revm_primitives::Address::from_slice(latest_block.author.unwrap_or_default().as_bytes()),
        ..Default::default()
    };
    
    // Initialize both simulators
    info!("\nInitializing simulators...");
    let fast_simulator = match SimulatorWrapper::new_fast_rpc(&rpc_url).await {
        Ok(sim) => {
            info!("✅ Fast RPC simulator initialized");
            sim
        }
        Err(e) => {
            error!("❌ Failed to initialize Fast RPC simulator: {}", e);
            error!("Make sure your node has debug API enabled (--http.api eth,net,web3,debug)");
            return Err(e.into());
        }
    };
    
    let revm_simulator = match SimulatorWrapper::new_revm(&rpc_url, 1, SpecId::CANCUN).await {
        Ok(sim) => {
            info!("✅ REVM simulator initialized");
            sim
        }
        Err(e) => {
            error!("❌ Failed to initialize REVM simulator: {}", e);
            return Err(e.into());
        }
    };
    
    // Get some recent transactions to test with
    info!("\nFetching recent transactions...");
    let transactions = get_recent_transactions(&provider).await?;
    
    if transactions.is_empty() {
        error!("No transactions found to test");
        return Err("No transactions available".into());
    }
    
    info!("✅ Found {} transactions to test", transactions.len());
    
    // Test Fast RPC performance
    info!("\n📊 Testing Fast RPC Simulator Performance:");
    info!("─────────────────────────────────────────");
    
    let mut fast_times = Vec::new();
    let mut fast_errors = 0;
    let test_count = std::cmp::min(transactions.len(), 20);
    
    for (i, tx) in transactions.iter().take(test_count).enumerate() {
        let start = Instant::now();
        match fast_simulator.process_transaction(tx, &block_env).await {
            Ok(Some(_)) => {
                let elapsed = start.elapsed();
                fast_times.push(elapsed.as_millis() as f64);
                info!("  Transaction {}: {:>4}ms", i + 1, elapsed.as_millis());
            }
            Ok(None) => {
                let elapsed = start.elapsed();
                fast_times.push(elapsed.as_millis() as f64);
                info!("  Transaction {}: {:>4}ms (no state changes)", i + 1, elapsed.as_millis());
            }
            Err(e) => {
                fast_errors += 1;
                warn!("  Transaction {}: Error - {}", i + 1, e);
            }
        }
    }
    
    if fast_times.is_empty() {
        error!("No successful Fast RPC simulations");
        return Err("All Fast RPC simulations failed".into());
    }
    
    let fast_avg = fast_times.iter().sum::<f64>() / fast_times.len() as f64;
    let fast_min = fast_times.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let fast_max = fast_times.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let mut fast_sorted = fast_times.clone();
    fast_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let fast_p95 = if fast_sorted.len() > 1 {
        fast_sorted[((fast_sorted.len() - 1) as f64 * 0.95) as usize]
    } else {
        *fast_max
    };
    
    info!("\n  Fast RPC Statistics:");
    info!("  ├─ Average: {:.1}ms", fast_avg);
    info!("  ├─ Min: {:.1}ms", fast_min);
    info!("  ├─ Max: {:.1}ms", fast_max);
    info!("  ├─ P95: {:.1}ms", fast_p95);
    info!("  ├─ Success rate: {:.1}%", (fast_times.len() as f64 / test_count as f64) * 100.0);
    info!("  └─ Errors: {}", fast_errors);
    
    // Test REVM performance (fewer tests as it's slower)
    info!("\n📊 Testing REVM Simulator Performance:");
    info!("─────────────────────────────────────");
    
    let mut revm_times = Vec::new();
    let mut revm_errors = 0;
    let revm_test_count = std::cmp::min(transactions.len(), 5);
    
    for (i, tx) in transactions.iter().take(revm_test_count).enumerate() {
        let start = Instant::now();
        match revm_simulator.process_transaction(tx, &block_env).await {
            Ok(Some(_)) => {
                let elapsed = start.elapsed();
                revm_times.push(elapsed.as_millis() as f64);
                info!("  Transaction {}: {:>4}ms", i + 1, elapsed.as_millis());
            }
            Ok(None) => {
                let elapsed = start.elapsed();
                revm_times.push(elapsed.as_millis() as f64);
                info!("  Transaction {}: {:>4}ms (no state changes)", i + 1, elapsed.as_millis());
            }
            Err(e) => {
                revm_errors += 1;
                warn!("  Transaction {}: Error - {}", i + 1, e);
            }
        }
    }
    
    if revm_times.is_empty() {
        error!("No successful REVM simulations");
        return Err("All REVM simulations failed".into());
    }
    
    let revm_avg = revm_times.iter().sum::<f64>() / revm_times.len() as f64;
    let revm_min = revm_times.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let revm_max = revm_times.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    
    info!("\n  REVM Statistics:");
    info!("  ├─ Average: {:.1}ms", revm_avg);
    info!("  ├─ Min: {:.1}ms", revm_min);
    info!("  ├─ Max: {:.1}ms", revm_max);
    info!("  ├─ Success rate: {:.1}%", (revm_times.len() as f64 / revm_test_count as f64) * 100.0);
    info!("  └─ Errors: {}", revm_errors);
    
    // Performance comparison
    info!("\n📊 Performance Comparison:");
    info!("══════════════════════════");
    info!("  Fast RPC: ~{:.0}ms average", fast_avg);
    info!("  REVM: ~{:.0}ms average", revm_avg);
    info!("  Speedup: {:.1}x faster", revm_avg / fast_avg);
    
    // Verify claims
    info!("\n✅ Performance Verification:");
    if fast_avg <= 10.0 {
        info!("  ✓ Fast RPC meets <10ms target (actual: {:.1}ms)", fast_avg);
    } else {
        warn!("  ✗ Fast RPC exceeds 10ms target (actual: {:.1}ms)", fast_avg);
    }
    
    if revm_avg <= 60.0 {
        info!("  ✓ REVM meets <60ms target (actual: {:.1}ms)", revm_avg);
    } else {
        warn!("  ✗ REVM exceeds 60ms target (actual: {:.1}ms)", revm_avg);
    }
    
    // Throughput calculation
    let fast_tps = 1000.0 / fast_avg;
    let revm_tps = 1000.0 / revm_avg;
    
    info!("\n📈 Theoretical Throughput:");
    info!("  Fast RPC: ~{:.0} TPS", fast_tps);
    info!("  REVM: ~{:.0} TPS", revm_tps);
    
    // Test concurrent performance
    info!("\n📊 Testing Concurrent Performance (10 parallel):");
    let start = Instant::now();
    let mut handles = vec![];
    
    for tx in transactions.iter().take(10) {
        let tx_clone = tx.clone();
        let block_env_clone = block_env.clone();
        
        // Create a new Fast RPC simulator for concurrent test
        let rpc_url_clone = rpc_url.clone();
        handles.push(tokio::spawn(async move {
            let sim = FastRpcSimulator::new(&rpc_url_clone).await?;
            sim.process_transaction(&tx_clone, &block_env_clone).await
        }));
    }
    
    let mut concurrent_success = 0;
    for handle in handles {
        match handle.await? {
            Ok(_) => concurrent_success += 1,
            Err(e) => warn!("Concurrent simulation error: {}", e),
        }
    }
    
    let concurrent_elapsed = start.elapsed();
    let concurrent_avg = concurrent_elapsed.as_millis() as f64 / 10.0;
    
    info!("  Total time: {:?}", concurrent_elapsed);
    info!("  Average per tx: {:.1}ms", concurrent_avg);
    info!("  Concurrent speedup: {:.1}x", fast_avg / concurrent_avg);
    info!("  Success rate: {:.1}%", (concurrent_success as f64 / 10.0) * 100.0);
    
    info!("\n✅ Performance test completed successfully!");
    
    Ok(())
}

// Helper function to get recent transactions
async fn get_recent_transactions(
    provider: &Arc<Provider<Http>>
) -> Result<Vec<TransactionView>, Box<dyn std::error::Error>> {
    let mut transactions = Vec::new();
    let current_block = provider.get_block_number().await?;
    
    info!("Looking for recent transactions in blocks...");
    
    // Look back up to 20 blocks to find transactions
    for i in 0..20 {
        let block_num = current_block - i;
        if let Ok(Some(block)) = provider.get_block_with_txs(block_num).await {
            if !block.transactions.is_empty() {
                info!("  Block {}: {} transactions", block_num, block.transactions.len());
                
                for tx in block.transactions.iter().take(10) {
                    // Skip simple ETH transfers, look for contract interactions
                    if tx.input.len() > 4 {
                        transactions.push(TransactionView {
                            hash: tx.hash.as_bytes().to_vec(),
                            from: tx.from.as_bytes().to_vec(),
                            to: tx.to.map(|addr| addr.as_bytes().to_vec()),
                            value: tx.value,
                            gas_price: tx.gas_price,
                            gas_limit: Some(tx.gas),
                            nonce: Some(tx.nonce),
                            input_data: Some(tx.input.to_vec()),
                        });
                    }
                }
            }
            
            if transactions.len() >= 20 {
                break;
            }
        }
    }
    
    Ok(transactions)
}