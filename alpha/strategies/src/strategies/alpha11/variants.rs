//! Alpha11 sub-strategies as complete resolved specs.
//!
//! Every variant shares the alpha11 base policy and differs only in hold horizon
//! (`max_hold_blocks`) plus two tweaks: the all-pools variant drops the protocol
//! filter, and the hold3 validation probe uses a tiny bankroll and a single
//! entry pool. Each builder returns a fully resolved [`LiveStrategySpec`] so live
//! and backtest instantiate an identical engine.

use super::config::{
    BUY_WEI, ENTRY_INIT_MAX_AGE_BLOCKS, ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL,
    HOLD16_ALL_POOLS_STRATEGY_NAME, HOLD3_VALIDATION_STRATEGY_NAME, INITIAL_ENTRY_BANKROLL_ETH,
    LIVE_VALIDATION_ENTRY_BANKROLL_ETH, LIVE_VALIDATION_MAX_ENTRY_POOLS,
    LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS, MIN_LIQUIDITY_ETH, MIN_LIQUIDITY_USD,
    STRATEGY_IMPL,
};
use crate::core::spec::{LiveEntryInitPolicySpec, LiveStrategySpec, LiveStrategySpecOptions};

const MIN_SELL_POOL_DENOM_RESERVE: &str = "0";

pub fn hold15_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    spec(15, options)
}

pub fn hold16_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    spec(16, options)
}

pub fn hold16_all_pools_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    let mut spec = spec(16, options);
    spec.strategy_name = HOLD16_ALL_POOLS_STRATEGY_NAME.to_string();
    spec.strategy_label = "Alpha11 all pools LP30 pool-update-block hold 16".to_string();
    spec.allowed_protocols = Vec::new();
    spec
}

pub fn hold3_validation_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    let mut spec = spec(3, options);
    spec.strategy_name = HOLD3_VALIDATION_STRATEGY_NAME.to_string();
    spec.strategy_label = "Alpha11 validation Uniswap V2 LP30 pool-update-block hold 3".to_string();
    spec.entry_bankroll_eth = Some(LIVE_VALIDATION_ENTRY_BANKROLL_ETH.to_string());
    spec.max_entry_pools = Some(LIVE_VALIDATION_MAX_ENTRY_POOLS);
    spec
}

pub fn spec(max_hold_blocks: u64, _options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: format!("alpha11-univ2-lp30-pool-update-block-hold{max_hold_blocks}"),
        strategy_impl: STRATEGY_IMPL.to_string(),
        strategy_label: format!("Alpha11 Uniswap V2 LP30 pool-update-block hold {max_hold_blocks}"),
        exit_tax: true,
        exit_lp_approval: true,
        exit_lp_approval_critical_only: false,
        exit_scam: true,
        allowed_protocols: vec!["UNISWAP-V2".to_string()],
        block_entry_on_lp_approval: true,
        lp_approval_gate_min_pct: Some(
            crate::core::rules::lp_approval::DEFAULT_GATE_MIN_APPROVED_PCT.to_string(),
        ),
        entry_init_policy: LiveEntryInitPolicySpec {
            max_age_blocks: Some(ENTRY_INIT_MAX_AGE_BLOCKS),
            require_pool_creation_block: false,
            max_price_ratio_to_initial: Some(ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL.to_string()),
            allow_missing_price_ratio: true,
        },
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
        lp_approval_exit_defer_max_trading_enabled_age_blocks: Some(
            LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS,
        ),
        min_sell_pool_denom_reserve: Some(MIN_SELL_POOL_DENOM_RESERVE.to_string()),
        buy_wei: BUY_WEI.to_string(),
        min_liquidity_eth: MIN_LIQUIDITY_ETH.to_string(),
        min_liquidity_usd: MIN_LIQUIDITY_USD.to_string(),
        max_entry_pools: None,
        entry_bankroll_eth: Some(INITIAL_ENTRY_BANKROLL_ETH.to_string()),
        stop_loss_ratio: None,
        take_profit_ratio: None,
        max_hold_blocks: Some(max_hold_blocks),
    }
}
