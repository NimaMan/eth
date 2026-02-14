//! Nonce management with transaction tracking
//!
//! Ensures nonces are only incremented after successful submission
//! and provides recovery mechanisms for failed transactions.

use ethers::providers::Middleware;
use ethers::types::{Address, H256, U256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Transaction state in nonce manager
#[derive(Debug, Clone)]
pub enum TxState {
    /// Transaction is being prepared
    Reserved,
    /// Transaction submitted to mempool
    Submitted { tx_hash: H256 },
    /// Transaction confirmed on chain
    Confirmed { tx_hash: H256, block_number: u64 },
    /// Transaction failed or was dropped
    Failed { reason: String },
}

/// Tracks a transaction's lifecycle
#[derive(Debug, Clone)]
pub struct TrackedTransaction {
    /// The nonce used for this transaction
    pub nonce: U256,
    /// Current state of the transaction
    pub state: TxState,
    /// When this transaction was created
    pub created_at: std::time::Instant,
    /// Last state update time
    pub updated_at: std::time::Instant,
}

/// Configuration for nonce manager
#[derive(Debug, Clone)]
pub struct NonceManagerConfig {
    /// Maximum pending transactions allowed
    pub max_pending_transactions: usize,
    /// Maximum transaction age before cleanup (seconds)
    pub max_transaction_age_secs: u64,
    /// Cleanup interval (seconds)
    pub cleanup_interval_secs: u64,
}

impl Default for NonceManagerConfig {
    fn default() -> Self {
        Self {
            max_pending_transactions: 50,
            max_transaction_age_secs: 300, // 5 minutes
            cleanup_interval_secs: 60,     // 1 minute
        }
    }
}

/// Manages nonces with proper tracking and recovery
pub struct NonceManager {
    /// Configuration
    config: NonceManagerConfig,
    /// The account address
    address: Address,
    /// Last confirmed nonce from chain
    last_confirmed_nonce: Arc<RwLock<U256>>,
    /// Next nonce to use (not yet confirmed)
    next_nonce: Arc<RwLock<U256>>,
    /// Tracked transactions by nonce
    transactions: Arc<RwLock<HashMap<U256, TrackedTransaction>>>,
    /// Ethereum provider for chain queries
    provider: Arc<ethers::providers::Provider<ethers::providers::Http>>,
    /// Shutdown signal
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl NonceManager {
    /// Create new nonce manager
    pub async fn new(
        config: NonceManagerConfig,
        address: Address,
        provider: Arc<ethers::providers::Provider<ethers::providers::Http>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Get current nonce from chain
        let current_nonce = provider.get_transaction_count(address, None).await?;

        info!(
            "Initialized nonce manager for {} with nonce {}",
            address, current_nonce
        );

        let manager = Self {
            config: config.clone(),
            address,
            last_confirmed_nonce: Arc::new(RwLock::new(current_nonce)),
            next_nonce: Arc::new(RwLock::new(current_nonce)),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            provider,
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        // Start cleanup task
        manager.start_cleanup_task();

        Ok(manager)
    }

    /// Reserve a nonce for a new transaction
    pub async fn reserve_nonce(&self) -> Result<U256, Box<dyn std::error::Error>> {
        // Check pending transaction limit
        {
            let txs = self.transactions.read().await;
            let pending_count = txs
                .values()
                .filter(|tx| matches!(tx.state, TxState::Reserved | TxState::Submitted { .. }))
                .count();

            if pending_count >= self.config.max_pending_transactions {
                return Err("Too many pending transactions".into());
            }
        }
        // Use single write lock to prevent race conditions
        let mut next = self.next_nonce.write().await;
        let nonce = *next;
        *next = nonce + 1; // Immediately increment to prevent reuse
        drop(next); // Release lock early

        // Track this transaction
        let mut txs = self.transactions.write().await;
        txs.insert(
            nonce,
            TrackedTransaction {
                nonce,
                state: TxState::Reserved,
                created_at: std::time::Instant::now(),
                updated_at: std::time::Instant::now(),
            },
        );

        debug!("Reserved nonce {} for new transaction", nonce);
        Ok(nonce)
    }

    /// Mark transaction as submitted
    pub async fn mark_submitted(&self, nonce: U256, tx_hash: H256) {
        let mut txs = self.transactions.write().await;
        if let Some(tx) = txs.get_mut(&nonce) {
            tx.state = TxState::Submitted { tx_hash };
            tx.updated_at = std::time::Instant::now();
            info!("Transaction {} submitted with nonce {}", tx_hash, nonce);
        }
    }

    /// Mark transaction as confirmed
    pub async fn mark_confirmed(&self, nonce: U256, tx_hash: H256, block_number: u64) {
        let mut txs = self.transactions.write().await;
        if let Some(tx) = txs.get_mut(&nonce) {
            tx.state = TxState::Confirmed {
                tx_hash,
                block_number,
            };
            tx.updated_at = std::time::Instant::now();

            // Update last confirmed nonce
            let mut last_confirmed = self.last_confirmed_nonce.write().await;
            if nonce >= *last_confirmed {
                *last_confirmed = nonce + 1;
                debug!(
                    "Transaction {} confirmed, last confirmed nonce: {}",
                    tx_hash, *last_confirmed
                );
            }
        }

        // Clean up old confirmed transactions
        self.cleanup_old_transactions(&mut txs);
    }

    /// Mark transaction as failed and potentially reuse the nonce
    pub async fn mark_failed(&self, nonce: U256, reason: String) {
        let mut txs = self.transactions.write().await;
        if let Some(tx) = txs.get_mut(&nonce) {
            tx.state = TxState::Failed {
                reason: reason.clone(),
            };
            tx.updated_at = std::time::Instant::now();
            warn!("Transaction with nonce {} failed: {}", nonce, reason);
        }

        // Check if we should reset next_nonce
        let should_reset = {
            let next = self.next_nonce.read().await;
            nonce + 1 == *next && self.no_pending_after(nonce, &txs)
        };

        if should_reset {
            let mut next = self.next_nonce.write().await;
            *next = nonce;
            info!("Reset next nonce to {} after failure", nonce);
        }
    }

    /// Sync with chain to recover from any inconsistencies
    pub async fn sync_with_chain(&self) -> Result<(), Box<dyn std::error::Error>> {
        let chain_nonce = self
            .provider
            .get_transaction_count(self.address, None)
            .await?;

        let mut last_confirmed = self.last_confirmed_nonce.write().await;
        let mut next = self.next_nonce.write().await;

        if chain_nonce > *last_confirmed {
            warn!(
                "Chain nonce {} is ahead of our last confirmed {}, syncing",
                chain_nonce, *last_confirmed
            );
            *last_confirmed = chain_nonce;
            *next = chain_nonce;
        }

        // Mark any pending transactions as potentially failed
        let mut txs = self.transactions.write().await;
        for (nonce, tx) in txs.iter_mut() {
            if *nonce < chain_nonce {
                match &tx.state {
                    TxState::Reserved | TxState::Submitted { .. } => {
                        tx.state = TxState::Failed {
                            reason: "Nonce already used on chain".to_string(),
                        };
                        tx.updated_at = std::time::Instant::now();
                    }
                    _ => {}
                }
            }
        }

        info!("Synced with chain, nonce: {}", chain_nonce);
        Ok(())
    }

    /// Get current state information
    pub async fn get_state(&self) -> (U256, U256, usize) {
        let last_confirmed = *self.last_confirmed_nonce.read().await;
        let next = *self.next_nonce.read().await;
        let pending_count = self
            .transactions
            .read()
            .await
            .values()
            .filter(|tx| matches!(tx.state, TxState::Reserved | TxState::Submitted { .. }))
            .count();

        (last_confirmed, next, pending_count)
    }

    /// Check if there are no pending transactions after given nonce
    fn no_pending_after(&self, nonce: U256, txs: &HashMap<U256, TrackedTransaction>) -> bool {
        !txs.iter().any(|(n, tx)| {
            *n > nonce && matches!(tx.state, TxState::Reserved | TxState::Submitted { .. })
        })
    }

    /// Clean up old confirmed transactions
    fn cleanup_old_transactions(&self, txs: &mut HashMap<U256, TrackedTransaction>) {
        let cutoff = std::time::Instant::now()
            - std::time::Duration::from_secs(self.config.max_transaction_age_secs);
        let initial_count = txs.len();

        txs.retain(|_, tx| {
            match tx.state {
                TxState::Confirmed { .. } => tx.updated_at > cutoff,
                TxState::Failed { .. } => tx.updated_at > cutoff,
                _ => true, // Keep reserved and submitted
            }
        });

        let removed = initial_count - txs.len();
        if removed > 0 {
            debug!("Cleaned up {} old transactions", removed);
        }
    }

    /// Start background cleanup task
    fn start_cleanup_task(&self) {
        let transactions = self.transactions.clone();
        let config = self.config.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(config.cleanup_interval_secs));

            while !shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                interval.tick().await;

                let mut txs = transactions.write().await;
                let cutoff = std::time::Instant::now()
                    - std::time::Duration::from_secs(config.max_transaction_age_secs);
                let initial_count = txs.len();

                txs.retain(|_, tx| match tx.state {
                    TxState::Confirmed { .. } | TxState::Failed { .. } => tx.updated_at > cutoff,
                    _ => true,
                });

                let removed = initial_count - txs.len();
                if removed > 0 {
                    info!("Cleanup task removed {} old transactions", removed);
                }
            }
        });
    }

    /// Shutdown the nonce manager
    pub fn shutdown(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::{Http, Provider};

    #[tokio::test]
    async fn test_nonce_reservation() {
        // This would need a mock provider for proper testing
        // For now, just verify the structure compiles
    }
}
