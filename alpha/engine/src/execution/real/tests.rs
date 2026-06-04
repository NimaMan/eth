use std::sync::Mutex;

use alloy_primitives::{Address, U256};
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    error::AlphaCoreError,
    ids::{PoolAddress, PortfolioId, PositionId, StrategyName, TradeId, WalletId},
    market::{PoolProtocol, PoolSnapshot},
    order::{OrderSide, RouteHint},
    position::{Position, PositionKey, PositionState},
};
use eth_live_trading::{
    LiveDirectRawTransactionRequest, LivePrioritySellPlannerError, LivePrioritySellPlannerInput,
    PlannerTxContext, PriorityFeeBudget, PriorityFeeBudgetInput, PrioritySellPlannerOutcome,
    TxPrepRequestContext, TxSubmissionPolicy,
};
use serde_json::{json, Value};

use super::*;

#[derive(Clone)]
struct FixedPlanner {
    signal: LiveTraderTxSignal,
}

#[async_trait]
impl LiveTxPlanner for FixedPlanner {
    async fn prepare_signal(&self, _intent: &OrderIntent) -> Result<LiveTraderTxSignal> {
        Ok(self.signal.clone())
    }
}

struct FailingPlanner;

#[async_trait]
impl LiveTxPlanner for FailingPlanner {
    async fn prepare_signal(&self, _intent: &OrderIntent) -> Result<LiveTraderTxSignal> {
        Err(AlphaCoreError::Execution("route not available".to_string()))
    }
}

struct CancellingPlanner;

#[async_trait]
impl LiveTxPlanner for CancellingPlanner {
    async fn prepare_signal(&self, _intent: &OrderIntent) -> Result<LiveTraderTxSignal> {
        Err(AlphaCoreError::ExecutionCancelled {
            reason: "priority sell tx prep rejected: gas_rank_exceeds_value_cap {}".to_string(),
            block_number: Some(25_159_022),
        })
    }
}

struct DeferredPlanner;

#[async_trait]
impl LiveTxPlanner for DeferredPlanner {
    async fn prepare_signal(&self, _intent: &OrderIntent) -> Result<LiveTraderTxSignal> {
        Err(AlphaCoreError::ExecutionDeferred {
            reason: "simulation state not ready".to_string(),
            block_number: Some(25_128_247),
        })
    }
}

#[derive(Clone)]
struct FixedPrioritySellPlanner {
    outcome: PrioritySellPlannerOutcome,
}

#[async_trait]
impl eth_live_trading::PrioritySellPlanner for FixedPrioritySellPlanner {
    async fn plan_priority_sell(
        &self,
        _input: LivePrioritySellPlannerInput,
    ) -> std::result::Result<PrioritySellPlannerOutcome, LivePrioritySellPlannerError> {
        Ok(self.outcome.clone())
    }
}

struct RevertingPrioritySellPlanner;

#[async_trait]
impl eth_live_trading::PrioritySellPlanner for RevertingPrioritySellPlanner {
    async fn plan_priority_sell(
        &self,
        _input: LivePrioritySellPlannerInput,
    ) -> std::result::Result<PrioritySellPlannerOutcome, LivePrioritySellPlannerError> {
        Err(LivePrioritySellPlannerError::Simulation(
                "cannot derive min-output from failed provisional simulation: pre-submit simulation indicates the sell would revert".to_string(),
            ))
    }
}

struct CannotSellPrioritySellPlanner;

#[async_trait]
impl eth_live_trading::PrioritySellPlanner for CannotSellPrioritySellPlanner {
    async fn plan_priority_sell(
        &self,
        _input: LivePrioritySellPlannerInput,
    ) -> std::result::Result<PrioritySellPlannerOutcome, LivePrioritySellPlannerError> {
        Err(LivePrioritySellPlannerError::InvalidInput(
            "pool snapshot says can_sell=false".to_string(),
        ))
    }
}

struct FixedPlanningInputResolver {
    input: LivePrioritySellPlannerInput,
}

#[async_trait]
impl LiveTxPlanningInputResolver for FixedPlanningInputResolver {
    async fn resolve_priority_sell_input(
        &self,
        _intent: &OrderIntent,
    ) -> Result<LivePrioritySellPlannerInput> {
        Ok(self.input.clone())
    }
}

struct FixedSubmitter {
    result: Mutex<Option<std::result::Result<LiveTxSubmissionResult, String>>>,
}

#[async_trait]
impl LiveTxSubmitter for FixedSubmitter {
    async fn submit_signal(
        &self,
        _signal: &LiveTraderTxSignal,
    ) -> std::result::Result<LiveTxSubmissionResult, String> {
        self.result
            .lock()
            .expect("submitter result lock")
            .take()
            .expect("submitter result")
    }
}

fn intent() -> OrderIntent {
    OrderIntent {
        trade_id: None,
        portfolio_id: PortfolioId::from("portfolio"),
        wallet_id: WalletId::from("wallet"),
        strategy_name: StrategyName::from("strategy"),
        side: OrderSide::Sell,
        token_address: Address::with_last_byte(0x11),
        pool_address: PoolAddress::from("0xtoken:0xpool"),
        protocol: PoolProtocol::UniswapV2,
        amount: Amount {
            raw: U256::from(1u64),
            decimals: 18,
        },
        route: None::<RouteHint>,
        max_slippage_bps: 500,
        deadline_secs: 60,
        decision_reason: None,
    }
}

fn signal(attempt_id: Option<&str>) -> LiveTraderTxSignal {
    LiveTraderTxSignal {
        strategy_name: "strategy".to_string(),
        strategy_run_id: Some("run-1".to_string()),
        trade_id: None,
        token_address: Some(Address::with_last_byte(0x11)),
        pool_address: Some(PoolAddress::from("0xtoken:0xpool")),
        observed_block: Some(25_128_246),
        submission_policy: TxSubmissionPolicy::PublicRpcBroadcast,
        request: LiveDirectRawTransactionRequest {
            attempt_id: attempt_id.map(str::to_string),
            chain_id: 1,
            from: "0x0000000000000000000000000000000000000001".to_string(),
            to: "0x0000000000000000000000000000000000000002".to_string(),
            value: "0".to_string(),
            data: "0x".to_string(),
            gas_limit: "21000".to_string(),
            max_fee_per_gas: "1000000000".to_string(),
            max_priority_fee_per_gas: "1000000000".to_string(),
            nonce: None,
            bribe: None,
            simulation: None,
            metadata: json!({
                "gas_policy": {
                    "action": "entry_buy",
                    "signal": "entry.buy_eligible_pool_once",
                    "status": "selected",
                    "profiles": ["p50", "normal"],
                    "selected_profile": "p50",
                    "gas_rank_source": "chain_server_gas_rank",
                    "guard": "entry_estimated_gas_fee_cap",
                    "estimated_max_cost_eth": "0.0021",
                    "estimated_priority_spend_eth": "0.0002"
                }
            }),
        },
    }
}

fn planner_budget() -> PriorityFeeBudget {
    PriorityFeeBudget::from_input(&PriorityFeeBudgetInput {
        protected_exit_value_eth: DecimalAmount::new(10, 3),
        expected_late_recovery_eth: DecimalAmount::ZERO,
        safety_buffer_eth: DecimalAmount::ZERO,
        predicted_base_fee_gwei: DecimalAmount::from(10),
        estimated_gas_used: 100_000,
        max_total_fee_eth: DecimalAmount::new(10, 3),
        configured_max_priority_fee_gwei: DecimalAmount::from(50),
    })
}

fn planning_input(intent: OrderIntent) -> LivePrioritySellPlannerInput {
    let mut position = Position::with_trade_id(
        PositionId("pos-1".to_string()),
        TradeId("trade-1".to_string()),
        PositionKey {
            portfolio_id: intent.portfolio_id.clone(),
            wallet_id: intent.wallet_id.clone(),
            strategy_name: intent.strategy_name.clone(),
            token_address: intent.token_address,
            pool_address: intent.pool_address.clone(),
            protocol: intent.protocol.clone(),
        },
    );
    position.state = PositionState::BuyConfirmed;

    LivePrioritySellPlannerInput {
        context: PlannerTxContext {
            tx: TxPrepRequestContext {
                chain_id: 1,
                from: "0x0000000000000000000000000000000000000001".to_string(),
                strategy_name: intent.strategy_name.0.clone(),
                strategy_run_id: Some("run-1".to_string()),
                observed_block: Some(25_128_246),
                required_state_block: 25_128_246,
                source_metadata: json!({ "signal_id": 222 }),
            },
            current_block: 25_128_246,
            deadline_unix_secs: 1_800_000_000,
        },
        pool: PoolSnapshot {
            address: intent.pool_address.clone(),
            token_address: intent.token_address,
            protocol: intent.protocol.clone(),
            denom_address: None,
            denom_symbol: Some("WETH".to_string()),
            denom_reserve: DecimalAmount::from(10),
            token_reserve: DecimalAmount::from(1_000),
            price_denom_per_token: Some(DecimalAmount::new(1, 2)),
            initial_price_denom_per_token: Some(DecimalAmount::new(1, 2)),
            price_ratio_to_initial: Some(DecimalAmount::from(1)),
            creation_block: Some(25_128_246),
            token_decimals: Some(18),
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 25_128_246,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        },
        intent,
        position,
        min_output_amount: None,
        source_metadata: json!({ "observation_kind": "lp_approval" }),
    }
}

fn submit_result(status: &str, error: Option<&str>) -> EthTxSubmitDirectRawResult {
    EthTxSubmitDirectRawResult {
        attempt_id: "attempt-1".to_string(),
        status: status.to_string(),
        tx_hash: Some(format!("0x{}", "11".repeat(32))),
        from: "0x0000000000000000000000000000000000000001".to_string(),
        to: "0x0000000000000000000000000000000000000002".to_string(),
        nonce: json!("1"),
        gas_limit: json!("21000"),
        max_fee_per_gas: json!("1000000000"),
        max_priority_fee_per_gas: json!("1000000000"),
        error: error.map(str::to_string),
        elapsed_ms: Value::from(10),
    }
}

fn submission_result(status: &str, error: Option<&str>) -> LiveTxSubmissionResult {
    submit_result(status, error).into()
}

#[tokio::test]
async fn bridge_turns_priority_sell_planner_submit_into_signal() {
    let bridge = LiveTradingPlannerBridge::new(
        FixedPrioritySellPlanner {
            outcome: PrioritySellPlannerOutcome::Submit {
                signal: signal(Some("attempt-1")),
                budget: planner_budget(),
            },
        },
        FixedPlanningInputResolver {
            input: planning_input(intent()),
        },
    );

    let prepared = bridge.prepare_signal(&intent()).await.unwrap();

    assert_eq!(prepared.request.attempt_id.as_deref(), Some("attempt-1"));
    assert_eq!(prepared.observed_block, Some(25_128_246));
}

#[tokio::test]
async fn bridge_turns_reverting_pre_submit_simulation_into_cancelled_error() {
    let bridge = LiveTradingPlannerBridge::new(
        RevertingPrioritySellPlanner,
        FixedPlanningInputResolver {
            input: planning_input(intent()),
        },
    );

    let error = bridge.prepare_signal(&intent()).await.unwrap_err();

    match error {
        AlphaCoreError::ExecutionCancelled {
            reason,
            block_number,
        } => {
            assert_eq!(block_number, Some(25_128_246));
            assert!(reason.contains("would revert"));
            assert!(reason.contains("rejected before broadcast"));
        }
        other => panic!("expected cancelled execution, got {other:?}"),
    }
}

#[tokio::test]
async fn bridge_surfaces_invalid_planner_input_as_execution_error() {
    let bridge = LiveTradingPlannerBridge::new(
        CannotSellPrioritySellPlanner,
        FixedPlanningInputResolver {
            input: planning_input(intent()),
        },
    );

    let error = bridge.prepare_signal(&intent()).await.unwrap_err();

    match error {
        AlphaCoreError::Execution(reason) => {
            assert!(reason.contains("invalid planner input"));
            assert!(reason.contains("can_sell=false"));
        }
        other => panic!("expected execution error, got {other:?}"),
    }
}

#[tokio::test]
async fn broadcast_result_becomes_submitted_report() {
    let adapter = TxExecutorAdapter::new(
        FixedPlanner {
            signal: signal(Some("attempt-1")),
        },
        FixedSubmitter {
            result: Mutex::new(Some(Ok(submission_result("broadcast", None)))),
        },
    );

    let report = adapter.execute(intent()).await.unwrap();

    assert_eq!(report.order_id, OrderId("attempt-1".to_string()));
    assert_eq!(report.status, ExecutionStatus::Submitted);
    assert!(report.tx_hash.is_some());
    assert_eq!(report.block_number, Some(25_128_246));
    let evidence = report.mined_evidence.expect("submitted evidence");
    assert_eq!(evidence.gas_policy_action.as_deref(), Some("entry_buy"));
    assert_eq!(evidence.gas_policy_profile.as_deref(), Some("p50"));
    assert_eq!(
        evidence.gas_policy_profiles.as_deref(),
        Some(["p50".to_string(), "normal".to_string()].as_slice())
    );
}

#[tokio::test]
async fn public_tail_result_records_dependency_evidence() {
    let tail_hash = format!("0x{}", "33".repeat(32));
    let mut signal = signal(Some("attempt-1"));
    signal.submission_policy = TxSubmissionPolicy::PublicRpcBroadcast;
    signal.request.metadata["gas_policy"]["action"] = json!("tail_entry_buy");
    signal.request.metadata["tail_entry_ordering"] = json!({
        "tail_after_tx_hash": tail_hash.clone(),
        "dependency_priority_fee_wei": "2000000000"
    });
    let adapter = TxExecutorAdapter::new(
        FixedPlanner { signal },
        FixedSubmitter {
            result: Mutex::new(Some(Ok(LiveTxSubmissionResult {
                attempt_id: "attempt-1".to_string(),
                status: "broadcast".to_string(),
                tx_hash: Some(format!("0x{}", "11".repeat(32))),
                error: None,
                bundle_hash: None,
                bundle_target_block: None,
                bundle_max_block: None,
                bundle_tail_after_tx_hash: None,
            }))),
        },
    );

    let report = adapter.execute(intent()).await.unwrap();
    let evidence = report.mined_evidence.expect("submitted evidence");

    assert_eq!(report.status, ExecutionStatus::Submitted);
    assert_eq!(evidence.private_execution_transport, None);
    assert_eq!(evidence.bundle_target_block, None);
    assert_eq!(evidence.bundle_max_block, None);
    assert_eq!(
        evidence.gas_policy_tail_after_tx_hash.as_deref(),
        Some(tail_hash.as_str())
    );
    assert_eq!(
        evidence.gas_policy_dependency_priority_fee_wei.as_deref(),
        Some("2000000000")
    );
}

#[tokio::test]
async fn regular_broadcast_does_not_record_null_tail_dependency_evidence() {
    let mut signal = signal(Some("attempt-1"));
    signal.submission_policy = TxSubmissionPolicy::PublicRpcBroadcast;
    signal.request.metadata["gas_policy"]["action"] = json!("entry_buy");
    signal.request.metadata["tail_entry_ordering"] = json!({
        "tail_after_tx_hash": null,
        "dependency_priority_fee_wei": null,
        "dependency_gas_price_wei": null
    });
    let adapter = TxExecutorAdapter::new(
        FixedPlanner { signal },
        FixedSubmitter {
            result: Mutex::new(Some(Ok(LiveTxSubmissionResult {
                attempt_id: "attempt-1".to_string(),
                status: "broadcast".to_string(),
                tx_hash: Some(format!("0x{}", "11".repeat(32))),
                error: None,
                bundle_hash: None,
                bundle_target_block: None,
                bundle_max_block: None,
                bundle_tail_after_tx_hash: None,
            }))),
        },
    );

    let report = adapter.execute(intent()).await.unwrap();
    let evidence = report.mined_evidence.expect("submitted evidence");

    assert_eq!(report.status, ExecutionStatus::Submitted);
    assert_eq!(evidence.gas_policy_tail_after_tx_hash, None);
    assert_eq!(evidence.gas_policy_dependency_priority_fee_wei, None);
    assert_eq!(evidence.gas_policy_dependency_gas_price_wei, None);
}

#[tokio::test]
async fn dry_run_result_is_cancelled_because_nothing_was_broadcast() {
    let adapter = TxExecutorAdapter::new(
        FixedPlanner {
            signal: signal(Some("attempt-1")),
        },
        FixedSubmitter {
            result: Mutex::new(Some(Ok(submission_result("dry_run", None)))),
        },
    );

    let report = adapter.execute(intent()).await.unwrap();

    assert_eq!(report.status, ExecutionStatus::Cancelled);
    assert_eq!(
        report.error.as_deref(),
        Some("tx executor dry-run; transaction was not broadcast")
    );
}

#[tokio::test]
async fn planning_error_is_recorded_as_failed_report() {
    let adapter = TxExecutorAdapter::with_order_prefix(
        FailingPlanner,
        FixedSubmitter {
            result: Mutex::new(None),
        },
        "live-test",
    );

    let report = adapter.execute(intent()).await.unwrap();

    assert_eq!(report.order_id, OrderId("live-test-1".to_string()));
    assert_eq!(report.status, ExecutionStatus::Failed);
    assert_eq!(
        report.error.as_deref(),
        Some("live tx planning failed: execution adapter error: route not available")
    );
}

#[tokio::test]
async fn planner_rejection_is_recorded_as_cancelled_report() {
    let adapter = TxExecutorAdapter::with_order_prefix(
        CancellingPlanner,
        FixedSubmitter {
            result: Mutex::new(None),
        },
        "live-test",
    );

    let report = adapter.execute(intent()).await.unwrap();

    assert_eq!(report.order_id, OrderId("live-test-1".to_string()));
    assert_eq!(report.status, ExecutionStatus::Cancelled);
    assert_eq!(report.block_number, Some(25_159_022));
    assert_eq!(
        report.error.as_deref(),
        Some("priority sell tx prep rejected: gas_rank_exceeds_value_cap {}")
    );
}

#[tokio::test]
async fn deferred_planning_error_is_recorded_as_deferred_report() {
    let adapter = TxExecutorAdapter::with_order_prefix(
        DeferredPlanner,
        FixedSubmitter {
            result: Mutex::new(None),
        },
        "live-test",
    );

    let report = adapter.execute(intent()).await.unwrap();

    assert_eq!(report.order_id, OrderId("live-test-1".to_string()));
    assert_eq!(report.status, ExecutionStatus::Deferred);
    assert_eq!(report.block_number, Some(25_128_247));
    assert_eq!(report.error.as_deref(), Some("simulation state not ready"));
}
