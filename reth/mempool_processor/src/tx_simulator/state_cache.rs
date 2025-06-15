/*
 * In-Memory State Cache
 * 
 * This module implements an efficient in-memory cache for:
 * 1. Storing transactions in a limited-size FIFO queue
 * 2. Aggregating state diffs by address
 * 3. Tracking when transactions were first seen
 * 4. Providing fast access to accumulated state changes
 */

use crate::mempool_fetcher::types::TransactionView;
use crate::tx_simulator::state_diff::StateChange;
use ethers::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tracing::debug;

/// Maximum number of transactions to keep in the cache
const DEFAULT_MAX_TRANSACTIONS: usize = 100_000;
/// Maximum number of addresses to track state changes for
const DEFAULT_MAX_ADDRESSES: usize = 10_000;

/// Represents aggregated state changes for an address
#[derive(Debug, Clone)]
pub struct AggregatedStateChange {
    /// Last known balance
    pub balance: U256,
    /// Cumulative ETH value change (negative for outflows)
    pub eth_value_change: f64,
    /// When this address was last updated
    pub last_updated: u64,
    /// Storage changes for this address (slot => current value)
    pub storage: HashMap<H256, H256>,
    /// Number of transactions affecting this address
    pub transaction_count: usize,
}

/// Entry in the transaction cache with metadata
#[derive(Debug, Clone)]
struct TxCacheEntry {
    /// Transaction data
    pub tx: TransactionView,
    /// When this transaction was first seen
    pub first_seen: u64,
    /// State changes caused by this transaction
    pub state_changes: Vec<StateChange>,
}

/// In-memory state cache with FIFO behavior
pub struct StateCache {
    /// Maximum number of transactions to keep
    max_transactions: usize,
    /// Maximum number of addresses to track
    max_addresses: usize,
    /// Transaction queue with FIFO behavior
    tx_queue: VecDeque<H256>,
    /// Transaction data by hash
    transactions: HashMap<H256, TxCacheEntry>,
    /// Aggregated state changes by address
    state_diffs: HashMap<H160, AggregatedStateChange>,
}

impl StateCache {
    /// Create a new state cache with default settings
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_TRANSACTIONS, DEFAULT_MAX_ADDRESSES)
    }
    
    /// Create a new state cache with custom capacity
    pub fn with_capacity(max_transactions: usize, max_addresses: usize) -> Self {
        Self {
            max_transactions,
            max_addresses,
            tx_queue: VecDeque::with_capacity(max_transactions),
            transactions: HashMap::with_capacity(max_transactions),
            state_diffs: HashMap::with_capacity(max_addresses),
        }
    }
    
    /// Get current timestamp in seconds
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }
    
    /// Add a transaction to the cache
    pub fn add_transaction(&mut self, tx_hash: H256, tx: TransactionView, state_changes: Vec<StateChange>) {
        // Check if transaction already exists
        if self.transactions.contains_key(&tx_hash) {
            return;
        }
        
        // Remove oldest transaction if at capacity
        if self.tx_queue.len() >= self.max_transactions {
            if let Some(old_hash) = self.tx_queue.pop_front() {
                self.transactions.remove(&old_hash);
                debug!("Removed oldest transaction from cache: {}", hex::encode(old_hash.as_bytes()));
            }
        }
        
        // Add new transaction
        let entry = TxCacheEntry {
            tx,
            first_seen: Self::now(),
            state_changes: state_changes.clone(),
        };
        
        self.tx_queue.push_back(tx_hash);
        self.transactions.insert(tx_hash, entry);
        
        // Update aggregated state changes
        for change in state_changes {
            self.update_state_diff(change);
        }
    }
    
    /// Update aggregated state diff for an address
    fn update_state_diff(&mut self, change: StateChange) {
        let address = change.address;
        
        // Check if we need to prune before inserting
        if !self.state_diffs.contains_key(&address) && self.state_diffs.len() >= self.max_addresses {
            // Find oldest address by last_updated
            let oldest_addr = self.state_diffs.iter()
                .min_by_key(|(_, agg)| agg.last_updated)
                .map(|(addr, _)| *addr);
                
            // Remove oldest if found
            if let Some(addr) = oldest_addr {
                self.state_diffs.remove(&addr);
                debug!("Removed oldest address from cache: {}", hex::encode(addr.as_bytes()));
            }
        }
        
        // Now update or insert the state
        if let Some(aggregate) = self.state_diffs.get_mut(&address) {
            // Update existing entry
            aggregate.balance = change.balance_after;
            aggregate.eth_value_change += change.eth_value;
            aggregate.last_updated = Self::now();
            aggregate.transaction_count += 1;
            
            // Update storage changes
            for (slot, (_, after)) in change.storage_changes {
                aggregate.storage.insert(slot, after);
            }
        } else {
            // Create new entry
            let mut storage = HashMap::new();
            for (slot, (_, after)) in change.storage_changes {
                storage.insert(slot, after);
            }
            
            let aggregate = AggregatedStateChange {
                balance: change.balance_after,
                eth_value_change: change.eth_value,
                last_updated: Self::now(),
                storage,
                transaction_count: 1,
            };
            
            self.state_diffs.insert(address, aggregate);
        }
    }
    
    /// Get a transaction by hash
    pub fn get_transaction(&self, tx_hash: &H256) -> Option<&TransactionView> {
        self.transactions.get(tx_hash).map(|entry| &entry.tx)
    }
    
    /// Get when a transaction was first seen
    pub fn get_first_seen(&self, tx_hash: &H256) -> Option<u64> {
        self.transactions.get(tx_hash).map(|entry| entry.first_seen)
    }
    
    /// Get state changes for a transaction
    pub fn get_transaction_changes(&self, tx_hash: &H256) -> Option<&Vec<StateChange>> {
        self.transactions.get(tx_hash).map(|entry| &entry.state_changes)
    }
    
    /// Get aggregated state change for an address
    pub fn get_address_state(&self, address: &H160) -> Option<&AggregatedStateChange> {
        self.state_diffs.get(address)
    }
    
    /// Get all addresses with aggregated state changes
    pub fn get_all_addresses(&self) -> Vec<(H160, &AggregatedStateChange)> {
        self.state_diffs.iter().map(|(addr, state)| (*addr, state)).collect()
    }
    
    /// Get high value addresses (with significant ETH value changes)
    pub fn get_high_value_addresses(&self, min_eth_value: f64) -> Vec<(H160, &AggregatedStateChange)> {
        self.state_diffs.iter()
            .filter(|(_, state)| state.eth_value_change.abs() >= min_eth_value)
            .map(|(addr, state)| (*addr, state))
            .collect()
    }
    
    /// Get the number of transactions in the cache
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
    
    /// Get the number of addresses being tracked
    pub fn address_count(&self) -> usize {
        self.state_diffs.len()
    }
    
    /// Clear all cached data
    pub fn clear(&mut self) {
        self.tx_queue.clear();
        self.transactions.clear();
        self.state_diffs.clear();
    }
}

impl Default for StateCache {
    fn default() -> Self {
        Self::new()
    }
} 