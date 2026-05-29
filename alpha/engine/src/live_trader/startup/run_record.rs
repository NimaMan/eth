use alloy_primitives::U256;
use eth_alpha_store::PostgresTradingStore;
use eth_strategies::shared_rules::live::{LiveStrategySpec, STRATEGY_RUNTIME};
use eyre::{Result, WrapErr};
use serde_json::{json, Value};

use super::cli::{Args, RealExecutionArgs};
use super::strategy::live_strategy_spec_config_json;
use super::support::TraderExecutionMode;

pub(super) struct RunStartRecord<'a> {
    pub(super) execution_mode: TraderExecutionMode,
    pub(super) observation_strategy_name: &'a str,
    pub(super) strategy_specs: &'a [LiveStrategySpec],
    pub(super) args: &'a Args,
    pub(super) real_args: Option<&'a RealExecutionArgs>,
    pub(super) process_started_at_text: &'a str,
    pub(super) process_started_at_unix_secs: i64,
    pub(super) token_server_url: &'a str,
    pub(super) reth_datadir: &'a str,
    pub(super) mempool_since_days: i64,
    pub(super) signal_limit: i64,
    pub(super) single_buy_wei: &'a Option<String>,
    pub(super) single_max_entry_pools: Option<usize>,
    pub(super) single_entry_bankroll_eth: &'a Option<String>,
    pub(super) single_entry_bankroll_wei: Option<U256>,
    pub(super) entry_bankroll_summary: &'a [Value],
    pub(super) single_min_liquidity_eth: &'a Option<String>,
    pub(super) single_min_liquidity_usd: &'a Option<String>,
    pub(super) gas_policy_metadata: Value,
    pub(super) run_session_source: &'a str,
    pub(super) run_session_path: Option<String>,
}

pub(super) async fn record_alpha_trader_run_start(
    store: &PostgresTradingStore,
    record: RunStartRecord<'_>,
) -> Result<()> {
    store
        .start_run(
            record.execution_mode.label(),
            json!({
                "strategy_name": record.observation_strategy_name,
                "strategy_impl": if record.strategy_specs.len() == 1 {
                    record.strategy_specs[0].strategy_impl.clone()
                } else {
                    "multi-strategy-set".to_string()
                },
                "strategy_label": if record.strategy_specs.len() == 1 {
                    record.strategy_specs[0].strategy_label.clone()
                } else {
                    "Strategy Set".to_string()
                },
                "strategy_set": record.args.strategy_set.clone(),
                "strategy_suite": record.args.strategy_set.clone(),
                "strategy_count": record.strategy_specs.len(),
                "strategies": record
                    .strategy_specs
                    .iter()
                    .map(live_strategy_spec_config_json)
                    .collect::<Vec<_>>(),
                "strategy_runtime": STRATEGY_RUNTIME,
                "observation_strategy_name": record.observation_strategy_name,
                "execution_model": record.execution_mode.execution_model(),
                "run_session": {
                    "source": record.run_session_source,
                    "path": record.run_session_path,
                },
                "entry_enabled": !record.args.disable_entry,
                "kartal": if let Some(real_args) = record.real_args {
                    json!({
                        "url": &real_args.kartal_url,
                        "token_env": &real_args.kartal_token_env,
                        "from": &real_args.live_real_from,
                        "vault_address": &real_args.live_real_vault_address,
                        "broadcast_requirement": if real_args.allow_broadcast_live_validation {
                            "dry_run_or_explicit_hold16_deploy_broadcast"
                        } else {
                            "dry_run"
                        },
                        "allow_broadcast_live_validation": real_args.allow_broadcast_live_validation,
                        "tail_entry_submission": "public_mempool_priority_undercut"
                    })
                } else {
                    Value::Null
                },
                "process_started_at": record.process_started_at_text,
                "process_started_at_unix_secs": record.process_started_at_unix_secs,
                "token_server_url": record.token_server_url,
                "reth_datadir": record.reth_datadir,
                "loop_wait": if record.execution_mode == TraderExecutionMode::ChainSim {
                    "chain_server_live_updates"
                } else {
                    "internal_real_mempool_signal_cadence"
                },
                "mempool_since_days": record.mempool_since_days,
                "signal_limit": record.signal_limit,
                "buy_wei": record.single_buy_wei,
                "max_entry_pools": record.single_max_entry_pools,
                "entry_bankroll_eth": record.single_entry_bankroll_eth,
                "entry_bankroll_wei": record.single_entry_bankroll_wei.map(|value| value.to_string()),
                "entry_bankrolls": record.entry_bankroll_summary,
                "min_liquidity_eth": record.single_min_liquidity_eth,
                "min_liquidity_usd": record.single_min_liquidity_usd,
                "replay_current": record.args.replay_current,
                "gas_policy": record.gas_policy_metadata,
            }),
        )
        .await
        .wrap_err("failed to record alpha trader run")
}
