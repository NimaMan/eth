//! Alpha11 factory: assemble the resolved sub-strategy spec sets.
//!
//! Used by both live and backtest so the alpha11 variant set is identical in
//! every mode. The individual sub-strategy spec builders live in
//! [`super::variants`] and are re-exported here for callers that resolve a
//! single variant by name.

use super::config::HOLD_SWEEP_SET_NAME;
use crate::core::spec::{LiveStrategySpec, LiveStrategySpecOptions};

pub use super::variants::{
    hold15_spec, hold16_all_pools_spec, hold16_spec, hold3_validation_spec, spec,
};

pub const SET_NAME: &str = HOLD_SWEEP_SET_NAME;

/// The full alpha11 hold sweep: hold12..hold100 plus the all-pools hold16
/// variant inserted at index 4. This is the single source of truth for the
/// alpha11 variant set across live and backtest.
pub fn specs(options: &LiveStrategySpecOptions) -> Vec<LiveStrategySpec> {
    let mut specs = [12_u64, 14, 15, 16, 18, 20, 25, 50, 100]
        .into_iter()
        .map(|max_hold_blocks| spec(max_hold_blocks, options))
        .collect::<Vec<_>>();
    specs.insert(4, hold16_all_pools_spec(options));
    specs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategies::alpha11::{
        BUY_WEI, ENTRY_INIT_MAX_AGE_BLOCKS, ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL,
        HOLD15_STRATEGY_NAME, HOLD16_ALL_POOLS_STRATEGY_NAME, HOLD16_STRATEGY_NAME,
        HOLD3_VALIDATION_STRATEGY_NAME, INITIAL_ENTRY_BANKROLL_ETH,
        LIVE_VALIDATION_ENTRY_BANKROLL_ETH, LIVE_VALIDATION_MAX_ENTRY_POOLS,
        LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS, MIN_LIQUIDITY_ETH, MIN_LIQUIDITY_USD,
        STRATEGY_IMPL,
    };

    #[test]
    fn set_matches_hold_sweep() {
        let specs = specs(&LiveStrategySpecOptions::default());

        assert_eq!(specs.len(), 10);
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.strategy_name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "alpha11-univ2-lp30-pool-update-block-hold12",
                "alpha11-univ2-lp30-pool-update-block-hold14",
                HOLD15_STRATEGY_NAME,
                HOLD16_STRATEGY_NAME,
                HOLD16_ALL_POOLS_STRATEGY_NAME,
                "alpha11-univ2-lp30-pool-update-block-hold18",
                "alpha11-univ2-lp30-pool-update-block-hold20",
                "alpha11-univ2-lp30-pool-update-block-hold25",
                "alpha11-univ2-lp30-pool-update-block-hold50",
                "alpha11-univ2-lp30-pool-update-block-hold100",
            ]
        );
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.max_hold_blocks)
                .collect::<Vec<_>>(),
            vec![
                Some(12),
                Some(14),
                Some(15),
                Some(16),
                Some(16),
                Some(18),
                Some(20),
                Some(25),
                Some(50),
                Some(100),
            ]
        );
        assert!(specs.iter().all(|spec| spec.strategy_impl == STRATEGY_IMPL));
        assert!(specs.iter().all(|spec| spec.exit_lp_approval));
        assert!(specs
            .iter()
            .all(|spec| !spec.exit_lp_approval_critical_only));
        assert!(specs.iter().all(|spec| spec.exit_tax));
        assert!(specs.iter().all(|spec| spec.exit_scam));
        assert!(specs.iter().all(|spec| spec.block_entry_on_lp_approval));
        assert!(specs
            .iter()
            .all(|spec| spec.lp_approval_gate_min_pct.as_deref() == Some("30")));
        assert!(specs
            .iter()
            .all(|spec| spec.entry_init_policy.max_age_blocks == Some(ENTRY_INIT_MAX_AGE_BLOCKS)));
        assert!(specs.iter().all(|spec| {
            spec.entry_init_policy.max_price_ratio_to_initial.as_deref()
                == Some(ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL)
        }));
        assert!(specs
            .iter()
            .all(|spec| spec.defer_buy_confirm_block_lp_approval_to_max_hold));
        assert!(specs.iter().all(|spec| {
            spec.lp_approval_exit_defer_max_trading_enabled_age_blocks
                == Some(LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS)
        }));
        assert!(specs
            .iter()
            .all(|spec| spec.min_sell_pool_denom_reserve.as_deref() == Some("0")));
        assert!(specs
            .iter()
            .all(|spec| spec.entry_bankroll_eth.as_deref() == Some(INITIAL_ENTRY_BANKROLL_ETH)));
        assert!(specs.iter().all(|spec| spec.buy_wei == BUY_WEI));
        assert!(specs
            .iter()
            .all(|spec| spec.min_liquidity_eth == MIN_LIQUIDITY_ETH));
        assert!(specs
            .iter()
            .all(|spec| spec.min_liquidity_usd == MIN_LIQUIDITY_USD));
        assert!(specs.iter().all(|spec| spec.max_entry_pools.is_none()));
        let all_pools = specs
            .iter()
            .find(|spec| spec.strategy_name == HOLD16_ALL_POOLS_STRATEGY_NAME)
            .expect("all-pools hold16 spec in sweep");
        assert!(all_pools.allowed_protocols.is_empty());
        assert!(specs
            .iter()
            .filter(|spec| spec.strategy_name != HOLD16_ALL_POOLS_STRATEGY_NAME)
            .all(|spec| spec.allowed_protocols == vec!["UNISWAP-V2".to_string()]));
    }

    #[test]
    fn hold16_single_spec_matches_deploy_target() {
        let spec = hold16_spec(&LiveStrategySpecOptions::default());

        assert_eq!(spec.strategy_name, HOLD16_STRATEGY_NAME);
        assert_eq!(spec.strategy_impl, STRATEGY_IMPL);
        assert_eq!(spec.max_hold_blocks, Some(16));
        assert_eq!(
            spec.entry_bankroll_eth.as_deref(),
            Some(INITIAL_ENTRY_BANKROLL_ETH)
        );
        assert_eq!(spec.max_entry_pools, None);
        assert_eq!(spec.buy_wei, BUY_WEI);
        assert_eq!(spec.allowed_protocols, vec!["UNISWAP-V2".to_string()]);
        assert_eq!(spec.lp_approval_gate_min_pct.as_deref(), Some("30"));
        assert!(spec.exit_lp_approval);
        assert!(spec.defer_buy_confirm_block_lp_approval_to_max_hold);
    }

    #[test]
    fn hold16_all_pools_spec_removes_protocol_filter_only() {
        let spec = hold16_all_pools_spec(&LiveStrategySpecOptions::default());

        assert_eq!(spec.strategy_name, HOLD16_ALL_POOLS_STRATEGY_NAME);
        assert_eq!(spec.strategy_impl, STRATEGY_IMPL);
        assert_eq!(
            spec.strategy_label,
            "Alpha11 all pools LP30 pool-update-block hold 16"
        );
        assert_eq!(spec.max_hold_blocks, Some(16));
        assert_eq!(
            spec.entry_bankroll_eth.as_deref(),
            Some(INITIAL_ENTRY_BANKROLL_ETH)
        );
        assert_eq!(spec.max_entry_pools, None);
        assert_eq!(spec.buy_wei, BUY_WEI);
        assert!(spec.allowed_protocols.is_empty());
        assert_eq!(spec.lp_approval_gate_min_pct.as_deref(), Some("30"));
        assert!(spec.exit_lp_approval);
        assert!(spec.defer_buy_confirm_block_lp_approval_to_max_hold);
    }

    #[test]
    fn hold3_validation_spec_is_single_trade_live_probe() {
        let spec = hold3_validation_spec(&LiveStrategySpecOptions::default());

        assert_eq!(spec.strategy_name, HOLD3_VALIDATION_STRATEGY_NAME);
        assert_eq!(spec.strategy_impl, STRATEGY_IMPL);
        assert_eq!(spec.max_hold_blocks, Some(3));
        assert_eq!(
            spec.entry_bankroll_eth.as_deref(),
            Some(LIVE_VALIDATION_ENTRY_BANKROLL_ETH)
        );
        assert_eq!(spec.max_entry_pools, Some(LIVE_VALIDATION_MAX_ENTRY_POOLS));
        assert_eq!(spec.buy_wei, BUY_WEI);
        assert_eq!(spec.allowed_protocols, vec!["UNISWAP-V2".to_string()]);
        assert!(spec.exit_lp_approval);
        assert!(spec.defer_buy_confirm_block_lp_approval_to_max_hold);
        assert_eq!(
            spec.lp_approval_exit_defer_max_trading_enabled_age_blocks,
            Some(LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS)
        );
        assert!(spec.strategy_label.contains("validation"));
        assert!(!spec.strategy_name.contains("price-to-initial"));
    }
}
