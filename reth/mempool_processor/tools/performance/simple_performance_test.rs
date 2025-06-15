use mempool_fetcher::mempool_fetcher::fetcher::{MempoolFetcher, FetchMode};
use mempool_fetcher::mempool_fetcher::TransactionSource;
use mempool_fetcher::tx_simulator::TransactionSimulator;
use revm_context::BlockEnv as RevmBlockEnv;
use revm_primitives::hardfork::SpecId;
use ethers::providers::{Http as EthersHttp, Middleware, Provider as EthersProvider};
use ethers::types::{BlockId as EthersBlockId, BlockNumber as EthersBlockNumber};
use revm_tx_simulator_lib::conversions::{ethers_to_revm_u256, ethers_to_revm_address};
use tracing::{info, Level};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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

    info!("🚀 Simple Performance Test - Measuring 1K Transaction Processing");
    
    let eth_rpc_url = "http://localhost:8545";
    
    // Initialize components
    info!("📡 Initializing mempool fetcher...");
    let mut fetcher = MempoolFetcher::with_websocket_support(
        eth_rpc_url,
        None,
        5000,  // Smaller cache
        true,
        100,   // Smaller batch size
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
    
    info!("📈 Block: #{}", block_env.number);
    
    let test_start = Instant::now();
    let mut processed = 0;
    let mut successful = 0;
    let mut total_fetch_time_ms = 0.0;
    let mut total_sim_time_ms = 0.0;
    let mut total_accounts = 0;
    
    // Track unique transactions
    let mut seen_hashes = std::collections::HashSet::new();
    
    info!("⏳ Processing transactions...");
    
    while processed < 1000 {
        // Measure fetch time
        let fetch_start = Instant::now();
        let transactions = fetcher.get_transactions().await?;
        let fetch_time = fetch_start.elapsed();
        
        if transactions.is_empty() {
            continue;
        }
        
        total_fetch_time_ms += fetch_time.as_secs_f64() * 1000.0;
        
        // Process each transaction
        for tx in transactions.iter().take(10) { // Process max 10 at a time
            let tx_hash = hex::encode(&tx.hash);
            
            // Skip duplicates
            if seen_hashes.contains(&tx_hash) {
                continue;
            }
            seen_hashes.insert(tx_hash.clone());
            
            // Measure simulation time
            let sim_start = Instant::now();
            let result = tx_simulator.process_transaction(tx, &block_env).await;
            let sim_time = sim_start.elapsed();
            
            total_sim_time_ms += sim_time.as_secs_f64() * 1000.0;
            
            match result {
                Ok(Some(changes)) => {
                    successful += 1;
                    total_accounts += changes.len();
                }
                Ok(None) => {}
                Err(_) => {}
            }
            
            processed += 1;
            
            if processed % 100 == 0 {
                let elapsed = test_start.elapsed().as_secs_f64();
                info!("📊 Progress: {}/1000 ({:.1} tx/s)", processed, processed as f64 / elapsed);
            }
            
            if processed >= 1000 {
                break;
            }
        }
    }
    
    let total_time = test_start.elapsed();
    
    // Calculate results
    let avg_fetch_ms = total_fetch_time_ms / processed as f64;
    let avg_sim_ms = total_sim_time_ms / processed as f64;
    let tx_per_sec = processed as f64 / total_time.as_secs_f64();
    let success_rate = (successful as f64 / processed as f64) * 100.0;
    let avg_accounts = if successful > 0 { total_accounts as f64 / successful as f64 } else { 0.0 };
    
    info!("\n🎉 Performance Test Complete!");
    info!("=====================================");
    info!("📊 SUMMARY:");
    info!("  Total transactions:     {}", processed);
    info!("  Successful simulations: {} ({:.1}%)", successful, success_rate);
    info!("  Total time:            {:.1}s", total_time.as_secs_f64());
    info!("  Throughput:            {:.1} tx/s", tx_per_sec);
    info!("");
    info!("⏱️  TIMING BREAKDOWN:");
    info!("  Avg fetch time:        {:.2}ms", avg_fetch_ms);
    info!("  Avg simulation time:   {:.2}ms", avg_sim_ms);
    info!("  Total per tx:          {:.2}ms", avg_fetch_ms + avg_sim_ms);
    info!("");
    info!("📊 STATE CHANGES:");
    info!("  Avg accounts per tx:   {:.1}", avg_accounts);
    info!("  Total accounts:        {}", total_accounts);
    info!("");
    info!("🎯 KEY INSIGHTS:");
    info!("  - Mempool arrival → fetch: ~{}ms (estimated)", avg_fetch_ms as u64);
    info!("  - Fetch → state diffs:     ~{}ms", avg_sim_ms as u64);
    info!("  - Total latency:           ~{}ms", (avg_fetch_ms + avg_sim_ms) as u64);
    
    Ok(())
}