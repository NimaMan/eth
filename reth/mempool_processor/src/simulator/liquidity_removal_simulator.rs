use crate::token_tracking::TokenTrackingCache;
use alloy_primitives::{Address, I256, U256};
use eyre::Result;
use reth_chain_query::to_checksum_address;
use reth_chain_query::RethQueryProvider;
/// Liquidity Removal Simulator (minimal)
///
/// Thin wrapper over `tx_processor::process_unsigned_tx` that returns a
/// `LiquidityRemovalResult` by deriving pool-drain metrics from the
/// `ProcessedTransaction.address_balance_changes`.
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;
use tx_processor::tx_processor::data_models::AddressBalanceChange;
use tx_processor::{process_unsigned_tx, ProcessedTransaction};
use tx_simulator::{TxSimulator, UnsignedTransaction};

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
        Self {
            simulator,
            token_cache: None,
        }
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
        let processed = match process_unsigned_tx(
            &self.simulator,
            unsigned_tx.clone(),
            block_number,
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                // If this looks like an insufficient funds error, log sender balance at the block
                let mut debug_info: Option<String> = None;
                let msg = e.to_string();
                let lower = msg.to_lowercase();
                if lower.contains("insufficient") || lower.contains("lack of funds") {
                    let from = unsigned_tx.from.unwrap_or_default();
                    let sim_block = block_number
                        .unwrap_or_else(|| self.simulator.get_latest_block().unwrap_or(0));
                    if sim_block > 0 {
                        if let Ok(rqp) = RethQueryProvider::with_simulator(self.simulator.clone()) {
                            // Gather diagnostics
                            let mut lines: Vec<String> = Vec::new();
                            let from_cs = to_checksum_address(&from);

                            if let Ok(balance) = rqp.get_eth_balance(from, Some(sim_block)).await {
                                let bal_eth = (balance.to::<u128>() as f64) / 1e18;
                                lines.push(format!(
                                    "sender {} balance at block {} = {:.6} ETH",
                                    from_cs, sim_block, bal_eth
                                ));
                            }

                            // Block gas metadata and base fees at sim_block and next block
                            if let Ok((gas_limit_hdr, _gas_used_hdr, base_fee_opt)) =
                                rqp.get_block_gas_metadata(sim_block)
                            {
                                if let Some(base_fee) = base_fee_opt {
                                    let base_gwei = (base_fee as f64) / 1e9;
                                    lines.push(format!(
                                        "base fee @{} = {:.9} gwei, header_gas_limit = {}",
                                        sim_block, base_gwei, gas_limit_hdr
                                    ));
                                    if let Ok(prev_bf) =
                                        rqp.get_base_fee_at_block(sim_block.saturating_sub(1))
                                    {
                                        let prev_gwei = (prev_bf as f64) / 1e9;
                                        lines.push(format!(
                                            "base fee @{} = {:.9} gwei",
                                            sim_block.saturating_sub(1),
                                            prev_gwei
                                        ));
                                    }
                                }

                                // Compute required upfront cost using the params we will pass
                                let gas_for_check: U256 = if let Some(g) = unsigned_tx.gas {
                                    U256::from(g)
                                } else {
                                    U256::from(gas_limit_hdr)
                                };
                                let value: U256 = unsigned_tx.value.unwrap_or(U256::ZERO);

                                // Determine fee model
                                if let Some(max_fee) = unsigned_tx.max_fee_per_gas {
                                    let required = value.saturating_add(
                                        gas_for_check.saturating_mul(U256::from(max_fee)),
                                    );
                                    let req_eth = (required.to::<u128>() as f64) / 1e18;
                                    let tip = unsigned_tx.max_priority_fee_per_gas.unwrap_or(0);
                                    lines.push(format!("EIP-1559 caps: max_fee={} gwei, tip={} gwei, gas={} (fallback used if None)", (max_fee as f64)/1e9, (tip as f64)/1e9, gas_for_check.to_string()));
                                    lines.push(format!(
                                        "upfront requirement ~= value + gas*max_fee = {:.9} ETH",
                                        req_eth
                                    ));
                                } else if let Some(gp) = unsigned_tx.gas_price {
                                    let required = value.saturating_add(
                                        gas_for_check.saturating_mul(U256::from(gp)),
                                    );
                                    let req_eth = (required.to::<u128>() as f64) / 1e18;
                                    lines.push(format!(
                                        "legacy gas_price={} gwei, gas={} (fallback used if None)",
                                        (gp as f64) / 1e9,
                                        gas_for_check.to_string()
                                    ));
                                    lines.push(format!(
                                        "upfront requirement ~= value + gas*gas_price = {:.9} ETH",
                                        req_eth
                                    ));
                                } else {
                                    lines.push("no fee fields on unsigned tx; defaults may inflate upfront requirement".to_string());
                                }
                            }

                            let line = lines.join(" | ");
                            tracing::warn!(target: "sim", "insufficient funds diagnostics: {}", line);
                            debug_info = Some(line);
                        }
                    }
                }
                error!("Liquidity removal processing failed: {}", msg);
                return Ok(LiquidityRemovalResult {
                    success: false,
                    revert_reason: Some(msg),
                    address_balance_changes: HashMap::new(),
                    pool_address: None,
                    eth_removed: 0.0,
                    drain_percentage: 0.0,
                    remaining_eth: 0.0,
                    is_scam: false,
                    debug_info,
                });
            }
        };

        // 2) Convert balance deltas to canonical map
        let address_balance_changes = Self::convert_processed_changes(&processed);

        // 3) Compute drain from deltas (deterministic, cache-backed pools only)
        let drain = self.compute_pool_drain(&address_balance_changes).await;

        let (pool_address, eth_removed, drain_percentage, remaining_eth) = if let Some(d) = drain {
            (
                Some(d.pool_address),
                d.eth_removed,
                d.percentage,
                d.remaining_eth,
            )
        } else {
            (None, 0.0, 0.0, 0.0)
        };

        let is_scam = pool_address.is_some() && (drain_percentage > 60.0 || remaining_eth < 0.3);
        let success = processed.status == "1";
        let revert_reason = if success {
            None
        } else {
            Some("Reverted in simulation".to_string())
        };

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

    fn convert_processed_changes(
        processed: &ProcessedTransaction,
    ) -> HashMap<Address, AddressBalanceChange> {
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
            let mut eth_change = change
                .currency_net
                .get("ETH")
                .and_then(|u| I256::try_from(*u).ok())
                .map(|s| s.to_string().parse::<f64>().unwrap_or(0.0) / 1e18)
                .unwrap_or(0.0);
            if eth_change >= -0.01 {
                continue;
            } // ignore tiny or positive

            // Only consider addresses that are tracked pools in the token cache
            if let Some(ref cache) = self.token_cache {
                let addr_str = to_checksum_address(address);
                if let Some(pool_state) = cache.get_pool(&addr_str).await {
                    let initial = pool_state.eth_reserve;
                    if initial > 0.0 {
                        let removed = eth_change.abs();
                        let remaining = (initial + eth_change).max(0.0);
                        let pct = ((initial - remaining) / initial * 100.0).min(100.0);
                        let candidate = PoolDrainInfo {
                            pool_address: *address,
                            initial_eth: initial,
                            eth_removed: removed,
                            remaining_eth: remaining,
                            percentage: pct,
                        };
                        // Keep the one with largest removal
                        if let Some(cur) = &best {
                            if candidate.eth_removed > cur.eth_removed {
                                best = Some(candidate);
                            }
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
