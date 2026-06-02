use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::utils::append_with_history_limit;

use super::base::{meaningful_liquidity_threshold, meaningful_token_reserve_for_ratio, BasePool};

pub const SCAM_DIRECT_LP_LIQUIDITY_REMOVAL: &str = "direct_lp_liquidity_removal";
pub const SCAM_PAIR_BALANCE_BACKDOOR_DRAIN: &str = "pair_balance_backdoor_drain";
/// Control/creator logic that drains a *holder's* token balance (e.g. our
/// trading vault) via `transferFrom(holder, dead, amount)` with zero allowance
/// and no normal Transfer log. Distinct from the pair-balance variant: the pool
/// reserves do not move, so this is set from token-control evidence, not from
/// reserve-drain inference.
pub const SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN: &str = "holder_balance_backdoor_drain";
/// Systematic rug built on the holder-balance backdoor: the operator confiscates
/// the bulk (>=95%) of MULTIPLE buyers' balances via event-less
/// `transferFrom(holder, dead, amount)`, then eventually empties the pool (the
/// liquidity pull is the cash-out). Distinct from a single
/// `holder_balance_backdoor_drain` finding: this is the aggregate pattern where
/// independently-bought positions are each wiped, so buyers are left holding
/// nothing while `can_sell` may still read true. We classify on the mass
/// confiscation (early) — before the pool drain — so a live position can exit on
/// the confiscation signal rather than waiting for the liquidity removal.
pub const SCAM_CUSTODY_BUYER_TOKEN_CONFISCATION: &str = "custody_buyer_token_confiscation";
pub const SCAM_PRIVILEGED_SELLER_RESERVE_DRAIN: &str = "privileged_seller_reserve_drain";
pub const SCAM_RESERVE_DUMP_DRAIN: &str = "reserve_dump_drain";
pub const SCAM_UNKNOWN_RESERVE_DRAIN: &str = "unknown_reserve_drain";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolScamMechanism {
    pub mechanism: String,
    pub label: String,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: Value,
}

#[derive(Clone, Debug)]
struct ReserveDrainEvidence {
    block_number: Option<u64>,
    tx_hash: Option<String>,
    denom_reserve: f64,
    token_reserve: f64,
    max_denom_reserve_before: Option<f64>,
    denom_reserve_ratio_to_max: Option<f64>,
    source: &'static str,
}

impl BasePool {
    pub fn mark_scam_mechanism(
        &mut self,
        mechanism: impl Into<String>,
        block_number: Option<u64>,
        tx_hash: Option<String>,
        evidence: Value,
    ) {
        let mechanism = mechanism.into();
        let label = scam_mechanism_label(&mechanism).to_string();
        self.scam_mechanism = Some(mechanism);
        self.scam_mechanism_label = Some(label.clone());
        self.scam_mechanism_evidence = Some(evidence);
        self.scam_label = Some(label);
        if let Some(block_number) = block_number {
            self.scam_block = Some(block_number);
            self.reserve_tracker.scam_block = Some(block_number);
        }
        if let Some(tx_hash) = tx_hash {
            self.scam_tx_hash = Some(tx_hash.clone());
            self.reserve_tracker.scam_tx_hash = Some(tx_hash);
        }
    }

    pub fn record_pair_token_transfer(
        &mut self,
        from_address: impl AsRef<str>,
        to_address: impl AsRef<str>,
        amount: f64,
        block_number: u64,
        timestamp: u64,
        tx_hash: impl Into<String>,
        log_index: Option<u64>,
        has_pool_event: bool,
    ) {
        if !amount.is_finite() || amount <= 0.0 {
            return;
        }
        let from_address = normalize_address_string(from_address);
        let to_address = normalize_address_string(to_address);
        if from_address != self.identity.pool_address || has_pool_event {
            return;
        }

        let token_reserve_before = self.token_reserve();
        let pooled_token_share = meaningful_token_reserve_for_ratio(token_reserve_before)
            .map(|reserve| amount / reserve);
        append_with_history_limit(
            &mut self.suspicious_pair_token_out_transfers,
            json!({
                "from_address": from_address,
                "to_address": to_address,
                "amount": amount,
                "block_number": block_number,
                "timestamp": timestamp,
                "tx_hash": tx_hash.into(),
                "log_index": log_index,
                "has_pool_event": has_pool_event,
                "token_reserve_before": token_reserve_before,
                "pooled_token_share": pooled_token_share,
            }),
            self.config.history_limit,
        );
    }

    pub fn inferred_scam_mechanism(&self) -> Option<PoolScamMechanism> {
        if let (Some(mechanism), Some(label)) = (&self.scam_mechanism, &self.scam_mechanism_label) {
            return Some(PoolScamMechanism {
                mechanism: mechanism.clone(),
                label: label.clone(),
                block_number: self.scam_block,
                tx_hash: self.scam_tx_hash.clone(),
                evidence: self
                    .scam_mechanism_evidence
                    .clone()
                    .unwrap_or_else(|| json!({})),
            });
        }

        let drain = self.reserve_drain_evidence()?;
        let matching_burn = self.matching_event(&self.burn_events, &drain);
        let matching_swap = self.matching_event(&self.swap_events, &drain);
        let suspicious_pair_out = self.matching_suspicious_pair_out_transfer(&drain);

        let mechanism = if matching_burn.is_some() {
            SCAM_DIRECT_LP_LIQUIDITY_REMOVAL
        } else if suspicious_pair_out.is_some() {
            SCAM_PAIR_BALANCE_BACKDOOR_DRAIN
        } else if matching_swap.is_some() && self.has_sell_blocked_evidence() {
            SCAM_PRIVILEGED_SELLER_RESERVE_DRAIN
        } else if matching_swap.is_some() {
            SCAM_RESERVE_DUMP_DRAIN
        } else {
            SCAM_UNKNOWN_RESERVE_DRAIN
        };

        Some(PoolScamMechanism {
            mechanism: mechanism.to_string(),
            label: scam_mechanism_label(mechanism).to_string(),
            block_number: drain.block_number,
            tx_hash: drain.tx_hash.clone(),
            evidence: json!({
                "drain": {
                    "block_number": drain.block_number,
                    "tx_hash": drain.tx_hash,
                    "denom_reserve": drain.denom_reserve,
                    "token_reserve": drain.token_reserve,
                    "max_denom_reserve_before": drain.max_denom_reserve_before,
                    "denom_reserve_ratio_to_max": drain.denom_reserve_ratio_to_max,
                    "source": drain.source,
                },
                "burn_event": matching_burn.cloned(),
                "swap_event": matching_swap.cloned(),
                "suspicious_pair_token_out_transfer": suspicious_pair_out.cloned(),
                "last_trading_failure_reason": self.last_trading_failure_reason,
                "last_trading_failure_class": self.last_trading_failure_class,
            }),
        })
    }

    fn reserve_drain_evidence(&self) -> Option<ReserveDrainEvidence> {
        if self.reserve_tracker.is_scam {
            return Some(ReserveDrainEvidence {
                block_number: self.reserve_tracker.scam_block,
                tx_hash: self.reserve_tracker.scam_tx_hash.clone(),
                denom_reserve: self.state.denom_reserve,
                token_reserve: self.state.token_reserve,
                max_denom_reserve_before: None,
                denom_reserve_ratio_to_max: None,
                source: "reserve_tracker",
            });
        }

        let mut max_denom_reserve = 0.0_f64;
        let meaningful_threshold = self
            .config
            .denom_threshold
            .max(meaningful_liquidity_threshold(&self.identity.denom_address));
        let max_observed_denom_reserve = self
            .reserve_tracker
            .reserve_history
            .iter()
            .map(|snapshot| snapshot.denom_reserve)
            .filter(|value| value.is_finite() && *value > 0.0)
            .fold(0.0_f64, f64::max);
        let latest = self
            .reserve_tracker
            .latest_snapshot
            .as_ref()
            .or_else(|| self.reserve_tracker.reserve_history.last())?;
        if max_observed_denom_reserve <= meaningful_threshold {
            return None;
        }
        let latest_ratio = latest.denom_reserve / max_observed_denom_reserve;
        let current_is_collapsed = latest_ratio.is_finite() && latest_ratio <= 0.10;
        let current_is_below_threshold = meaningful_threshold > 0.0
            && latest.denom_reserve.is_finite()
            && latest.denom_reserve < meaningful_threshold;
        if !current_is_collapsed && !current_is_below_threshold {
            return None;
        }

        for snapshot in &self.reserve_tracker.reserve_history {
            if snapshot.denom_reserve > max_denom_reserve {
                max_denom_reserve = snapshot.denom_reserve;
                continue;
            }
            if !max_denom_reserve.is_finite() || max_denom_reserve <= meaningful_threshold {
                continue;
            }
            let ratio = snapshot.denom_reserve / max_denom_reserve;
            let ratio_collapse = ratio.is_finite() && ratio <= 0.10;
            let below_threshold = meaningful_threshold > 0.0
                && snapshot.denom_reserve.is_finite()
                && snapshot.denom_reserve < meaningful_threshold;
            if ratio_collapse || below_threshold {
                return Some(ReserveDrainEvidence {
                    block_number: Some(snapshot.block_number),
                    tx_hash: Some(snapshot.tx_hash.clone()),
                    denom_reserve: snapshot.denom_reserve,
                    token_reserve: snapshot.token_reserve,
                    max_denom_reserve_before: Some(max_denom_reserve),
                    denom_reserve_ratio_to_max: Some(ratio),
                    source: "reserve_history",
                });
            }
        }

        None
    }

    fn matching_event<'a>(
        &self,
        events: &'a [Value],
        drain: &ReserveDrainEvidence,
    ) -> Option<&'a Value> {
        events
            .iter()
            .rev()
            .find(|event| event_matches_drain(event, drain))
    }

    fn matching_suspicious_pair_out_transfer(
        &self,
        drain: &ReserveDrainEvidence,
    ) -> Option<&Value> {
        let drain_block = drain.block_number?;
        self.suspicious_pair_token_out_transfers
            .iter()
            .rev()
            .find(|event| {
                value_u64(event, "block_number").is_some_and(|transfer_block| {
                    transfer_block <= drain_block && drain_block.saturating_sub(transfer_block) <= 1
                })
            })
    }

    fn has_sell_blocked_evidence(&self) -> bool {
        let mut text = String::new();
        if let Some(reason) = &self.last_trading_failure_reason {
            text.push_str(reason);
            text.push(' ');
        }
        if let Some(class) = &self.last_trading_failure_class {
            text.push_str(class);
        }
        let normalized = text.to_ascii_lowercase();
        normalized.contains("transfer_from_failed")
            || normalized.contains("cannot sell")
            || (normalized.contains("sell") && normalized.contains("failed"))
    }
}

pub fn scam_mechanism_label(mechanism: &str) -> &'static str {
    match mechanism {
        SCAM_DIRECT_LP_LIQUIDITY_REMOVAL => "Direct LP Liquidity Removal",
        SCAM_PAIR_BALANCE_BACKDOOR_DRAIN => "Backdoored Pair-Balance Drain",
        SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN => "Backdoored Holder-Balance Drain",
        SCAM_CUSTODY_BUYER_TOKEN_CONFISCATION => "Backdoor Buyer-Token Confiscation Rug",
        SCAM_PRIVILEGED_SELLER_RESERVE_DRAIN => "Privileged Seller Reserve Drain",
        SCAM_RESERVE_DUMP_DRAIN => "Reserve Dump / External Holder Drain",
        SCAM_UNKNOWN_RESERVE_DRAIN => "Unknown Reserve Drain",
        _ => "Unknown Scam Mechanism",
    }
}

fn event_matches_drain(event: &Value, drain: &ReserveDrainEvidence) -> bool {
    if let Some(tx_hash) = drain.tx_hash.as_deref() {
        if value_str(event, "tx_hash")
            .is_some_and(|event_tx| event_tx.eq_ignore_ascii_case(tx_hash))
        {
            return true;
        }
    }
    if let Some(block_number) = drain.block_number {
        return value_u64(event, "block") == Some(block_number)
            || value_u64(event, "block_number") == Some(block_number);
    }
    false
}

fn normalize_address_string(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn value_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn value_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::pools::base::{BasePoolConfig, PoolIdentity};

    const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

    fn test_pool() -> BasePool {
        BasePool::new(
            PoolIdentity::new("0xPOOL", "0xTOKEN", "0xDENOM", "UNISWAP-V2"),
            BasePoolConfig {
                denom_threshold: 0.05,
                threshold_unit: Some("ETH".to_string()),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
        )
    }

    fn weth_pool() -> BasePool {
        BasePool::new(
            PoolIdentity::new("0xPOOL", "0xTOKEN", WETH_ADDRESS, "UNISWAP-V2"),
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                history_limit: 10,
                denom_threshold: 0.0,
                threshold_unit: None,
                ..BasePoolConfig::new(18)
            },
        )
    }

    #[test]
    fn labels_direct_lp_burn_removal() {
        let mut pool = test_pool();
        pool.update_reserves(100.0, 1.0, 10, 1_700, "0xSYNC");
        pool.update_reserves(1_000.0, 0.01, 11, 1_712, "0xBURN");
        pool.burn_events
            .push(json!({"block": 11, "tx_hash": "0xBURN"}));

        let mechanism = pool.inferred_scam_mechanism().unwrap();

        assert_eq!(mechanism.mechanism, SCAM_DIRECT_LP_LIQUIDITY_REMOVAL);
        assert_eq!(mechanism.label, "Direct LP Liquidity Removal");
    }

    #[test]
    fn labels_backdoored_pair_balance_drain() {
        let mut pool = test_pool();
        pool.update_reserves(1_000.0, 1.0, 10, 1_700, "0xSYNC");
        pool.record_pair_token_transfer(
            "0xPOOL", "0xACTOR", 600.0, 11, 1_710, "0xXFER", None, false,
        );
        pool.update_reserves(400.0, 0.01, 11, 1_712, "0xSWAP");
        pool.swap_events
            .push(json!({"block": 11, "tx_hash": "0xSWAP", "is_sell": true}));

        let mechanism = pool.inferred_scam_mechanism().unwrap();

        assert_eq!(mechanism.mechanism, SCAM_PAIR_BALANCE_BACKDOOR_DRAIN);
        assert_eq!(mechanism.label, "Backdoored Pair-Balance Drain");
    }

    #[test]
    fn pair_token_transfer_share_ignores_dust_reserve_denominator() {
        let mut pool = test_pool();
        pool.update_reserves(5.6e-17, 1.0, 10, 1_700, "0xDUST");

        pool.record_pair_token_transfer("0xPOOL", "0xACTOR", 1.0, 11, 1_710, "0xXFER", None, false);

        let transfer = pool.suspicious_pair_token_out_transfers.last().unwrap();
        assert!(transfer.get("pooled_token_share").unwrap().is_null());
    }

    #[test]
    fn labels_privileged_seller_drain_when_retail_sell_was_blocked() {
        let mut pool = test_pool();
        pool.set_trading_failure_context(
            Some(
                "Simulation failed at step SELL: TransferHelper: TRANSFER_FROM_FAILED".to_string(),
            ),
            Some("sell_failed".to_string()),
        );
        pool.update_reserves(1_000.0, 1.0, 10, 1_700, "0xSYNC");
        pool.update_reserves(2_000.0, 0.01, 11, 1_712, "0xSWAP");
        pool.swap_events
            .push(json!({"block": 11, "tx_hash": "0xSWAP", "is_sell": true}));

        let mechanism = pool.inferred_scam_mechanism().unwrap();

        assert_eq!(mechanism.mechanism, SCAM_PRIVILEGED_SELLER_RESERVE_DRAIN);
        assert_eq!(mechanism.label, "Privileged Seller Reserve Drain");
    }

    #[test]
    fn labels_generic_reserve_dump_without_blocked_sell_context() {
        let mut pool = test_pool();
        pool.update_reserves(1_000.0, 1.0, 10, 1_700, "0xSYNC");
        pool.update_reserves(2_000.0, 0.01, 11, 1_712, "0xSWAP");
        pool.swap_events
            .push(json!({"block": 11, "tx_hash": "0xSWAP", "is_sell": true}));

        let mechanism = pool.inferred_scam_mechanism().unwrap();

        assert_eq!(mechanism.mechanism, SCAM_RESERVE_DUMP_DRAIN);
        assert_eq!(mechanism.label, "Reserve Dump / External Holder Drain");
    }

    #[test]
    fn can_be_inferred_from_reserve_history_without_explicit_threshold() {
        let mut pool = weth_pool();
        pool.update_reserves(1_000.0, 1.0, 10, 1_700, "0xSYNC");
        pool.update_reserves(2_000.0, 0.001, 11, 1_712, "0xSWAP");
        pool.swap_events
            .push(json!({"block": 11, "tx_hash": "0xSWAP", "is_sell": true}));

        let mechanism = pool.inferred_scam_mechanism().unwrap();

        assert!(!pool.has_liquidity_removal());
        assert_eq!(mechanism.mechanism, SCAM_RESERVE_DUMP_DRAIN);
    }

    #[test]
    fn ignores_recovered_temporary_reserve_drop() {
        let mut pool = weth_pool();
        pool.update_reserves(1_000.0, 1.0, 10, 1_700, "0xSYNC");
        pool.update_reserves(2_000.0, 0.001, 11, 1_712, "0xSWAP");
        pool.swap_events
            .push(json!({"block": 11, "tx_hash": "0xSWAP", "is_sell": true}));
        pool.update_reserves(1_000.0, 1.2, 12, 1_724, "0xREFILL");

        assert!(pool.inferred_scam_mechanism().is_none());
    }
}
