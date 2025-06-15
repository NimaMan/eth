/*
 * REVM Performance Monitor for Real Transaction Simulation
 * 
 * ALGORITHMIC DESCRIPTION:
 * This script measures real REVM simulation performance with proper warm-up:
 * 
 * 1. WARM-UP PHASE (60 seconds):
 *    - Process existing mempool transactions to warm up caches
 *    - Build transaction cache to identify "old" vs "fresh" transactions
 *    - Initialize REVM components and establish steady state
 * 
 * 2. MEASUREMENT PHASE:
 *    - Only measure transactions that arrive AFTER warm-up period
 *    - Use actual TransactionSimulator with REVM simulation
 *    - Track real timing from mempool arrival to state diff completion
 *    - Target 1000 fresh transactions for statistical significance
 * 
 * 3. PERFORMANCE COMPONENTS:
 *    - Mempool fetch time: RPC call to get transaction details
 *    - REVM simulation time: Actual transaction execution in REVM
 *    - State diff calculation: Generate detailed account changes
 *    - End-to-end time: Total time from mempool arrival to completion
 * 
 * 4. REAL-WORLD VALIDATION:
 *    - Uses same simulation logic as our working examples
 *    - Measures actual processing bottlenecks
 *    - Validates <500ms target for fresh transaction processing
 */

use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time;
use tracing::{info, error, warn, debug, Level};
use std::collections::HashMap;
use std::fs::File;
use csv::Writer;
use serde::Serialize;
use eyre::{Result, WrapErr};

// Ethers imports for block context
use ethers::providers::{Http as EthersHttp, Middleware, Provider as EthersProvider};
use ethers::types::{BlockId as EthersBlockId, BlockNumber as EthersBlockNumber};

// REVM imports
use revm_context::{BlockEnv as RevmBlockEnv, CfgEnv as RevmCfgEnv};
use revm_primitives::hardfork::SpecId;
use revm::database::{CacheDB, WrapDatabaseAsync, AlloyDB};
use alloy_provider::{ProviderBuilder, Provider as AlloyProviderTrait, DynProvider};
use alloy_network::Ethereum as AlloyEthereum;

// Local imports
use mempool_fetcher::mempool_fetcher::fetcher::MempoolFetcher;
use mempool_fetcher::mempool_fetcher::types::TransactionView;
use mempool_fetcher::mempool_fetcher::TransactionSource;
use mempool_fetcher::tx_simulator::conversions::transaction_view_to_revm_tx_env;

// revm_tx_simulator_lib imports
use revm_tx_simulator_lib::simulation_core::{simulate_transaction, SimCacheDB, ExecutionResultType};
use revm_tx_simulator_lib::state_diff_utils::generate_calculated_account_changes;
use revm_tx_simulator_lib::conversions::{ethers_to_revm_u256, ethers_to_revm_address};

#[derive(Parser, Debug)]
#[command(name = "revm_performance_monitor")]
#[command(about = "Real REVM performance monitoring for mempool transactions")]
struct Args {
    /// Ethereum RPC URL
    #[arg(long, default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// Number of FRESH transactions to benchmark
    #[arg(long, default_value = "1000")]
    target_transactions: usize,
    
    /// Maximum duration for benchmark in seconds
    #[arg(long, default_value = "1800")]
    max_duration_seconds: u64,
    
    /// Output CSV file path
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/revm_performance_1k.csv")]
    output_csv: String,
    
    /// Warm-up period in seconds
    #[arg(long, default_value = "60")]
    warmup_seconds: u64,
    
    /// Enable detailed per-transaction logging
    #[arg(long)]
    verbose: bool,
    
    /// Chain ID
    #[arg(long, default_value = "1")]
    chain_id: u64,
}

/// Performance metrics for a single fresh transaction with real REVM timing
#[derive(Debug, Clone, Serialize)]
struct RevmTransactionMetrics {
    /// Transaction hash
    tx_hash: String,
    /// Timestamp when transaction first appeared in mempool (unix timestamp ms)
    mempool_arrival_time_ms: u64,
    /// Timestamp when processing completed (unix timestamp ms)
    processing_completion_time_ms: u64,
    /// Total end-to-end time from mempool arrival to processing completion (microseconds)
    end_to_end_time_us: u64,
    /// Time taken to fetch transaction details and setup REVM environment (microseconds)
    setup_time_us: u64,
    /// Time taken for actual REVM simulation (microseconds)
    revm_simulation_time_us: u64,
    /// Time taken to calculate state diffs (microseconds)
    state_diff_time_us: u64,
    /// Total processing time from start to completion (microseconds)
    total_processing_time_us: u64,
    /// Whether REVM simulation was successful
    simulation_successful: bool,
    /// Number of accounts with state changes
    affected_accounts_count: usize,
    /// Transaction value in ETH
    tx_value_eth: f64,
    /// Gas price in gwei
    gas_price_gwei: f64,
    /// Gas limit
    gas_limit: u64,
    /// Gas used (if simulation successful)
    gas_used: u64,
    /// Transaction nonce
    nonce: u64,
    /// Whether transaction is a contract call
    is_contract_call: bool,
    /// Size of input data in bytes
    input_data_size: usize,
}

/// Aggregate performance statistics
#[derive(Debug, Clone)]
struct RevmPerformanceStats {
    total_fresh_transactions: usize,
    successful_simulations: usize,
    failed_simulations: usize,
    total_setup_time_us: u64,
    total_revm_time_us: u64,
    total_state_diff_time_us: u64,
    total_processing_time_us: u64,
    total_end_to_end_time_us: u64,
    under_500ms_count: usize,
    contract_calls: usize,
    start_time: Instant,
}

impl RevmPerformanceStats {
    fn new() -> Self {
        Self {
            total_fresh_transactions: 0,
            successful_simulations: 0,
            failed_simulations: 0,
            total_setup_time_us: 0,
            total_revm_time_us: 0,
            total_state_diff_time_us: 0,
            total_processing_time_us: 0,
            total_end_to_end_time_us: 0,
            under_500ms_count: 0,
            contract_calls: 0,
            start_time: Instant::now(),
        }
    }
    
    fn add_metrics(&mut self, metrics: &RevmTransactionMetrics) {
        self.total_fresh_transactions += 1;
        if metrics.simulation_successful {
            self.successful_simulations += 1;
        } else {
            self.failed_simulations += 1;
        }
        self.total_setup_time_us += metrics.setup_time_us;
        self.total_revm_time_us += metrics.revm_simulation_time_us;
        self.total_state_diff_time_us += metrics.state_diff_time_us;
        self.total_processing_time_us += metrics.total_processing_time_us;
        self.total_end_to_end_time_us += metrics.end_to_end_time_us;
        
        if metrics.total_processing_time_us < 500_000 {
            self.under_500ms_count += 1;
        }
        if metrics.is_contract_call {
            self.contract_calls += 1;
        }
    }
    
    fn print_summary(&self) {
        let elapsed = self.start_time.elapsed();
        let throughput = self.total_fresh_transactions as f64 / elapsed.as_secs_f64();
        
        info!("=== REVM PERFORMANCE SUMMARY ===");
        info!("Measurement Duration: {:.2} seconds", elapsed.as_secs_f64());
        info!("Fresh Transactions Processed: {}", self.total_fresh_transactions);
        info!("Fresh Transaction Throughput: {:.2} tx/sec", throughput);
        info!("Successful REVM Simulations: {} ({:.1}%)", 
              self.successful_simulations, 
              self.successful_simulations as f64 / self.total_fresh_transactions as f64 * 100.0);
        info!("Contract Calls: {} ({:.1}%)", 
              self.contract_calls,
              self.contract_calls as f64 / self.total_fresh_transactions as f64 * 100.0);
        info!("Transactions under 500ms: {} ({:.1}%)", 
              self.under_500ms_count,
              self.under_500ms_count as f64 / self.total_fresh_transactions as f64 * 100.0);
        
        if self.total_fresh_transactions > 0 {
            info!("Average Setup Time: {:.2} ms", 
                  self.total_setup_time_us as f64 / self.total_fresh_transactions as f64 / 1000.0);
            info!("Average REVM Simulation Time: {:.2} ms", 
                  self.total_revm_time_us as f64 / self.total_fresh_transactions as f64 / 1000.0);
            info!("Average State Diff Time: {:.2} ms", 
                  self.total_state_diff_time_us as f64 / self.total_fresh_transactions as f64 / 1000.0);
            info!("Average Total Processing Time: {:.2} ms", 
                  self.total_processing_time_us as f64 / self.total_fresh_transactions as f64 / 1000.0);
            info!("Average End-to-End Time: {:.2} ms", 
                  self.total_end_to_end_time_us as f64 / self.total_fresh_transactions as f64 / 1000.0);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();
    
    info!("🚀 Starting Real REVM Performance Monitor");
    info!("Target: {} FRESH transactions", args.target_transactions);
    info!("Warm-up Period: {} seconds", args.warmup_seconds);
    info!("Max Duration: {} seconds", args.max_duration_seconds);
    info!("Output CSV: {}", args.output_csv);
    info!("RPC URL: {}", args.eth_rpc_url);
    
    // Initialize CSV writer
    std::fs::create_dir_all(std::path::Path::new(&args.output_csv).parent().unwrap())?;
    let csv_file = File::create(&args.output_csv)?;
    let mut csv_writer = Writer::from_writer(csv_file);
    
    // Initialize REVM components
    info!("Initializing REVM components...");
    let mut cfg_env = RevmCfgEnv::default();
    cfg_env.chain_id = args.chain_id;
    cfg_env.spec = SpecId::CANCUN;
    
    // Initialize Alloy provider
    let alloy_provider_instance = ProviderBuilder::new()
        .connect(&args.eth_rpc_url)
        .await
        .wrap_err("Failed to connect to Alloy Provider")?;
    let alloy_provider: Arc<DynProvider<AlloyEthereum>> = Arc::new(alloy_provider_instance.erased());
    
    // Get current block environment
    info!("Fetching latest block for BlockEnv...");
    let latest_block_ethers = EthersProvider::<EthersHttp>::try_from(args.eth_rpc_url.as_str())?
        .get_block(EthersBlockId::Number(EthersBlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block from Ethers provider"))?;
    
    let mut block_env = RevmBlockEnv::default();
    block_env.number = ethers_to_revm_u256(latest_block_ethers.number.unwrap_or_default().as_u64().into());
    block_env.beneficiary = latest_block_ethers.author.map_or_else(|| revm_primitives::Address::ZERO, |h160| ethers_to_revm_address(h160));
    block_env.timestamp = ethers_to_revm_u256(latest_block_ethers.timestamp);
    block_env.gas_limit = latest_block_ethers.gas_limit.as_u64();
    block_env.basefee = latest_block_ethers.base_fee_per_gas.map_or(0, |bf| bf.as_u64());
    block_env.difficulty = ethers_to_revm_u256(latest_block_ethers.difficulty);
    block_env.prevrandao = latest_block_ethers.mix_hash.map(|h| revm_primitives::B256::from(h.0));
    
    info!("Block environment: #{}, basefee: {} wei", block_env.number, block_env.basefee);
    
    // Initialize mempool fetcher
    info!("Initializing mempool fetcher...");
    let fetcher = MempoolFetcher::new(&args.eth_rpc_url)
        .wrap_err("Failed to initialize MempoolFetcher")?;
    
    // PHASE 1: WARM-UP PERIOD
    info!("🔥 Starting WARM-UP PHASE ({} seconds)...", args.warmup_seconds);
    
    let warmup_start = Instant::now();
    let warmup_duration = Duration::from_secs(args.warmup_seconds);
    let mut warmup_seen_transactions = HashMap::new();
    let mut warmup_processed = 0;
    
    while warmup_start.elapsed() < warmup_duration {
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                if !transactions.is_empty() {
                    debug!("Warm-up: Fetched {} transactions", transactions.len());
                    
                    for tx in transactions {
                        let tx_hash_hex = hex::encode(&tx.hash);
                        
                        if !warmup_seen_transactions.contains_key(&tx_hash_hex) {
                            warmup_seen_transactions.insert(tx_hash_hex, Instant::now());
                            warmup_processed += 1;
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Warm-up fetch error: {}", e);
                time::sleep(Duration::from_millis(500)).await;
            }
        }
        
        let elapsed_secs = warmup_start.elapsed().as_secs();
        if elapsed_secs > 0 && elapsed_secs % 10 == 0 {
            info!("Warm-up progress: {} seconds, {} transactions seen", 
                  elapsed_secs, warmup_processed);
        }
        
        time::sleep(Duration::from_millis(100)).await;
    }
    
    info!("✅ Warm-up completed! Seen {} existing transactions", warmup_processed);
    info!("🎯 Starting MEASUREMENT PHASE - Real REVM simulation");
    
    // PHASE 2: MEASUREMENT PHASE
    let measurement_start = Instant::now();
    let mut stats = RevmPerformanceStats::new();
    let max_duration = Duration::from_secs(args.max_duration_seconds);
    
    loop {
        if stats.total_fresh_transactions >= args.target_transactions {
            info!("✅ Reached target of {} fresh transactions", args.target_transactions);
            break;
        }
        
        if measurement_start.elapsed() >= max_duration {
            warn!("⏰ Reached maximum duration of {} seconds", args.max_duration_seconds);
            break;
        }
        
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                if !transactions.is_empty() {
                    debug!("Measurement: Fetched {} transactions", transactions.len());
                    
                    for tx in transactions {
                        let tx_hash_hex = hex::encode(&tx.hash);
                        
                        // Only process fresh transactions
                        if warmup_seen_transactions.contains_key(&tx_hash_hex) {
                            continue;
                        }
                        
                        let arrival_time_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        
                        info!("🆕 Fresh transaction: {}", &tx_hash_hex[..8]);
                        
                        // Process with real REVM simulation
                        let metrics = process_transaction_with_revm(
                            &tx,
                            &cfg_env,
                            &block_env,
                            &alloy_provider,
                            args.chain_id,
                            arrival_time_ms,
                        ).await;
                        
                        csv_writer.serialize(&metrics)?;
                        stats.add_metrics(&metrics);
                        
                        warmup_seen_transactions.insert(tx_hash_hex, Instant::now());
                        
                        if args.verbose {
                            info!("Fresh TX {}: E2E={:.2}ms, REVM={:.2}ms, Setup={:.2}ms, StateDiff={:.2}ms", 
                                   &metrics.tx_hash[..8],
                                   metrics.end_to_end_time_us as f64 / 1000.0,
                                   metrics.revm_simulation_time_us as f64 / 1000.0,
                                   metrics.setup_time_us as f64 / 1000.0,
                                   metrics.state_diff_time_us as f64 / 1000.0);
                        }
                        
                        if stats.total_fresh_transactions >= args.target_transactions {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error fetching transactions: {}", e);
                time::sleep(Duration::from_millis(500)).await;
            }
        }
        
        // Progress update every 50 fresh transactions
        if stats.total_fresh_transactions > 0 && stats.total_fresh_transactions % 50 == 0 {
            let elapsed = measurement_start.elapsed();
            let throughput = stats.total_fresh_transactions as f64 / elapsed.as_secs_f64();
            info!("Fresh TX Progress: {} transactions processed ({:.1} fresh tx/sec)", 
                  stats.total_fresh_transactions, throughput);
        }
        
        time::sleep(Duration::from_millis(50)).await;
    }
    
    csv_writer.flush()?;
    stats.print_summary();
    
    info!("✅ REVM performance benchmark completed!");
    info!("📊 Results saved to: {}", args.output_csv);
    
    Ok(())
}

/// Process a transaction with real REVM simulation and detailed timing
async fn process_transaction_with_revm(
    tx: &TransactionView,
    cfg_env: &RevmCfgEnv,
    block_env: &RevmBlockEnv,
    alloy_provider: &Arc<DynProvider<AlloyEthereum>>,
    chain_id: u64,
    arrival_time_ms: u64,
) -> RevmTransactionMetrics {
    let processing_start = Instant::now();
    let tx_hash_hex = hex::encode(&tx.hash);
    
    // Extract transaction metadata
    let tx_value_eth = tx.value.as_u128() as f64 / 1e18;
    let gas_price_gwei = tx.gas_price.unwrap_or_default().as_u128() as f64 / 1e9;
    let gas_limit = tx.gas_limit.map_or(21000, |gl| gl.as_u64());
    let nonce = tx.nonce.map_or(0, |n| n.as_u64());
    let is_contract_call = tx.to.is_some();
    let input_data_size = tx.input_data.as_ref().map_or(0, |data| data.len());
    
    // TIMING: Setup phase
    let setup_start = Instant::now();
    
    // Convert TransactionView to REVM TxEnv
    let revm_tx_env = match transaction_view_to_revm_tx_env(tx, chain_id) {
        Ok(env) => env,
        Err(_) => {
            // Early return for conversion failure
            let setup_time_us = setup_start.elapsed().as_micros() as u64;
            let total_processing_time_us = processing_start.elapsed().as_micros() as u64;
            let completion_time_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            
            return RevmTransactionMetrics {
                tx_hash: tx_hash_hex,
                mempool_arrival_time_ms: arrival_time_ms,
                processing_completion_time_ms: completion_time_ms,
                end_to_end_time_us: (completion_time_ms - arrival_time_ms) * 1000,
                setup_time_us,
                revm_simulation_time_us: 0,
                state_diff_time_us: 0,
                total_processing_time_us,
                simulation_successful: false,
                affected_accounts_count: 0,
                tx_value_eth,
                gas_price_gwei,
                gas_limit,
                gas_used: 0,
                nonce,
                is_contract_call,
                input_data_size,
            };
        }
    };
    
    // Setup SimCacheDB
    let fork_block_number_u256 = if block_env.number > revm_primitives::U256::ZERO {
        block_env.number - revm_primitives::U256::from(1)
    } else {
        revm_primitives::U256::ZERO 
    };
    let fork_block_u64 = fork_block_number_u256.try_into().unwrap_or(0u64);
    let fork_block_id_alloy = alloy_eips::BlockId::from(fork_block_u64);
    
    let alloy_db_for_revm = AlloyDB::new(alloy_provider.clone(), fork_block_id_alloy);
    let wrapped_db_for_revm = WrapDatabaseAsync::new(alloy_db_for_revm);
    let cache_db_for_revm: SimCacheDB = CacheDB::new(wrapped_db_for_revm.expect("Database wrapping failed"));
    
    // Get initial balances for state diff
    let mut initial_eth_balances = HashMap::new();
    let caller_address = revm_tx_env.caller;
    if let Ok(bal) = alloy_provider.get_balance(caller_address.into()).block_id(fork_block_id_alloy).await {
        initial_eth_balances.insert(caller_address, bal);
    }
    
    let setup_time_us = setup_start.elapsed().as_micros() as u64;
    
    // TIMING: REVM simulation phase
    let simulation_start = Instant::now();
    
    let (simulation_successful, gas_used, final_db_state, logs) = match simulate_transaction(
        revm_tx_env.clone(),
        block_env.clone(),
        cfg_env.clone(),
        cache_db_for_revm,
    ) {
        Ok((sim_output, final_db)) => {
            let success = matches!(sim_output.result_type, ExecutionResultType::Success(_));
            (success, sim_output.gas_used, final_db, sim_output.logs)
        }
        Err(_) => {
            // Create a fresh cache_db for the error case
            let alloy_db_fallback = AlloyDB::new(alloy_provider.clone(), fork_block_id_alloy);
            let wrapped_db_fallback = WrapDatabaseAsync::new(alloy_db_fallback);
            let cache_db_fallback: SimCacheDB = CacheDB::new(wrapped_db_fallback.expect("Database wrapping failed"));
            (false, 0, cache_db_fallback, Vec::new())
        }
    };
    
    let revm_simulation_time_us = simulation_start.elapsed().as_micros() as u64;
    
    // TIMING: State diff calculation phase
    let state_diff_start = Instant::now();
    
    let affected_accounts_count = if simulation_successful {
        match generate_calculated_account_changes(
            &final_db_state,
            &initial_eth_balances,
            &logs,
            &revm_tx_env,
            block_env,
            gas_used,
            alloy_provider.clone(),
            fork_block_id_alloy,
        ).await {
            Ok(changes) => changes.len(),
            Err(_) => 0,
        }
    } else {
        0
    };
    
    let state_diff_time_us = state_diff_start.elapsed().as_micros() as u64;
    
    let total_processing_time_us = processing_start.elapsed().as_micros() as u64;
    let completion_time_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    
    RevmTransactionMetrics {
        tx_hash: tx_hash_hex,
        mempool_arrival_time_ms: arrival_time_ms,
        processing_completion_time_ms: completion_time_ms,
        end_to_end_time_us: (completion_time_ms - arrival_time_ms) * 1000,
        setup_time_us,
        revm_simulation_time_us,
        state_diff_time_us,
        total_processing_time_us,
        simulation_successful,
        affected_accounts_count,
        tx_value_eth,
        gas_price_gwei,
        gas_limit,
        gas_used,
        nonce,
        is_contract_call,
        input_data_size,
    }
} 