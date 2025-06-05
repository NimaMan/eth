/*
 * Scam Detection Service Binary
 * 
 * ALGORITHMIC DESCRIPTION:
 * This binary provides real-time scam detection for Ethereum mempool transactions:
 * 
 * 1. POOL STATE MONITORING:
 *    - ZeroMQ subscriber receives pool updates from Python service
 *    - Maintains real-time cache of pool reserves and addresses
 *    - Tracks ETH reserves for thousands of DeFi pools
 * 
 * 2. MEMPOOL TRANSACTION MONITORING:
 *    - HTTP RPC polling for new mempool transactions (every 50ms)
 *    - Filters and processes only new transactions to avoid duplicates
 *    - Prioritizes transactions that interact with known pools
 * 
 * 3. REVM TRANSACTION SIMULATION:
 *    - Uses validated REVM TransactionSimulator for accurate state prediction
 *    - Simulates each transaction against current blockchain state
 *    - Calculates precise ETH balance changes for all affected accounts
 *    - Generates detailed account state changes using revm_tx_simulator_lib
 * 
 * 4. SCAM DETECTION ANALYSIS:
 *    - Cross-references simulated state changes with pool cache
 *    - Detects pools being drained below ETH threshold (default: 0.15 ETH)
 *    - Identifies large percentage withdrawals (default: >50%)
 *    - Flags suspicious transactions before they can execute
 * 
 * 5. ALERT PERSISTENCE:
 *    - Logs detected scams to PostgreSQL database
 *    - Provides detailed transaction and pool information
 *    - Continues service operation even during database errors
 * 
 * This service serves the main objective of detecting and preventing DeFi pool scams
 * by simulating transactions before they execute and identifying suspicious patterns.
 */

use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time;
use tracing::{info, error, Level, debug, warn};
use std::collections::HashMap;
use ethers::types::H256;
use revm_primitives::alloy_primitives::Address;
use chrono;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

// REVM imports for new simulation approach
use revm_context::BlockEnv as RevmBlockEnv;
use revm_primitives::hardfork::SpecId;
use ethers::providers::{Http as EthersHttp, Middleware, Provider as EthersProvider};
use ethers::types::{BlockId as EthersBlockId, BlockNumber as EthersBlockNumber};
use revm_tx_simulator_lib::conversions::{ethers_to_revm_u256, ethers_to_revm_address};

use mempool_processor::mempool_processor::fetcher::{MempoolFetcher, FetchMode};
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::mempool_processor::TransactionSource;
use mempool_processor::mempool_processor::db_logger::DbLogger;
use mempool_processor::tx_simulator::TransactionSimulator;
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::scam_detection::{
    ScamDetectionService, 
    SimulationResult, 
    PoolEffect,
    ScamDetectionConfig
};

#[derive(Parser, Debug)]
struct Args {
    /// JSON-RPC URL for Ethereum node
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// WebSocket URL for Ethereum node (for real-time mempool subscription)
    #[arg(long, env = "ETH_WS_URL", default_value = "ws://localhost:8546")]
    eth_ws_url: String,
    
    /// ZeroMQ socket address for pool updates
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    /// Database hostname
    #[arg(long, env = "DB_HOST", default_value = "localhost")]
    db_host: String,
    
    /// Database port
    #[arg(long, env = "DB_PORT", default_value = "5432")]
    db_port: u16,
    
    /// Database name
    #[arg(long, env = "DB_NAME", default_value = "eth_db")]
    db_name: String,
    
    /// Database user
    #[arg(long, env = "DB_USER", default_value = "postgres")]
    db_user: String,
    
    /// Database password
    #[arg(long, env = "DB_PASSWORD", default_value = "postgres")]
    db_password: String,
    
    /// Print statistics every N seconds
    #[arg(long, default_value = "60")]
    stats_interval_seconds: u64,
    
    /// ETH reserve threshold for scam detection (in ETH)
    #[arg(long, default_value = "0.15")]
    eth_threshold: f64,
    
    /// Percentage threshold for large withdrawals (0.0-1.0)
    #[arg(long, default_value = "0.5")]
    percentage_threshold: f64,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Enable DevP2P peer-to-peer transaction fetching (faster than RPC) - DEFAULT
    #[arg(long, default_value_t = true)]
    enable_devp2p: bool,
    
    /// Use slow RPC polling instead of DevP2P (not recommended)
    #[arg(long)]
    use_rpc_polling: bool,
    
    /// Use WebSocket streaming to get only NEW transactions (fastest)
    #[arg(long)]
    use_streaming: bool,
    
    
    /// Log file path
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/scam_detection_service.log")]
    log_file: String,
    
    /// Process all transactions (bypass pool filtering) for load testing and queue analysis
    #[arg(long, default_value_t = true)]
    process_all_transactions: bool,
    
    /// Only process pool transactions (legacy mode for scam detection only)
    #[arg(long)]
    pool_transactions_only: bool,
}

/// Address normalization utility
/// Instead of trying to match Python's checksumming exactly, we use lowercase for consistency
fn normalize_address(address: Address) -> String {
    format!("0x{:040x}", address).to_lowercase()
}

/// Enhanced performance metrics for comprehensive transaction timing analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionTiming {
    /// Unique transaction hash
    pub tx_hash: String,
    
    // === ABSOLUTE TIMESTAMPS (Unix milliseconds) ===
    /// When transaction was first detected in mempool
    pub mempool_arrival_timestamp_ms: u64,
    /// When transaction entered our processing queue
    pub queue_entry_timestamp_ms: u64,
    /// When we started processing this transaction
    pub processing_start_timestamp_ms: u64,
    /// When pool address lookup started
    pub pool_check_start_timestamp_ms: u64,
    /// When pool address lookup completed
    pub pool_check_end_timestamp_ms: u64,
    /// When REVM simulation started
    pub revm_simulation_start_timestamp_ms: u64,
    /// When REVM simulation completed
    pub revm_simulation_end_timestamp_ms: u64,
    /// When state analysis started
    pub state_analysis_start_timestamp_ms: u64,
    /// When state analysis completed
    pub state_analysis_end_timestamp_ms: u64,
    /// When scam detection started
    pub scam_detection_start_timestamp_ms: u64,
    /// When scam detection completed
    pub scam_detection_end_timestamp_ms: u64,
    /// When processing completely finished
    pub processing_end_timestamp_ms: u64,
    
    // === DURATION MEASUREMENTS (microseconds for precision) ===
    /// Time spent waiting in our internal queue (queue entry to processing start)
    pub internal_queue_time_us: u64,
    /// Time spent in mempool before we detected it (estimated)
    pub mempool_residence_time_us: u64,
    /// Time spent checking pool address
    pub pool_check_time_us: u64,
    /// Time spent in REVM simulation
    pub revm_simulation_time_us: u64,
    /// Time spent in state diff analysis
    pub state_analysis_time_us: u64,
    /// Time spent in scam detection
    pub scam_detection_time_us: u64,
    /// Total processing time (processing start to end)
    pub total_processing_time_us: u64,
    /// Total end-to-end time (mempool arrival to processing completion)
    pub end_to_end_time_us: u64,
    
    // === TRANSACTION METADATA ===
    /// Whether this transaction was targeted at a pool
    pub is_pool_transaction: bool,
    /// Pool address if this is a pool transaction
    pub pool_address: Option<String>,
    /// Whether scam was detected
    pub scam_detected: bool,
    /// Transaction value in wei
    pub tx_value_wei: String,
    /// Gas price in wei
    pub gas_price_wei: String,
    /// Gas limit
    pub gas_limit: u64,
    /// Whether simulation was successful
    pub simulation_successful: bool,
    /// Number of accounts affected by simulation
    pub affected_accounts_count: usize,
    
    // === PERFORMANCE INDICATORS ===
    /// Whether this transaction exceeded SLA (>50ms end-to-end)
    pub sla_violation: bool,
    /// Performance category for frontend visualization
    pub performance_category: String,
}

impl TransactionTiming {
    /// Create a new timing tracker for a transaction
    pub fn new(tx_hash: String) -> Self {
        let now_ms = current_timestamp_ms();
        Self {
            tx_hash,
            mempool_arrival_timestamp_ms: now_ms,
            queue_entry_timestamp_ms: now_ms,
            processing_start_timestamp_ms: 0,
            pool_check_start_timestamp_ms: 0,
            pool_check_end_timestamp_ms: 0,
            revm_simulation_start_timestamp_ms: 0,
            revm_simulation_end_timestamp_ms: 0,
            state_analysis_start_timestamp_ms: 0,
            state_analysis_end_timestamp_ms: 0,
            scam_detection_start_timestamp_ms: 0,
            scam_detection_end_timestamp_ms: 0,
            processing_end_timestamp_ms: 0,
            internal_queue_time_us: 0,
            mempool_residence_time_us: 0,
            pool_check_time_us: 0,
            revm_simulation_time_us: 0,
            state_analysis_time_us: 0,
            scam_detection_time_us: 0,
            total_processing_time_us: 0,
            end_to_end_time_us: 0,
            is_pool_transaction: false,
            pool_address: None,
            scam_detected: false,
            tx_value_wei: "0".to_string(),
            gas_price_wei: "0".to_string(),
            gas_limit: 0,
            simulation_successful: false,
            affected_accounts_count: 0,
            sla_violation: false,
            performance_category: "normal".to_string(),
        }
    }
    
    /// Mark processing as started
    pub fn start_processing(&mut self) {
        self.processing_start_timestamp_ms = current_timestamp_ms();
        // Calculate internal queue time
        if self.processing_start_timestamp_ms > self.queue_entry_timestamp_ms {
            self.internal_queue_time_us = (self.processing_start_timestamp_ms - self.queue_entry_timestamp_ms) * 1000;
        }
    }
    
    /// Mark pool check phase
    pub fn start_pool_check(&mut self) {
        self.pool_check_start_timestamp_ms = current_timestamp_ms();
    }
    
    pub fn end_pool_check(&mut self) {
        self.pool_check_end_timestamp_ms = current_timestamp_ms();
        if self.pool_check_end_timestamp_ms > self.pool_check_start_timestamp_ms {
            self.pool_check_time_us = (self.pool_check_end_timestamp_ms - self.pool_check_start_timestamp_ms) * 1000;
        }
    }
    
    /// Mark REVM simulation phase
    pub fn start_revm_simulation(&mut self) {
        self.revm_simulation_start_timestamp_ms = current_timestamp_ms();
    }
    
    pub fn end_revm_simulation(&mut self) {
        self.revm_simulation_end_timestamp_ms = current_timestamp_ms();
        if self.revm_simulation_end_timestamp_ms > self.revm_simulation_start_timestamp_ms {
            self.revm_simulation_time_us = (self.revm_simulation_end_timestamp_ms - self.revm_simulation_start_timestamp_ms) * 1000;
        }
    }
    
    /// Mark state analysis phase
    pub fn start_state_analysis(&mut self) {
        self.state_analysis_start_timestamp_ms = current_timestamp_ms();
    }
    
    pub fn end_state_analysis(&mut self) {
        self.state_analysis_end_timestamp_ms = current_timestamp_ms();
        if self.state_analysis_end_timestamp_ms > self.state_analysis_start_timestamp_ms {
            self.state_analysis_time_us = (self.state_analysis_end_timestamp_ms - self.state_analysis_start_timestamp_ms) * 1000;
        }
    }
    
    /// Mark scam detection phase
    pub fn start_scam_detection(&mut self) {
        self.scam_detection_start_timestamp_ms = current_timestamp_ms();
    }
    
    pub fn end_scam_detection(&mut self) {
        self.scam_detection_end_timestamp_ms = current_timestamp_ms();
        if self.scam_detection_end_timestamp_ms > self.scam_detection_start_timestamp_ms {
            self.scam_detection_time_us = (self.scam_detection_end_timestamp_ms - self.scam_detection_start_timestamp_ms) * 1000;
        }
    }
    
    /// Finalize timing and calculate all derived metrics
    pub fn finalize(&mut self, is_warmup_phase: bool) {
        self.processing_end_timestamp_ms = current_timestamp_ms();
        
        // Calculate total processing time
        if self.processing_end_timestamp_ms > self.processing_start_timestamp_ms {
            self.total_processing_time_us = (self.processing_end_timestamp_ms - self.processing_start_timestamp_ms) * 1000;
        }
        
        // Calculate end-to-end time
        if self.processing_end_timestamp_ms > self.mempool_arrival_timestamp_ms {
            self.end_to_end_time_us = (self.processing_end_timestamp_ms - self.mempool_arrival_timestamp_ms) * 1000;
        }
        
        // Estimate mempool residence time (arrival to queue entry)
        if self.queue_entry_timestamp_ms > self.mempool_arrival_timestamp_ms {
            self.mempool_residence_time_us = (self.queue_entry_timestamp_ms - self.mempool_arrival_timestamp_ms) * 1000;
        }
        
        // Use warmup-aware SLA thresholds
        let sla_threshold_us = if is_warmup_phase {
            8_000  // 8ms during warmup for ultra-fast detection
        } else {
            50_000  // 50ms post-warmup for production stability
        };
        
        self.sla_violation = self.end_to_end_time_us > sla_threshold_us;
        
        // Categorize performance for frontend (adjusted for warmup)
        self.performance_category = if is_warmup_phase {
            // Stricter categories during warmup
            if self.end_to_end_time_us <= 5_000 {
                "excellent".to_string()
            } else if self.end_to_end_time_us <= 8_000 {
                "good".to_string()
            } else if self.end_to_end_time_us <= 15_000 {
                "acceptable".to_string()
            } else {
                "poor".to_string()
            }
        } else {
            // Standard categories post-warmup (50ms SLA target)
            if self.end_to_end_time_us <= 20_000 {
                "excellent".to_string()
            } else if self.end_to_end_time_us <= 50_000 {
                "good".to_string()
            } else if self.end_to_end_time_us <= 100_000 {
                "acceptable".to_string()
            } else {
                "poor".to_string()
            }
        };
    }
    
    /// Get a summary string for logging
    pub fn summary(&self) -> String {
        format!("Mempool: {:.1}ms | Queue: {:.1}ms | Pool: {:.1}ms | REVM: {:.1}ms | Analysis: {:.1}ms | Detection: {:.1}ms | Total: {:.1}ms",
               self.mempool_residence_time_us as f64 / 1000.0,
               self.internal_queue_time_us as f64 / 1000.0,
               self.pool_check_time_us as f64 / 1000.0,
               self.revm_simulation_time_us as f64 / 1000.0,
               self.state_analysis_time_us as f64 / 1000.0,
               self.scam_detection_time_us as f64 / 1000.0,
               self.end_to_end_time_us as f64 / 1000.0)
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Enhanced performance metrics with real-time tracking and frontend-ready data
struct PerformanceMetrics {
    total_processed: u64,
    warmup_period_end: Option<Instant>,
    recent_latencies: VecDeque<Duration>,
    max_recent_samples: usize,
    sla_violations: u64,
    start_time: Instant,
    timing_log_file: std::fs::File,
    realtime_log_file: std::fs::File,
    /// Track actual transaction arrival times
    transaction_arrival_tracker: HashMap<String, Instant>,
    /// Performance statistics for frontend
    current_stats: Arc<AtomicPerformanceStats>,
}

/// Atomic performance statistics for real-time frontend access
#[derive(Debug, Default)]
struct AtomicPerformanceStats {
    /// Total transactions processed
    total_processed: AtomicU64,
    /// Average queue time in microseconds
    avg_queue_time_us: AtomicU64,
    /// Average processing time in microseconds
    avg_processing_time_us: AtomicU64,
    /// Average end-to-end time in microseconds
    avg_end_to_end_time_us: AtomicU64,
    /// Number of pool transactions
    pool_transactions: AtomicU64,
    /// Number of scams detected
    scams_detected: AtomicU64,
    /// SLA violations (>50ms end-to-end)
    sla_violations: AtomicU64,
    /// Current throughput (transactions per second)
    current_tps: AtomicU64,
    /// Average REVM simulation time in microseconds
    avg_revm_simulation_time_us: AtomicU64,
    /// Average pool check time in microseconds
    avg_pool_check_time_us: AtomicU64,
    /// Average state analysis time in microseconds
    avg_state_analysis_time_us: AtomicU64,
    /// Average scam detection time in microseconds
    avg_scam_detection_time_us: AtomicU64,
    /// Total mined transactions tracked
    total_mined_transactions: AtomicU64,
    /// Average mempool to mining duration in seconds (x100 for precision)
    avg_mempool_to_mining_duration_cs: AtomicU64, // centiseconds for precision
}

impl PerformanceMetrics {
    fn new() -> eyre::Result<Self> {
        // Create timing analysis log file
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let timing_log_path = format!("/home/nima/code/crypto/logs/mempool/transaction_timing_analysis_{}.csv", timestamp);
        let realtime_log_path = format!("/home/nima/code/crypto/logs/mempool/realtime_metrics_{}.json", timestamp);
        
        // Create the log directory if it doesn't exist
        if let Some(log_dir) = std::path::Path::new(&timing_log_path).parent() {
            std::fs::create_dir_all(log_dir)?;
        }
        
        let mut timing_file = std::fs::File::create(&timing_log_path)?;
        let realtime_file = std::fs::File::create(&realtime_log_path)?;
        
        // Write enhanced CSV header with comprehensive timing breakdown
        writeln!(timing_file, "tx_hash,mempool_arrival_timestamp_ms,queue_entry_timestamp_ms,processing_start_timestamp_ms,pool_check_start_timestamp_ms,pool_check_end_timestamp_ms,revm_simulation_start_timestamp_ms,revm_simulation_end_timestamp_ms,state_analysis_start_timestamp_ms,state_analysis_end_timestamp_ms,scam_detection_start_timestamp_ms,scam_detection_end_timestamp_ms,processing_end_timestamp_ms,internal_queue_time_us,mempool_residence_time_us,pool_check_time_us,revm_simulation_time_us,state_analysis_time_us,scam_detection_time_us,total_processing_time_us,end_to_end_time_us,is_pool_transaction,pool_address,scam_detected,tx_value_wei,gas_price_wei,gas_limit,simulation_successful,affected_accounts_count,sla_violation,performance_category")?;
        
        info!("📊 Enhanced transaction timing analysis logging to: {}", timing_log_path);
        info!("📊 Real-time metrics logging to: {}", realtime_log_path);
        
        Ok(Self {
            total_processed: 0,
            warmup_period_end: None,
            recent_latencies: VecDeque::new(),
            max_recent_samples: 1000,
            sla_violations: 0,
            start_time: Instant::now(),
            timing_log_file: timing_file,
            realtime_log_file: realtime_file,
            transaction_arrival_tracker: HashMap::new(),
            current_stats: Arc::new(AtomicPerformanceStats::default()),
        })
    }
    
    /// Track when a transaction first arrives in mempool
    fn track_transaction_arrival(&mut self, tx_hash: &str) {
        let arrival_time = Instant::now();
        self.transaction_arrival_tracker.insert(tx_hash.to_string(), arrival_time);
        
        // Clean up old entries (older than 60 seconds)
        let cutoff_time = arrival_time - Duration::from_secs(60);
        self.transaction_arrival_tracker.retain(|_, &mut time| time > cutoff_time);
    }
    
    /// Get the actual arrival time for a transaction
    fn get_transaction_arrival_time(&self, tx_hash: &str) -> Option<Instant> {
        self.transaction_arrival_tracker.get(tx_hash).copied()
    }
    
    fn record_processing_time(&mut self, duration: Duration) {
        self.total_processed += 1;
        
        // Set warmup end after processing 75,000 transactions or 300 seconds (5 minutes)
        if self.warmup_period_end.is_none() && 
           (self.total_processed >= 100_000 || self.start_time.elapsed() > Duration::from_secs(300)) {
            self.warmup_period_end = Some(Instant::now());
            info!("🏁 Warmup period completed after {} transactions in {:.1}s. Now monitoring 50ms SLA (end-to-end from mempool arrival).", 
                  self.total_processed, self.start_time.elapsed().as_secs_f64());
        }
        
        // Track recent latencies for SLA monitoring
        self.recent_latencies.push_back(duration);
        if self.recent_latencies.len() > self.max_recent_samples {
            self.recent_latencies.pop_front();
        }
        
        // Check SLA violation only after warmup
        if self.warmup_period_end.is_some() && duration > Duration::from_millis(50) {
            self.sla_violations += 1;
            self.current_stats.sla_violations.store(self.sla_violations, Ordering::Relaxed);
            info!("⚠️  SLA VIOLATION: Transaction processed in {:.1}ms (target: 50ms)", 
                  duration.as_millis());
        }
    }
    
    /// Log comprehensive transaction timing data
    fn log_transaction_timing(&mut self, timing: &TransactionTiming) {
        // Write to CSV for analysis
        if let Err(e) = writeln!(
            self.timing_log_file,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            timing.tx_hash,
            timing.mempool_arrival_timestamp_ms,
            timing.queue_entry_timestamp_ms,
            timing.processing_start_timestamp_ms,
            timing.pool_check_start_timestamp_ms,
            timing.pool_check_end_timestamp_ms,
            timing.revm_simulation_start_timestamp_ms,
            timing.revm_simulation_end_timestamp_ms,
            timing.state_analysis_start_timestamp_ms,
            timing.state_analysis_end_timestamp_ms,
            timing.scam_detection_start_timestamp_ms,
            timing.scam_detection_end_timestamp_ms,
            timing.processing_end_timestamp_ms,
            timing.internal_queue_time_us,
            timing.mempool_residence_time_us,
            timing.pool_check_time_us,
            timing.revm_simulation_time_us,
            timing.state_analysis_time_us,
            timing.scam_detection_time_us,
            timing.total_processing_time_us,
            timing.end_to_end_time_us,
            timing.is_pool_transaction,
            timing.pool_address.as_deref().unwrap_or(""),
            timing.scam_detected,
            timing.tx_value_wei,
            timing.gas_price_wei,
            timing.gas_limit,
            timing.simulation_successful,
            timing.affected_accounts_count,
            timing.sla_violation,
            timing.performance_category
        ) {
            debug!("Failed to write timing log: {}", e);
        }
        
        // Update real-time statistics
        self.update_realtime_stats(timing);
        
        // Flush every 50 transactions to ensure data is written promptly
        if self.total_processed % 50 == 0 {
            let _ = self.timing_log_file.flush();
            self.write_realtime_metrics();
        }
    }
    
    /// Update real-time performance statistics
    fn update_realtime_stats(&self, timing: &TransactionTiming) {
        self.current_stats.total_processed.store(self.total_processed, Ordering::Relaxed);
        
        // Update running averages (simplified exponential moving average)
        let alpha = 0.1; // Smoothing factor
        
        let current_queue = self.current_stats.avg_queue_time_us.load(Ordering::Relaxed);
        let new_queue = (current_queue as f64 * (1.0 - alpha) + timing.internal_queue_time_us as f64 * alpha) as u64;
        self.current_stats.avg_queue_time_us.store(new_queue, Ordering::Relaxed);
        
        let current_processing = self.current_stats.avg_processing_time_us.load(Ordering::Relaxed);
        let new_processing = (current_processing as f64 * (1.0 - alpha) + timing.total_processing_time_us as f64 * alpha) as u64;
        self.current_stats.avg_processing_time_us.store(new_processing, Ordering::Relaxed);
        
        let current_e2e = self.current_stats.avg_end_to_end_time_us.load(Ordering::Relaxed);
        let new_e2e = (current_e2e as f64 * (1.0 - alpha) + timing.end_to_end_time_us as f64 * alpha) as u64;
        self.current_stats.avg_end_to_end_time_us.store(new_e2e, Ordering::Relaxed);
        
        // === UPDATE DETAILED TIMING BREAKDOWNS ===
        
        // REVM simulation time
        let current_revm = self.current_stats.avg_revm_simulation_time_us.load(Ordering::Relaxed);
        let new_revm = (current_revm as f64 * (1.0 - alpha) + timing.revm_simulation_time_us as f64 * alpha) as u64;
        self.current_stats.avg_revm_simulation_time_us.store(new_revm, Ordering::Relaxed);
        
        // Pool check time
        let current_pool_check = self.current_stats.avg_pool_check_time_us.load(Ordering::Relaxed);
        let new_pool_check = (current_pool_check as f64 * (1.0 - alpha) + timing.pool_check_time_us as f64 * alpha) as u64;
        self.current_stats.avg_pool_check_time_us.store(new_pool_check, Ordering::Relaxed);
        
        // State analysis time
        let current_state = self.current_stats.avg_state_analysis_time_us.load(Ordering::Relaxed);
        let new_state = (current_state as f64 * (1.0 - alpha) + timing.state_analysis_time_us as f64 * alpha) as u64;
        self.current_stats.avg_state_analysis_time_us.store(new_state, Ordering::Relaxed);
        
        // Scam detection time
        let current_scam = self.current_stats.avg_scam_detection_time_us.load(Ordering::Relaxed);
        let new_scam = (current_scam as f64 * (1.0 - alpha) + timing.scam_detection_time_us as f64 * alpha) as u64;
        self.current_stats.avg_scam_detection_time_us.store(new_scam, Ordering::Relaxed);
        
        // Mining statistics
        if timing.is_pool_transaction {
            self.current_stats.pool_transactions.fetch_add(1, Ordering::Relaxed);
        }
        
        if timing.scam_detected {
            self.current_stats.scams_detected.fetch_add(1, Ordering::Relaxed);
        }
        
        // Calculate current TPS based on recent activity
        let elapsed_seconds = self.start_time.elapsed().as_secs().max(1);
        let tps = self.total_processed / elapsed_seconds;
        self.current_stats.current_tps.store(tps, Ordering::Relaxed);
    }
    
    /// Write real-time metrics to JSON file for frontend consumption
    fn write_realtime_metrics(&mut self) {
        let avg_e2e_ms = self.current_stats.avg_end_to_end_time_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let performance_category = if avg_e2e_ms <= 50.0 {
            "excellent"
        } else if avg_e2e_ms <= 100.0 {
            "good"
        } else if avg_e2e_ms <= 200.0 {
            "acceptable"
            } else {
            "poor"
        };
        
        let metrics = serde_json::json!({
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            "total_processed": self.current_stats.total_processed.load(Ordering::Relaxed),
            "avg_queue_time_us": self.current_stats.avg_queue_time_us.load(Ordering::Relaxed),
            "avg_processing_time_us": self.current_stats.avg_processing_time_us.load(Ordering::Relaxed),
            "avg_end_to_end_time_us": self.current_stats.avg_end_to_end_time_us.load(Ordering::Relaxed),
            "pool_transactions": self.current_stats.pool_transactions.load(Ordering::Relaxed),
            "scams_detected": self.current_stats.scams_detected.load(Ordering::Relaxed),
            "sla_violations": self.current_stats.sla_violations.load(Ordering::Relaxed),
            "current_tps": self.current_stats.current_tps.load(Ordering::Relaxed),
            "avg_queue_time_ms": self.current_stats.avg_queue_time_us.load(Ordering::Relaxed) as f64 / 1000.0,
            "avg_processing_time_ms": self.current_stats.avg_processing_time_us.load(Ordering::Relaxed) as f64 / 1000.0,
            "avg_end_to_end_time_ms": self.current_stats.avg_end_to_end_time_us.load(Ordering::Relaxed) as f64 / 1000.0,
            "sla_compliance_percentage": if self.total_processed > 0 {
                ((self.total_processed - self.sla_violations) as f64 / self.total_processed as f64) * 100.0
            } else {
                100.0
            },
            
            // === ENHANCED METRICS FOR DASHBOARD ===
            
            // Detailed timing breakdowns (microseconds)
            "avg_revm_simulation_time_us": self.current_stats.avg_revm_simulation_time_us.load(Ordering::Relaxed),
            "avg_pool_check_time_us": self.current_stats.avg_pool_check_time_us.load(Ordering::Relaxed),
            "avg_state_analysis_time_us": self.current_stats.avg_state_analysis_time_us.load(Ordering::Relaxed),
            "avg_scam_detection_time_us": self.current_stats.avg_scam_detection_time_us.load(Ordering::Relaxed),
            
            // Mining statistics
            "total_mined_transactions": self.current_stats.total_mined_transactions.load(Ordering::Relaxed),
            "avg_mempool_to_mining_duration_s": self.current_stats.avg_mempool_to_mining_duration_cs.load(Ordering::Relaxed) as f64 / 100.0,
            
            // Performance analytics
            "service_uptime_seconds": self.start_time.elapsed().as_secs(),
            
            // Additional derived metrics
            "pool_transaction_percentage": if self.total_processed > 0 {
                (self.current_stats.pool_transactions.load(Ordering::Relaxed) as f64 / self.total_processed as f64) * 100.0
            } else {
                0.0
            },
            "mining_coverage_percentage": if self.total_processed > 0 {
                (self.current_stats.total_mined_transactions.load(Ordering::Relaxed) as f64 / self.total_processed as f64) * 100.0
            } else {
                0.0
            },
            "dominant_performance_category": performance_category
        });
        
        if let Err(e) = writeln!(self.realtime_log_file, "{}", metrics) {
            debug!("Failed to write realtime metrics: {}", e);
        }
    }
    
    fn should_report_metrics(&self) -> bool {
        self.total_processed % 500 == 0 && self.total_processed > 0
    }
    
    fn report_metrics(&self) {
        if self.recent_latencies.is_empty() { return; }
        
        let avg_latency = self.recent_latencies.iter()
            .map(|d| d.as_millis())
            .sum::<u128>() / self.recent_latencies.len() as u128;
            
        let max_latency = self.recent_latencies.iter()
            .map(|d| d.as_millis())
            .max().unwrap_or(0);
            
        let p95_latency = {
            let mut sorted: Vec<u128> = self.recent_latencies.iter()
                .map(|d| d.as_millis()).collect();
            sorted.sort_unstable();
            let idx = (sorted.len() as f64 * 0.95) as usize;
            sorted.get(idx).copied().unwrap_or(0)
        };
        
        let sla_compliance = if self.warmup_period_end.is_some() {
            let post_warmup_count = self.total_processed.saturating_sub(100_000);
            if post_warmup_count > 0 {
                ((post_warmup_count - self.sla_violations) as f64 / post_warmup_count as f64) * 100.0
            } else {
                100.0
            }
        } else {
            100.0
        };
        
        let warmup_progress = if self.warmup_period_end.is_none() {
            format!(" ({:.1}% to 100K)", (self.total_processed as f64 / 100_000.0) * 100.0)
        } else {
            String::new()
        };
        
        // Enhanced metrics display
        let current_tps = self.current_stats.current_tps.load(Ordering::Relaxed);
        let avg_queue_ms = self.current_stats.avg_queue_time_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let avg_processing_ms = self.current_stats.avg_processing_time_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let pool_count = self.current_stats.pool_transactions.load(Ordering::Relaxed);
        let scam_count = self.current_stats.scams_detected.load(Ordering::Relaxed);
        
        info!("📊 ENHANCED METRICS [{}{}{}] - Processed: {} | TPS: {} | Queue: {:.1}ms | Processing: {:.1}ms | P95: {}ms | SLA: {:.1}% | Pools: {} | Scams: {}", 
              if self.warmup_period_end.is_some() { "POST-WARMUP" } else { "WARMUP" },
              warmup_progress,
              if self.warmup_period_end.is_some() && sla_compliance < 95.0 { " 🚨" } else { "" },
              self.total_processed, current_tps, avg_queue_ms, avg_processing_ms, p95_latency, sla_compliance, pool_count, scam_count);
    }
    
    /// Get current performance statistics for external access
    pub fn get_current_stats(&self) -> Arc<AtomicPerformanceStats> {
        self.current_stats.clone()
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Configure logging - use INFO level to see scam detection logs
    let log_level = if args.verbose { Level::DEBUG } else { Level::WARN };
    
    // Create log directory if it doesn't exist
    if let Some(log_dir) = std::path::Path::new(&args.log_file).parent() {
        std::fs::create_dir_all(log_dir)?;
        println!("Created log directory: {:?}", log_dir);
    }
    
    // Create timestamped log filename to avoid conflicts
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M").to_string();
    let log_path = std::path::Path::new(&args.log_file);
    let log_dir = log_path.parent().unwrap_or(std::path::Path::new("logs"));
    let log_stem = log_path.file_stem().unwrap_or(std::ffi::OsStr::new("scam_detection_service"));
    let log_ext = log_path.extension().unwrap_or(std::ffi::OsStr::new("log"));
    let timestamped_log_file = format!("{}_{}.{}", 
                                      log_stem.to_string_lossy(), 
                                      timestamp, 
                                      log_ext.to_string_lossy());
    let full_log_path = log_dir.join(timestamped_log_file);
    
    println!("Logging to: {:?}", full_log_path);
    
    // Configure logging to file
    let file_appender = tracing_appender::rolling::never(
        log_dir,
        full_log_path.file_name().unwrap()
    );
    
    // Use a custom filter that shows important logs but suppresses noisy external crates
    use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};
    
    let filter = if args.verbose {
        // Verbose mode: show everything
        EnvFilter::from_default_env()
            .add_directive("mempool_processor=debug".parse()?)
            .add_directive("scam_detection_service=debug".parse()?)
    } else {
        // Production mode: only show important logs
        EnvFilter::from_default_env()
            .add_directive("mempool_processor=info".parse()?)
            .add_directive("scam_detection_service=info".parse()?)
            .add_directive("hyper=warn".parse()?)
            .add_directive("tokio_postgres=warn".parse()?)
            .add_directive("h2=warn".parse()?)
            .add_directive("tower=warn".parse()?)
            .add_directive("reqwest=warn".parse()?)
    };
    
    // Custom timer that uses local time to match Python logs
    struct LocalTimer;
    
    impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
        fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
            let now = chrono::Local::now();
            write!(w, "{}", now.format("%Y-%m-%d %H:%M:%S"))
        }
    }
    
    tracing_subscriber::registry()
        .with(fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false)  // Disable ANSI color codes in log files
            .with_target(false)  // Don't show the target module in logs for cleaner output
            .with_timer(LocalTimer)  // Use local time to match Python logs
        )
        .with(filter)
        .init();
    
    // Log startup message
    info!("Scam Detection Service Starting");
    info!("ETH RPC URL: {}", args.eth_rpc_url);
    info!("Pool ZMQ Address: {}", args.pool_zmq_address);
    info!("Database: {}:{}/{}", args.db_host, args.db_port, args.db_name);
    
    // Create the ZeroMQ subscriber for pool updates with the proper ETH threshold and ZMQ address
    info!("Initializing pool subscriber with ETH threshold: {} and ZMQ address: {}", 
         args.eth_threshold, args.pool_zmq_address);
    let mut pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    
    // Get the pool cache from the subscriber before moving it
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Initialize fetcher for mempool transactions optimized for real-time scam detection
    info!("Initializing mempool transaction fetcher...");
    
    let fetch_mode = if args.use_streaming {
        info!("⚡ STREAMING MODE enabled - getting only NEW transactions!");
        info!("🚀 This is the FASTEST mode - no polling, no full mempool fetches");
        info!("📡 Will use transaction filters to get only new arrivals");
        FetchMode::Streaming
    } else if args.use_rpc_polling {
        warn!("📡 RPC POLLING MODE (not recommended) - 12-92ms transaction arrival");
        warn!("   Consider using DevP2P or Streaming for real-time performance");
        FetchMode::RpcBatch
    } else {
        info!("🚀 DevP2P mode enabled (DEFAULT) - targeting <50ms end-to-end processing");
        info!("🔗 Will attempt to connect to Reth IPC at /tmp/reth.ipc");
        info!("💡 Ensure Reth is running with IPC enabled");
        FetchMode::DevP2p
    };
    
    // Use WebSocket for ultra-low latency transaction detection
    info!("🚀 ULTRA-LOW LATENCY MODE: Initializing transaction fetcher");
    info!("   🎯 Target: <50ms end-to-end (from mempool arrival to processing completion)");
    info!("   🔗 RPC URL: {}", args.eth_rpc_url);
    if fetch_mode == FetchMode::DevP2p {
        info!("   🔌 IPC Path: /tmp/reth.ipc (DevP2P mode)");
    } else {
        info!("   📡 WebSocket URL: {}", args.eth_ws_url);
    }
    
    let mut fetcher = MempoolFetcher::with_websocket_support(
        &args.eth_rpc_url,
        None, // TEMPORARILY DISABLE WebSocket until subscription issues are resolved
        50000, // cache size - increased for mempool-wide processing
        true, // use batch requests
        1000, // LARGE batch size - start big, get entire mempool, adapt down if errors
        2000, // NORMAL timeout - increased for large batches
        fetch_mode
    )?;
    
    // Initialize WebSocket connection
    match fetcher.connect_websocket().await {
        Ok(()) => {
            info!("✅ WebSocket connection established - <8ms transaction detection enabled");
        }
        Err(e) => {
            warn!("⚠️ WebSocket connection failed: {} - falling back to optimized RPC", e);
            info!("📡 Using ultra-fast RPC polling (5ms intervals) as fallback");
        }
    }
    
    // Initialize REVM transaction simulator instead of old StateDiffTracker
    info!("Initializing REVM transaction simulator...");
    let tx_simulator = Arc::new(TransactionSimulator::new(
        &args.eth_rpc_url,
        1, // chain_id (mainnet)
        SpecId::CANCUN
    ).await?);
    
    // Get current block environment for simulation
    info!("Fetching latest block for simulation context...");
    let provider = EthersProvider::<EthersHttp>::try_from(args.eth_rpc_url.clone())?;
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
    block_env.difficulty = ethers_to_revm_u256(latest_block.difficulty);
    block_env.prevrandao = latest_block.mix_hash.map(|h| revm_primitives::B256::from(h.0));
    
    info!("Block environment: #{}, basefee: {} wei", block_env.number, block_env.basefee);
    
    // Initialize database logger separately
    info!("Connecting to database...");
    let db_logger = DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name
    ).await?;
    
    let db_logger = Arc::new(db_logger);
    
    // Initialize scam detection service
    info!("Initializing scam detection service...");
    let scam_config = ScamDetectionConfig {
        eth_threshold: args.eth_threshold,
        percentage_threshold: args.percentage_threshold,
    };
    
    info!("🎯 Scam detection thresholds: ETH < {:.3}, Percentage > {:.1}%", 
          args.eth_threshold, args.percentage_threshold * 100.0);
    
    let service = ScamDetectionService::new(
        pool_cache.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    // Start the service
    info!("Scam Detection Service has been initialized. Starting processing...");

    // Spawn the pool subscriber listener in its own task
    tokio::spawn({
        // Move the pool_subscriber into the task (it's already mutable)
        async move {
            info!("Starting pool subscriber listener with blockchain querying...");
            if let Err(e) = pool_subscriber.start_listening().await {
                error!("Pool subscriber listener failed: {}", e);
            }
        }
    });
    
    // Main processing loop
    let mut last_stats_time = Instant::now();
    let mut total_txs_processed = 0;
    let mut total_scams_detected = 0;
    
    info!("Starting main transaction processing loop");
    
    let mut performance_metrics = PerformanceMetrics::new()?;
    
    info!("🚀 Starting main transaction processing loop (Rust: fast processing, Python: mining analysis)");
    
    loop {
        // Fetch new transactions from mempool
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                if !transactions.is_empty() {
                    let batch_start_time = Instant::now();
                    let batch_size = transactions.len();
                    
                    // Check if we're still in warmup phase
                    let is_warmup = performance_metrics.warmup_period_end.is_none();
                    
                    // Use optimized processing function
                    let scams_found = process_transactions(
                        transactions, 
                        tx_simulator.clone(), 
                        pool_cache.clone(), 
                        args.verbose,
                        !args.pool_transactions_only,  // Process all unless explicitly pool-only
                        &mut performance_metrics,
                        is_warmup,
                        ).await;
                        
                    if scams_found > 0 {
                        error!("🚨 DETECTED {} POTENTIAL SCAM(S)", scams_found);
                    }
                        
                    let batch_processing_time = batch_start_time.elapsed();
                    
                    // Update metrics
                    for _ in 0..batch_size {
                        total_txs_processed += 1;
                        total_scams_detected += scams_found;
                        
                        // Record per-transaction processing time (approximate)
                        let per_tx_time = batch_processing_time / batch_size.max(1) as u32;
                        performance_metrics.record_processing_time(per_tx_time);
                    }
                }
            },
            Err(e) => {
                error!("Error fetching transactions: {}", e);
                time::sleep(Duration::from_secs(1)).await;
            }
        }
        
        // Check if it's time to print stats
        let elapsed = last_stats_time.elapsed();
        if elapsed >= Duration::from_secs(args.stats_interval_seconds) {
            // Get performance metrics from fetcher
            let (success_rate, current_timeout, batch_size) = fetcher.get_performance_metrics();
            
            info!("Stats: {} transactions processed, {} scams detected", 
                 total_txs_processed, total_scams_detected);
            info!("Fetcher performance: {:.2}% success rate, {}ms timeout, {} batch size", 
                 success_rate * 100.0, current_timeout, batch_size);
            last_stats_time = Instant::now();
            
            performance_metrics.report_metrics();
        }
        
        // Small delay between fetches
        // This determines how often we poll for new transactions
        let poll_interval = if args.use_streaming {
            Duration::from_millis(10)  // 10ms = 100 polls/second for streaming mode
        } else {
            Duration::from_millis(100) // 100ms = 10 polls/second for other modes
        };
        time::sleep(poll_interval).await;
    }
}

// Process a single transaction using REVM simulation and return number of scams detected
async fn process_transaction_with_revm(
    tx: &TransactionView,
    simulator: &TransactionSimulator,
    block_env: &RevmBlockEnv,
    service: &ScamDetectionService,
    _db_logger: &Arc<DbLogger>,
    pool_cache: &Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) -> usize {
    let tx_hash_hex = hex::encode(&tx.hash);
    
    // Log full transaction details for debugging
    let to_addr = if let Some(to_bytes) = &tx.to {
        if !to_bytes.is_empty() {
            format!("0x{}", hex::encode(to_bytes))
        } else {
            "Empty".to_string()
        }
    } else {
        "None".to_string()
    };
    
    let from_addr = format!("0x{}", hex::encode(&tx.from));
    let value_eth = tx.value.as_u128() as f64 / 1e18;
    let gas_price_gwei = tx.gas_price.map(|p| p.as_u128() as f64 / 1e9).unwrap_or(0.0);
    
    // Check if this transaction involves any known pools
    let involves_pool = pool_cache.get_pool(&to_addr).is_some();
    
    if involves_pool {
        // LOG FULL TRANSACTION DETAILS FOR POOL-RELATED TRANSACTIONS
        info!("🎯 POOL TRANSACTION DETECTED");
        info!("  📋 Hash: 0x{}", tx_hash_hex);
        info!("  📤 From: {}", from_addr);
        info!("  📥 To: {}", to_addr);
        info!("  💰 Value: {:.6} ETH", value_eth);
        info!("  ⛽ Gas Price: {:.2} Gwei", gas_price_gwei);
        info!("  📊 Gas Limit: {}", tx.gas_limit.map(|g| g.to_string()).unwrap_or("None".to_string()));
        info!("  📋 Nonce: {:?}", tx.nonce);
        info!("  📋 Input Size: {} bytes", tx.input_data.as_ref().map(|d| d.len()).unwrap_or(0));
        
        // LOG CURRENT POOL STATE BEFORE SIMULATION
        if let Some(pool_state) = pool_cache.get_pool(&to_addr) {
            info!("  🏊 Pool {} current state:", to_addr);
            info!("    💧 ETH Reserve: {:.6} ETH", pool_state.eth_reserve);
            info!("    🪙 Token: {}", pool_state.token_address);
            info!("    📊 Block: {}", pool_state.last_updated_block);
            info!("    ⏰ Updated: {:.2}s ago", chrono::Utc::now().timestamp() as f64 - pool_state.last_updated_time);
        } else {
            info!("  ⚠️  Pool {} state not found in cache!", to_addr);
        }
    }
    
    // Simulate the transaction using REVM
    if tx.hash.len() == 32 {
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&tx.hash);
        let _tx_hash = H256::from(hash_bytes);
        
        if involves_pool {
            info!("  🧪 Starting REVM simulation...");
        }
        
        // Use REVM TransactionSimulator to get detailed state changes
        match simulator.process_transaction(tx, block_env).await {
            Ok(Some(account_changes)) => {
                debug!("REVM simulation successful for tx {}: {} accounts affected", 
                      &tx_hash_hex[..8], account_changes.len());
                
                // LOG DETAILED STATE CHANGES FROM REVM
                if involves_pool {
                    info!("  ✅ REVM simulation completed successfully");
                    info!("  📊 {} account(s) affected by simulation", account_changes.len());
                    
                    for (address, changes) in account_changes.iter() {
                        let addr_hex = format!("0x{:040x}", address);
                        info!("    👤 Account {}", addr_hex);
                        
                        // Convert SignedAmount to displayable ETH values
                        let eth_change_wei = changes.eth_net_change.absolute_value;
                        let eth_change_eth = eth_change_wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                        let sign = if changes.eth_net_change.is_negative { "-" } else { "+" };
                        info!("      💰 ETH Net Change: {}{:.6} ETH", sign, eth_change_eth);
                        
                        if !changes.token_net_changes.is_empty() {
                            info!("      🪙 Token changes: {} different tokens", changes.token_net_changes.len());
                            for (token_addr, token_change) in &changes.token_net_changes {
                                info!("        🏷️ 0x{:040x}: {} (raw)", 
                                      token_addr, token_change.to_signed_string());
                            }
                        }
                    }
                }
                
                // Convert REVM account changes to pool effects format
                let simulation = prepare_simulation_result_from_revm_changes(
                    tx, 
                    &account_changes, 
                    pool_cache.clone()
                );
                
                if let Some(sim_result) = simulation {
                    info!("🔬 Created simulation result for tx {} with {} affected pools", 
                          &tx_hash_hex[..8], sim_result.affected_pools.len());
                    
                    // LOG DETAILED POOL EFFECTS
                    for (pool_addr, effect) in &sim_result.affected_pools {
                        info!("  🏊 Affected pool {}: {:.6} → {:.6} ETH ({:.2}% change)", 
                              pool_addr, 
                              effect.current_eth_reserve,
                              effect.simulated_eth_reserve,
                              effect.percentage_change * 100.0);
                        
                        // HIGHLIGHT SUSPICIOUS PATTERNS
                        if effect.current_eth_reserve == 0.0 && effect.simulated_eth_reserve < 0.0 {
                            error!("  🚨 SUSPICIOUS: Pool {} went from 0 ETH to {:.6} ETH!", 
                                   pool_addr, effect.simulated_eth_reserve);
                            error!("  🔍 This suggests either:");
                            error!("    1. Pool state cache has stale/incorrect data (should be > 0 ETH)");
                            error!("    2. Transaction is doing something unexpected");
                            error!("    3. Simulation logic has issues");
                            
                            // LOG THE ACTUAL TRANSACTION INPUT FOR ANALYSIS
                            if tx.input_data.as_ref().map(|d| d.len()).unwrap_or(0) > 4 {
                                if let Some(input_data) = &tx.input_data {
                                    let method_sig = hex::encode(&input_data[0..4]);
                                    info!("  📋 Transaction method signature: 0x{}", method_sig);
                                    if input_data.len() > 68 {
                                        info!("  📋 First 64 bytes of input: {}", hex::encode(&input_data[4..68]));
                                    }
                                }
                            }
                        }
                    }
                    
                    // Process simulation result with the service
                    match service.process_transaction(sim_result).await {
                        Ok(alerts) => {
                            if !alerts.is_empty() {
                                // Log urgent scam alerts with full transaction hash
                                info!("🚨 SCAM DETECTED: {} alerts in tx {}", alerts.len(), tx_hash_hex);
                                
                                for alert in &alerts {
                                    info!("  🚨 Pool {} depleted: {:.6} → {:.6} ETH (Full tx: {})", 
                                        alert.pool_address, 
                                        alert.current_eth_reserve,
                                        alert.simulated_eth_reserve,
                                        tx_hash_hex);
                                }
                                return alerts.len();
                            }
                        },
                        Err(e) => {
                            error!("❌ Scam detection error for tx {}: {}", &tx_hash_hex[..8], e);
                        }
                    }
                }
            },
            Ok(None) => {
                // Only log no state changes for pool-related transactions
                if involves_pool {
                    debug!("⚪ No state changes for pool-related tx {}", &tx_hash_hex[..8]);
                }
            },
            Err(e) => {
                // Only log errors for pool-related transactions or unexpected errors
                if involves_pool || !e.to_string().contains("nonce") {
                    debug!("❌ REVM simulation failed for tx {}: {}", &tx_hash_hex[..8], e);
                }
            }
        }
    } else {
        error!("❌ Invalid transaction hash length for tx {}", tx_hash_hex);
    }
    
    0 // No scams detected
}

// Helper function to prepare a simulation result from REVM account changes
fn prepare_simulation_result_from_revm_changes(
    tx: &TransactionView,
    account_changes: &HashMap<revm_primitives::Address, revm_tx_simulator_lib::state_diff_utils::CalculatedAccountChanges>,
    pool_cache: Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) -> Option<SimulationResult> {
    let mut affected_pools = HashMap::new();
    let tx_hash_hex = hex::encode(&tx.hash);
    
    // Only log if we find pool interactions
    let mut pool_interactions_found = false;
    
    // Extract pool effects from REVM account changes
    for (address, changes) in account_changes {
        // Convert REVM address to normalized lowercase format for consistent lookup
        let addr_str = normalize_address(*address);
        
        // Calculate ETH delta from REVM account changes (using correct field name)
        let eth_delta = if changes.eth_net_change.is_negative {
            -(changes.eth_net_change.absolute_value.into_limbs()[0] as u128 as f64 / 1e18)
        } else {
            changes.eth_net_change.absolute_value.into_limbs()[0] as u128 as f64 / 1e18
        };
        
        // Skip addresses where ETH is being added (positive delta)
        if eth_delta >= 0.0 {
            continue;
        }
        
        // Skip very small changes (less than 0.001 ETH)
        if eth_delta.abs() < 0.001 {
            continue;
        }
        
        // Check if this address is a known pool (using normalized address)
        if let Some(pool_state) = pool_cache.get_pool(&addr_str) {
            pool_interactions_found = true;
            let current_eth = pool_state.eth_reserve;
            let simulated_eth = current_eth + eth_delta;
            let percentage_change = if current_eth > 0.0 {
                eth_delta / current_eth
            } else {
                0.0
            };
            
            debug!("🏊 Pool {} affected: {:.6} → {:.6} ETH ({:.2}% change)", 
                  addr_str, current_eth, simulated_eth, percentage_change * 100.0);
            
            let effect = PoolEffect {
                pool_address: addr_str.clone(),
                current_eth_reserve: current_eth,
                simulated_eth_reserve: simulated_eth,
                eth_delta,
                percentage_change,
            };
            
            affected_pools.insert(addr_str, effect);
        }
    }
    
    // Only log preparation details if we found pool interactions
    if pool_interactions_found {
        debug!("🔧 Preparing simulation result for tx {} with {} pool interactions found", 
               &tx_hash_hex[..8], affected_pools.len());
    }
    
    // If we have affected pools, create a simulation result
    if !affected_pools.is_empty() {
        let from_addr = if !tx.from.is_empty() {
            format!("0x{}", hex::encode(&tx.from))
        } else {
            "unknown".to_string()
        };
        
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&tx.hash);
        let tx_hash = H256::from(hash_bytes);
        
        // Remove duplicate log - already logged above as "🔬 Created simulation result"
        debug!("Simulation result prepared for tx {} with {} affected pools", 
              &tx_hash_hex[..8], affected_pools.len());
        
        Some(SimulationResult {
            tx_hash,
            from: from_addr,
            affected_pools,
        })
    } else {
        None
    }
}

async fn process_transactions(
    transactions: Vec<TransactionView>,
    tx_simulator: Arc<TransactionSimulator>,
    pool_cache: Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
    verbose: bool,
    process_all_transactions: bool,
    performance_metrics: &mut PerformanceMetrics,
    is_warmup: bool,
) -> u32 {
    let mut scams_detected = 0;
    
    for (tx_index, tx) in transactions.iter().enumerate() {
        let tx_hash_full = format!("0x{}", hex::encode(&tx.hash));
        let tx_hash_short = format!("0x{}", hex::encode(&tx.hash[..4])); // Short hash for logging
        
        // === STEP 1: Initialize timing tracker ===
        let mut timing = TransactionTiming::new(tx_hash_full.clone());
        
        // For now, assume transactions just arrived in mempool when we fetch them
        // This gives us a more realistic view of our actual processing performance
        timing.mempool_arrival_timestamp_ms = current_timestamp_ms();
        
        // Track this transaction for mining detection
        performance_metrics.track_transaction_arrival(&tx_hash_full);
        
        // Mark queue entry (when we started processing this batch)
        timing.queue_entry_timestamp_ms = current_timestamp_ms();
        
        // === STEP 2: Start processing ===
        timing.start_processing();
        
        if verbose {
            info!("🔄 PROCESSING TX: {} ({})", tx_hash_short, tx_hash_full);
        }
        
        // === STEP 3: Pool address check ===
        timing.start_pool_check();
        
        let (to_address, is_pool_tx) = match &tx.to {
            Some(addr_bytes) => {
                // Convert Vec<u8> to Address first, then normalize
                if addr_bytes.len() >= 20 {
                    let mut addr_array = [0u8; 20];
                    addr_array.copy_from_slice(&addr_bytes[addr_bytes.len()-20..]);
                    let addr = Address::from(addr_array);
                    let normalized = normalize_address(addr);
                    let has_pool = pool_cache.get_pool(&normalized).is_some();
                    (Some(normalized), has_pool)
                } else {
                    (None, false) // Invalid address
                }
            }
            None => (None, false), // Contract creation
        };
        
        timing.end_pool_check();
        timing.is_pool_transaction = is_pool_tx;
        timing.pool_address = to_address.clone();
        
        // Set transaction metadata
        timing.tx_value_wei = tx.value.to_string();
        timing.gas_price_wei = tx.gas_price.map(|p| p.to_string()).unwrap_or_else(|| "0".to_string());
        timing.gas_limit = tx.gas_limit.map(|g| g.as_u64()).unwrap_or(21000);
        
        // === STEP 4: Early exit for non-pool transactions (unless processing all) ===
        if !is_pool_tx && !process_all_transactions {
            if verbose {
                debug!("⏩ SKIP: {} (not pool transaction)", tx_hash_short);
            }
            
            // Finalize timing and log
            timing.finalize(false);
            performance_metrics.log_transaction_timing(&timing);
            
            // Log timing summary for verbose mode
            if verbose {
                info!("⏱️  TIMING [{}]: {}", tx_hash_short, timing.summary());
            }
            
            // === STEP 11: Real-time event logging for frontend ===
            if verbose {
                info!("📊 EVENT LOG [{}]:", tx_hash_short);
                info!("  📥 Mempool Arrival:    {}ms", timing.mempool_arrival_timestamp_ms);
                info!("  🚪 Queue Entry:        {}ms (+{:.1}ms)", 
                      timing.queue_entry_timestamp_ms,
                      timing.mempool_residence_time_us as f64 / 1000.0);
                info!("  🔄 Processing Start:   {}ms (+{:.1}ms)", 
                      timing.processing_start_timestamp_ms,
                      timing.internal_queue_time_us as f64 / 1000.0);
                info!("  🔍 Pool Check:         {}ms - {}ms ({:.1}ms)", 
                      timing.pool_check_start_timestamp_ms,
                      timing.pool_check_end_timestamp_ms,
                      timing.pool_check_time_us as f64 / 1000.0);
                info!("  🧪 REVM Simulation:    {}ms - {}ms ({:.1}ms)", 
                      timing.revm_simulation_start_timestamp_ms,
                      timing.revm_simulation_end_timestamp_ms,
                      timing.revm_simulation_time_us as f64 / 1000.0);
                info!("  📊 State Analysis:     {}ms - {}ms ({:.1}ms)", 
                      timing.state_analysis_start_timestamp_ms,
                      timing.state_analysis_end_timestamp_ms,
                      timing.state_analysis_time_us as f64 / 1000.0);
                info!("  🚨 Scam Detection:     {}ms - {}ms ({:.1}ms)", 
                      timing.scam_detection_start_timestamp_ms,
                      timing.scam_detection_end_timestamp_ms,
                      timing.scam_detection_time_us as f64 / 1000.0);
                info!("  ✅ Processing End:     {}ms", timing.processing_end_timestamp_ms);
                info!("  🎯 End-to-End Total:   {:.1}ms", timing.end_to_end_time_us as f64 / 1000.0);
                info!("  ⏳ Mining:             Analyzed by Python post-processing");
            }
            continue;
        }
        
        // === STEP 5: Log pool transaction details ===
        if verbose {
            let contract_creation = "CONTRACT_CREATION".to_string();
            let to_addr = to_address.as_ref().unwrap_or(&contract_creation);
            let from_addr = format!("0x{}", hex::encode(&tx.from));
            let value_eth = tx.value.as_u128() as f64 / 1e18;
            let gas_price_gwei = tx.gas_price.map(|p| p.as_u128() as f64 / 1e9).unwrap_or(0.0);
            
            info!("🎯 POOL TRANSACTION DETECTED");
            info!("  📋 Hash: {}", tx_hash_full);
            info!("  📤 From: {}", from_addr);
            info!("  📥 To: {}", to_addr);
            info!("  💰 Value: {:.6} ETH", value_eth);
            info!("  ⛽ Gas Price: {:.2} Gwei", gas_price_gwei);
            info!("  📊 Gas Limit: {}", timing.gas_limit);
            info!("  📋 Nonce: {:?}", tx.nonce);
            info!("  📋 Input Size: {} bytes", tx.input_data.as_ref().map(|d| d.len()).unwrap_or(0));
            
            // Log current pool state
            if let Some(pool_state) = pool_cache.get_pool(to_addr) {
                info!("  🏊 Pool {} current state:", to_addr);
                info!("    💧 ETH Reserve: {:.6} ETH", pool_state.eth_reserve);
                info!("    🪙 Token: {}", pool_state.token_address);
                info!("    📊 Block: {}", pool_state.last_updated_block);
                info!("    ⏰ Updated: {:.2}s ago", chrono::Utc::now().timestamp() as f64 - pool_state.last_updated_time);
            } else {
                warn!("  ⚠️  Pool {} state not found in cache!", to_addr);
            }
        }
        
        // === STEP 6: REVM simulation ===
        timing.start_revm_simulation();
        
        if verbose {
            info!("  🧪 Starting REVM simulation...");
        }
        
        let block_env = revm_context::BlockEnv::default();
        let simulation_result = tx_simulator.process_transaction(tx, &block_env).await;
        
        timing.end_revm_simulation();
        
        let mut scam_detected = false;
        
        match simulation_result {
            Ok(Some(account_changes)) => {
                timing.simulation_successful = true;
                timing.affected_accounts_count = account_changes.len();
                
                if verbose {
                    info!("  ✅ REVM simulation completed successfully");
                    info!("  📊 {} account(s) affected by simulation", account_changes.len());
                    
                    // Log detailed state changes
                    for (address, changes) in account_changes.iter() {
                        let addr_hex = format!("0x{:040x}", address);
                        info!("    👤 Account {}", addr_hex);
                        
                        let eth_change_wei = changes.eth_net_change.absolute_value;
                        let eth_change_eth = eth_change_wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                        let sign = if changes.eth_net_change.is_negative { "-" } else { "+" };
                        info!("      💰 ETH Net Change: {}{:.6} ETH", sign, eth_change_eth);
                        
                        if !changes.token_net_changes.is_empty() {
                            info!("      🪙 Token changes: {} different tokens", changes.token_net_changes.len());
                            for (token_addr, token_change) in &changes.token_net_changes {
                                info!("        🏷️ 0x{:040x}: {} (raw)", 
                                      token_addr, token_change.to_signed_string());
                            }
                        }
                    }
                }
                
                // === STEP 7: State analysis ===
                timing.start_state_analysis();
                
                let sim_result = prepare_simulation_result_from_revm_changes(
                    tx, 
                    &account_changes, 
                    pool_cache.clone()
                );
                
                timing.end_state_analysis();
                
                if let Some(simulation_result) = sim_result {
                    if verbose {
                        info!("🔬 Created simulation result for tx {} with {} affected pools", 
                              tx_hash_short, simulation_result.affected_pools.len());
                        
                        // Log detailed pool effects
                        for (pool_addr, effect) in &simulation_result.affected_pools {
                            info!("  🏊 Affected pool {}: {:.6} → {:.6} ETH ({:.2}% change)", 
                                  pool_addr, 
                                  effect.current_eth_reserve,
                                  effect.simulated_eth_reserve,
                                  effect.percentage_change * 100.0);
                            
                            // Highlight suspicious patterns
                            if effect.current_eth_reserve == 0.0 && effect.simulated_eth_reserve < 0.0 {
                                error!("  🚨 SUSPICIOUS: Pool {} went from 0 ETH to {:.6} ETH!", 
                                       pool_addr, effect.simulated_eth_reserve);
                                error!("  🔍 This suggests either:");
                                error!("    1. Pool state cache has stale/incorrect data (should be > 0 ETH)");
                                error!("    2. Transaction is doing something unexpected");
                                error!("    3. Simulation logic has issues");
                                
                                // Log transaction input for analysis
                                if tx.input_data.as_ref().map(|d| d.len()).unwrap_or(0) > 4 {
                                    if let Some(input_data) = &tx.input_data {
                                        let method_sig = hex::encode(&input_data[0..4]);
                                        info!("  📋 Transaction method signature: 0x{}", method_sig);
                                        if input_data.len() > 68 {
                                            info!("  📋 First 64 bytes of input: {}", hex::encode(&input_data[4..68]));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // === STEP 8: Scam detection ===
                    timing.start_scam_detection();
                    
                    scam_detected = check_for_scam_patterns(&simulation_result, verbose);
                    
                    timing.end_scam_detection();
                    
                    if scam_detected {
                        scams_detected += 1;
                        error!("🚨 SCAM DETECTED in tx {}", tx_hash_full);
                        
                        for (pool_addr, effect) in &simulation_result.affected_pools {
                            error!("  🚨 Pool {} depleted: {:.6} → {:.6} ETH", 
                                pool_addr, effect.current_eth_reserve, effect.simulated_eth_reserve);
                        }
                    }
                } else {
                    timing.start_state_analysis();
                    timing.end_state_analysis();
                    timing.start_scam_detection();
                    timing.end_scam_detection();
                }
            }
            Ok(None) => {
                timing.simulation_successful = true;
                timing.affected_accounts_count = 0;
                
                if verbose {
                    debug!("  ⚪ No state changes for pool-related tx {}", tx_hash_short);
                }
                
                // Still need to record timing for these phases
                timing.start_state_analysis();
                timing.end_state_analysis();
                timing.start_scam_detection();
                timing.end_scam_detection();
            }
            Err(e) => {
                timing.simulation_successful = false;
                timing.affected_accounts_count = 0;
                
                // Record timing for failed simulation
                timing.start_state_analysis();
                timing.end_state_analysis();
                timing.start_scam_detection();
                timing.end_scam_detection();
                
                if verbose || !e.to_string().contains("nonce") {
                    warn!("❌ REVM simulation failed for tx {}: {}", tx_hash_short, e);
                }
            }
        }
        
        // === STEP 9: Finalize timing and log ===
        timing.scam_detected = scam_detected;
        timing.finalize(is_warmup);
        performance_metrics.log_transaction_timing(&timing);
        
        // === STEP 10: Performance logging ===
        // Only log SLA violations or in verbose mode
        if timing.sla_violation && !is_warmup {
            // Concise violation log
            debug!("SLA VIOLATION [{}]: {}ms (REVM: {}ms)", 
                  tx_hash_short, 
                  timing.end_to_end_time_us as f64 / 1000.0,
                  timing.revm_simulation_time_us as f64 / 1000.0);
        } else if verbose && timing.end_to_end_time_us > 50_000 {
            // Verbose mode - only log slow transactions
            info!("SLOW TX [{}]: {}", tx_hash_short, timing.summary());
        }
        
        // === STEP 11: Real-time event logging for frontend ===
        if verbose {
            info!("📊 EVENT LOG [{}]:", tx_hash_short);
            info!("  📥 Mempool Arrival:    {}ms", timing.mempool_arrival_timestamp_ms);
            info!("  🚪 Queue Entry:        {}ms (+{:.1}ms)", 
                  timing.queue_entry_timestamp_ms,
                  timing.mempool_residence_time_us as f64 / 1000.0);
            info!("  🔄 Processing Start:   {}ms (+{:.1}ms)", 
                  timing.processing_start_timestamp_ms,
                  timing.internal_queue_time_us as f64 / 1000.0);
            info!("  🔍 Pool Check:         {}ms - {}ms ({:.1}ms)", 
                  timing.pool_check_start_timestamp_ms,
                  timing.pool_check_end_timestamp_ms,
                  timing.pool_check_time_us as f64 / 1000.0);
            info!("  🧪 REVM Simulation:    {}ms - {}ms ({:.1}ms)", 
                  timing.revm_simulation_start_timestamp_ms,
                  timing.revm_simulation_end_timestamp_ms,
                  timing.revm_simulation_time_us as f64 / 1000.0);
            info!("  📊 State Analysis:     {}ms - {}ms ({:.1}ms)", 
                  timing.state_analysis_start_timestamp_ms,
                  timing.state_analysis_end_timestamp_ms,
                  timing.state_analysis_time_us as f64 / 1000.0);
            info!("  🚨 Scam Detection:     {}ms - {}ms ({:.1}ms)", 
                  timing.scam_detection_start_timestamp_ms,
                  timing.scam_detection_end_timestamp_ms,
                  timing.scam_detection_time_us as f64 / 1000.0);
            info!("  ✅ Processing End:     {}ms", timing.processing_end_timestamp_ms);
            info!("  🎯 End-to-End Total:   {:.1}ms", timing.end_to_end_time_us as f64 / 1000.0);
            info!("  ⏳ Mining:             Analyzed by Python post-processing");
        }
    }
    
    scams_detected
}

/// Check if a simulation result contains scam patterns
fn check_for_scam_patterns(simulation_result: &SimulationResult, verbose: bool) -> bool {
    for (pool_address, effect) in &simulation_result.affected_pools {
        // Check for suspicious patterns like we do in the main detection service
        if effect.current_eth_reserve == 0.0 && effect.simulated_eth_reserve < 0.0 {
            if verbose {
                info!("🚨 SCAM PATTERN: Pool {} went from 0 ETH to {:.6} ETH", 
                      pool_address, effect.simulated_eth_reserve);
            }
            return true;
        }
        
        // Check for large percentage drains
        if effect.simulated_eth_reserve < 0.15 && effect.percentage_change < -0.5 {
            if verbose {
                info!("🚨 SCAM PATTERN: Pool {} drained from {:.6} to {:.6} ETH ({:.1}% change)", 
                      pool_address, effect.current_eth_reserve, effect.simulated_eth_reserve, 
                      effect.percentage_change * 100.0);
            }
            return true;
        }
    }
    false
} 