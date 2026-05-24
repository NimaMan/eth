use eth_live_trading::{
    KartalDailySpendStatus, KartalEthTxPolicyStatus, KartalStatusBroadcastMode,
};
use eth_strategies::shared_rules::live::{strategy_set_specs, LiveStrategySpecOptions};

use super::*;

fn live_args() -> Args {
    Args {
        poll_interval_ms: Some(2_000),
        mempool_since_days: Some(14),
        signal_limit: Some(200),
        run_id: None,
        disable_entry: false,
        replay_current: false,
        once: false,
        strategy_set: Some(HOLD16_STRATEGY_NAME.to_string()),
    }
}

fn specs(args: &Args) -> Vec<LiveStrategySpec> {
    strategy_set_specs(
        args.strategy_set.as_deref().expect("test strategy set"),
        &LiveStrategySpecOptions,
    )
    .expect("test strategy specs")
}

fn real_args(allow_public_mempool_live_validation: bool) -> RealExecutionArgs {
    RealExecutionArgs {
        kartal_url: "http://127.0.0.1:5004".to_string(),
        kartal_token_env: "ETH_TX_EXECUTOR_API_TOKEN".to_string(),
        live_real_from: "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27".to_string(),
        live_real_vault_address: "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597".to_string(),
        allow_public_mempool_live_validation,
    }
}

fn status(mode: KartalStatusBroadcastMode) -> KartalEthTxExecutorStatus {
    KartalEthTxExecutorStatus {
        service: "eth_tx_executor".to_string(),
        enabled: true,
        api_token_configured: true,
        signer_available: true,
        execution_disabled: false,
        broadcast_mode: mode,
        chain_id: 1,
        rpc_url: "http://172.18.0.1:8545".to_string(),
        journal_path: Some("/data/eth-tx-executions.jsonl".to_string()),
        direct_raw_endpoint: "/eth/tx/direct-raw".to_string(),
        policy: KartalEthTxPolicyStatus {
            version: "eth_tx_policy_v1".to_string(),
            allowed_from_count: 1,
            allowed_target_count: 1,
            allowed_selector_count: 2,
            max_value_wei: HOLD16_DEPLOY_BUY_WEI.to_string(),
            max_gas_limit: 500_000,
            max_fee_per_gas_wei: "1000000000000".to_string(),
            max_priority_fee_per_gas_wei: "500000000000".to_string(),
            max_transaction_cost_wei: "30000000000000000".to_string(),
            max_daily_cost_wei: "50000000000000000".to_string(),
            daily_spend_cap_enabled: Some(true),
            daily_spend: KartalDailySpendStatus {
                spend_day: "2026-05-21".to_string(),
                spent_wei: "0".to_string(),
                remaining_daily_cost_wei: Some("50000000000000000".to_string()),
            },
            require_simulation: true,
            max_simulation_age_blocks: 2,
            required_metadata_fields: vec![
                "wire_protocol".to_string(),
                "intent_kind".to_string(),
                "strategy_name".to_string(),
                "strategy_run_id".to_string(),
                "trade_id".to_string(),
                "token_address".to_string(),
                "pool_address".to_string(),
            ],
        },
    }
}

#[test]
fn dry_run_status_is_allowed_without_public_validation_flag() {
    validate_kartal_real_status(
        &status(KartalStatusBroadcastMode::DryRun),
        &real_args(false),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap();
}

#[test]
fn public_mempool_requires_explicit_validation_flag() {
    let error = validate_kartal_real_status(
        &status(KartalStatusBroadcastMode::PublicMempool),
        &real_args(false),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("--allow-public-mempool-live-validation"));
}

#[test]
fn public_mempool_hold16_deploy_rejects_other_strategy_scopes() {
    use eth_strategies::alpha11::{HOLD15_STRATEGY_NAME, HOLD3_VALIDATION_STRATEGY_NAME};

    for disallowed_strategy_set in [HOLD3_VALIDATION_STRATEGY_NAME, HOLD15_STRATEGY_NAME] {
        let mut args = live_args();
        args.strategy_set = Some(disallowed_strategy_set.to_string());

        let error = validate_kartal_real_status(
            &status(KartalStatusBroadcastMode::PublicMempool),
            &real_args(true),
            &args,
            &specs(&args),
        )
        .unwrap_err();

        assert!(error.to_string().contains(HOLD16_STRATEGY_NAME));
    }
}

#[test]
fn public_mempool_hold16_deploy_accepts_hold16_scope() {
    validate_kartal_real_status(
        &status(KartalStatusBroadcastMode::PublicMempool),
        &real_args(true),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap();
}

#[test]
fn public_mempool_hold16_deploy_accepts_disabled_daily_budget() {
    let mut status = status(KartalStatusBroadcastMode::PublicMempool);
    status.policy.max_daily_cost_wei = "0".to_string();
    status.policy.daily_spend_cap_enabled = Some(false);
    status.policy.daily_spend.remaining_daily_cost_wei = None;

    validate_kartal_real_status(
        &status,
        &real_args(true),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap();
}

#[test]
fn public_mempool_hold16_deploy_requires_value_cap_for_buy() {
    let mut status = status(KartalStatusBroadcastMode::PublicMempool);
    status.policy.max_value_wei = "0".to_string();

    let error = validate_kartal_real_status(
        &status,
        &real_args(true),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("max_value_wei"));
}
