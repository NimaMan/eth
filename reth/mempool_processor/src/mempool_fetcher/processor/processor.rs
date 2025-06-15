use crate::mempool_fetcher::types::*;
use crate::tx_simulator::state_diff::StateDiffTracker;
use crate::tx_simulator::MempoolStateDiff;
use ethers::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, trace, info, warn};
use hex::encode as hex_encode;
use std::sync::Arc;
use std::collections::HashMap;

/// Transaction processor that filters and generates alerts
pub struct TransactionProcessor {
    filter: TransactionFilter,
    state_tracker: Option<StateDiffTracker>,
    cached_state_changes: HashMap<String, HashMap<String, MempoolStateDiff>>,
}

impl TransactionProcessor {
    pub fn new(filter: TransactionFilter) -> Self {
        Self { 
            filter,
            state_tracker: None,
            cached_state_changes: HashMap::new(),
        }
    }
    
    /// Set the minimum value threshold for alerts
    pub fn with_value_threshold(mut self, threshold: U256) -> Self {
        self.filter.min_value = threshold;
        self
    }
    
    /// Add addresses to watch list
    pub fn with_watched_addresses(mut self, addresses: Vec<String>) -> Self {
        // Ensure addresses don't have 0x prefix
        for addr in addresses {
            let clean_addr = addr.strip_prefix("0x").unwrap_or(&addr).to_string();
            self.filter.watched_addresses.insert(clean_addr);
        }
        self
    }
    
    /// Set minimum gas price threshold
    pub fn with_min_gas_price(mut self, min_gas_price: U256) -> Self {
        self.filter.min_gas_price = Some(min_gas_price);
        self
    }
    
    /// Enable state diff tracking with REVM
    pub fn with_state_diff_tracking(mut self, provider: Arc<Provider<Http>>, cache_dir: Option<&str>) -> Self {
        self.state_tracker = Some(StateDiffTracker::new(provider, cache_dir));
        self
    }
    
    /// Process a list of transactions and return alerts for interesting ones
    pub async fn process_transactions(&mut self, transactions: Vec<TransactionView>) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let tx_count = transactions.len();
        
        for tx in transactions {
            if let Some(alert) = self.process_transaction(&tx).await {
                alerts.push(alert);
            }
        }
        
        debug!("Processed {} transactions, generated {} alerts", tx_count, alerts.len());
        alerts
    }
    
    /// Process a single transaction and return an alert if it's interesting
    pub async fn process_transaction(&mut self, tx: &TransactionView) -> Option<Alert> {
        // Skip if no destination
        if tx.to.is_none() {
            trace!("Skipping transaction with no destination");
            return None;
        }
        
        let to_bytes = tx.to.as_ref().unwrap();
        let hash_hex = hex_encode(&tx.hash);
        let from_hex = hex_encode(&tx.from);
        let to_hex = hex_encode(&to_bytes);
        
        // Check for state changes if tracker is enabled
        if let Some(tracker) = &mut self.state_tracker {
            match tracker.simulate_transaction(tx).await {
                Ok(Some(changes)) if !changes.is_empty() => {
                    debug!("Transaction {} produced {} state changes", hash_hex, changes.len());
                    
                    // Calculate total ETH value change
                    let total_eth_change: f64 = changes.values()
                        .map(|change| change.change.abs())
                        .sum();
                    
                    if total_eth_change >= (self.filter.min_value.as_u128() as f64 / 1e18) {
                        info!("Significant state change detected: {} ETH in transaction {}", 
                              total_eth_change, hash_hex);
                        
                        // Store state changes
                        self.cached_state_changes.insert(hash_hex.clone(), changes.clone());
                        
                        // Check for large value transfers (typical scam pattern)
                        for (address, change) in &changes {
                            if change.change <= -0.1 {  // Loss of ETH can indicate scam
                                warn!("Possible suspicious activity: Address {} lost {} ETH in tx {}",
                                      address, -change.change, hash_hex);
                                
                                return Some(Alert {
                                    transaction: tx.clone(),
                                    reason: AlertReason::StateChangeValue {
                                        eth_value: change.change,
                                        address: address.clone(),
                                    },
                                    timestamp: current_timestamp(),
                                });
                            }
                        }
                    }
                },
                Ok(None) => {
                    trace!("No state changes for transaction {}", hash_hex);
                },
                Ok(Some(_)) => {
                    trace!("Minor state changes for transaction {}", hash_hex);
                },
                Err(e) => {
                    warn!("Failed to simulate transaction {}: {}", hash_hex, e);
                }
            }
        }
        
        // Check for high value
        if tx.value >= self.filter.min_value {
            debug!("Found high-value transaction: {} ({} wei)", hash_hex, tx.value);
            return Some(Alert {
                transaction: tx.clone(),
                reason: AlertReason::HighValue,
                timestamp: current_timestamp(),
            });
        }
        
        // Check for watched addresses
        if self.filter.watched_addresses.contains(&from_hex) || 
           self.filter.watched_addresses.contains(&to_hex) {
            debug!("Found transaction with watched address: {}", hash_hex);
            return Some(Alert {
                transaction: tx.clone(),
                reason: AlertReason::WatchedAddress,
                timestamp: current_timestamp(),
            });
        }
        
        // Check for high gas price
        if let Some(min_gas) = self.filter.min_gas_price {
            if let Some(gas_price) = tx.gas_price {
                if gas_price >= min_gas {
                    debug!("Found transaction with high gas price: {} ({} gwei)", 
                          hash_hex, gas_price / U256::from(1_000_000_000u64));
                    return Some(Alert {
                        transaction: tx.clone(),
                        reason: AlertReason::HighGasPrice,
                        timestamp: current_timestamp(),
                    });
                }
            }
        }
        
        None
    }
    
    /// Get cached state changes for a transaction
    pub fn get_state_changes(&self, tx_hash: &str) -> Option<&HashMap<String, MempoolStateDiff>> {
        self.cached_state_changes.get(tx_hash)
    }
    
    /// Clear cached state changes to free memory
    pub fn clear_cached_state_changes(&mut self) {
        self.cached_state_changes.clear();
        if let Some(tracker) = &mut self.state_tracker {
            tracker.clear_recent_changes();
        }
    }
}

/// Helper to get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
} 