use eth_alpha_core::{
    amount::Amount,
    decision_rationale::{DecisionReason, ReasonCategory},
    ids::{PortfolioId, PositionId, StrategyName, TokenPoolId, TradeId, WalletId},
    market::{PoolProtocol, PoolSnapshot},
    mempool_entry::MEMPOOL_ENTRY_EVIDENCE_VERSION,
    position::{Position, PositionKey},
};
use eth_live_trading::{
    KartalDailySpendStatus, KartalEthTxExecutorStatus, KartalEthTxPolicyStatus,
    KartalStatusBroadcastMode, LivePrioritySellPlannerInput, PlannerTxContext, TxOrderingPolicy,
    TxPrepRequestContext,
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
        submit_endpoint: Some("/eth/tx/submit".to_string()),
        flashbots_tail_bundle_endpoint: Some("/eth/tx/flashbots/mev-share-tail".to_string()),
        flashbots_relay_url: Some("https://relay.flashbots.net".to_string()),
        flashbots_auth_configured: Some(true),
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
        entry_buy_gas_rank_policy: StrategyGasRankPolicy::p75_first(),
        tail_entry_buy_gas_rank_policy: StrategyGasRankPolicy::p85_first(),
        normal_exit_gas_rank_policy: StrategyGasRankPolicy::p75_first(),
        mempool_pre_mine_gas_rank_policy: StrategyGasRankPolicy::p95_first(),
        lp_approval_exit_gas_rank_policy: StrategyGasRankPolicy::p90_first(),
    }
}

#[test]
fn dry_run_status_is_allowed_without_public_validation_flag() {
    validate_kartal_real_status(
        &status(KartalStatusBroadcastMode::DryRun),
        &real_args(false),
        &live_args(),
        &specs(&live_args()),
        TailEntrySubmissionRoute::FlashbotsMevShare,
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
        TailEntrySubmissionRoute::FlashbotsMevShare,
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
            TailEntrySubmissionRoute::FlashbotsMevShare,
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
        TailEntrySubmissionRoute::FlashbotsMevShare,
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
        TailEntrySubmissionRoute::FlashbotsMevShare,
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
        TailEntrySubmissionRoute::FlashbotsMevShare,
    )
    .unwrap_err();

    assert!(error.to_string().contains("max_value_wei"));
}

#[test]
fn tail_entry_buy_uses_flashbots_submission_policy() {
    let tail_hash = format!("0x{}", "11".repeat(32));
    let policy = buy_submission_policy(
        TailEntrySubmissionRoute::FlashbotsMevShare,
        Some(3),
        "tail_entry_buy",
        &Some(TailEntryOrderingEvidence {
            tail_after_tx_hash: Some(tail_hash.clone()),
            dependency_priority_fee_wei: Some("100".to_string()),
            dependency_gas_price_wei: None,
        }),
        25_128_246,
    )
    .unwrap();

    assert_eq!(
        policy,
        TxSubmissionPolicy::FlashbotsMevShare {
            ordering: TxOrderingPolicy::TailAfter { tx_hash: tail_hash },
            target_block: Some(25_128_247),
            max_block: Some(25_128_249),
            can_revert: false,
        }
    );
}

#[test]
fn non_tail_entry_uses_public_mempool_policy() {
    let policy = buy_submission_policy(
        TailEntrySubmissionRoute::FlashbotsMevShare,
        Some(3),
        "entry_buy",
        &None,
        25_128_246,
    )
    .unwrap();

    assert_eq!(policy, TxSubmissionPolicy::PublicMempool);
}

#[test]
fn tail_entry_buy_can_use_public_mempool_policy() {
    let policy = buy_submission_policy(
        TailEntrySubmissionRoute::PublicMempool,
        None,
        "tail_entry_buy",
        &Some(TailEntryOrderingEvidence {
            tail_after_tx_hash: Some(format!("0x{}", "11".repeat(32))),
            dependency_priority_fee_wei: Some("100".to_string()),
            dependency_gas_price_wei: None,
        }),
        25_128_246,
    )
    .unwrap();

    assert_eq!(policy, TxSubmissionPolicy::PublicMempool);
}

#[test]
fn flashbots_tail_entry_keeps_unsigned_transaction_wire_protocol() {
    let policy = TxSubmissionPolicy::FlashbotsMevShare {
        ordering: TxOrderingPolicy::TailAfter {
            tx_hash: format!("0x{}", "11".repeat(32)),
        },
        target_block: Some(25_128_247),
        max_block: Some(25_128_249),
        can_revert: false,
    };

    assert_eq!(transaction_wire_protocol(&policy), "eth_unsigned_tx");
    assert_eq!(executor_boundary(&policy), "kartal_eth_tx_executor_policy");
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
