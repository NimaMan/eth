use crate::{
    alpha11::{
        BUY_WEI, HOLD3_VALIDATION_STRATEGY_NAME, HOLD_SWEEP_SET_NAME, INITIAL_ENTRY_BANKROLL_ETH,
        LIVE_VALIDATION_ENTRY_BANKROLL_ETH, LIVE_VALIDATION_MAX_ENTRY_POOLS, MIN_LIQUIDITY_ETH,
        MIN_LIQUIDITY_USD, STRATEGY_IMPL,
    },
    shared_rules::live::{LiveStrategySpec, LiveStrategySpecOptions},
};

const MIN_SELL_POOL_DENOM_RESERVE: &str = "0";

pub const SET_NAME: &str = HOLD_SWEEP_SET_NAME;

pub fn specs(options: &LiveStrategySpecOptions) -> Vec<LiveStrategySpec> {
    [12_u64, 15, 20]
        .into_iter()
        .map(|max_hold_blocks| spec(max_hold_blocks, options))
        .collect()
}

pub fn hold15_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    spec(15, options)
}

pub fn hold3_validation_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    let mut spec = spec(3, options);
    spec.strategy_name = HOLD3_VALIDATION_STRATEGY_NAME.to_string();
    spec.strategy_label =
        "Alpha11 live validation Uniswap V2 LP30 pool-update-block hold 3".to_string();
    spec.entry_bankroll_eth = Some(LIVE_VALIDATION_ENTRY_BANKROLL_ETH.to_string());
    spec.max_entry_pools = Some(LIVE_VALIDATION_MAX_ENTRY_POOLS);
    spec
}

fn spec(max_hold_blocks: u64, _options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: format!("alpha11-live-univ2-lp30-pool-update-block-hold{max_hold_blocks}"),
        strategy_impl: STRATEGY_IMPL.to_string(),
        strategy_label: format!(
            "Alpha11 live Uniswap V2 LP30 pool-update-block hold {max_hold_blocks}"
        ),
        exit_liquidity_removal: true,
        exit_tax: true,
        exit_lp_approval: true,
        exit_lp_approval_critical_only: false,
        exit_scam: true,
        allowed_protocols: vec!["UNISWAP-V2".to_string()],
        block_entry_on_lp_approval: true,
        lp_approval_gate_min_pct: Some(
            crate::shared_rules::lp_approval::DEFAULT_GATE_MIN_APPROVED_PCT.to_string(),
        ),
        max_entry_price_ratio_to_initial: None,
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpha11::HOLD15_STRATEGY_NAME;

    #[test]
    fn set_matches_hold_sweep() {
        let specs = specs(&LiveStrategySpecOptions::default());

        assert_eq!(specs.len(), 3);
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.strategy_name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "alpha11-live-univ2-lp30-pool-update-block-hold12",
                HOLD15_STRATEGY_NAME,
                "alpha11-live-univ2-lp30-pool-update-block-hold20",
            ]
        );
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.max_hold_blocks)
                .collect::<Vec<_>>(),
            vec![Some(12), Some(15), Some(20)]
        );
        assert!(specs.iter().all(|spec| spec.strategy_impl == STRATEGY_IMPL));
        assert!(specs.iter().all(|spec| spec.exit_liquidity_removal));
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
            .all(|spec| spec.max_entry_price_ratio_to_initial.is_none()));
        assert!(specs
            .iter()
            .all(|spec| spec.defer_buy_confirm_block_lp_approval_to_max_hold));
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
        assert!(specs
            .iter()
            .all(|spec| spec.allowed_protocols == vec!["UNISWAP-V2".to_string()]));
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
        assert!(spec.exit_liquidity_removal);
        assert!(spec.exit_lp_approval);
        assert!(spec.defer_buy_confirm_block_lp_approval_to_max_hold);
        assert!(spec.strategy_label.contains("validation"));
        assert!(!spec.strategy_name.contains("price-to-initial"));
    }
}
