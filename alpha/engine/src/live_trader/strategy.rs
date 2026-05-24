use eth_strategies::shared_rules::live::{
    default_strategy_spec, strategy_set_specs, LiveStrategySpec, LiveStrategySpecOptions,
    STRATEGY_RUNTIME,
};
use eyre::{eyre, Result};
use serde_json::{json, Value};

use super::{cli::Args, support::TraderExecutionMode};

pub(super) fn build_strategy_specs(
    args: &Args,
    _execution_mode: TraderExecutionMode,
) -> Result<Vec<LiveStrategySpec>> {
    let options = LiveStrategySpecOptions;

    let specs = if let Some(strategy_set) = args.strategy_set.as_deref() {
        strategy_set_specs(strategy_set, &options).map_err(|error| eyre!(error))?
    } else {
        vec![default_strategy_spec(&options)]
    };

    Ok(specs)
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
        "entry_init_policy": {
            "max_age_blocks": spec.entry_init_policy.max_age_blocks,
            "require_creation_block": spec.entry_init_policy.require_creation_block,
            "max_price_ratio_to_initial": spec.entry_init_policy.max_price_ratio_to_initial,
            "allow_missing_price_ratio": spec.entry_init_policy.allow_missing_price_ratio,
        },
        "defer_buy_confirm_block_lp_approval_to_max_hold": spec.defer_buy_confirm_block_lp_approval_to_max_hold,
        "lp_approval_exit_defer_max_trading_enabled_age_blocks": spec.lp_approval_exit_defer_max_trading_enabled_age_blocks,
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
        alpha11::{
            ENTRY_INIT_MAX_AGE_BLOCKS, ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL,
            INITIAL_ENTRY_BANKROLL_ETH, LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS,
        },
        ALPHA11_HOLD15_STRATEGY_NAME, ALPHA11_HOLD16_STRATEGY_NAME, ALPHA11_STRATEGY_IMPL,
    };
    use serde_json::json;

    use super::*;
    use crate::live_trader::support::TraderExecutionMode;

    fn alpha11_hold15_args() -> Args {
        Args {
            poll_interval_ms: Some(2_000),
            mempool_since_days: Some(14),
            signal_limit: Some(200),
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
        for spec in [&chain_sim_specs[0], &kartal_real_specs[0]] {
            assert_eq!(
                spec.entry_init_policy.max_age_blocks,
                Some(ENTRY_INIT_MAX_AGE_BLOCKS)
            );
            assert_eq!(
                spec.entry_init_policy.max_price_ratio_to_initial.as_deref(),
                Some(ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL)
            );
        }
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
        assert_eq!(
            spec.entry_init_policy.max_age_blocks,
            Some(ENTRY_INIT_MAX_AGE_BLOCKS)
        );
        assert_eq!(
            spec.entry_init_policy.max_price_ratio_to_initial.as_deref(),
            Some(ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL)
        );
        assert!(spec.defer_buy_confirm_block_lp_approval_to_max_hold);
        assert_eq!(
            spec.lp_approval_exit_defer_max_trading_enabled_age_blocks,
            Some(LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS)
        );
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
                "entry_init_policy": {
                    "max_age_blocks": ENTRY_INIT_MAX_AGE_BLOCKS,
                    "require_creation_block": false,
                    "max_price_ratio_to_initial": ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL,
                    "allow_missing_price_ratio": true,
                },
                "defer_buy_confirm_block_lp_approval_to_max_hold": true,
                "lp_approval_exit_defer_max_trading_enabled_age_blocks": LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS,
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
    fn alpha11_live_real_uses_explicit_init_policy() {
        let args = alpha11_hold15_args();

        let specs = build_strategy_specs(&args, TraderExecutionMode::KartalReal)
            .expect("alpha11 live-real hold15 specs");

        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(spec.strategy_name, ALPHA11_HOLD15_STRATEGY_NAME);
        assert_eq!(
            spec.entry_init_policy.max_age_blocks,
            Some(ENTRY_INIT_MAX_AGE_BLOCKS)
        );
        assert_eq!(
            spec.entry_init_policy.max_price_ratio_to_initial.as_deref(),
            Some(ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL)
        );
        assert!(!spec.strategy_name.contains("price-to-initial"));
        assert!(!spec.strategy_label.contains("price-to-initial"));
    }

    #[test]
    fn alpha11_hold16_strategy_set_resolves_single_deploy_spec() {
        let mut args = alpha11_hold15_args();
        args.strategy_set = Some(ALPHA11_HOLD16_STRATEGY_NAME.to_string());

        let specs = build_strategy_specs(&args, TraderExecutionMode::KartalReal)
            .expect("alpha11 live-real hold16 specs");

        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(spec.strategy_name, ALPHA11_HOLD16_STRATEGY_NAME);
        assert_eq!(spec.strategy_impl, ALPHA11_STRATEGY_IMPL);
        assert_eq!(spec.allowed_protocols, vec!["UNISWAP-V2".to_string()]);
        assert_eq!(spec.max_hold_blocks, Some(16));
        assert_eq!(
            spec.entry_bankroll_eth.as_deref(),
            Some(INITIAL_ENTRY_BANKROLL_ETH)
        );
        assert_eq!(spec.buy_wei, "10000000000000000");
        assert_eq!(spec.max_entry_pools, None);
    }
}
