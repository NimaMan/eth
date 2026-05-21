use super::*;

pub(super) fn build_strategy_specs(args: &Args) -> Result<Vec<LiveStrategySpec>> {
    let options = LiveStrategySpecOptions {
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks: args.max_hold_blocks,
    };

    if let Some(strategy_set) = args.strategy_set.as_deref() {
        return strategy_set_specs(strategy_set, &options).map_err(|error| eyre!(error));
    }

    Ok(vec![default_strategy_spec(&options)])
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
        "defer_buy_confirm_block_lp_approval_to_max_hold": spec.defer_buy_confirm_block_lp_approval_to_max_hold,
        "min_sell_pool_denom_reserve": spec.min_sell_pool_denom_reserve,
        "stop_loss_ratio": spec.stop_loss_ratio,
        "take_profit_ratio": spec.take_profit_ratio,
        "max_hold_blocks": spec.max_hold_blocks,
    })
}
