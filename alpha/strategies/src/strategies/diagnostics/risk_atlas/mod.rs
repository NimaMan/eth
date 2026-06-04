use crate::core::spec::{LiveStrategySpec, LiveStrategySpecOptions, CORE_STRATEGY_IMPL};

pub const LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME: &str =
    "snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold";

pub fn lp_gate_hold15_buy_confirm_lp_maxhold_spec(
    _options: &LiveStrategySpecOptions,
) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME.to_string(),
        strategy_impl: CORE_STRATEGY_IMPL.to_string(),
        strategy_label: "Snipe All risk atlas LP gate hold 15 buy-confirm LP maxhold".to_string(),
        exit_liquidity_removal: true,
        exit_tax: false,
        exit_lp_approval: true,
        exit_lp_approval_critical_only: false,
        exit_scam: false,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: true,
        lp_approval_gate_min_pct: Some(
            crate::core::rules::lp_approval::DEFAULT_GATE_MIN_APPROVED_PCT.to_string(),
        ),
        entry_init_policy: Default::default(),
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
        lp_approval_exit_defer_max_trading_enabled_age_blocks: Some(
            crate::core::rules::exit::lp_approval::DEFAULT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS,
        ),
        min_sell_pool_denom_reserve: None,
        buy_wei: "10000000000000000".to_string(),
        min_liquidity_eth: "0.5".to_string(),
        min_liquidity_usd: "1000".to_string(),
        max_entry_pools: None,
        entry_bankroll_eth: None,
        stop_loss_ratio: None,
        take_profit_ratio: None,
        max_hold_blocks: Some(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_confirm_maxhold_suite_has_no_protocol_filter() {
        let spec = lp_gate_hold15_buy_confirm_lp_maxhold_spec(&LiveStrategySpecOptions::default());

        assert_eq!(
            spec.strategy_name,
            LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME
        );
        assert!(spec.allowed_protocols.is_empty());
        assert!(spec.block_entry_on_lp_approval);
        assert_eq!(spec.lp_approval_gate_min_pct.as_deref(), Some("30"));
        assert!(spec.defer_buy_confirm_block_lp_approval_to_max_hold);
        assert_eq!(
            spec.lp_approval_exit_defer_max_trading_enabled_age_blocks,
            Some(crate::core::rules::exit::lp_approval::DEFAULT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS)
        );
        assert_eq!(spec.max_hold_blocks, Some(15));
        assert!(spec.exit_liquidity_removal);
        assert!(spec.exit_lp_approval);
        assert!(!spec.exit_lp_approval_critical_only);
        assert!(!spec.exit_tax);
        assert!(!spec.exit_scam);
    }
}
