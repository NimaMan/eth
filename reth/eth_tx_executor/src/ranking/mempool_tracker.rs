//! Real-time mempool monitoring and transaction tracking
//!
//! Connects to Reth WebSocket to monitor pending transactions and maintain
//! a live view of mempool state for gas optimization.

use ethers::prelude::*;
use ethers::providers::Ws;
use futures_util::StreamExt;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use super::{MempoolStats, GasPercentiles, CongestionLevel};

/// Tracks pending transactions in real-time
pub struct MempoolTracker {
    /// WebSocket connection to Reth
    provider: Arc<Provider<Ws>>,
    /// Current mempool state
    state: Arc<RwLock<MempoolState>>,
    /// Configuration
    config: TrackerConfig,
}

/// Current state of the mempool
#[derive(Debug)]
pub struct MempoolState {
    /// Pending transactions indexed by gas price
    pub gas_price_index: BTreeMap<U256, Vec<PendingTransaction>>,
    /// Transaction hash to details mapping
    pub transactions: std::collections::HashMap<H256, PendingTransaction>,
    /// Recent arrival times for rate calculation
    pub arrival_times: VecDeque<Instant>,
    /// Current block number
    pub current_block: u64,
    /// Last update timestamp
    pub last_update: Instant,
    /// Total pending transaction count
    pub total_pending: u64,
}

/// Pending transaction information
#[derive(Debug, Clone)]
pub struct PendingTransaction {
    pub hash: H256,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub nonce: U256,
    pub data: Bytes,
    
    // EIP-1559 fields
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee: Option<U256>,
    
    // Timing information
    pub first_seen: Instant,
    
    // Analysis flags
    pub is_mev_candidate: bool,
    pub estimated_profit: Option<U256>,
}

/// Tracker configuration
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Maximum transactions to track (memory limit)
    pub max_transactions: usize,
    /// How long to keep arrival time history
    pub arrival_history_duration: Duration,
    /// Minimum gas price to track (filter spam)
    pub min_gas_price: U256,
    /// Maximum gas price to track (filter unrealistic)
    pub max_gas_price: U256,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            max_transactions: 50_000,
            arrival_history_duration: Duration::from_secs(300), // 5 minutes
            min_gas_price: U256::from(1_000_000_000u64), // 1 gwei
            max_gas_price: U256::from(1_000_000_000_000u64), // 1000 gwei
        }
    }
}

impl MempoolTracker {
    /// Create new mempool tracker
    pub async fn new(ws_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Arc::new(Provider::<Ws>::connect(ws_url).await?);
        
        let state = Arc::new(RwLock::new(MempoolState {
            gas_price_index: BTreeMap::new(),
            transactions: std::collections::HashMap::new(),
            arrival_times: VecDeque::new(),
            current_block: 0,
            last_update: Instant::now(),
            total_pending: 0,
        }));
        
        Ok(Self {
            provider,
            state,
            config: TrackerConfig::default(),
        })
    }
    
    /// Start monitoring the mempool
    pub async fn start_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting mempool monitoring");
        
        // Subscribe to pending transactions
        let mut pending_stream = self.provider.subscribe_pending_txs().await?;
        
        // Also subscribe to new blocks to track confirmations
        let mut block_stream = self.provider.subscribe_blocks().await?;
        
        loop {
            tokio::select! {
                // Handle new pending transaction
                Some(tx_hash) = pending_stream.next() => {
                    if let Err(e) = self.handle_pending_transaction(tx_hash).await {
                        warn!("Failed to handle pending transaction: {}", e);
                    }
                }
                
                // Handle new block (clean up confirmed transactions)
                Some(block) = block_stream.next() => {
                    if let Err(e) = self.handle_new_block(block).await {
                        warn!("Failed to handle new block: {}", e);
                    }
                }
                
                // Periodic cleanup
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    self.cleanup_old_data().await;
                }
            }
        }
    }
    
    /// Handle new pending transaction
    async fn handle_pending_transaction(&self, tx_hash: H256) -> Result<(), Box<dyn std::error::Error>> {
        // Get transaction details
        let tx = match self.provider.get_transaction(tx_hash).await? {
            Some(tx) => tx,
            None => {
                debug!("Transaction {} not found", tx_hash);
                return Ok(());
            }
        };
        
        // Filter by gas price
        let gas_price = tx.gas_price.unwrap_or_default();
        if gas_price < self.config.min_gas_price || gas_price > self.config.max_gas_price {
            return Ok(());
        }
        
        let pending_tx = PendingTransaction {
            hash: tx.hash,
            from: tx.from,
            to: tx.to,
            value: tx.value,
            gas_price,
            gas_limit: tx.gas,
            nonce: tx.nonce,
            data: tx.input.clone(),
            max_fee_per_gas: tx.max_fee_per_gas,
            max_priority_fee: tx.max_priority_fee_per_gas,
            first_seen: Instant::now(),
            is_mev_candidate: self.is_mev_candidate(&tx),
            estimated_profit: None, // TODO: Implement MEV profit estimation
        };
        
        // Update state
        self.update_state_with_transaction(pending_tx).await;
        
        Ok(())
    }
    
    /// Handle new block (clean up confirmed transactions)
    async fn handle_new_block(&self, block: Block<H256>) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.state.write().await;
        state.current_block = block.number.unwrap_or_default().as_u64();
        
        // Remove confirmed transactions from our tracking
        for tx_hash in &block.transactions {
            if let Some(tx) = state.transactions.remove(tx_hash) {
                // Remove from gas price index
                if let Some(txs) = state.gas_price_index.get_mut(&tx.gas_price) {
                    txs.retain(|t| t.hash != *tx_hash);
                    if txs.is_empty() {
                        state.gas_price_index.remove(&tx.gas_price);
                    }
                }
                state.total_pending = state.total_pending.saturating_sub(1);
            }
        }
        
        debug!("Block {}: removed {} confirmed transactions", 
            state.current_block, block.transactions.len());
        
        Ok(())
    }
    
    /// Update mempool state with new transaction
    async fn update_state_with_transaction(&self, tx: PendingTransaction) {
        let mut state = self.state.write().await;
        
        // Check if we already have this transaction
        if state.transactions.contains_key(&tx.hash) {
            return;
        }
        
        // Add to gas price index
        state.gas_price_index
            .entry(tx.gas_price)
            .or_insert_with(Vec::new)
            .push(tx.clone());
        
        // Add to transactions map
        state.transactions.insert(tx.hash, tx);
        
        // Update arrival rate tracking
        state.arrival_times.push_back(Instant::now());
        
        // Update counters
        state.total_pending += 1;
        state.last_update = Instant::now();
        
        // Enforce memory limits
        if state.transactions.len() > self.config.max_transactions {
            self.evict_oldest_transactions(&mut state).await;
        }
        
        debug!("Mempool updated: {} pending transactions", state.total_pending);
    }
    
    /// Remove oldest transactions to stay under memory limit
    async fn evict_oldest_transactions(&self, state: &mut MempoolState) {
        let target_size = self.config.max_transactions * 9 / 10; // Remove 10%
        let mut transactions_by_age: Vec<_> = state.transactions.values().collect();
        transactions_by_age.sort_by_key(|tx| tx.first_seen);
        
        let to_remove = state.transactions.len().saturating_sub(target_size);
        let hashes_to_remove: Vec<H256> = transactions_by_age
            .iter()
            .take(to_remove)
            .map(|tx| tx.hash)
            .collect();
        
        for hash in &hashes_to_remove {
            if let Some(tx) = state.transactions.remove(hash) {
                // Remove from gas price index
                if let Some(txs) = state.gas_price_index.get_mut(&tx.gas_price) {
                    txs.retain(|t| t.hash != *hash);
                    if txs.is_empty() {
                        state.gas_price_index.remove(&tx.gas_price);
                    }
                }
                
                state.total_pending = state.total_pending.saturating_sub(1);
            }
        }
        
        debug!("Evicted {} old transactions", hashes_to_remove.len());
    }
    
    /// Clean up old arrival time data
    async fn cleanup_old_data(&self) {
        let mut state = self.state.write().await;
        let cutoff = Instant::now() - self.config.arrival_history_duration;
        
        while let Some(&front_time) = state.arrival_times.front() {
            if front_time < cutoff {
                state.arrival_times.pop_front();
            } else {
                break;
            }
        }
    }
    
    /// Check if transaction is potential MEV candidate
    fn is_mev_candidate(&self, tx: &Transaction) -> bool {
        // Simple heuristics for MEV detection
        if tx.to.is_none() {
            return false; // Contract creation
        }
        
        // High gas price relative to base fee could indicate MEV
        let gas_price = tx.gas_price.unwrap_or_default();
        if gas_price > U256::from(50_000_000_000u64) { // > 50 gwei
            return true;
        }
        
        // Large value transfers could be MEV
        if tx.value > U256::from(10).pow(U256::from(18)) { // > 1 ETH
            return true;
        }
        
        // TODO: Add more sophisticated MEV detection
        // - Check if interacting with known DEX contracts
        // - Analyze transaction data for swap signatures
        // - Look for sandwich attack patterns
        
        false
    }
    
    /// Get current mempool statistics
    pub async fn get_stats(&self) -> MempoolStats {
        let state = self.state.read().await;
        
        // Calculate gas price percentiles
        let gas_prices: Vec<U256> = state.gas_price_index.keys().cloned().collect();
        let percentiles = self.calculate_percentiles(&gas_prices);
        
        // Calculate arrival rate
        let recent_arrivals = state.arrival_times.len() as f64;
        let time_window = self.config.arrival_history_duration.as_secs_f64();
        let arrival_rate = recent_arrivals / time_window;
        
        // Determine congestion level
        let congestion = match state.total_pending {
            0..=1000 => CongestionLevel::Low,
            1001..=5000 => CongestionLevel::Medium,
            5001..=15000 => CongestionLevel::High,
            _ => CongestionLevel::Extreme,
        };
        
        // Count MEV transactions
        let mev_count = state.transactions.values()
            .filter(|tx| tx.is_mev_candidate)
            .count() as u64;
        
        MempoolStats {
            pending_tx_count: state.total_pending,
            gas_price_percentiles: percentiles,
            avg_arrival_rate: arrival_rate,
            congestion_level: congestion,
            mev_tx_count: mev_count,
        }
    }
    
    /// Calculate gas price percentiles
    fn calculate_percentiles(&self, gas_prices: &[U256]) -> GasPercentiles {
        if gas_prices.is_empty() {
            return GasPercentiles {
                p50: U256::zero(),
                p75: U256::zero(),
                p90: U256::zero(),
                p95: U256::zero(),
                p99: U256::zero(),
            };
        }
        
        let mut sorted_prices = gas_prices.to_vec();
        sorted_prices.sort();
        
        let len = sorted_prices.len();
        
        GasPercentiles {
            p50: sorted_prices[len * 50 / 100],
            p75: sorted_prices[len * 75 / 100],
            p90: sorted_prices[len * 90 / 100],
            p95: sorted_prices[len * 95 / 100],
            p99: sorted_prices[len * 99 / 100],
        }
    }
    
    /// Get current block number
    pub async fn get_current_block_number(&self) -> u64 {
        let state = self.state.read().await;
        state.current_block
    }
    
    /// Get gas price needed to be in top N positions
    pub async fn get_gas_price_for_position(&self, target_position: u64) -> Option<U256> {
        let state = self.state.read().await;
        
        let mut position = 0u64;
        for (&gas_price, txs) in state.gas_price_index.iter().rev() {
            position += txs.len() as u64;
            if position >= target_position {
                return Some(gas_price);
            }
        }
        
        None
    }
    
    /// Get estimated position for given gas price
    pub async fn get_position_for_gas_price(&self, gas_price: U256) -> u64 {
        let state = self.state.read().await;
        
        let mut position = 0u64;
        for (&price, txs) in state.gas_price_index.iter().rev() {
            if price > gas_price {
                position += txs.len() as u64;
            } else {
                break;
            }
        }
        
        position + 1 // +1 for our transaction
    }
}