use super::super::spec::{LiveStrategySpec, LiveStrategySpecOptions, DEFAULT_STRATEGY_NAME};

pub fn spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: crate::shared_rules::lp_approval_warning_exit::STRATEGY_NAME.to_string(),
        strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
        strategy_label: crate::shared_rules::lp_approval_warning_exit::STRATEGY_LABEL.to_string(),
        exit_liquidity_removal:
            crate::shared_rules::lp_approval_warning_exit::EXIT_LIQUIDITY_REMOVAL,
        exit_tax: crate::shared_rules::lp_approval_warning_exit::EXIT_TAX,
        exit_lp_approval: crate::shared_rules::lp_approval_warning_exit::EXIT_LP_APPROVAL,
        exit_lp_approval_critical_only:
            crate::shared_rules::lp_approval_warning_exit::EXIT_LP_APPROVAL_CRITICAL_ONLY,
        exit_scam: crate::shared_rules::lp_approval_warning_exit::EXIT_SCAM,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: false,
        lp_approval_gate_min_pct: None,
        max_entry_price_ratio_to_initial: None,
        defer_buy_confirm_block_lp_approval_to_max_hold: false,
        min_sell_pool_denom_reserve: None,
        entry_bankroll_eth: None,
        stop_loss_ratio: options.stop_loss_ratio.clone(),
        take_profit_ratio: options.take_profit_ratio.clone(),
        max_hold_blocks: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_is_single_non_hold_strategy() {
        let spec = spec(&LiveStrategySpecOptions {
            max_hold_blocks: Some(20),
            ..LiveStrategySpecOptions::default()
        });

        assert_eq!(
            spec.strategy_name,
            crate::shared_rules::lp_approval_warning_exit::STRATEGY_NAME
        );
        assert_eq!(spec.max_hold_blocks, None);
        assert!(spec.exit_liquidity_removal);
        assert!(spec.exit_lp_approval);
        assert!(!spec.exit_lp_approval_critical_only);
        assert!(!spec.exit_tax);
        assert!(!spec.exit_scam);
    }
}
