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
use std::sync::Arc;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn};
use tokio::sync::Mutex;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use chrono::Local;
use ethers::types::H256;
use hex;

// Mempool processor imports
use mempool_processor::{
    mempool_fetcher::{NonBlockingIpcClient, MempoolTransaction},
    function_detector::FunctionDetector,
    tx_router::{TransactionRouter as TxRouter, TransactionCategory, SimulationPriority},
    simulator::{UnifiedSimulator, BuySellSimulatorConfig},
    token_tracking::{TokenTrackingCache, TokenTrackingSubscriber},
};
use std::collections::{HashMap, VecDeque};
use alloy_primitives::Address;
use reth_tx_simulator::AddressStateChange;
use mempool_processor::simulator::SequenceSimulationResult;

// ========== Simplified Simulation Manager (No Signal Detection) ==========

/// Types of simulation to perform
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimulationType {
    /// Only simulate the transaction itself
    TransactionOnly,
    /// Simulate transaction then buy/sell sequence
    TransactionWithBuySell,
    /// Only simulate buy/sell (for existing tokens)
    BuySellOnly,
}

/// Request for simulation
#[derive(Debug, Clone)]
pub struct SimulationRequest {
    pub tx: MempoolTransaction,
    pub category: TransactionCategory,
    pub priority: SimulationPriority,
    pub simulation_type: SimulationType,
    pub tx_hash: H256,
}

/// Result of simulation
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub request: SimulationRequest,
    pub tx_state_changes: Option<HashMap<Address, AddressStateChange>>,
    pub buy_sell_result: Option<BuySellResult>,
    pub error: Option<String>,
    pub simulation_time_ms: f64,
    // Addresses needed for tax calculation
    pub token_address: Option<Address>,
    pub pool_address: Option<Address>,
    // Full sequence simulation details for debugging
    pub sequence_result: Option<SequenceSimulationResult>,
    // Debug info for error analysis
    pub debug_info: Option<String>,
}

/// Buy/sell simulation result
#[derive(Debug, Clone)]
pub struct BuySellResult {
    pub can_buy: bool,
    pub can_sell: bool,
    // Raw state changes for tax calculation
    pub buy_state_changes: Option<HashMap<Address, AddressStateChange>>,
    pub sell_state_changes: Option<HashMap<Address, AddressStateChange>>,
}

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
            SimulationPriority::Critical | SimulationPriority::High => self.high_priority.push_back(request),
            SimulationPriority::Normal => self.normal_priority.push_back(request),
            SimulationPriority::Low => self.low_priority.push_back(request),
        }
    }
    
    fn pop(&mut self) -> Option<SimulationRequest> {
        self.high_priority.pop_front()
            .or_else(|| self.normal_priority.pop_front())
            .or_else(|| self.low_priority.pop_front())
    }
    
    fn len(&self) -> usize {
        self.high_priority.len() + self.normal_priority.len() + self.low_priority.len()
    }
}

/// Simplified manager for transaction simulations (no signal detection)
struct SimplifiedSimulationManager {
    simulator: Arc<UnifiedSimulator>,
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
        simulator: Arc<UnifiedSimulator>,
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
                if result.buy_sell_result.is_some() {
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
        simulator: Arc<UnifiedSimulator>,
        token_cache: Arc<TokenTrackingCache>,
    ) -> SimulationResult {
        let mut result = SimulationResult {
            request: request.clone(),
            tx_state_changes: None,
            buy_sell_result: None,
            error: None,
            simulation_time_ms: 0.0,
            token_address: None,
            pool_address: None,
            sequence_result: None,
            debug_info: None,
        };
        
        let start = Instant::now();
        
        // Get token and pool addresses based on category
        let (token_address, pool_address) = match &request.category {
            TransactionCategory::ContractCreation { contract_address, .. } => {
                // For contract creation, the contract is the token
                let token_addr = contract_address.trim_start_matches("0x")
                    .parse::<Address>()
                    .ok();
                // Get pool address from TokenInfo pools directly
                let pool_addr = if let Some(token_info) = token_cache.get_token(contract_address).await {
                    // Get the pool with highest ETH reserve from the token's pools
                    token_info.pools.values()
                        .max_by(|a, b| a.denom_reserve.partial_cmp(&b.denom_reserve).unwrap())
                        .and_then(|pool| pool.pool_address.trim_start_matches("0x").parse::<Address>().ok())
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
                // Get pool address from TokenInfo pools directly
                let pool_addr = if let Some(token_str) = target_token {
                    // Get the full token info which includes all pools
                    if let Some(token_info) = token_cache.get_token(token_str).await {
                        // Get the pool with highest ETH reserve from the token's pools
                        token_info.pools.values()
                            .max_by(|a, b| a.denom_reserve.partial_cmp(&b.denom_reserve).unwrap())
                            .and_then(|pool| pool.pool_address.trim_start_matches("0x").parse::<Address>().ok())
                    } else {
                        None
                    }
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
                match simulator.simulate_single_tx(&full_tx).await {
                    Ok(sim_result) => {
                        result.tx_state_changes = None; // State changes would need separate call
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
                    let tx_call_request = if request.simulation_type == SimulationType::TransactionWithBuySell {
                        match reth_tx_simulator::ipc_to_call_request(&full_tx.tx_data) {
                            Ok(call) => Some(call),
                            Err(e) => {
                                result.error = Some(format!("Failed to convert transaction: {}", e));
                                None
                            }
                        }
                    } else {
                        None
                    };
                    
                    // Run sequence simulation
                    match simulator.simulate_sequence_with_tx(
                        tx_call_request,
                        token_addr,
                        pool_addr,
                        None, // Use latest block
                    ).await {
                        Ok(seq_result) => {
                            // Convert to BuySellResult
                            result.buy_sell_result = Some(BuySellResult {
                                can_buy: seq_result.buy_result.success,
                                can_sell: seq_result.sell_result.success,
                                buy_state_changes: Some(seq_result.buy_result.state_changes.clone()),
                                sell_state_changes: Some(seq_result.sell_result.state_changes.clone()),
                            });
                            
                            // Store the full sequence result for debugging
                            result.sequence_result = Some(seq_result);
                        }
                        Err(e) => {
                            result.error = Some(format!("Buy/sell simulation failed: {}", e));
                        }
                    }
                } else {
                    result.error = Some("Missing token or pool address for buy/sell simulation".to_string());
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
    #[arg(long, env = "RETH_DB_PATH", default_value = "/home/nima/.local/share/reth/mainnet")]
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
    
    let initial_pools = token_cache.pools.get_pool_count().await;
    let initial_creators = token_cache.get_creator_count().await;
    info!("📊 Token Cache Statistics:");
    info!("   Total Pools: {}", initial_pools);
    info!("   Total Creators: {}", initial_creators);
    
    // Log some sample pools
    info!("\n📋 Sample pools in cache:");
    let all_pools = token_cache.pools.get_all_pools().await;
    for (i, (pool_addr, pool_state)) in all_pools.iter().take(5).enumerate() {
        info!("   {}: {} - {:.4} ETH, token: {}", 
            i+1, 
            pool_addr, 
            pool_state.eth_reserve,
            pool_state.token_address
        );
    }
    
    // Log cache details
    info!("\n📡 Cache configuration:");
    info!("   ETH threshold: 0.1 ETH");
    info!("   Monitoring pools above threshold: {}", initial_pools);
    
    // Initialize components
    info!("\n🔧 Initializing pipeline components...");
    
    // IPC client
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");
    
    // Function detector
    let function_detector = FunctionDetector::new();
    info!("✅ Function detector initialized");
    
    // TX Router
    let tx_router = TxRouter::new(Some(token_cache.clone()));
    info!("✅ Transaction router initialized");
    
    // Initialize unified simulator with custom config
    let buy_sell_config = BuySellSimulatorConfig::default();
    let simulator = Arc::new(UnifiedSimulator::with_config(&args.reth_db_path, buy_sell_config)?);
    info!("✅ UnifiedSimulator initialized (no database lock issues!)");
    
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
                TransactionCategory::ContractCreation { .. } |
                TransactionCategory::CreatorTransaction { .. } => {
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
                tx_hash: H256::from_slice(hex::decode(&tx.hash.trim_start_matches("0x")).unwrap_or_default().as_slice()),
            };
            
            // 4. Submit for simulation with more details
            let tx_type = match &category {
                TransactionCategory::ContractCreation { contract_address, .. } => {
                    format!("Creation[{}]", contract_address)
                }
                TransactionCategory::CreatorTransaction { target_token, .. } => {
                    if let Some(token) = target_token {
                        // Check if we have token info in cache
                        if let Some(token_info) = token_cache.get_token(token).await {
                            let pool_count = token_info.pools.len();
                            let has_pools = !token_info.pools.is_empty();
                            format!("Creator[{} p:{} has_pools:{}]", token, pool_count, has_pools)
                        } else {
                            format!("Creator[{} NOT_IN_CACHE]", token)
                        }
                    } else {
                        format!("Creator[Unknown]")
                    }
                }
                _ => "Other".to_string(),
            };
            
            writeln!(log_file, "Submit: {} from 0x{} Type:{}", 
                tx.hash, hex::encode(&tx.from), tx_type)?;
            
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
                TransactionCategory::ContractCreation { contract_address, .. } => {
                    format!(" Token:{} Pool:{}", 
                        result.token_address.map(|a| format!("{:?}", a)).unwrap_or("None".to_string()),
                        result.pool_address.map(|a| format!("{:?}", a)).unwrap_or("None".to_string())
                    )
                }
                TransactionCategory::CreatorTransaction { target_token, .. } => {
                    format!(" Token:{} Pool:{}", 
                        target_token.as_ref().unwrap_or(&"None".to_string()),
                        result.pool_address.map(|a| format!("{:?}", a)).unwrap_or("None".to_string())
                    )
                }
                _ => String::new(),
            };
            
            write!(log_file, "Result: {} from 0x{} - {:.2}ms{}", 
                result.request.tx.hash, 
                hex::encode(&result.request.tx.from),
                result.simulation_time_ms,
                token_info)?;
            
            // Check buy/sell results
            if let Some(buy_sell) = &result.buy_sell_result {
                write!(log_file, " Buy:{} Sell:{}", 
                    if buy_sell.can_buy { "✓" } else { "✗" },
                    if buy_sell.can_sell { "✓" } else { "✗" }
                )?;
                
                if buy_sell.can_buy {
                    can_buy_count += 1;
                }
                if buy_sell.can_sell {
                    can_sell_count += 1;
                }
                if buy_sell.can_buy && buy_sell.can_sell {
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
            info!("Progress: {}/{} transactions ({:.1} tx/sec)", 
                total_processed, args.target_count, rate);
            
            if total_simulations > 0 {
                info!("  Simulations: {} total, {} success, {} failed", 
                    total_simulations, successful_simulations, failed_simulations);
                info!("  Trading: {} can buy, {} can sell, {} both", 
                    can_buy_count, can_sell_count, both_tradeable);
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