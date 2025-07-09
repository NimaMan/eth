/// Simulator Processor for batch transaction simulation
/// 
/// This module handles the processing pipeline for transaction simulation:
/// 1. Receives batches of transactions with function info
/// 2. Filters transactions based on criteria
/// 3. Queues transactions for batch processing
/// 4. Simulates using reth_tx_simulator
/// 5. Analyzes results with SignalDetector

use std::time::{Duration, Instant};
use tracing::{info, warn};
use eyre::Result;

use crate::signal_engine::TransactionWithFunctions;
use crate::pool_subscriber::cache::PoolStateCache;
use super::{RethDirectTxSimulator, BatchSimulationOptions, ipc_to_call_request};
use super::signal_detector::SignalDetector;

/// Configuration for simulator processor
#[derive(Debug, Clone)]
pub struct SimulatorProcessorConfig {
    pub batch_size: usize,
    pub batch_timeout: Duration,
    pub max_queue_size: usize,
}

impl Default for SimulatorProcessorConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            batch_timeout: Duration::from_millis(100),
            max_queue_size: 1000,
        }
    }
}

/// Main processor for transaction simulation pipeline
pub struct SimulatorProcessor {
    reth_simulator: RethDirectTxSimulator,
    signal_detector: SignalDetector,
    transaction_queue: Vec<TransactionWithFunctions>,
    config: SimulatorProcessorConfig,
    last_batch_time: Instant,
    stats: ProcessorStats,
}

#[derive(Debug, Default, Clone)]
struct ProcessorStats {
    total_received: u64,
    total_filtered: u64,
    total_simulated: u64,
    batches_processed: u64,
}

impl SimulatorProcessor {
    /// Create new simulator processor
    pub fn new(reth_datadir: &str, log_dir: std::path::PathBuf) -> Result<Self> {
        let reth_simulator = RethDirectTxSimulator::new(reth_datadir)?;
        let signal_detector = SignalDetector::new(Default::default(), log_dir);
        let config = SimulatorProcessorConfig::default();
        
        info!("🔄 SimulatorProcessor initialized");
        info!("   Batch size: {}", config.batch_size);
        info!("   Batch timeout: {:?}", config.batch_timeout);
        info!("   Max queue size: {}", config.max_queue_size);
        
        Ok(Self {
            reth_simulator,
            signal_detector,
            transaction_queue: Vec::new(),
            config,
            last_batch_time: Instant::now(),
            stats: ProcessorStats::default(),
        })
    }
    
    /// Set the pool state cache for advanced drain detection
    pub fn set_pool_cache(&mut self, pool_cache: PoolStateCache) {
        self.signal_detector.set_pool_cache(pool_cache);
    }
    
    /// Process a batch of transactions with function information
    pub async fn process_batch(&mut self, transactions: Vec<TransactionWithFunctions>) -> Result<()> {
        self.stats.total_received += transactions.len() as u64;
        
        // Filter transactions
        let filtered = self.filter_transactions(transactions);
        self.stats.total_filtered += filtered.len() as u64;
        
        if filtered.is_empty() {
            return Ok(());
        }
        
        // Add to queue
        self.transaction_queue.extend(filtered);
        
        // Check if we need to process batch
        if self.should_process_batch() {
            self.simulate_and_analyze().await?;
        }
        
        Ok(())
    }
    
    /// Filter transactions based on criteria
    fn filter_transactions(&self, transactions: Vec<TransactionWithFunctions>) -> Vec<TransactionWithFunctions> {
        transactions.into_iter()
            .filter(|tx| self.should_simulate(tx))
            .collect()
    }
    
    /// Determine if a transaction should be simulated
    fn should_simulate(&self, tx: &TransactionWithFunctions) -> bool {
        // Filter out simple transfers (no function calls)
        if tx.tx.input.len() <= 4 {
            return false;
        }
        
        // Include transactions with detected functions
        if tx.has_liquidity_removal || tx.has_trading_enabled {
            return true;
        }
        
        // Include transactions with function calls (non-empty input data)
        tx.tx.input.len() > 4
    }
    
    /// Check if we should process the current batch
    fn should_process_batch(&self) -> bool {
        // Process if we have enough transactions
        if self.transaction_queue.len() >= self.config.batch_size {
            return true;
        }
        
        // Process if timeout reached and we have transactions
        if !self.transaction_queue.is_empty() && 
           self.last_batch_time.elapsed() >= self.config.batch_timeout {
            return true;
        }
        
        false
    }
    
    /// Simulate the current batch and analyze results
    async fn simulate_and_analyze(&mut self) -> Result<()> {
        if self.transaction_queue.is_empty() {
            return Ok(());
        }
        
        let batch_size = self.transaction_queue.len();
        
        // Convert to CallRequest batch
        let mut call_requests = Vec::new();
        for tx in &self.transaction_queue {
            match ipc_to_call_request(&tx.tx.data) {
                Ok(call_request) => {
                    call_requests.push((tx.tx.hash.clone(), call_request));
                }
                Err(e) => {
                    warn!("Failed to convert transaction {}: {}", tx.tx.hash, e);
                }
            }
        }
        
        if call_requests.is_empty() {
            warn!("No valid transactions to simulate in batch");
            self.transaction_queue.clear();
            return Ok(());
        }
        
        // Use existing reth_tx_simulator batch method
        let simulation_start = Instant::now();
        let results = self.reth_simulator
            .simulate_unsigned_batch_with_call_trace(call_requests, BatchSimulationOptions::default())
            .await?;
        
        let simulation_time = simulation_start.elapsed();
        
        // Process results with signal detector
        let mut successful_simulations = 0;
        let mut total_state_changes = 0;
        
        for (tx_hash, state_changes_result) in results {
            match state_changes_result {
                Ok(state_changes) => {
                    successful_simulations += 1;
                    total_state_changes += state_changes.len();
                    
                    // Find the corresponding transaction to get the from address
                    let from_address = self.transaction_queue.iter()
                        .find(|tx| tx.tx.hash == tx_hash)
                        .map(|tx| {
                            // Convert Vec<u8> to Address (20 bytes)
                            if tx.tx.from.len() == 20 {
                                let mut addr_bytes = [0u8; 20];
                                addr_bytes.copy_from_slice(&tx.tx.from);
                                alloy_primitives::Address::from(addr_bytes)
                            } else {
                                alloy_primitives::Address::ZERO
                            }
                        })
                        .unwrap_or(alloy_primitives::Address::ZERO);
                    
                    let (to_address, functions) = self.transaction_queue.iter()
                        .find(|tx| tx.tx.hash == tx_hash)
                        .map(|tx_with_functions| {
                            let to = tx_with_functions.tx.to.as_ref()
                                .map(|to_bytes| {
                                    // Convert Vec<u8> to Address (20 bytes)
                                    if to_bytes.len() == 20 {
                                        let mut addr_bytes = [0u8; 20];
                                        addr_bytes.copy_from_slice(to_bytes);
                                        alloy_primitives::Address::from(addr_bytes)
                                    } else {
                                        alloy_primitives::Address::ZERO
                                    }
                                });
                            
                            (to, tx_with_functions.functions.clone())
                        })
                        .unwrap_or((None, Vec::new()));
                    
                    // Set functions in signal detector
                    self.signal_detector.set_functions(functions);
                    
                    // Analyze with signal detector
                    let _signals = self.signal_detector.analyze_simulation_result(
                        &tx_hash,
                        from_address,
                        to_address,
                        &state_changes,
                        simulation_time.as_micros() as u64,
                    );
                }
                Err(e) => {
                    warn!("Simulation failed for {}: {}", tx_hash, e);
                }
            }
        }
        
        // Update stats
        self.stats.total_simulated += successful_simulations;
        self.stats.batches_processed += 1;
        
        // Clear queue and reset timer
        self.transaction_queue.clear();
        self.last_batch_time = Instant::now();
        
        Ok(())
    }
    
    /// Force process current batch (useful for shutdown)
    pub async fn flush_batch(&mut self) -> Result<()> {
        if !self.transaction_queue.is_empty() {
            self.simulate_and_analyze().await?;
        }
        Ok(())
    }
    
    /// Get processor statistics
    pub fn get_stats(&self) -> ProcessorStats {
        self.stats.clone()
    }
    
    /// Log performance summary
    pub fn log_performance_summary(&self) {
        info!("📊 SimulatorProcessor Performance:");
        info!("   Total received: {}", self.stats.total_received);
        info!("   Total filtered: {}", self.stats.total_filtered);
        info!("   Total simulated: {}", self.stats.total_simulated);
        info!("   Batches processed: {}", self.stats.batches_processed);
        info!("   Current queue size: {}", self.transaction_queue.len());
        
        if self.stats.total_received > 0 {
            let filter_rate = (self.stats.total_filtered as f64 / self.stats.total_received as f64) * 100.0;
            info!("   Filter rate: {:.1}%", filter_rate);
        }
    }
}

