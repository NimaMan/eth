use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::custody::{CustodyFinding, CustodyState};

use super::base::BasePool;
use super::scam_mechanism::{
    SCAM_CUSTODY_BUYER_TOKEN_CONFISCATION, SCAM_DIRECT_LP_LIQUIDITY_REMOVAL,
    SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN, SCAM_PAIR_BALANCE_BACKDOOR_DRAIN,
    SCAM_PRIVILEGED_SELLER_RESERVE_DRAIN, SCAM_RESERVE_DUMP_DRAIN, SCAM_UNKNOWN_RESERVE_DRAIN,
};

/// A flat, explicit pool-state projection for UI/reporting/validation.
///
/// The axes are intentionally separate:
/// - routeability answers whether the swap path currently works;
/// - liquidity answers what the AMM reserves/state look like;
/// - custody answers what powers exist or have fired against holder balances;
/// - risk answers whether the position should be treated as terminal/unsafe.
///
/// This prevents USDT-like latent custody authority from being mislabeled as a
/// drained/scam pool, while still making a realized holder-balance drain a
/// terminal position risk even when AMM reserves did not move.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolStateFlags {
    pub pool_address: String,
    pub token_address: String,
    pub denom_address: String,
    pub protocol: String,
    pub latest_block_number: Option<u64>,
    pub lifecycle: String,
    pub route: PoolRouteFlags,
    pub liquidity: PoolLiquidityFlags,
    pub custody: PoolCustodyFlags,
    pub risk: PoolRiskFlags,
    pub labels: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolRouteFlags {
    pub trading_enabled: bool,
    pub raw_can_buy: bool,
    pub raw_can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub can_buy_and_sell: bool,
    pub economic_sellable: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolLiquidityFlags {
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub total_liquidity: f64,
    pub price_denom_per_token: f64,
    pub has_reserves: bool,
    pub has_effective_liquidity: bool,
    /// Existing legacy/compatibility flag exposed by reserve tracking. Some
    /// realized custody risks set this so old consumers stop trading.
    pub legacy_liquidity_removal_flag: bool,
    /// True for reserve/pool-drain mechanisms. Holder-balance custody drains are
    /// terminal, but they are not reserve liquidity removal.
    pub reserve_liquidity_removed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolCustodyFlags {
    pub latent: bool,
    pub realized: bool,
    pub finding_count: usize,
    pub capabilities: Vec<PoolCustodyCapabilityFlag>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolCustodyCapabilityFlag {
    pub capability: String,
    pub state: String,
    pub label: String,
    pub block_number: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolRiskFlags {
    pub terminal_position_risk: bool,
    pub scam_mechanism: Option<String>,
    pub scam_label: Option<String>,
    pub direct_lp_liquidity_removal: bool,
    pub pair_balance_backdoor_drain: bool,
    pub holder_balance_backdoor_drain: bool,
    pub custody_buyer_token_confiscation: bool,
    pub privileged_seller_reserve_drain: bool,
    pub reserve_dump_drain: bool,
    pub unknown_reserve_drain: bool,
}

impl PoolStateFlags {
    pub fn from_base(base: &BasePool) -> Self {
        Self::from_base_and_custody_findings(base, &[])
    }

    pub fn from_base_and_custody_findings(
        base: &BasePool,
        custody_findings: &[CustodyFinding],
    ) -> Self {
        let trading = base.trading_status();
        let mechanism = base.inferred_scam_mechanism();
        let mechanism_key = mechanism
            .as_ref()
            .map(|mechanism| mechanism.mechanism.clone());
        let scam_label = base
            .scam_label
            .clone()
            .or_else(|| mechanism.as_ref().map(|mechanism| mechanism.label.clone()));
        let route = PoolRouteFlags {
            trading_enabled: trading.trading_enabled,
            raw_can_buy: trading.can_buy,
            raw_can_sell: trading.can_sell,
            effective_can_buy: trading.effective_can_buy,
            effective_can_sell: trading.effective_can_sell,
            can_buy_and_sell: trading.can_buy_and_sell,
            economic_sellable: trading.economic_sellable,
        };
        let risk = risk_flags(base, mechanism_key, scam_label);
        let liquidity = liquidity_flags(base, &risk);
        let custody = custody_flags(custody_findings);
        let labels = labels(base, &route, &liquidity, &custody, &risk);

        Self {
            pool_address: base.identity.pool_address.clone(),
            token_address: base.identity.token_address.clone(),
            denom_address: base.identity.denom_address.clone(),
            protocol: base.identity.protocol.clone(),
            latest_block_number: base.latest_block_number,
            lifecycle: base.state.lifecycle.as_str().to_string(),
            route,
            liquidity,
            custody,
            risk,
            labels,
        }
    }
}

fn risk_flags(
    base: &BasePool,
    mechanism_key: Option<String>,
    scam_label: Option<String>,
) -> PoolRiskFlags {
    let direct_lp_liquidity_removal =
        mechanism_key.as_deref() == Some(SCAM_DIRECT_LP_LIQUIDITY_REMOVAL);
    let pair_balance_backdoor_drain =
        mechanism_key.as_deref() == Some(SCAM_PAIR_BALANCE_BACKDOOR_DRAIN);
    let holder_balance_backdoor_drain =
        mechanism_key.as_deref() == Some(SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN);
    let custody_buyer_token_confiscation =
        mechanism_key.as_deref() == Some(SCAM_CUSTODY_BUYER_TOKEN_CONFISCATION);
    let privileged_seller_reserve_drain =
        mechanism_key.as_deref() == Some(SCAM_PRIVILEGED_SELLER_RESERVE_DRAIN);
    let reserve_dump_drain = mechanism_key.as_deref() == Some(SCAM_RESERVE_DUMP_DRAIN);
    let unknown_reserve_drain = mechanism_key.as_deref() == Some(SCAM_UNKNOWN_RESERVE_DRAIN);
    let terminal_position_risk = base.has_liquidity_removal() || mechanism_key.is_some();

    PoolRiskFlags {
        terminal_position_risk,
        scam_mechanism: mechanism_key,
        scam_label,
        direct_lp_liquidity_removal,
        pair_balance_backdoor_drain,
        holder_balance_backdoor_drain,
        custody_buyer_token_confiscation,
        privileged_seller_reserve_drain,
        reserve_dump_drain,
        unknown_reserve_drain,
    }
}

fn liquidity_flags(base: &BasePool, risk: &PoolRiskFlags) -> PoolLiquidityFlags {
    let token_reserve = base.token_reserve();
    let denom_reserve = base.denom_reserve();
    let has_reserves = positive_finite(token_reserve) && positive_finite(denom_reserve);
    let reserve_liquidity_removed = risk.direct_lp_liquidity_removal
        || risk.pair_balance_backdoor_drain
        || risk.privileged_seller_reserve_drain
        || risk.reserve_dump_drain
        || risk.unknown_reserve_drain
        || (base.has_liquidity_removal()
            && !risk.holder_balance_backdoor_drain
            && !risk.custody_buyer_token_confiscation);

    PoolLiquidityFlags {
        token_reserve,
        denom_reserve,
        total_liquidity: base.state.total_liquidity,
        price_denom_per_token: base.price(),
        has_reserves,
        has_effective_liquidity: base.current_liquidity_allows_trading(),
        legacy_liquidity_removal_flag: base.has_liquidity_removal(),
        reserve_liquidity_removed,
    }
}

fn custody_flags(findings: &[CustodyFinding]) -> PoolCustodyFlags {
    let mut capabilities = BTreeMap::<String, PoolCustodyCapabilityFlag>::new();
    let mut latent = false;
    let mut realized = false;

    for finding in findings {
        match finding.state {
            CustodyState::Latent => latent = true,
            CustodyState::Realized => realized = true,
        }
        let key = format!("{}:{}", finding.state.as_str(), finding.capability.as_str());
        capabilities
            .entry(key)
            .and_modify(|existing| {
                existing.block_number = earliest_block(existing.block_number, finding.block_number);
            })
            .or_insert_with(|| PoolCustodyCapabilityFlag {
                capability: finding.capability.as_str().to_string(),
                state: finding.state.as_str().to_string(),
                label: finding.capability.label().to_string(),
                block_number: finding.block_number,
            });
    }

    PoolCustodyFlags {
        latent,
        realized,
        finding_count: findings.len(),
        capabilities: capabilities.into_values().collect(),
    }
}

fn labels(
    base: &BasePool,
    route: &PoolRouteFlags,
    liquidity: &PoolLiquidityFlags,
    custody: &PoolCustodyFlags,
    risk: &PoolRiskFlags,
) -> Vec<String> {
    let mut labels = BTreeSet::new();
    labels.insert(format!("lifecycle:{}", base.state.lifecycle.as_str()));

    if route.trading_enabled {
        labels.insert("route:trading_enabled".to_string());
    }
    if route.effective_can_buy {
        labels.insert("route:effective_can_buy".to_string());
    } else if route.raw_can_buy {
        labels.insert("route:raw_can_buy_only".to_string());
    }
    if route.effective_can_sell {
        labels.insert("route:effective_can_sell".to_string());
    } else if route.raw_can_sell {
        labels.insert("route:raw_can_sell_only".to_string());
    }
    match route.economic_sellable {
        Some(true) => {
            labels.insert("route:economic_sellable".to_string());
        }
        Some(false) => {
            labels.insert("route:not_economic_sellable".to_string());
        }
        None => {}
    }

    if liquidity.has_reserves {
        labels.insert("liquidity:has_reserves".to_string());
    }
    if liquidity.reserve_liquidity_removed {
        labels.insert("liquidity:reserve_removed".to_string());
    } else if liquidity.legacy_liquidity_removal_flag {
        labels.insert("liquidity:legacy_terminal_flag".to_string());
    }

    if custody.latent {
        labels.insert("custody:latent".to_string());
    }
    if custody.realized {
        labels.insert("custody:realized".to_string());
    }
    for capability in &custody.capabilities {
        labels.insert(format!(
            "custody:{}:{}",
            capability.state, capability.capability
        ));
    }

    if risk.terminal_position_risk {
        labels.insert("risk:terminal_position".to_string());
    }
    if let Some(mechanism) = risk.scam_mechanism.as_deref() {
        labels.insert(format!("risk:{mechanism}"));
    }

    labels.into_iter().collect()
}

fn earliest_block(current: Option<u64>, incoming: Option<u64>) -> Option<u64> {
    match (current, incoming) {
        (Some(current), Some(incoming)) => Some(current.min(incoming)),
        (Some(current), None) => Some(current),
        (None, Some(incoming)) => Some(incoming),
        (None, None) => None,
    }
}

fn positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::custody::CustodyCapability;
    use crate::pools::base::{BasePoolConfig, PoolIdentity};

    const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

    fn trading_pool() -> BasePool {
        let mut pool = BasePool::new(
            PoolIdentity::new("0x1111", "0x2222", WETH, "uniswap_v2"),
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
        );
        pool.update_reserves(1_000_000.0, 1.0, 100, 1_700_000_000, "0xsync");
        pool.set_simulated_buy_status(true, 100, "0xbuy", 1_700_000_000);
        pool.set_simulated_sell_status(true, Some(0.0), Some(0.0), 100, "0xsell");
        pool
    }

    #[test]
    fn latent_custody_is_not_terminal_or_liquidity_removal() {
        let pool = trading_pool();
        let finding = CustodyFinding {
            capability: CustodyCapability::Freeze,
            state: CustodyState::Latent,
            block_number: None,
            evidence: json!({"source": "unit_test"}),
        };

        let flags = PoolStateFlags::from_base_and_custody_findings(&pool, &[finding]);

        assert!(flags.custody.latent);
        assert!(!flags.custody.realized);
        assert!(!flags.risk.terminal_position_risk);
        assert!(!flags.liquidity.legacy_liquidity_removal_flag);
        assert!(!flags.liquidity.reserve_liquidity_removed);
        assert!(flags.route.effective_can_buy);
        assert!(flags
            .labels
            .contains(&"custody:latent:custody_freeze".to_string()));
    }

    #[test]
    fn holder_balance_backdoor_is_terminal_without_reserve_removal() {
        let mut pool = trading_pool();
        pool.mark_scam_mechanism(
            SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN,
            Some(101),
            Some("0xdrain".to_string()),
            json!({"detail": "holder balance drain"}),
        );
        pool.reserve_tracker.is_scam = true;
        let finding = CustodyFinding {
            capability: CustodyCapability::BurnDrain,
            state: CustodyState::Realized,
            block_number: Some(101),
            evidence: json!({"source": "unit_test"}),
        };

        let flags = PoolStateFlags::from_base_and_custody_findings(&pool, &[finding]);

        assert!(flags.custody.realized);
        assert!(flags.risk.terminal_position_risk);
        assert!(flags.risk.holder_balance_backdoor_drain);
        assert!(flags.liquidity.legacy_liquidity_removal_flag);
        assert!(!flags.liquidity.reserve_liquidity_removed);
        assert!(!flags.route.effective_can_buy);
        assert!(flags
            .labels
            .contains(&"risk:holder_balance_backdoor_drain".to_string()));
    }

    #[test]
    fn direct_reserve_drain_is_liquidity_removal() {
        let mut pool = trading_pool();
        pool.mark_scam_mechanism(
            SCAM_DIRECT_LP_LIQUIDITY_REMOVAL,
            Some(101),
            Some("0xdrain".to_string()),
            json!({"detail": "lp burn"}),
        );
        pool.reserve_tracker.is_scam = true;

        let flags = PoolStateFlags::from_base(&pool);

        assert!(flags.risk.terminal_position_risk);
        assert!(flags.risk.direct_lp_liquidity_removal);
        assert!(flags.liquidity.reserve_liquidity_removed);
    }
}
