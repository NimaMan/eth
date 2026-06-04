use eth_alpha_core::{
    amount::Amount,
    decision_rationale::{DecisionReason, ReasonCategory},
    ids::{PortfolioId, PositionId, StrategyName, TokenPoolId, TradeId, WalletId},
    market::{PoolProtocol, PoolSnapshot},
    mempool_entry::MEMPOOL_ENTRY_EVIDENCE_VERSION,
    position::{Position, PositionKey},
};
use eth_live_trading::{
    EthTxDailySpendStatus, EthTxExecutorBroadcastMode, EthTxExecutorStatus, EthTxPolicyStatus,
    LivePrioritySellPlannerInput, PlannerTxContext, TxPrepRequestContext, TxSubmissionPolicy,
    TxSubmissionRoute,
};
use eth_strategies::{
    alpha11::HOLD16_STRATEGY_NAME,
    shared_rules::live::{strategy_set_specs, LiveStrategySpec, LiveStrategySpecOptions},
};
use rust_decimal::Decimal;

use super::super::cli::{Args, RealExecutionArgs};
use super::*;

fn live_args() -> Args {
    Args {
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

fn real_args(allow_broadcast_live_validation: bool) -> RealExecutionArgs {
    RealExecutionArgs {
        eth_tx_executor_url: "http://127.0.0.1:5006".to_string(),
        eth_tx_executor_token_env: "ETH_TX_EXECUTOR_API_TOKEN".to_string(),
        live_real_from: "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27".to_string(),
        live_real_vault_address: "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597".to_string(),
        allow_broadcast_live_validation,
    }
}

fn status(mode: EthTxExecutorBroadcastMode) -> EthTxExecutorStatus {
    EthTxExecutorStatus {
        service: "eth_tx_executor".to_string(),
        enabled: true,
        api_token_configured: true,
        signer_available: true,
        execution_disabled: false,
        broadcast_mode: mode,
        submission_policy_kinds: vec!["public_rpc_broadcast".to_string()],
        chain_id: 1,
        rpc_url: "http://172.18.0.1:8545".to_string(),
        journal_path: Some("/data/eth-tx-executions.jsonl".to_string()),
        direct_raw_endpoint: "/eth/tx/direct-raw".to_string(),
        submit_endpoint: Some("/eth/tx/submit".to_string()),
        policy: EthTxPolicyStatus {
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
            daily_spend: EthTxDailySpendStatus {
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

fn gas_policy() -> LiveRealGasPolicy {
    LiveRealGasPolicy {
        required_gas_rank_source: "eth_chain_server_gas_rank".to_string(),
        gas_rank_lookback_blocks: 100,
        gas_rank_priority_tie_breaker_gwei: Decimal::new(1456, 4),
        simulated_gas_buffer_bps: 2500,
        min_priority_fee_gwei: Decimal::ONE,
        max_priority_fee_gwei: Decimal::new(35, 1),
        entry_max_estimated_gas_fee_eth: Decimal::new(12, 4),
        exit_max_estimated_gas_fee_eth: Decimal::new(2, 3),
        safety_buffer_eth: Decimal::new(1, 3),
        v2_vault_buy_gas_limit: 300_000,
        v2_vault_sell_gas_limit: 300_000,
        mempool_race_priority_buffer_min_gwei: Decimal::new(1, 1),
        mempool_race_priority_buffer_max_gwei: Decimal::new(2, 1),
        tail_entry_priority_undercut_wei: 500,
        tail_entry_max_fee_buffer_bps: 1250,
        entry_buy_gas_rank_policy: StrategyGasRankPolicy::p75_first(),
        tail_entry_buy_gas_rank_policy: StrategyGasRankPolicy::p85_first()
            .with_submission_route(TxSubmissionRoute::PublicMempoolTail),
        normal_exit_gas_rank_policy: StrategyGasRankPolicy::p75_first(),
        mempool_pre_mine_gas_rank_policy: StrategyGasRankPolicy::p95_first(),
        lp_approval_exit_gas_rank_policy: StrategyGasRankPolicy::p90_first(),
    }
}

#[test]
fn dry_run_status_is_allowed_without_public_validation_flag() {
    validate_eth_tx_executor_real_status(
        &status(EthTxExecutorBroadcastMode::DryRun),
        &real_args(false),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap();
}

#[test]
fn broadcast_requires_explicit_validation_flag() {
    let error = validate_eth_tx_executor_real_status(
        &status(EthTxExecutorBroadcastMode::Broadcast),
        &real_args(false),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("--allow-broadcast-live-validation"));
}

#[test]
fn broadcast_hold16_deploy_rejects_other_strategy_scopes() {
    use eth_strategies::alpha11::{HOLD15_STRATEGY_NAME, HOLD3_VALIDATION_STRATEGY_NAME};

    for disallowed_strategy_set in [HOLD3_VALIDATION_STRATEGY_NAME, HOLD15_STRATEGY_NAME] {
        let mut args = live_args();
        args.strategy_set = Some(disallowed_strategy_set.to_string());

        let error = validate_eth_tx_executor_real_status(
            &status(EthTxExecutorBroadcastMode::Broadcast),
            &real_args(true),
            &args,
            &specs(&args),
        )
        .unwrap_err();

        assert!(error.to_string().contains(HOLD16_STRATEGY_NAME));
    }
}

#[test]
fn broadcast_hold16_deploy_accepts_hold16_scope() {
    validate_eth_tx_executor_real_status(
        &status(EthTxExecutorBroadcastMode::Broadcast),
        &real_args(true),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap();
}

#[test]
fn broadcast_hold16_deploy_accepts_disabled_daily_budget() {
    let mut status = status(EthTxExecutorBroadcastMode::Broadcast);
    status.policy.max_daily_cost_wei = "0".to_string();
    status.policy.daily_spend_cap_enabled = Some(false);
    status.policy.daily_spend.remaining_daily_cost_wei = None;

    validate_eth_tx_executor_real_status(
        &status,
        &real_args(true),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap();
}

#[test]
fn broadcast_hold16_deploy_requires_value_cap_for_buy() {
    let mut status = status(EthTxExecutorBroadcastMode::Broadcast);
    status.policy.max_value_wei = "0".to_string();

    let error = validate_eth_tx_executor_real_status(
        &status,
        &real_args(true),
        &live_args(),
        &specs(&live_args()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("max_value_wei"));
}

#[test]
fn tail_entry_buy_uses_public_submission_policy() {
    let tail_hash = format!("0x{}", "11".repeat(32));
    let gas_rank_policy = StrategyGasRankPolicy::p85_first()
        .with_submission_route(TxSubmissionRoute::PublicMempoolTail);
    let policy = buy_submission_policy(
        &gas_rank_policy,
        "tail_entry_buy",
        &Some(TailEntryOrderingEvidence {
            tail_after_tx_hash: Some(tail_hash.clone()),
            dependency_priority_fee_wei: Some("100".to_string()),
            dependency_max_fee_per_gas_wei: Some("1000".to_string()),
            dependency_gas_price_wei: None,
            signal_source: Some("mempool".to_string()),
            evidence_source: Some("mempool_processor".to_string()),
        }),
        25_128_246,
    )
    .unwrap();

    assert_eq!(policy, TxSubmissionPolicy::PublicRpcBroadcast);
}

#[test]
fn public_tail_entry_rejects_non_mempool_source() {
    let gas_rank_policy = StrategyGasRankPolicy::p85_first()
        .with_submission_route(TxSubmissionRoute::PublicMempoolTail);
    let error = buy_submission_policy(
        &gas_rank_policy,
        "tail_entry_buy",
        &Some(TailEntryOrderingEvidence {
            tail_after_tx_hash: Some(format!("0x{}", "11".repeat(32))),
            dependency_priority_fee_wei: Some("100".to_string()),
            dependency_max_fee_per_gas_wei: Some("1000".to_string()),
            dependency_gas_price_wei: None,
            signal_source: Some("unknown".to_string()),
            evidence_source: Some("unknown".to_string()),
        }),
        25_128_246,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("public_mempool_tail requires public mempool processor source evidence"));
}

#[test]
fn non_tail_entry_uses_broadcast_policy() {
    let gas_rank_policy = StrategyGasRankPolicy::p85_first()
        .with_submission_route(TxSubmissionRoute::PublicRpcBroadcast);
    let policy = buy_submission_policy(&gas_rank_policy, "entry_buy", &None, 25_128_246).unwrap();

    assert_eq!(policy, TxSubmissionPolicy::PublicRpcBroadcast);
}

#[test]
fn non_tail_entry_rejects_public_mempool_tail_submission_route() {
    let gas_rank_policy = StrategyGasRankPolicy::p85_first()
        .with_submission_route(TxSubmissionRoute::PublicMempoolTail);
    let error =
        buy_submission_policy(&gas_rank_policy, "entry_buy", &None, 25_128_246).unwrap_err();

    assert!(error.to_string().contains("requires tail_entry_buy action"));
}

#[test]
fn tail_entry_buy_can_use_broadcast_policy() {
    let gas_rank_policy = StrategyGasRankPolicy::p85_first()
        .with_submission_route(TxSubmissionRoute::PublicRpcBroadcast);
    let policy = buy_submission_policy(
        &gas_rank_policy,
        "tail_entry_buy",
        &Some(TailEntryOrderingEvidence {
            tail_after_tx_hash: Some(format!("0x{}", "11".repeat(32))),
            dependency_priority_fee_wei: Some("100".to_string()),
            dependency_max_fee_per_gas_wei: Some("1000".to_string()),
            dependency_gas_price_wei: None,
            signal_source: Some("mempool".to_string()),
            evidence_source: Some("mempool_processor".to_string()),
        }),
        25_128_246,
    )
    .unwrap();

    assert_eq!(policy, TxSubmissionPolicy::PublicRpcBroadcast);
}

#[test]
fn public_submission_keeps_unsigned_transaction_wire_protocol() {
    let policy = TxSubmissionPolicy::PublicRpcBroadcast;

    assert_eq!(transaction_wire_protocol(&policy), "eth_unsigned_tx");
    assert_eq!(executor_boundary(&policy), "eth_tx_executor");
}

#[test]
fn tail_entry_overlay_plan_uses_mempool_exact_vault_evidence() {
    let input = tail_entry_input(tail_entry_evidence_json(json!({})));

    let plan = tail_entry_overlay_plan(&input, "tail_entry_buy")
        .unwrap()
        .expect("tail-entry overlay plan");

    assert_eq!(plan.min_output, U256::from(950_000u64));
    assert_eq!(plan.simulation.block_number, 25_174_725);
    assert_eq!(plan.simulation.gas_used, Some(136_389));
    assert_eq!(
        plan.simulation.expected_output_amount.as_deref(),
        Some("1000000")
    );
    assert_eq!(
        plan.simulation
            .metadata
            .get("provider")
            .and_then(|value| value.as_str()),
        Some("mempool_entry_exact_vault_overlay")
    );
}

#[test]
fn tail_entry_overlay_plan_rejects_wrong_owner() {
    let input = tail_entry_input(tail_entry_evidence_json(json!({
        "owner_address": "0x0000000000000000000000000000000000000001"
    })));

    let error = tail_entry_overlay_plan(&input, "tail_entry_buy").unwrap_err();

    assert!(error.to_string().contains("does not match configured from"));
}

#[test]
fn entry_gas_fee_applies_strategy_min_priority_floor() {
    let policy = gas_policy();
    let route = PreparedSellRoute {
        protocol: "UniswapV2".to_string(),
        router_address: "0x0000000000000000000000000000000000000002".to_string(),
        calldata: "0x1234".to_string(),
        value_wei: "0".to_string(),
        gas_limit: 120_000,
        estimated_gas_used: Some(100_000),
        max_slippage_bps: Some(500),
    };
    let plan = GasRankPlan {
        predicted_base_fee_gwei: Decimal::new(1, 1),
        candidates: vec![RankedFeeCandidate {
            label: "p85".to_string(),
            priority_fee_gwei: Decimal::new(5, 1),
            max_fee_per_gas_gwei: Decimal::new(6, 1),
            rank_position_p50: Some(8),
            gas_before_p50: Some(210_000),
            likely_fits_at_p50: Some(true),
            source: Some(policy.required_gas_rank_source.clone()),
            metadata: None,
        }],
    };

    let fee = select_entry_gas_fee(
        &plan,
        &StrategyGasRankPolicy::p85_first(),
        &route,
        &policy,
        &None,
        25_128_246,
    )
    .unwrap();

    assert_eq!(fee.priority_fee_gwei, Decimal::ONE);
    assert_eq!(fee.max_fee_per_gas_gwei, Decimal::new(11, 1));
    assert_eq!(
        fee.metadata
            .as_ref()
            .and_then(|metadata| metadata["strategy_min_priority_fee_floor_applied"].as_bool()),
        Some(true)
    );
}

#[test]
fn tail_entry_gas_fee_undercuts_dependency_without_min_floor() {
    let policy = gas_policy();
    let route = PreparedSellRoute {
        protocol: "UniswapV2".to_string(),
        router_address: "0x0000000000000000000000000000000000000002".to_string(),
        calldata: "0x1234".to_string(),
        value_wei: "0".to_string(),
        gas_limit: 120_000,
        estimated_gas_used: Some(100_000),
        max_slippage_bps: Some(500),
    };
    let plan = GasRankPlan {
        predicted_base_fee_gwei: Decimal::new(38695358, 8),
        candidates: vec![RankedFeeCandidate {
            label: "p85".to_string(),
            priority_fee_gwei: Decimal::new(5, 1),
            max_fee_per_gas_gwei: Decimal::new(88695358, 8),
            rank_position_p50: Some(8),
            gas_before_p50: Some(210_000),
            likely_fits_at_p50: Some(true),
            source: Some(policy.required_gas_rank_source.clone()),
            metadata: None,
        }],
    };
    let ordering = Some(TailEntryOrderingEvidence {
        tail_after_tx_hash: Some(format!("0x{}", "11".repeat(32))),
        dependency_priority_fee_wei: Some("150000".to_string()),
        dependency_max_fee_per_gas_wei: Some("783964148".to_string()),
        dependency_gas_price_wei: None,
        signal_source: Some("mempool".to_string()),
        evidence_source: Some("mempool_processor".to_string()),
    });

    let fee = select_entry_gas_fee(
        &plan,
        &policy.tail_entry_buy_gas_rank_policy,
        &route,
        &policy,
        &ordering,
        25_128_246,
    )
    .unwrap();

    assert_eq!(fee.label, "public_tail_after_dependency");
    assert_eq!(fee.priority_fee_gwei, Decimal::new(1495, 7));
    assert!(fee.priority_fee_gwei < policy.min_priority_fee_gwei);
    assert_eq!(
        fee.metadata
            .as_ref()
            .and_then(|metadata| metadata["strategy_min_priority_fee_floor_applied"].as_bool()),
        Some(false)
    );
}

fn tail_entry_input(vault_buy_overrides: serde_json::Value) -> LivePrioritySellPlannerInput {
    let token = "0x57CA3bfB51FF4085f9afe662Bba9DECea79de79f"
        .parse()
        .unwrap();
    let vault = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
    let from = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
    let pool_address = TokenPoolId::new(token, "0x047b4653e2089792443235ad6b8fdb1f1a73e35c");
    let strategy_name = StrategyName(HOLD16_STRATEGY_NAME.to_string());
    let trade_id = TradeId("trd_tail_entry_test".to_string());
    let portfolio_id = PortfolioId("chain-sim".to_string());
    let wallet_id = WalletId("chain-sim-wallet".to_string());
    let decision_reason = DecisionReason {
        code: "entry.tail_after_enabling_tx".to_string(),
        category: ReasonCategory::Entry,
        label: "Entry: tail after enabling tx".to_string(),
        source: Some("mempool_signal".to_string()),
        raw: Some("entry.tail_after_enabling_tx".to_string()),
        details: json!({
            "risk_event_evidence": {
                "mempool_entry_evidence": {
                    "evidence_version": MEMPOOL_ENTRY_EVIDENCE_VERSION,
                    "base_block": 25_174_725,
                    "simulated_block": 25_174_725,
                    "dependency_tx_hashes": [format!("0x{}", "11".repeat(32))],
                    "projected_pool": {
                        "protocol": "UNISWAP-V2",
                        "denom_reserve": "1.1",
                        "token_reserve": "74000000000",
                        "pool_creation_block": 25_174_725,
                        "latest_block": 25_174_725,
                        "can_buy": true,
                        "can_sell": true
                    },
                    "viability": {
                        "can_buy": true,
                        "can_approve": true,
                        "can_sell": true
                    },
                    "vault_buy_simulation": merge_json(
                        json!({
                            "route": "uniswap_v2_trading_vault",
                            "chain_id": 1,
                            "vault_address": vault,
                            "owner_address": from,
                            "would_revert": false,
                            "gas_used": 136389,
                            "eth_spent_wei": HOLD16_DEPLOY_BUY_WEI,
                            "tokens_received_raw": "1000000",
                            "metadata": {
                                "exact_vault_calldata": true,
                                "expected_tokens_raw": "1000000",
                                "min_tokens_out": "950000"
                            }
                        }),
                        vault_buy_overrides
                    ),
                    "dependency_fee_metadata": {
                        "tail_after_tx_hash": format!("0x{}", "11".repeat(32))
                    }
                }
            }
        }),
    };
    let intent = OrderIntent {
        trade_id: Some(trade_id.clone()),
        portfolio_id: portfolio_id.clone(),
        wallet_id: wallet_id.clone(),
        strategy_name: strategy_name.clone(),
        side: OrderSide::Buy,
        token_address: token,
        pool_address: pool_address.clone(),
        protocol: PoolProtocol::UniswapV2,
        amount: Amount {
            raw: U256::from_str_radix(HOLD16_DEPLOY_BUY_WEI, 10).unwrap(),
            decimals: 18,
        },
        route: None,
        max_slippage_bps: 500,
        deadline_secs: 30,
        decision_reason: Some(decision_reason),
    };
    let position = Position::with_trade_id(
        PositionId(trade_id.0.clone()),
        trade_id,
        PositionKey {
            portfolio_id,
            wallet_id,
            strategy_name,
            token_address: token,
            pool_address: pool_address.clone(),
            protocol: PoolProtocol::UniswapV2,
        },
    );
    LivePrioritySellPlannerInput {
        context: PlannerTxContext {
            tx: TxPrepRequestContext {
                chain_id: 1,
                from: from.to_string(),
                strategy_name: HOLD16_STRATEGY_NAME.to_string(),
                strategy_run_id: Some("tail-entry-test-run".to_string()),
                observed_block: Some(25_174_726),
                required_state_block: 25_174_726,
                source_metadata: json!({ "test": "tail_entry" }),
            },
            current_block: 25_174_726,
            deadline_unix_secs: 1_800_000_000,
        },
        intent,
        position,
        pool: PoolSnapshot {
            address: pool_address,
            token_address: token,
            protocol: PoolProtocol::UniswapV2,
            denom_address: Some(
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(),
            ),
            denom_symbol: Some("WETH".to_string()),
            denom_reserve: Decimal::from_str_exact("1.1").unwrap(),
            token_reserve: Decimal::from(74_000_000_000u64),
            price_denom_per_token: None,
            initial_price_denom_per_token: None,
            price_ratio_to_initial: None,
            creation_block: Some(25_174_725),
            token_decimals: Some(18),
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 25_174_726,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        },
        min_output_amount: None,
        source_metadata: json!({ "test": "tail_entry" }),
    }
}

fn tail_entry_evidence_json(vault_buy_overrides: serde_json::Value) -> serde_json::Value {
    vault_buy_overrides
}

fn merge_json(mut base: serde_json::Value, overrides: serde_json::Value) -> serde_json::Value {
    if let (Some(base), Some(overrides)) = (base.as_object_mut(), overrides.as_object()) {
        for (key, value) in overrides {
            base.insert(key.clone(), value.clone());
        }
    }
    base
}
