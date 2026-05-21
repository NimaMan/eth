use crate::{
    alpha11::{
        DEPLOY_HOLD_SWEEP_SET_NAME, HOLD_SWEEP_SET_NAME, INITIAL_ENTRY_BANKROLL_ETH,
        MAX_ENTRY_PRICE_RATIO_TO_INITIAL, STRATEGY_IMPL,
    },
    shared_rules::live::{LiveStrategySpec, LiveStrategySpecOptions},
};

const MIN_SELL_POOL_DENOM_RESERVE: &str = "0";

pub const SET_NAME: &str = HOLD_SWEEP_SET_NAME;
pub const DEPLOY_SET_NAME: &str = DEPLOY_HOLD_SWEEP_SET_NAME;

pub fn specs(options: &LiveStrategySpecOptions) -> Vec<LiveStrategySpec> {
    [12_u64, 15, 20]
        .into_iter()
        .map(|max_hold_blocks| spec(max_hold_blocks, options, false))
        .collect()
}

pub fn hold15_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    spec(15, options, false)
}

pub fn deploy_specs(options: &LiveStrategySpecOptions) -> Vec<LiveStrategySpec> {
    [12_u64, 15, 20]
        .into_iter()
        .map(|max_hold_blocks| spec(max_hold_blocks, options, true))
        .collect()
}

pub fn deploy_hold15_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    spec(15, options, true)
}

fn spec(
    max_hold_blocks: u64,
    options: &LiveStrategySpecOptions,
    price_to_initial_entry_cap: bool,
) -> LiveStrategySpec {
    let price_gate_name = if price_to_initial_entry_cap {
        "-price-to-initial-lte1p5"
    } else {
        ""
    };
    let price_gate_label = if price_to_initial_entry_cap {
        " price-to-initial <= 1.5"
    } else {
        ""
    };
    LiveStrategySpec {
        strategy_name: format!(
            "alpha11-live-univ2-lp30{price_gate_name}-pool-update-block-hold{max_hold_blocks}"
        ),
        strategy_impl: STRATEGY_IMPL.to_string(),
        strategy_label: format!(
            "Alpha11 live Uniswap V2 LP30{price_gate_label} pool-update-block hold {max_hold_blocks}"
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
        max_entry_price_ratio_to_initial: price_to_initial_entry_cap
            .then(|| MAX_ENTRY_PRICE_RATIO_TO_INITIAL.to_string()),
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
        min_sell_pool_denom_reserve: Some(MIN_SELL_POOL_DENOM_RESERVE.to_string()),
        entry_bankroll_eth: Some(INITIAL_ENTRY_BANKROLL_ETH.to_string()),
        stop_loss_ratio: options.stop_loss_ratio.clone(),
        take_profit_ratio: options.take_profit_ratio.clone(),
        max_hold_blocks: Some(max_hold_blocks),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpha11::{DEPLOY_HOLD15_STRATEGY_NAME, HOLD15_STRATEGY_NAME};

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
        assert!(specs
            .iter()
            .all(|spec| spec.allowed_protocols == vec!["UNISWAP-V2".to_string()]));
    }

    #[test]
    fn deploy_set_adds_price_to_initial_entry_cap() {
        let spec = deploy_hold15_spec(&LiveStrategySpecOptions::default());

        assert_eq!(spec.strategy_name, DEPLOY_HOLD15_STRATEGY_NAME);
        assert_eq!(
            spec.max_entry_price_ratio_to_initial.as_deref(),
            Some(MAX_ENTRY_PRICE_RATIO_TO_INITIAL)
        );
        assert!(spec.strategy_label.contains("price-to-initial <= 1.5"));
    }
}
