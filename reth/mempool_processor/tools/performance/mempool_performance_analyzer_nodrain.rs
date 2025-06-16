use mempool_processor::mempool_fetcher::fetcher::{MempoolFetcher, FetchMode};
use mempool_processor::mempool_fetcher::TransactionSource;
use mempool_processor::tx_simulator::TransactionSimulator;
use revm_context::BlockEnv as RevmBlockEnv;
use revm_primitives::hardfork::SpecId;
use ethers::providers::{Http as EthersHttp, Middleware, Provider as EthersProvider};
use ethers::types::{BlockId as EthersBlockId, BlockNumber as EthersBlockNumber};
use revm_tx_simulator_lib::conversions::{ethers_to_revm_u256, ethers_to_revm_address};
use tracing::{info, warn, error, Level};
use std::sync::Arc;
use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Write;

/// Performance metrics for a single transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionMetrics {
    tx_hash: String,
    // Timestamps
    mempool_arrival_ms: u64,
    fetch_time_ms: u64,
    simulation_start_ms: u64,
    simulation_end_ms: u64,
    // Durations
    time_to_fetch_us: u64,
    simulation_time_us: u64,
    state_extraction_time_us: u64,
    total_processing_time_us: u64,
    // Results
    simulation_successful: bool,
    accounts_affected: usize,
    eth_changes: usize,
    token_changes: usize,
}

/// Summary statistics
#[derive(Debug, Serialize, Deserialize)]
struct PerformanceStats {
    total_transactions: usize,
    successful_simulations: usize,
    failed_simulations: usize,
    avg_time_to_fetch_ms: f64,
    avg_simulation_time_ms: f64,
    avg_state_extraction_ms: f64,
    avg_total_processing_ms: f64,
    p50_simulation_time_ms: f64,
    p95_simulation_time_ms: f64,
    p99_simulation_time_ms: f64,
    max_simulation_time_ms: f64,
    transactions_per_second: f64,
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 Mempool Performance Analyzer - 1K Transaction Test (No Initial Drain)");
    
    let eth_rpc_url = "http://localhost:8545";
    
    // Initialize components
    info!("📡 Initializing mempool fetcher...");
    let mut fetcher = MempoolFetcher::with_websocket_support(
        eth_rpc_url,
        None,
        50000,
        true,
        1000,  // Large batch size for performance
        2000,
        FetchMode::RpcBatch
    )?;
    
    info!("🧪 Initializing REVM transaction simulator...");
    let tx_simulator = Arc::new(TransactionSimulator::new(
        eth_rpc_url,
        1, // mainnet
        SpecId::CANCUN
    ).await?);
    
    // Get current block for simulation context
    info!("📦 Fetching latest block for simulation context...");
    let provider = EthersProvider::<EthersHttp>::try_from(eth_rpc_url)?;
    let latest_block = provider
        .get_block(EthersBlockId::Number(EthersBlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    let mut block_env = RevmBlockEnv::default();
    block_env.number = ethers_to_revm_u256(latest_block.number.unwrap_or_default().as_u64().into());
    block_env.beneficiary = latest_block.author.map_or_else(|| revm_primitives::Address::ZERO, |h160| ethers_to_revm_address(h160));
    block_env.timestamp = ethers_to_revm_u256(latest_block.timestamp);
    block_env.gas_limit = latest_block.gas_limit.as_u64();
    block_env.basefee = latest_block.base_fee_per_gas.map_or(0, |bf| bf.as_u64());
    
    info!("📈 Block environment: #{}, basefee: {} gwei", 
          block_env.number, block_env.basefee / 1_000_000_000);
    
    // Start the performance measurement WITHOUT initial drain
    info!("\n🎯 Starting 1K transaction performance measurement...");
    info!("⏳ Processing mempool transactions as they arrive...");
    
    let mut metrics: Vec<TransactionMetrics> = Vec::new();
    let mut processed_hashes = std::collections::HashSet::new();
    let test_start = Instant::now();
    let target_count = 1000;
    
    // Create CSV file for detailed metrics
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let csv_path = format!("/home/nima/code/crypto/logs/mempool/performance_analysis_{}.csv", timestamp);
    let mut csv_file = File::create(&csv_path)?;
    writeln!(csv_file, "tx_hash,mempool_arrival_ms,fetch_time_ms,simulation_start_ms,simulation_end_ms,time_to_fetch_us,simulation_time_us,state_extraction_time_us,total_processing_time_us,simulation_successful,accounts_affected,eth_changes,token_changes")?;
    
    while metrics.len() < target_count {
        // Fetch new transactions
        let fetch_start = Instant::now();
        let fetch_timestamp = current_timestamp_ms();
        let transactions = fetcher.get_transactions().await?;
        let fetch_duration = fetch_start.elapsed();
        
        if transactions.is_empty() {
            // Wait a bit for new transactions
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        
        info!("📦 Fetched {} transactions in {:.1}ms", 
              transactions.len(), fetch_duration.as_secs_f64() * 1000.0);
        
        // Process only the first few to avoid overwhelming
        let to_process = transactions.into_iter().take(50).collect::<Vec<_>>();
        
        for tx in to_process {
            let tx_hash = hex::encode(&tx.hash);
            
            // Skip if already processed
            if processed_hashes.contains(&tx_hash) {
                continue;
            }
            processed_hashes.insert(tx_hash.clone());
            
            // Assume transaction just arrived when we fetched it
            let mempool_arrival = fetch_timestamp;
            
            // Start simulation
            let simulation_start = Instant::now();
            let simulation_start_ms = current_timestamp_ms();
            
            let simulation_result = tx_simulator.process_transaction(&tx, &block_env).await;
            
            let simulation_end = Instant::now();
            let simulation_end_ms = current_timestamp_ms();
            let simulation_duration = simulation_end.duration_since(simulation_start);
            
            // Extract metrics
            let (simulation_successful, accounts_affected, eth_changes, token_changes) = match simulation_result {
                Ok(Some(changes)) => {
                    let eth_changes = changes.iter()
                        .filter(|(_, c)| c.eth_net_change.absolute_value > revm_primitives::U256::ZERO)
                        .count();
                    let token_changes = changes.iter()
                        .map(|(_, c)| c.token_net_changes.len())
                        .sum();
                    (true, changes.len(), eth_changes, token_changes)
                }
                Ok(None) => (false, 0, 0, 0),
                Err(_) => (false, 0, 0, 0),
            };
            
            let metric = TransactionMetrics {
                tx_hash: tx_hash.clone(),
                mempool_arrival_ms: mempool_arrival,
                fetch_time_ms: fetch_timestamp,
                simulation_start_ms,
                simulation_end_ms,
                time_to_fetch_us: 0, // We don't know exact arrival time
                simulation_time_us: simulation_duration.as_micros() as u64,
                state_extraction_time_us: 0, // Included in simulation time
                total_processing_time_us: simulation_duration.as_micros() as u64,
                simulation_successful,
                accounts_affected,
                eth_changes,
                token_changes,
            };
            
            // Write to CSV
            writeln!(csv_file, "{},{},{},{},{},{},{},{},{},{},{},{},{}",
                metric.tx_hash,
                metric.mempool_arrival_ms,
                metric.fetch_time_ms,
                metric.simulation_start_ms,
                metric.simulation_end_ms,
                metric.time_to_fetch_us,
                metric.simulation_time_us,
                metric.state_extraction_time_us,
                metric.total_processing_time_us,
                metric.simulation_successful,
                metric.accounts_affected,
                metric.eth_changes,
                metric.token_changes
            )?;
            
            metrics.push(metric);
            
            if metrics.len() % 100 == 0 {
                info!("📊 Progress: {}/{} transactions processed ({:.1} tx/s)", 
                      metrics.len(), target_count,
                      metrics.len() as f64 / test_start.elapsed().as_secs_f64());
            }
            
            if metrics.len() >= target_count {
                break;
            }
        }
    }
    
    let test_duration = test_start.elapsed();
    
    // Calculate statistics
    info!("\n📊 Calculating performance statistics...");
    
    let successful_count = metrics.iter().filter(|m| m.simulation_successful).count();
    let failed_count = metrics.len() - successful_count;
    
    let simulation_times: Vec<f64> = metrics.iter()
        .map(|m| m.simulation_time_us as f64 / 1000.0)
        .collect();
    
    let mut sorted_sim_times = simulation_times.clone();
    sorted_sim_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let stats = PerformanceStats {
        total_transactions: metrics.len(),
        successful_simulations: successful_count,
        failed_simulations: failed_count,
        avg_time_to_fetch_ms: 0.0, // Not measured in this version
        avg_simulation_time_ms: simulation_times.iter().sum::<f64>() / simulation_times.len() as f64,
        avg_state_extraction_ms: 0.0, // Included in simulation
        avg_total_processing_ms: metrics.iter()
            .map(|m| m.total_processing_time_us as f64 / 1000.0)
            .sum::<f64>() / metrics.len() as f64,
        p50_simulation_time_ms: sorted_sim_times[sorted_sim_times.len() / 2],
        p95_simulation_time_ms: sorted_sim_times[sorted_sim_times.len() * 95 / 100],
        p99_simulation_time_ms: sorted_sim_times[sorted_sim_times.len() * 99 / 100],
        max_simulation_time_ms: sorted_sim_times[sorted_sim_times.len() - 1],
        transactions_per_second: metrics.len() as f64 / test_duration.as_secs_f64(),
    };
    
    // Save summary statistics
    let json_path = format!("/home/nima/code/crypto/logs/mempool/performance_summary_{}.json", timestamp);
    let json_file = File::create(&json_path)?;
    serde_json::to_writer_pretty(json_file, &stats)?;
    
    // Display results
    info!("\n🎉 Performance Analysis Complete!");
    info!("📊 Summary Statistics:");
    info!("  Total transactions:      {}", stats.total_transactions);
    info!("  Successful simulations:  {} ({:.1}%)", 
          stats.successful_simulations, 
          (stats.successful_simulations as f64 / stats.total_transactions as f64) * 100.0);
    info!("  Failed simulations:      {}", stats.failed_simulations);
    info!("");
    info!("⏱️  Timing Metrics:");
    info!("  Avg simulation time:     {:.2} ms", stats.avg_simulation_time_ms);
    info!("  P50 simulation time:     {:.2} ms", stats.p50_simulation_time_ms);
    info!("  P95 simulation time:     {:.2} ms", stats.p95_simulation_time_ms);
    info!("  P99 simulation time:     {:.2} ms", stats.p99_simulation_time_ms);
    info!("  Max simulation time:     {:.2} ms", stats.max_simulation_time_ms);
    info!("  Avg total processing:    {:.2} ms", stats.avg_total_processing_ms);
    info!("");
    info!("📈 Throughput:");
    info!("  Transactions per second: {:.1}", stats.transactions_per_second);
    info!("  Total test duration:     {:.1} seconds", test_duration.as_secs_f64());
    info!("");
    info!("💾 Results saved to:");
    info!("  CSV: {}", csv_path);
    info!("  JSON: {}", json_path);
    
    // Show sample of state changes
    let total_accounts = metrics.iter().map(|m| m.accounts_affected).sum::<usize>();
    let total_eth_changes = metrics.iter().map(|m| m.eth_changes).sum::<usize>();
    let total_token_changes = metrics.iter().map(|m| m.token_changes).sum::<usize>();
    
    info!("");
    info!("📊 State Change Statistics:");
    info!("  Total accounts affected:  {}", total_accounts);
    info!("  Total ETH changes:        {}", total_eth_changes);
    info!("  Total token changes:      {}", total_token_changes);
    if successful_count > 0 {
        info!("  Avg accounts per tx:      {:.1}", total_accounts as f64 / successful_count as f64);
    }
    
    Ok(())
}