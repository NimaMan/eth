/*
Python-Compatible Mempool State Cache

Algorithm:
----------
This module implements a state cache that matches Python's MempoolTxCache behavior:

1. **Transaction Storage**: Store transactions with FIFO behavior and O(1) lookup
2. **State Diff Aggregation**: Aggregate state diffs by address across multiple pending transactions
3. **Address-Based Access**: Provide get_address_state_diffs(address) method like Python
4. **Automatic Cleanup**: Remove old transactions when cache reaches max size

Python Compatibility:
--------------------
This implementation matches Python's MempoolTxCache.add_state_diff() method:
- For each address in state_diff, if it doesn't exist, add it
- If it exists, update 'after' value and sum the 'change' values
- Maintain before/after/change format like Python

State Diff Format:
-----------------
Each state diff entry contains:
- before: ETH balance before transaction (in ETH units)
- after: ETH balance after transaction (in ETH units)  
- change: Net change in ETH balance (in ETH units)

This matches Python's StateDiffProcessor._extract_balance_change() output format.
*/

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolStateDiff {
    pub before: Option<f64>,  // ETH balance before (None if unknown)
    pub after: Option<f64>,   // ETH balance after (None if unknown)
    pub change: f64,          // Net change in ETH
}

#[derive(Debug, Clone)]
pub struct MempoolTransaction {
    pub hash: String,
    pub data: serde_json::Value,
    pub first_seen: u64,
    pub tx_state: String, // "pending" or "queued"
}

pub struct MempoolStateCache {
    max_size: usize,
    transactions: VecDeque<MempoolTransaction>,
    tx_lookup: HashMap<String, usize>, // hash -> index in transactions
    state_diffs: HashMap<String, MempoolStateDiff>, // address -> aggregated state diff
}

impl MempoolStateCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            transactions: VecDeque::new(),
            tx_lookup: HashMap::new(),
            state_diffs: HashMap::new(),
        }
    }

    /// Add a transaction to the cache (matches Python's MempoolTxCache.add)
    pub fn add_transaction(&mut self, tx_hash: String, tx_data: serde_json::Value, tx_state: String) {
        // Remove oldest transaction if at capacity
        if self.transactions.len() >= self.max_size {
            if let Some(old_tx) = self.transactions.pop_front() {
                self.tx_lookup.remove(&old_tx.hash);
                debug!("Removed old transaction {} from mempool cache", old_tx.hash);
            }
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let tx = MempoolTransaction {
            hash: tx_hash.clone(),
            data: tx_data,
            first_seen: now,
            tx_state,
        };

        // Add to end of queue and update lookup
        self.transactions.push_back(tx);
        self.tx_lookup.insert(tx_hash, self.transactions.len() - 1);
    }

    /// Add state diff and aggregate by address (matches Python's MempoolTxCache.add_state_diff)
    pub fn add_state_diff(&mut self, state_diff: HashMap<String, MempoolStateDiff>) {
        for (address, diff) in state_diff {
            if let Some(existing) = self.state_diffs.get_mut(&address) {
                // Update existing entry: update 'after' and sum 'change'
                existing.after = diff.after;
                existing.change += diff.change;
                debug!("Updated aggregated state diff for {}: change now {:.6} ETH", 
                       address, existing.change);
            } else {
                // Add new entry
                self.state_diffs.insert(address.clone(), diff);
                debug!("Added new state diff for {}: change {:.6} ETH", 
                       address, self.state_diffs[&address].change);
            }
        }
    }

    /// Get aggregated state diff for an address (matches Python's MempoolTxCache.get_state_diff)
    pub fn get_address_state_diffs(&self, address: &str) -> Option<&MempoolStateDiff> {
        self.state_diffs.get(address)
    }

    /// Get transaction by hash
    pub fn get_transaction(&self, tx_hash: &str) -> Option<&MempoolTransaction> {
        if let Some(&index) = self.tx_lookup.get(tx_hash) {
            self.transactions.get(index)
        } else {
            None
        }
    }

    /// Check if transaction exists in cache
    pub fn contains_transaction(&self, tx_hash: &str) -> bool {
        self.tx_lookup.contains_key(tx_hash)
    }

    /// Get all transactions with optional state filter
    pub fn get_transactions(&self, state_filter: Option<&str>) -> Vec<&MempoolTransaction> {
        match state_filter {
            Some(state) => self.transactions.iter()
                .filter(|tx| tx.tx_state == state)
                .collect(),
            None => self.transactions.iter().collect(),
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> (usize, usize) {
        (self.transactions.len(), self.state_diffs.len())
    }

    /// Clear old state diffs (optional cleanup)
    pub fn cleanup_old_state_diffs(&mut self, max_age_seconds: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Remove state diffs for addresses not affected by recent transactions
        let recent_addresses: std::collections::HashSet<String> = self.transactions
            .iter()
            .filter(|tx| now - tx.first_seen < max_age_seconds)
            .filter_map(|tx| {
                // Extract 'to' address from transaction data
                tx.data.get("to")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        let initial_count = self.state_diffs.len();
        self.state_diffs.retain(|address, _| recent_addresses.contains(address));
        let removed_count = initial_count - self.state_diffs.len();

        if removed_count > 0 {
            debug!("Cleaned up {} old state diffs, {} remaining", removed_count, self.state_diffs.len());
        }
    }
} 