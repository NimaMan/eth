use super::*;
use eth_strategies::alpha11::MAX_ENTRY_PRICE_RATIO_TO_INITIAL;

pub(super) fn build_strategy_specs(
    args: &Args,
    execution_mode: TraderExecutionMode,
) -> Result<Vec<LiveStrategySpec>> {
    let options = LiveStrategySpecOptions;

    let mut specs = if let Some(strategy_set) = args.strategy_set.as_deref() {
        strategy_set_specs(strategy_set, &options).map_err(|error| eyre!(error))?
    } else {
        vec![default_strategy_spec(&options)]
    };

    if execution_mode.uses_kartal() {
        apply_live_real_deploy_defaults(&mut specs);
    }

    Ok(specs)
}

fn apply_live_real_deploy_defaults(specs: &mut [LiveStrategySpec]) {
    for spec in specs {
        if spec.strategy_impl == ALPHA11_STRATEGY_IMPL {
            spec.max_entry_price_ratio_to_initial
                .get_or_insert_with(|| MAX_ENTRY_PRICE_RATIO_TO_INITIAL.to_string());
        }
    }
}

pub(super) fn live_strategy_spec_config_json(spec: &LiveStrategySpec) -> Value {
    json!({
        "strategy_name": spec.strategy_name,
        "strategy_impl": spec.strategy_impl,
        "strategy_label": spec.strategy_label,
        "strategy_runtime": STRATEGY_RUNTIME,
        "exit_liquidity_removal": spec.exit_liquidity_removal,
        "exit_tax": spec.exit_tax,
        "exit_lp_approval": spec.exit_lp_approval,
        "exit_lp_approval_critical_only": spec.exit_lp_approval_critical_only,
        "exit_scam": spec.exit_scam,
        "allowed_protocols": spec.allowed_protocols,
        "block_entry_on_lp_approval": spec.block_entry_on_lp_approval,
        "lp_approval_gate_min_pct": spec.lp_approval_gate_min_pct,
        "max_entry_price_ratio_to_initial": spec.max_entry_price_ratio_to_initial,
        "defer_buy_confirm_block_lp_approval_to_max_hold": spec.defer_buy_confirm_block_lp_approval_to_max_hold,
        "min_sell_pool_denom_reserve": spec.min_sell_pool_denom_reserve,
        "buy_wei": spec.buy_wei,
        "min_liquidity_eth": spec.min_liquidity_eth,
        "min_liquidity_usd": spec.min_liquidity_usd,
        "max_entry_pools": spec.max_entry_pools,
        "entry_bankroll_eth": &spec.entry_bankroll_eth,
        "stop_loss_ratio": spec.stop_loss_ratio,
        "take_profit_ratio": spec.take_profit_ratio,
        "max_hold_blocks": spec.max_hold_blocks,
    })
}

#[cfg(test)]
mod tests {
    use eth_strategies::{
        alpha11::{INITIAL_ENTRY_BANKROLL_ETH, MAX_ENTRY_PRICE_RATIO_TO_INITIAL},
        ALPHA11_HOLD15_STRATEGY_NAME, ALPHA11_STRATEGY_IMPL,
    };
    use serde_json::json;

    use super::*;
    use crate::live_trader::support::TraderExecutionMode;

    fn alpha11_hold15_args() -> Args {
        Args {
            poll_interval_ms: 2_000,
            mempool_since_days: 14,
            signal_limit: 200,
            run_id: None,
            disable_entry: false,
            replay_current: false,
            once: false,
            strategy_set: Some(ALPHA11_HOLD15_STRATEGY_NAME.to_string()),
        }
    }

    #[test]
    fn alpha11_hold15_visible_name_is_execution_mode_independent() {
        let args = alpha11_hold15_args();
        let chain_sim_specs =
            build_strategy_specs(&args, TraderExecutionMode::ChainSim).expect("chain-sim specs");
        let kartal_real_specs = build_strategy_specs(&args, TraderExecutionMode::KartalReal)
            .expect("kartal-real specs");

        assert_eq!(chain_sim_specs.len(), 1);
        assert_eq!(kartal_real_specs.len(), 1);
        assert_eq!(
            chain_sim_specs[0].strategy_name,
            ALPHA11_HOLD15_STRATEGY_NAME
        );
        assert_eq!(
            kartal_real_specs[0].strategy_name,
            ALPHA11_HOLD15_STRATEGY_NAME
        );
        assert_eq!(chain_sim_specs[0].max_entry_price_ratio_to_initial, None);
        assert_eq!(
            kartal_real_specs[0]
                .max_entry_price_ratio_to_initial
                .as_deref(),
            Some(MAX_ENTRY_PRICE_RATIO_TO_INITIAL)
        );
        assert_eq!(TraderExecutionMode::ChainSim.label(), "chain-sim");
        assert_eq!(TraderExecutionMode::KartalReal.label(), "kartal-real");
        assert_ne!(
            TraderExecutionMode::ChainSim.execution_model(),
            TraderExecutionMode::KartalReal.execution_model()
        );
    }

    #[test]
    fn alpha11_hold15_strategy_spine_matches_gate_two_contract() {
        let args = alpha11_hold15_args();
        let specs = build_strategy_specs(&args, TraderExecutionMode::ChainSim)
            .expect("alpha11 hold15 specs");

        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(spec.strategy_name, ALPHA11_HOLD15_STRATEGY_NAME);
        assert_eq!(spec.strategy_impl, ALPHA11_STRATEGY_IMPL);
        assert_eq!(spec.allowed_protocols, vec!["UNISWAP-V2".to_string()]);
        assert!(spec.exit_liquidity_removal);
        assert!(spec.exit_tax);
        assert!(spec.exit_lp_approval);
        assert!(!spec.exit_lp_approval_critical_only);
        assert!(spec.exit_scam);
        assert!(spec.block_entry_on_lp_approval);
        assert_eq!(spec.lp_approval_gate_min_pct.as_deref(), Some("30"));
        assert_eq!(spec.max_entry_price_ratio_to_initial, None);
        assert!(spec.defer_buy_confirm_block_lp_approval_to_max_hold);
        assert_eq!(spec.min_sell_pool_denom_reserve.as_deref(), Some("0"));
        assert_eq!(spec.buy_wei, "10000000000000000");
        assert_eq!(spec.min_liquidity_eth, "0.5");
        assert_eq!(spec.min_liquidity_usd, "1000");
        assert_eq!(spec.max_entry_pools, None);
        assert_eq!(
            spec.entry_bankroll_eth.as_deref(),
            Some(INITIAL_ENTRY_BANKROLL_ETH)
        );
        assert_eq!(spec.max_hold_blocks, Some(15));
        assert_eq!(spec.stop_loss_ratio, None);
        assert_eq!(spec.take_profit_ratio, None);

        assert_eq!(
            live_strategy_spec_config_json(spec),
            json!({
                "strategy_name": ALPHA11_HOLD15_STRATEGY_NAME,
                "strategy_impl": ALPHA11_STRATEGY_IMPL,
                "strategy_label": "Alpha11 Uniswap V2 LP30 pool-update-block hold 15",
                "strategy_runtime": STRATEGY_RUNTIME,
                "exit_liquidity_removal": true,
                "exit_tax": true,
                "exit_lp_approval": true,
                "exit_lp_approval_critical_only": false,
                "exit_scam": true,
                "allowed_protocols": ["UNISWAP-V2"],
                "block_entry_on_lp_approval": true,
                "lp_approval_gate_min_pct": "30",
                "max_entry_price_ratio_to_initial": null,
                "defer_buy_confirm_block_lp_approval_to_max_hold": true,
                "min_sell_pool_denom_reserve": "0",
                "buy_wei": "10000000000000000",
                "min_liquidity_eth": "0.5",
                "min_liquidity_usd": "1000",
                "max_entry_pools": null,
                "entry_bankroll_eth": INITIAL_ENTRY_BANKROLL_ETH,
                "stop_loss_ratio": null,
                "take_profit_ratio": null,
                "max_hold_blocks": 15,
            })
        );
    }

    #[test]
    fn alpha11_live_real_adds_price_gate_without_renaming_strategy() {
        let args = alpha11_hold15_args();

        let specs = build_strategy_specs(&args, TraderExecutionMode::KartalReal)
            .expect("alpha11 live-real hold15 specs");

        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(spec.strategy_name, ALPHA11_HOLD15_STRATEGY_NAME);
        assert_eq!(
            spec.max_entry_price_ratio_to_initial.as_deref(),
            Some(MAX_ENTRY_PRICE_RATIO_TO_INITIAL)
        );
        assert!(!spec.strategy_name.contains("price-to-initial"));
        assert!(!spec.strategy_label.contains("price-to-initial"));
    }
}
