/// Simulator Processor for batch transaction simulation
/// 
/// This module handles the processing pipeline for transaction simulation:
/// 1. Receives batches of transactions with function info
/// 2. Filters transactions based on criteria
/// 3. Queues transactions for batch processing
/// 4. Simulates using reth_tx_simulator
/// 5. Analyzes results with SignalDetector

use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use eyre::Result;

use crate::signal_engine::{TransactionWithFunctions, CreatorAnalyzer};
use crate::token_tracking::{TokenTrackingCache, cache::PoolStateCache};
use super::{DirectTxSimulator, BatchSimulationOptions, CallRequest};
use super::signal_detector::SignalDetector;
use alloy_primitives::{Address, Bytes, U256};

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
    reth_simulator: DirectTxSimulator,
    signal_detector: SignalDetector,
    creator_analyzer: Option<CreatorAnalyzer>,
    // Thread-safe mutable state
    transaction_queue: Arc<Mutex<Vec<TransactionWithFunctions>>>,
    config: SimulatorProcessorConfig,
    last_batch_time: Arc<Mutex<Instant>>,
    stats: Arc<Mutex<ProcessorStats>>,
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
        let reth_simulator = DirectTxSimulator::new(reth_datadir)?;
        let signal_detector = SignalDetector::new(Default::default(), log_dir);
        let config = SimulatorProcessorConfig::default();
        
        info!("🔄 SimulatorProcessor initialized");
        info!("   Batch size: {}", config.batch_size);
        info!("   Batch timeout: {:?}", config.batch_timeout);
        info!("   Max queue size: {}", config.max_queue_size);
        
        Ok(Self {
            reth_simulator,
            signal_detector,
            creator_analyzer: None,
            transaction_queue: Arc::new(Mutex::new(Vec::new())),
            config,
            last_batch_time: Arc::new(Mutex::new(Instant::now())),
            stats: Arc::new(Mutex::new(ProcessorStats::default())),
        })
    }
    
    /// Set the pool state cache for advanced drain detection
    pub fn set_pool_cache(&mut self, pool_cache: PoolStateCache) {
        self.signal_detector.set_pool_cache(pool_cache);
    }
    
    /// Set the token tracking cache for creator analysis
    pub fn set_token_cache(&mut self, token_cache: Arc<TokenTrackingCache>) {
        self.creator_analyzer = Some(CreatorAnalyzer::new(token_cache));
    }
    
    /// Process a batch of transactions with function information
    pub async fn process_batch(&mut self, transactions: Vec<TransactionWithFunctions>) -> Result<()> {
        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.total_received += transactions.len() as u64;
        }
        
        // First check for creator actions - these get immediate alerts
        if let Some(ref creator_analyzer) = self.creator_analyzer {
            for tx in &transactions {
                if tx.has_creator_action {
                    if let Some(alert) = creator_analyzer.analyze_transaction(tx).await {
                        // Log critical creator actions
                        match alert.severity {
                            crate::signal_engine::AlertSeverity::Critical => {
                                warn!("🚨 CRITICAL: Creator {} calling {} on token {:?}", 
                                      alert.creator_address, alert.function_name, alert.token_address);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // Filter transactions for simulation
        let filtered = self.filter_transactions(transactions);
        {
            let mut stats = self.stats.lock().await;
            stats.total_filtered += filtered.len() as u64;
        }
        
        if filtered.is_empty() {
            return Ok(());
        }
        
        // Add to queue
        {
            let mut queue = self.transaction_queue.lock().await;
            queue.extend(filtered);
        }
        
        // Check if we need to process batch
        if self.should_process_batch().await {
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
        if tx.has_liquidity_removal || tx.has_trading_enabled || tx.has_creator_action {
            return true;
        }
        
        // Include transactions with function calls (non-empty input data)
        tx.tx.input.len() > 4
    }
    
    /// Check if we should process the current batch
    async fn should_process_batch(&self) -> bool {
        let queue_len = {
            let queue = self.transaction_queue.lock().await;
            queue.len()
        };
        
        // Process if we have enough transactions
        if queue_len >= self.config.batch_size {
            return true;
        }
        
        // Process if timeout reached and we have transactions
        if queue_len > 0 {
            let last_batch_time = self.last_batch_time.lock().await;
            if last_batch_time.elapsed() >= self.config.batch_timeout {
                return true;
            }
        }
        
        false
    }
    
    /// Convert NonBlockingTransaction to CallRequest using pre-parsed bytes
    fn nonblocking_tx_to_call_request(tx: &crate::mempool_fetcher::NonBlockingTransaction) -> CallRequest {
        // Convert addresses from Vec<u8> to Address
        let from = if tx.from.len() == 20 {
            let mut addr_bytes = [0u8; 20];
            addr_bytes.copy_from_slice(&tx.from);
            Some(Address::from(addr_bytes))
        } else {
            None
        };
        
        let to = tx.to.as_ref().and_then(|to_bytes| {
            if to_bytes.len() == 20 {
                let mut addr_bytes = [0u8; 20];
                addr_bytes.copy_from_slice(to_bytes);
                Some(Address::from(addr_bytes))
            } else {
                None
            }
        });
        
        // Convert U256 values
        let value = Some(U256::from_limbs(tx.value.0));
        let gas_price = tx.gas_price.map(|gp| gp.low_u128());
        
        // Use pre-parsed input bytes directly
        let data = if tx.input.is_empty() {
            None
        } else {
            Some(Bytes::from(tx.input.clone()))
        };
        
        CallRequest {
            from,
            to,
            gas: None, // Will be estimated
            gas_price,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            value,
            data,
            nonce: None, // Will be auto-adapted
        }
    }
    
    /// Simulate the current batch and analyze results
    async fn simulate_and_analyze(&mut self) -> Result<()> {
        // Take transactions from queue
        let transactions = {
            let mut queue = self.transaction_queue.lock().await;
            if queue.is_empty() {
                return Ok(());
            }
            std::mem::take(&mut *queue)
        };
        
        let _batch_size = transactions.len();
        
        // Convert to CallRequest batch using efficient byte conversion
        let mut call_requests = Vec::new();
        for tx in &transactions {
            let call_request = Self::nonblocking_tx_to_call_request(&tx.tx);
            call_requests.push((tx.tx.hash.clone(), call_request));
        }
        
        if call_requests.is_empty() {
            warn!("No valid transactions to simulate in batch");
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
        let mut _total_state_changes = 0;
        
        for (tx_hash, state_changes_result) in results {
            match state_changes_result {
                Ok(state_changes) => {
                    successful_simulations += 1;
                    _total_state_changes += state_changes.len();
                    
                    // Find the corresponding transaction to get the from address
                    let from_address = transactions.iter()
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
                    
                    let (to_address, functions) = transactions.iter()
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
                    ).await;
                }
                Err(e) => {
                    warn!("Simulation failed for {}: {}", tx_hash, e);
                }
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.total_simulated += successful_simulations;
            stats.batches_processed += 1;
        }
        
        // Reset timer
        {
            let mut last_batch_time = self.last_batch_time.lock().await;
            *last_batch_time = Instant::now();
        }
        
        Ok(())
    }
    
    /// Force process current batch (useful for shutdown)
    pub async fn flush_batch(&mut self) -> Result<()> {
        let has_transactions = {
            let queue = self.transaction_queue.lock().await;
            !queue.is_empty()
        };
        
        if has_transactions {
            self.simulate_and_analyze().await?;
        }
        Ok(())
    }
    
    /// Get processor statistics
    pub async fn get_stats(&self) -> ProcessorStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }
    
    /// Log performance summary
    pub async fn log_performance_summary(&self) {
        let stats = self.stats.lock().await;
        let queue_len = {
            let queue = self.transaction_queue.lock().await;
            queue.len()
        };
        
        info!("📊 SimulatorProcessor Performance:");
        info!("   Total received: {}", stats.total_received);
        info!("   Total filtered: {}", stats.total_filtered);
        info!("   Total simulated: {}", stats.total_simulated);
        info!("   Batches processed: {}", stats.batches_processed);
        info!("   Current queue size: {}", queue_len);
        
        if stats.total_received > 0 {
            let filter_rate = (stats.total_filtered as f64 / stats.total_received as f64) * 100.0;
            info!("   Filter rate: {:.1}%", filter_rate);
        }
    }
}

