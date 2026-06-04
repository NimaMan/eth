use crate::core::spec::{LiveStrategySpec, LiveStrategySpecOptions, CORE_STRATEGY_IMPL};

pub const SET_NAME: &str = "mempool-live-exits";

pub fn specs(_options: &LiveStrategySpecOptions) -> Vec<LiveStrategySpec> {
    [10_u64, 20, 50]
        .into_iter()
        .flat_map(|max_hold_blocks| {
            [
                LiveStrategySpec {
                    strategy_name: format!(
                        "snipe-all-live-hold{max_hold_blocks}-pool-updates-liquidity-exit"
                    ),
                    strategy_impl: CORE_STRATEGY_IMPL.to_string(),
                    strategy_label: format!(
                        "Snipe All live hold {max_hold_blocks} pool updates + liquidity exit"
                    ),
                    exit_tax: false,
                    exit_lp_approval: false,
                    exit_lp_approval_critical_only: false,
                    exit_scam: false,
                    allowed_protocols: Vec::new(),
                    block_entry_on_lp_approval: false,
                    lp_approval_gate_min_pct: None,
                    entry_init_policy: Default::default(),
                    defer_buy_confirm_block_lp_approval_to_max_hold: false,
                    lp_approval_exit_defer_max_trading_enabled_age_blocks: None,
                    min_sell_pool_denom_reserve: None,
                    buy_wei: "10000000000000000".to_string(),
                    min_liquidity_eth: "0.5".to_string(),
                    min_liquidity_usd: "1000".to_string(),
                    max_entry_pools: None,
                    entry_bankroll_eth: None,
                    stop_loss_ratio: None,
                    take_profit_ratio: None,
                    max_hold_blocks: Some(max_hold_blocks),
                },
                LiveStrategySpec {
                    strategy_name: format!(
                        "snipe-all-live-hold{max_hold_blocks}-pool-updates-liquidity-critical-lp-exit"
                    ),
                    strategy_impl: CORE_STRATEGY_IMPL.to_string(),
                    strategy_label: format!(
                        "Snipe All live hold {max_hold_blocks} pool updates + liquidity + critical LP exit"
                    ),
                    exit_tax: false,
                    exit_lp_approval: true,
                    exit_lp_approval_critical_only: true,
                    exit_scam: false,
                    allowed_protocols: Vec::new(),
                    block_entry_on_lp_approval: false,
                    lp_approval_gate_min_pct: None,
                    entry_init_policy: Default::default(),
                    defer_buy_confirm_block_lp_approval_to_max_hold: false,
                    lp_approval_exit_defer_max_trading_enabled_age_blocks: None,
                    min_sell_pool_denom_reserve: None,
                    buy_wei: "10000000000000000".to_string(),
                    min_liquidity_eth: "0.5".to_string(),
                    min_liquidity_usd: "1000".to_string(),
                    max_entry_pools: None,
                    entry_bankroll_eth: None,
                    stop_loss_ratio: None,
                    take_profit_ratio: None,
                    max_hold_blocks: Some(max_hold_blocks),
                },
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_resolves_six_variants() {
        // Liquidity-removal exit is fundamental/always-on (no per-spec flag); this
        // set just resolves a hold-block x critical-LP variant matrix.
        let specs = specs(&LiveStrategySpecOptions::default());

        assert_eq!(specs.len(), 6);
    }

    #[test]
    fn critical_lp_variants_keep_critical_lp_filter() {
        let specs = specs(&LiveStrategySpecOptions::default());

        let critical_lp_specs = specs
            .iter()
            .filter(|spec| spec.strategy_name.contains("critical-lp"))
            .collect::<Vec<_>>();
        assert_eq!(critical_lp_specs.len(), 3);
        assert!(critical_lp_specs.iter().all(|spec| spec.exit_lp_approval));
        assert!(critical_lp_specs
            .iter()
            .all(|spec| spec.exit_lp_approval_critical_only));
    }
}
