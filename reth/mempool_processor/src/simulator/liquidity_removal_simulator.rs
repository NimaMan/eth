/// Liquidity Removal Simulator (minimal)
///
/// Thin wrapper over `tx_processor::process_unsigned_tx` that returns a
/// `LiquidityRemovalResult` by deriving pool-drain metrics from the
/// `ProcessedTransaction.address_balance_changes`.

use std::collections::HashMap;
use std::sync::Arc;
use alloy_primitives::{Address, I256};
use eyre::Result;
use reth_chain_query::to_checksum_address;
use tracing::error;
use tx_processor::{process_unsigned_tx, ProcessedTransaction};
use tx_processor::tx_processor::data_models::AddressBalanceChange;
use tx_simulator::{TxSimulator, UnsignedTransaction};
use crate::token_tracking::TokenTrackingCache;


#[derive(Debug, Clone)]
pub struct LiquidityRemovalResult {
    pub success: bool,
    pub revert_reason: Option<String>,
    pub address_balance_changes: HashMap<Address, AddressBalanceChange>,
    pub pool_address: Option<Address>,
    pub eth_removed: f64,
    pub drain_percentage: f64,
    pub remaining_eth: f64,
    pub is_scam: bool,
    pub debug_info: Option<String>,
}

#[derive(Debug)]
struct PoolDrainInfo {
    pool_address: Address,
    initial_eth: f64,
    eth_removed: f64,
    remaining_eth: f64,
    percentage: f64,
}

pub struct LiquidityRemovalSimulator {
    simulator: Arc<TxSimulator>,
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl LiquidityRemovalSimulator {
    pub fn new(simulator: Arc<TxSimulator>) -> Self {
        Self { simulator, token_cache: None }
    }

    pub fn set_token_cache(&mut self, cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(cache);
    }

    /// Build a ProcessedTransaction for the unsigned tx and compute drain metrics.
    /// Only logs errors; otherwise stays quiet.
    pub async fn simulate_removal(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
    ) -> Result<LiquidityRemovalResult> {
        // 1) Process tx with full trace + deltas
        let processed = match process_unsigned_tx(&self.simulator, unsigned_tx.clone(), block_number).await {
            Ok(p) => p,
            Err(e) => {
                error!("Liquidity removal processing failed: {}", e);
                return Ok(LiquidityRemovalResult {
                    success: false,
                    revert_reason: Some(e.to_string()),
                    address_balance_changes: HashMap::new(),
                    pool_address: None,
                    eth_removed: 0.0,
                    drain_percentage: 0.0,
                    remaining_eth: 0.0,
                    is_scam: false,
                    debug_info: None,
                });
            }
        };

        // 2) Convert balance deltas to canonical map
        let address_balance_changes = Self::convert_processed_changes(&processed);

        // 3) Compute drain from deltas (deterministic, cache-backed pools only)
        let drain = self.compute_pool_drain(&address_balance_changes).await;

        let (pool_address, eth_removed, drain_percentage, remaining_eth) = if let Some(d) = drain {
            (Some(d.pool_address), d.eth_removed, d.percentage, d.remaining_eth)
        } else {
            (None, 0.0, 0.0, 0.0)
        };

        let is_scam = pool_address.is_some() && (drain_percentage > 60.0 || remaining_eth < 0.3);
        let success = processed.status == "1";
        let revert_reason = if success { None } else { Some("Reverted in simulation".to_string()) };

        Ok(LiquidityRemovalResult {
            success,
            revert_reason,
            address_balance_changes,
            pool_address,
            eth_removed,
            drain_percentage,
            remaining_eth,
            is_scam,
            debug_info: None,
        })
    }

    fn convert_processed_changes(processed: &ProcessedTransaction) -> HashMap<Address, AddressBalanceChange> {
        processed.address_balance_changes.clone()
    }

    async fn compute_pool_drain(
        &self,
        address_balance_changes: &HashMap<Address, AddressBalanceChange>,
    ) -> Option<PoolDrainInfo> {
        // Track the best (most negative) ETH delta among known pools
        let mut best: Option<PoolDrainInfo> = None;

        for (address, change) in address_balance_changes {
            // ETH delta in ETH units (negative for drains)
            let eth_change = change
                .currency_net
                .get("ETH")
                .and_then(|u| I256::try_from(*u).ok())
                .map(|s| s.to_string().parse::<f64>().unwrap_or(0.0) / 1e18)
                .unwrap_or(0.0);
            if eth_change >= -0.01 { continue; } // ignore tiny or positive

            // Only consider addresses that are tracked pools in the token cache
            if let Some(ref cache) = self.token_cache {
                let addr_str = to_checksum_address(address);
                if let Some(pool_state) = cache.get_pool(&addr_str).await {
                    let initial = pool_state.eth_reserve;
                    if initial > 0.0 {
                        let removed = eth_change.abs();
                        let remaining = (initial + eth_change).max(0.0);
                        let pct = ((initial - remaining) / initial * 100.0).min(100.0);
                        let candidate = PoolDrainInfo { pool_address: *address, initial_eth: initial, eth_removed: removed, remaining_eth: remaining, percentage: pct };
                        // Keep the one with largest removal
                        if let Some(cur) = &best {
                            if candidate.eth_removed > cur.eth_removed { best = Some(candidate); }
                        } else {
                            best = Some(candidate);
                        }
                    }
                }
            }
        }
        best
    }
}
