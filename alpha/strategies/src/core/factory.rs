//! Single instantiation path: resolved [`StrategySpec`] -> [`StrategyConfig`].
//!
//! Both live and backtest resolve a strategy through the registry into a
//! complete [`StrategySpec`], then build the engine config here. Keeping this in
//! one place is what guarantees a strategy id produces an identical engine in
//! every mode. Runtime-only inputs (entry bankroll, whether entry is enabled)
//! are passed in because they are not part of the resolved spec.

use std::str::FromStr;

use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::StrategyName,
};
use rust_decimal::Decimal;

use crate::core::rules::entry::init_policy::EntryInitPolicyConfig;
use crate::core::spec::StrategySpec;
use crate::core::StrategyConfig;

/// Runtime-only inputs that are not part of the resolved spec.
#[derive(Clone, Copy, Debug, Default)]
pub struct EngineRuntimeInputs {
    /// Starting ETH/WETH bankroll for entries (None = unbounded).
    pub entry_bankroll_wei: Option<U256>,
    /// If false, the engine manages/exits existing positions but opens none.
    pub entry_enabled: bool,
}

/// Build the engine [`StrategyConfig`] for a fully resolved [`StrategySpec`].
///
/// Errors only on malformed numeric strings in the spec.
pub fn config_from_spec(
    spec: &StrategySpec,
    runtime: EngineRuntimeInputs,
) -> Result<StrategyConfig, String> {
    let buy_wei = U256::from_str_radix(&spec.buy_wei, 10)
        .map_err(|err| format!("invalid buy_wei {}: {err}", spec.buy_wei))?;
    let min_denom_reserve = Decimal::from_str(&spec.min_liquidity_eth)
        .map_err(|err| format!("invalid min_liquidity_eth {}: {err}", spec.min_liquidity_eth))?;
    let min_stable_denom_reserve = Decimal::from_str(&spec.min_liquidity_usd)
        .map_err(|err| format!("invalid min_liquidity_usd {}: {err}", spec.min_liquidity_usd))?;

    let stop_loss_ratio = spec
        .stop_loss_ratio
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok());
    let take_profit_ratio = spec
        .take_profit_ratio
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok());
    let lp_approval_gate_min_pct = spec
        .lp_approval_gate_min_pct
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok());
    let max_price_ratio_to_initial = spec
        .entry_init_policy
        .max_price_ratio_to_initial
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok());
    let min_sell_pool_denom_reserve = spec
        .min_sell_pool_denom_reserve
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_else(|| StrategyConfig::default().min_sell_pool_denom_reserve);

    let entry_init_policy = EntryInitPolicyConfig {
        max_age_blocks: spec.entry_init_policy.max_age_blocks,
        require_pool_creation_block: spec.entry_init_policy.require_pool_creation_block,
        max_price_ratio_to_initial,
        allow_missing_price_ratio: spec.entry_init_policy.allow_missing_price_ratio,
    };

    Ok(StrategyConfig {
        strategy_name: StrategyName(spec.strategy_name.clone()),
        buy_amount: Amount {
            raw: buy_wei,
            decimals: 18,
        },
        sell_fraction: DecimalAmount::from(1),
        min_denom_reserve,
        min_stable_denom_reserve,
        min_sell_pool_denom_reserve,
        entry_enabled: runtime.entry_enabled,
        max_entry_pools: spec.max_entry_pools,
        entry_bankroll_wei: runtime.entry_bankroll_wei,
        stop_loss_ratio,
        take_profit_ratio,
        max_hold_blocks: spec.max_hold_blocks,
        exit_on_tax: spec.exit_tax,
        exit_on_lp_approval: spec.exit_lp_approval,
        exit_on_critical_lp_approval_only: spec.exit_lp_approval_critical_only,
        exit_on_scam: spec.exit_scam,
        allowed_protocols: spec.allowed_protocols.clone(),
        block_entry_on_lp_approval: spec.block_entry_on_lp_approval,
        lp_approval_gate_min_pct,
        entry_init_policy,
        defer_buy_confirm_block_lp_approval_to_max_hold: spec
            .defer_buy_confirm_block_lp_approval_to_max_hold,
        lp_approval_exit_defer_max_trading_enabled_age_blocks: spec
            .lp_approval_exit_defer_max_trading_enabled_age_blocks,
        ..StrategyConfig::default()
    })
}
