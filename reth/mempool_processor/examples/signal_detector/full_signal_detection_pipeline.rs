use chrono::Local;
use clap::Parser;
use ethers::types::H256;
use eyre::Result;
use hex;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::sync::Arc;
/// Simulation Only Test
///
/// Tests the simulation pipeline WITHOUT signal detection
/// Logs detailed simulation results for debugging
///
/// This example:
/// 1. Runs transactions through simulation
/// 2. Logs all simulation details to dev directory
/// 3. Skips signal manager processing entirely
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, warn};

// Mempool processor imports
use alloy_primitives::{Address, U256};
use mempool_processor::{
    function_detector::FunctionDetector,
    mempool_fetcher::{MempoolFetcherIPCClient, MempoolTransaction},
    simulator::MempoolSimulator,
    token_tracking::{TokenTrackingCache, TokenTrackingSubscriber},
    tx_router::{SimulationPriority, TransactionCategory, TransactionRouter as TxRouter},
};
use std::collections::{HashMap, VecDeque};
// Import pool simulation types from tx_processor
use std::str::FromStr;
use tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::{
    config::PoolViabilityConfig,
    types::{PoolType, PoolViabilityResult},
};
// Import the correct types from simulation_manager
use mempool_processor::simulator::simulation_manager::{
    BuySellResult, SimulationRequest, SimulationResult, SimulationType,
};

// ========== Simplified Simulation Manager (No Signal Detection) ==========

// SimulationRequest, SimulationType are imported from simulation_manager

// SimulationResult and BuySellResult are imported from simulation_manager

/// Simple queue for simulation requests
struct SimulationQueue {
    high_priority: VecDeque<SimulationRequest>,
    normal_priority: VecDeque<SimulationRequest>,
    low_priority: VecDeque<SimulationRequest>,
}

impl SimulationQueue {
    fn new() -> Self {
        Self {
            high_priority: VecDeque::new(),
            normal_priority: VecDeque::new(),
            low_priority: VecDeque::new(),
        }
    }

    fn push(&mut self, request: SimulationRequest) {
        match request.priority {
            SimulationPriority::Critical | SimulationPriority::High => {
                self.high_priority.push_back(request)
            }
            SimulationPriority::Normal => self.normal_priority.push_back(request),
            SimulationPriority::Low => self.low_priority.push_back(request),
        }
    }

    fn pop(&mut self) -> Option<SimulationRequest> {
        self.high_priority
            .pop_front()
            .or_else(|| self.normal_priority.pop_front())
            .or_else(|| self.low_priority.pop_front())
    }

    fn len(&self) -> usize {
        self.high_priority.len() + self.normal_priority.len() + self.low_priority.len()
    }
}

/// Simplified manager for transaction simulations (no signal detection)
struct SimplifiedSimulationManager {
    simulator: Arc<MempoolSimulator>,
    queue: Arc<Mutex<SimulationQueue>>,
    token_cache: Arc<TokenTrackingCache>,
    max_concurrent_simulations: usize,
    stats: Arc<Mutex<ManagerStats>>,
}

#[derive(Debug, Default, Clone)]
struct ManagerStats {
    total_requests: u64,
    successful_simulations: u64,
    failed_simulations: u64,
    buy_sell_tests: u64,
    avg_simulation_time_ms: f64,
    max_simulation_time_ms: f64,
}

impl SimplifiedSimulationManager {
    /// Create new simplified simulation manager
    pub fn new(
        simulator: Arc<MempoolSimulator>,
        token_cache: Arc<TokenTrackingCache>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            simulator,
            queue: Arc::new(Mutex::new(SimulationQueue::new())),
            token_cache,
            max_concurrent_simulations: max_concurrent,
            stats: Arc::new(Mutex::new(ManagerStats::default())),
        }
    }

    /// Submit a request for simulation
    pub async fn submit(&self, request: SimulationRequest) -> Result<(), String> {
        let mut queue = self.queue.lock().await;
        if queue.len() > 10000 {
            return Err("Queue is full".to_string());
        }
        queue.push(request);
        Ok(())
    }

    /// Process queued simulations and return results
    pub async fn process_queue(&self) -> Vec<SimulationResult> {
        let mut results = Vec::new();
        let mut handles = Vec::new();

        // Get up to max_concurrent requests from queue
        let requests: Vec<SimulationRequest> = {
            let mut queue = self.queue.lock().await;
            let mut batch = Vec::new();
            for _ in 0..self.max_concurrent_simulations {
                if let Some(req) = queue.pop() {
                    batch.push(req);
                } else {
                    break;
                }
            }
            batch
        };

        // Process each request concurrently
        for request in requests {
            let simulator = self.simulator.clone();
            let token_cache = self.token_cache.clone();
            let stats = self.stats.clone();

            let handle = tokio::spawn(async move {
                let start = Instant::now();
                let result = Self::simulate_request(request, simulator, token_cache).await;
                let elapsed = start.elapsed().as_millis() as f64;

                // Update stats
                let mut stats = stats.lock().await;
                stats.total_requests += 1;
                if result.error.is_none() {
                    stats.successful_simulations += 1;
                } else {
                    stats.failed_simulations += 1;
                }
                if result.pool_viability_result.is_some() {
                    stats.buy_sell_tests += 1;
                }
                stats.max_simulation_time_ms = stats.max_simulation_time_ms.max(elapsed);
                stats.avg_simulation_time_ms =
                    (stats.avg_simulation_time_ms * (stats.total_requests - 1) as f64 + elapsed)
                        / stats.total_requests as f64;

                result
            });
            handles.push(handle);
        }

        // Collect results
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        results
    }

    /// Simulate a single request
    async fn simulate_request(
        request: SimulationRequest,
        simulator: Arc<MempoolSimulator>,
        token_cache: Arc<TokenTrackingCache>,
    ) -> SimulationResult {
        let mut result = SimulationResult {
            request: request.clone(),
            pool_viability_result: None,
            error: None,
            simulation_time_ms: 0.0,
            token_address: None,
            pool_address: None,
            pool_type: None,
            debug_info: None,
            liquidity_removal_result: None,
        };

        let start = Instant::now();

        // Get token and pool addresses based on category
        let (token_address, pool_address) = match &request.category {
            TransactionCategory::ContractCreation {
                contract_address, ..
            } => {
                // For contract creation, the contract is the token
                let token_addr = contract_address
                    .trim_start_matches("0x")
                    .parse::<Address>()
                    .ok();
                // Get pool address from token's pools
                let pool_addr = if let Some(addr) = token_addr {
                    let pools = token_cache.get_pools_for_token(&addr).await;
                    pools
                        .iter()
                        .max_by(|a, b| a.eth_reserve.partial_cmp(&b.eth_reserve).unwrap())
                        .and_then(|pool| pool.address.parse::<Address>().ok())
                } else {
                    None
                };
                (token_addr, pool_addr)
            }
            TransactionCategory::CreatorTransaction { target_token, .. } => {
                // Get token address
                let token_addr = if let Some(token) = target_token {
                    token.trim_start_matches("0x").parse::<Address>().ok()
                } else {
                    None
                };
                // Get pool address from token's pools
                let pool_addr = if let Some(addr) = token_addr {
                    let pools = token_cache.get_pools_for_token(&addr).await;
                    pools
                        .iter()
                        .max_by(|a, b| a.eth_reserve.partial_cmp(&b.eth_reserve).unwrap())
                        .and_then(|pool| pool.address.parse::<Address>().ok())
                } else {
                    None
                };
                (token_addr, pool_addr)
            }
            _ => (None, None),
        };

        result.token_address = token_address;
        result.pool_address = pool_address;

        // Convert transaction for simulation
        let full_tx = mempool_processor::mempool_fetcher::FullTransaction {
            hash: request.tx.hash.clone(),
            tx_data: request.tx.data.clone(),
            detection_time: std::time::Instant::now(),
            latency_ns: request.tx.detection_ns,
        };

        // Perform simulation based on type
        match request.simulation_type {
            SimulationType::TransactionOnly => {
                // Only simulate the transaction
                match simulator.simulate_mempool_tx(&full_tx).await {
                    Ok(sim_result) => {
                        // State changes are in pool_viability_result if we had one
                        if let Some(reason) = sim_result.revert_reason {
                            result.error = Some(format!("Transaction reverted: {}", reason));
                        }
                    }
                    Err(e) => {
                        result.error = Some(format!("Transaction simulation failed: {}", e));
                    }
                }
            }
            SimulationType::TransactionWithBuySell | SimulationType::BuySellOnly => {
                // Need both token and pool addresses
                if let (Some(token_addr), Some(pool_addr)) = (token_address, pool_address) {
                    // Create call request for the transaction
                    let tx_call_request =
                        if request.simulation_type == SimulationType::TransactionWithBuySell {
                            match mempool_processor::common::convert::ipc_to_call_request(
                                &full_tx.tx_data,
                            ) {
                                Ok(call) => Some(call),
                                Err(e) => {
                                    result.error =
                                        Some(format!("Failed to convert transaction: {}", e));
                                    None
                                }
                            }
                        } else {
                            None
                        };

                    // Run pool buy/sell simulation
                    let config = tx_processor::config::PoolViabilityConfig {
                        token_address: token_addr,
                        pool_address: pool_addr.to_string(),
                        pool_type: PoolType::UniswapV2,
                        test_amount: U256::from(10_000_000_000_000_000u64), // 0.01 ETH
                        buyer_address: Address::from_str(
                            "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689",
                        )
                        .unwrap(),
                        block_number: None,
                        gas_limit: 500_000,
                        gas_price: 30_000_000_000,
                        prior_tx: tx_call_request,
                        block_delay: 0,
                        slippage_tolerance: 0.5,
                        token_decimals: 18,
                        weth_address: Address::from_str(
                            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                        )
                        .unwrap(),
                    };

                    match simulator.simulate_pool_buy_sell(config).await {
                        Ok(sim_result) => {
                            result.pool_viability_result = Some(sim_result);
                            /*
                                    can_buy: pool_result.can_buy,
                                    can_sell: pool_result.can_sell,
                                    buy_tax: Some(pool_result.buy_tax_percent),
                                    sell_tax: Some(pool_result.sell_tax_percent),
                            */
                        }
                        Err(e) => {
                            result.error = Some(format!("Buy/sell simulation failed: {}", e));
                        }
                    }
                } else {
                    result.error =
                        Some("Missing token or pool address for buy/sell simulation".to_string());
                }
            }
        }

        result.simulation_time_ms = start.elapsed().as_millis() as f64;
        result
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> ManagerStats {
        self.stats.lock().await.clone()
    }

    /// Get queue length
    pub async fn queue_length(&self) -> usize {
        self.queue.lock().await.len()
    }
}

// ========== End of Simplified Simulation Manager ==========

#[derive(Parser, Debug)]
struct Args {
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,

    /// Reth database path
    #[arg(
        long,
        env = "RETH_DB_PATH",
        default_value = "/home/nima/.local/share/reth/mainnet"
    )]
    reth_db_path: String,

    /// Number of transactions to process
    #[arg(long, default_value = "10000")]
    target_count: usize,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(if args.verbose { "debug" } else { "info" })
        .init();

    // Create log directory
    let log_dir = "/home/nima/code/crypto/logs/mempool/dev/simulation_only";
    create_dir_all(log_dir)?;

    // Create log file
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_path = format!("{}/simulation_test_{}.log", log_dir, timestamp);
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;

    writeln!(log_file, "Simulation Only Test - NO Signal Detection")?;
    writeln!(log_file, "Started: {}", Local::now())?;
    writeln!(log_file, "Target: {} transactions", args.target_count)?;
    writeln!(log_file, "======================================\n")?;

    info!("🚀 Starting Simulation Only Test");
    info!("📝 Logging to: {}", log_path);
    info!("🎯 Target: {} transactions", args.target_count);
    info!("⚠️  Signal detection is DISABLED for this test");

    // Initialize token tracking subscriber
    info!("\n📦 Initializing token tracking subscriber...");
    let mut token_subscriber = TokenTrackingSubscriber::new(0.1); // 0.1 ETH threshold
    let token_cache = token_subscriber.get_cache();

    // Start subscriber in background
    let _subscriber_handle = tokio::spawn(async move {
        if let Err(e) = token_subscriber.start_listening().await {
            warn!("Token subscriber error: {}", e);
        }
    });

    // Wait for initial cache population
    info!("⏳ Waiting for token cache population...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let initial_pools = token_cache.get_pool_count().await;
    let initial_creators = token_cache.get_creator_count().await;
    info!("📊 Token Cache Statistics:");
    info!("   Total Pools: {}", initial_pools);
    info!("   Total Creators: {}", initial_creators);

    // Log some sample pools
    info!("\n📋 Sample pools in cache:");
    // Note: get_all_pools is not exposed publicly
    // Commenting out sample pool logging for now
    /*
    for (i, (pool_addr, pool_state)) in all_pools.iter().take(5).enumerate() {
        info!("   {}: {} - {:.4} ETH, token: {}",
            i+1,
            pool_addr,
            pool_state.eth_reserve,
            pool_state.token_address
        );
    }
    */

    // Log cache details
    info!("\n📡 Cache configuration:");
    info!("   ETH threshold: 0.1 ETH");
    info!("   Monitoring pools above threshold: {}", initial_pools);

    // Initialize components
    info!("\n🔧 Initializing pipeline components...");

    // IPC client
    let ipc_client = MempoolFetcherIPCClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");

    // Function detector
    let function_detector = FunctionDetector::new();
    info!("✅ Function detector initialized");

    // TX Router
    let tx_router = TxRouter::new(Some(token_cache.clone()));
    info!("✅ Transaction router initialized");

    // Initialize unified simulator with custom config
    let simulator = Arc::new(MempoolSimulator::new(&args.reth_db_path)?);
    info!("✅ MempoolSimulator initialized (no database lock issues!)");

    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    info!("📦 Using latest block: {}", latest_block);

    // Create simplified simulation manager
    let simulation_manager = SimplifiedSimulationManager::new(
        simulator,
        token_cache.clone(),
        10, // max concurrent simulations
    );
    info!("✅ Simplified simulation manager initialized (no signal detection)");

    info!("\n🏃 Starting simulation testing...\n");

    let mut total_processed = 0;
    let pipeline_start = Instant::now();

    // Stats tracking
    let mut total_simulations = 0;
    let mut successful_simulations = 0;
    let mut failed_simulations = 0;
    let mut can_buy_count = 0;
    let mut can_sell_count = 0;
    let mut both_tradeable = 0;

    while total_processed < args.target_count {
        // Fetch transactions
        let new_txs = ipc_client.get_transactions_instant(100).await;

        if new_txs.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        // Process each transaction
        for tx in new_txs {
            if total_processed >= args.target_count {
                break;
            }

            total_processed += 1;

            // 1. Function Detection
            let _detected_function = function_detector.detect_function(&tx);

            // 2. Transaction Routing
            let classification = tx_router.classify(&tx).await;
            let category = classification.category;

            // Skip non-relevant transactions (but log what we're seeing)
            match &category {
                TransactionCategory::ContractCreation { .. }
                | TransactionCategory::CreatorTransaction { .. } => {
                    // Continue with simulation
                }
                _ => {
                    // Log what types we're skipping every 100 transactions
                    if total_processed % 100 == 0 {
                        writeln!(log_file, "Skipping: {} - {:?}", tx.hash, category)?;
                    }
                    continue;
                }
            }

            // 3. Create simulation request
            let sim_request = SimulationRequest {
                tx: tx.clone(),
                category: category.clone(),
                priority: match &category {
                    TransactionCategory::ContractCreation { .. } => SimulationPriority::High,
                    TransactionCategory::CreatorTransaction { .. } => SimulationPriority::Normal,
                    _ => SimulationPriority::Low,
                },
                simulation_type: SimulationType::TransactionWithBuySell,
                tx_hash: H256::from_slice(
                    hex::decode(&tx.hash.trim_start_matches("0x"))
                        .unwrap_or_default()
                        .as_slice(),
                ),
            };

            // 4. Submit for simulation with more details
            let tx_type = match &category {
                TransactionCategory::ContractCreation {
                    contract_address, ..
                } => {
                    format!("Creation[{}]", contract_address)
                }
                TransactionCategory::CreatorTransaction { target_token, .. } => {
                    if let Some(token) = target_token {
                        // Check if we have token info in cache
                        if let Some(token_info) = token_cache.get_token(token).await {
                            let pools = token_cache.get_pools_for_token(&token_info.address).await;
                            let pool_count = pools.len();
                            let has_pools = !pools.is_empty();
                            format!(
                                "Creator[{} p:{} has_pools:{}]",
                                token, pool_count, has_pools
                            )
                        } else {
                            format!("Creator[{} NOT_IN_CACHE]", token)
                        }
                    } else {
                        format!("Creator[Unknown]")
                    }
                }
                _ => "Other".to_string(),
            };

            writeln!(
                log_file,
                "Submit: {} from 0x{} Type:{}",
                tx.hash,
                hex::encode(&tx.from),
                tx_type
            )?;

            if let Err(e) = simulation_manager.submit(sim_request.clone()).await {
                warn!("Failed to submit simulation for {}: {}", tx.hash, e);
                continue;
            }
        }

        // Process simulations in batches
        let results = simulation_manager.process_queue().await;

        for result in results {
            total_simulations += 1;

            // Log simulation result with token/pool info
            let token_info = match &result.request.category {
                TransactionCategory::ContractCreation {
                    contract_address, ..
                } => {
                    format!(
                        " Token:{} Pool:{}",
                        result
                            .token_address
                            .map(|a| format!("{:?}", a))
                            .unwrap_or("None".to_string()),
                        result
                            .pool_address
                            .map(|a| format!("{:?}", a))
                            .unwrap_or("None".to_string())
                    )
                }
                TransactionCategory::CreatorTransaction { target_token, .. } => {
                    format!(
                        " Token:{} Pool:{}",
                        target_token.as_ref().unwrap_or(&"None".to_string()),
                        result
                            .pool_address
                            .map(|a| format!("{:?}", a))
                            .unwrap_or("None".to_string())
                    )
                }
                _ => String::new(),
            };

            write!(
                log_file,
                "Result: {} from 0x{} - {:.2}ms{}",
                result.request.tx.hash,
                hex::encode(&result.request.tx.from),
                result.simulation_time_ms,
                token_info
            )?;

            // Check buy/sell results
            if let Some(pool_result) = &result.pool_viability_result {
                write!(
                    log_file,
                    " Buy:{} Sell:{}",
                    if pool_result.can_buy { "✓" } else { "✗" },
                    if pool_result.can_sell { "✓" } else { "✗" }
                )?;

                if pool_result.can_buy {
                    can_buy_count += 1;
                }
                if pool_result.can_sell {
                    can_sell_count += 1;
                }
                if pool_result.can_buy && pool_result.can_sell {
                    both_tradeable += 1;
                }
            }

            if let Some(error) = &result.error {
                failed_simulations += 1;
                write!(log_file, " Error: {}", error)?;
            } else {
                successful_simulations += 1;
            }

            writeln!(log_file)?;
        }

        // Progress update every 1000 transactions
        if total_processed % 1000 == 0 {
            let elapsed = pipeline_start.elapsed();
            let rate = total_processed as f64 / elapsed.as_secs_f64();
            info!(
                "Progress: {}/{} transactions ({:.1} tx/sec)",
                total_processed, args.target_count, rate
            );

            if total_simulations > 0 {
                info!(
                    "  Simulations: {} total, {} success, {} failed",
                    total_simulations, successful_simulations, failed_simulations
                );
                info!(
                    "  Trading: {} can buy, {} can sell, {} both",
                    can_buy_count, can_sell_count, both_tradeable
                );
            }
        }
    }

    let total_time = pipeline_start.elapsed();

    // Final summary - only show meaningful metrics
    let summary = if total_simulations > 0 {
        format!(
            "\n📊 SIMULATION TEST SUMMARY\n\
            =====================================\n\
            Total Transactions Processed: {}\n\
            Total Simulations Run: {}\n\
            Successful Simulations: {} ({:.1}%)\n\
            Failed Simulations: {} ({:.1}%)\n\
            \n\
            Trading Results:\n\
            - Can Buy: {} ({:.1}% of successful)\n\
            - Can Sell: {} ({:.1}% of successful)\n\
            - Both (Tradeable): {} ({:.1}% of successful)\n\
            \n\
            Time Elapsed: {:.2}s\n\
            Avg Simulation Time: {:.1}ms",
            total_processed,
            total_simulations,
            successful_simulations,
            successful_simulations as f64 / total_simulations as f64 * 100.0,
            failed_simulations,
            failed_simulations as f64 / total_simulations as f64 * 100.0,
            can_buy_count,
            can_buy_count as f64 / successful_simulations as f64 * 100.0,
            can_sell_count,
            can_sell_count as f64 / successful_simulations as f64 * 100.0,
            both_tradeable,
            both_tradeable as f64 / successful_simulations as f64 * 100.0,
            total_time.as_secs_f64(),
            total_time.as_millis() as f64 / total_simulations as f64
        )
    } else {
        format!(
            "\n📊 SIMULATION TEST SUMMARY\n\
            =====================================\n\
            Total Transactions Processed: {}\n\
            No simulations were run (no contract creations or creator transactions found)\n\
            Time Elapsed: {:.2}s",
            total_processed,
            total_time.as_secs_f64()
        )
    };

    println!("{}", summary);
    writeln!(log_file, "{}", summary)?;

    info!("\n✅ Simulation test complete!");
    info!("📄 Results saved to: {}", log_path);
    info!("🔍 This test ran WITHOUT signal detection");
    info!("   Use this log to debug simulation issues");

    Ok(())
}
