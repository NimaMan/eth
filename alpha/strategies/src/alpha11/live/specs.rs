use crate::{
    alpha11::{
        BUY_WEI, ENTRY_INIT_MAX_AGE_BLOCKS, ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL,
        HOLD16_ALL_POOLS_STRATEGY_NAME, HOLD3_VALIDATION_STRATEGY_NAME, HOLD_SWEEP_SET_NAME,
        INITIAL_ENTRY_BANKROLL_ETH, LIVE_VALIDATION_ENTRY_BANKROLL_ETH,
        LIVE_VALIDATION_MAX_ENTRY_POOLS, LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS,
        MIN_LIQUIDITY_ETH, MIN_LIQUIDITY_USD, STRATEGY_IMPL,
    },
    shared_rules::live::{LiveEntryInitPolicySpec, LiveStrategySpec, LiveStrategySpecOptions},
};

const MIN_SELL_POOL_DENOM_RESERVE: &str = "0";

pub const SET_NAME: &str = HOLD_SWEEP_SET_NAME;

pub fn specs(options: &LiveStrategySpecOptions) -> Vec<LiveStrategySpec> {
    let mut specs = [12_u64, 14, 15, 16, 18, 20, 25, 50, 100]
        .into_iter()
        .map(|max_hold_blocks| spec(max_hold_blocks, options))
        .collect::<Vec<_>>();
    specs.insert(4, hold16_all_pools_spec(options));
    specs
}

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

fn spec(max_hold_blocks: u64, _options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: format!("alpha11-univ2-lp30-pool-update-block-hold{max_hold_blocks}"),
        strategy_impl: STRATEGY_IMPL.to_string(),
        strategy_label: format!("Alpha11 Uniswap V2 LP30 pool-update-block hold {max_hold_blocks}"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpha11::{
        HOLD15_STRATEGY_NAME, HOLD16_ALL_POOLS_STRATEGY_NAME, HOLD16_STRATEGY_NAME,
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
        assert!(spec.exit_liquidity_removal);
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
        assert!(spec.exit_liquidity_removal);
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
        assert!(spec.exit_liquidity_removal);
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
