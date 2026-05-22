//! Real on-chain execution adapter.
//!
//! The engine does not build live transaction calldata directly. It asks a
//! live transaction planner, owned by `alpha/live/trading`, to turn an approved
//! `OrderIntent` into a `LiveTraderTxSignal`. The adapter then submits that
//! signal through Kartal's ETH tx executor endpoint and maps the response back
//! into an `ExecutionReport`, so the existing position store path remains the
//! single owner of position state transitions.

use std::sync::atomic::{AtomicU64, Ordering};

use alloy_primitives::B256;
use async_trait::async_trait;
use eth_alpha_core::{
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus, MinedExecutionEvidence},
    ids::{BlockNumber, OrderId, TxHash},
    order::OrderIntent,
};
use eth_live_trading::{
    KartalExecutorClient, KartalSubmitDirectRawResult, LiveDirectRawTransactionRequest,
    LivePrioritySellPlannerInput, LiveTraderTxSignal, PrioritySellPlanner,
    PrioritySellPlannerOutcome,
};
use serde_json::Value;

use crate::EngineExecutionAdapter;

#[async_trait]
pub trait LiveTxPlanner: Send + Sync {
    /// Build the direct-raw transaction signal for a risk-approved order.
    ///
    /// Implementations belong in `alpha/live/trading` or code that wraps that
    /// crate's tx-prep modules. The engine should receive a prepared signal;
    /// it should not perform route discovery, calldata construction, slippage
    /// policy, or gas-rank selection itself.
    async fn prepare_signal(&self, intent: &OrderIntent) -> Result<LiveTraderTxSignal>;
}

#[async_trait]
pub trait LiveTxSubmitter: Send + Sync {
    /// Submit an already-prepared live transaction signal to the executor boundary.
    async fn submit_signal(
        &self,
        signal: &LiveTraderTxSignal,
    ) -> std::result::Result<KartalSubmitDirectRawResult, String>;
}

#[async_trait]
pub trait LiveTxPlanningInputResolver: Send + Sync {
    /// Resolve the engine's approved order into the full live-planning context.
    ///
    /// The engine knows the order intent, but the live planner also needs the
    /// matched open position, latest pool snapshot, wallet tx context, and
    /// observation metadata. This resolver is the runtime boundary for loading
    /// that state from stores and feeds.
    async fn resolve_priority_sell_input(
        &self,
        intent: &OrderIntent,
    ) -> Result<LivePrioritySellPlannerInput>;
}

pub struct LiveTradingPlannerBridge<P, R> {
    planner: P,
    resolver: R,
}

impl<P, R> LiveTradingPlannerBridge<P, R> {
    pub fn new(planner: P, resolver: R) -> Self {
        Self { planner, resolver }
    }
}

#[async_trait]
impl<P, R> LiveTxPlanner for LiveTradingPlannerBridge<P, R>
where
    P: PrioritySellPlanner,
    R: LiveTxPlanningInputResolver,
{
    async fn prepare_signal(&self, intent: &OrderIntent) -> Result<LiveTraderTxSignal> {
        let input = self.resolver.resolve_priority_sell_input(intent).await?;
        match self.planner.plan_priority_sell(input).await {
            Ok(PrioritySellPlannerOutcome::Submit { signal, .. }) => Ok(signal),
            Ok(PrioritySellPlannerOutcome::Reject(reject)) => {
                Err(AlphaCoreError::Execution(format!(
                    "priority sell tx prep rejected: {} {}",
                    reject.reason, reject.metadata
                )))
            }
            Err(error) => Err(AlphaCoreError::Execution(format!(
                "priority sell planner failed: {error}"
            ))),
        }
    }
}

#[async_trait]
impl LiveTxSubmitter for KartalExecutorClient {
    async fn submit_signal(
        &self,
        signal: &LiveTraderTxSignal,
    ) -> std::result::Result<KartalSubmitDirectRawResult, String> {
        KartalExecutorClient::submit_signal(self, signal)
            .await
            .map_err(|error| error.to_string())
    }
}

pub struct TxExecutorAdapter<P, S> {
    planner: P,
    submitter: S,
    order_prefix: String,
    next_order_id: AtomicU64,
}

impl<P, S> TxExecutorAdapter<P, S> {
    #[cfg(test)]
    pub fn new(planner: P, submitter: S) -> Self {
        Self::with_order_prefix(planner, submitter, "live-tx")
    }

    pub fn with_order_prefix(planner: P, submitter: S, order_prefix: impl Into<String>) -> Self {
        Self {
            planner,
            submitter,
            order_prefix: order_prefix.into(),
            next_order_id: AtomicU64::new(0),
        }
    }

    fn next_order_id(&self) -> OrderId {
        let sequence = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        OrderId(format!("{}-{sequence}", self.order_prefix))
    }
}

#[async_trait]
impl<P, S> EngineExecutionAdapter for TxExecutorAdapter<P, S>
where
    P: LiveTxPlanner,
    S: LiveTxSubmitter,
{
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let mut signal = match self.planner.prepare_signal(&intent).await {
            Ok(signal) => signal,
            Err(error) => {
                return Ok(failed_report(
                    self.next_order_id(),
                    format!("live tx planning failed: {error}"),
                    None,
                ));
            }
        };

        let order_id = signal
            .request
            .attempt_id
            .as_deref()
            .map(str::trim)
            .filter(|attempt_id| !attempt_id.is_empty())
            .map(|attempt_id| OrderId(attempt_id.to_string()))
            .unwrap_or_else(|| self.next_order_id());
        signal.request.attempt_id = Some(order_id.0.clone());

        let observed_block = signal.observed_block;
        let result = match self.submitter.submit_signal(&signal).await {
            Ok(result) => result,
            Err(error) => {
                return Ok(failed_report(
                    order_id,
                    format!("Kartal tx executor submission failed: {error}"),
                    observed_block,
                ));
            }
        };

        Ok(execution_report_from_kartal_result(
            order_id,
            result,
            observed_block,
            &signal.request,
        ))
    }
}

fn execution_report_from_kartal_result(
    order_id: OrderId,
    result: KartalSubmitDirectRawResult,
    observed_block: Option<BlockNumber>,
    request: &LiveDirectRawTransactionRequest,
) -> ExecutionReport {
    let status_key = result.status.trim().to_ascii_lowercase();
    let status = match status_key.as_str() {
        "broadcast" => ExecutionStatus::Submitted,
        "received" | "signed" => ExecutionStatus::Pending,
        "dry_run" => ExecutionStatus::Cancelled,
        "rejected" | "broadcast_error" => ExecutionStatus::Failed,
        _ => ExecutionStatus::Failed,
    };
    let (tx_hash, tx_hash_error) = parse_tx_hash(result.tx_hash.as_deref());
    let error = report_error(&status, &status_key, result.error, tx_hash_error);

    ExecutionReport {
        order_id,
        status,
        tx_hash,
        block_number: observed_block,
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: Some(submission_evidence(observed_block, request)),
        error,
    }
}

fn submission_evidence(
    observed_block: Option<BlockNumber>,
    request: &LiveDirectRawTransactionRequest,
) -> MinedExecutionEvidence {
    let bribe = request.bribe.as_ref();
    let metadata = &request.metadata;
    MinedExecutionEvidence {
        submitted_block_number: observed_block,
        selected_gas_limit: non_empty_string(&request.gas_limit),
        selected_max_fee_per_gas_wei: non_empty_string(&request.max_fee_per_gas),
        selected_max_priority_fee_per_gas_wei: non_empty_string(&request.max_priority_fee_per_gas),
        selected_bribe_priority_fee_per_gas_wei: bribe
            .and_then(|bribe| non_empty_string(&bribe.priority_fee_per_gas)),
        selected_bribe_max_fee_per_gas_wei: bribe
            .and_then(|bribe| bribe.max_fee_per_gas.as_deref())
            .and_then(non_empty_string),
        gas_policy_action: metadata_string(metadata, &["gas_policy", "action"])
            .or_else(|| metadata_string(metadata, &["intent_kind"])),
        gas_policy_signal: metadata_string(metadata, &["gas_policy", "signal"])
            .or_else(|| metadata_string(metadata, &["signal_source"]))
            .or_else(|| metadata_string(metadata, &["reason"])),
        gas_policy_status: metadata_string(metadata, &["gas_policy", "status"]),
        gas_policy_profile: metadata_string(metadata, &["gas_policy", "selected_profile"])
            .or_else(|| metadata_string(metadata, &["gas_plan", "label"])),
        gas_policy_profiles: metadata_string_array(metadata, &["gas_policy", "profiles"]).or_else(
            || metadata_string_array(metadata, &["strategy_gas_rank_policy", "preference_order"]),
        ),
        gas_rank_source: metadata_string(metadata, &["gas_policy", "gas_rank_source"])
            .or_else(|| metadata_string(metadata, &["gas_plan", "source"])),
        gas_estimated_max_cost_eth: metadata_string(
            metadata,
            &["gas_policy", "estimated_max_cost_eth"],
        )
        .or_else(|| metadata_string(metadata, &["gas_plan", "estimated_max_cost_eth"])),
        gas_estimated_priority_spend_eth: metadata_string(
            metadata,
            &["gas_policy", "estimated_priority_spend_eth"],
        )
        .or_else(|| metadata_string(metadata, &["gas_plan", "estimated_priority_spend_eth"])),
        gas_policy_guard: metadata_string(metadata, &["gas_policy", "guard"])
            .or_else(|| metadata_string(metadata, &["gas_policy", "economic_guard"])),
        ..MinedExecutionEvidence::default()
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn metadata_string(metadata: &Value, path: &[&str]) -> Option<String> {
    let value = metadata_path(metadata, path)?;
    let text = value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string());
    non_empty_string(&text)
}

fn metadata_string_array(metadata: &Value, path: &[&str]) -> Option<Vec<String>> {
    let values = metadata_path(metadata, path)?.as_array()?;
    let strings = values
        .iter()
        .filter_map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .or_else(|| Some(value.to_string()))
                .and_then(|text| non_empty_string(&text))
        })
        .collect::<Vec<_>>();
    (!strings.is_empty()).then_some(strings)
}

fn metadata_path<'a>(metadata: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(metadata, |value, key| value.get(*key))
}

fn parse_tx_hash(value: Option<&str>) -> (Option<TxHash>, Option<String>) {
    let Some(value) = value else {
        return (None, None);
    };
    match value.parse::<B256>() {
        Ok(hash) => (Some(hash), None),
        Err(error) => (
            None,
            Some(format!("invalid tx hash from Kartal tx executor: {error}")),
        ),
    }
}

fn report_error(
    status: &ExecutionStatus,
    status_key: &str,
    executor_error: Option<String>,
    tx_hash_error: Option<String>,
) -> Option<String> {
    if let Some(error) = executor_error.filter(|error| !error.trim().is_empty()) {
        return Some(error);
    }
    if let Some(error) = tx_hash_error {
        return Some(error);
    }
    match status {
        ExecutionStatus::Cancelled if status_key == "dry_run" => {
            Some("tx executor dry-run; transaction was not broadcast".to_string())
        }
        ExecutionStatus::Failed => Some(format!("tx executor returned status {status_key}")),
        _ => None,
    }
}

fn failed_report(
    order_id: OrderId,
    reason: impl Into<String>,
    block_number: Option<BlockNumber>,
) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Failed,
        tx_hash: None,
        block_number,
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: Some(reason.into()),
    }
}

#[cfg(test)]
mod tests {
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
        LiveDirectRawTransactionRequest, LivePrioritySellPlannerError,
        LivePrioritySellPlannerInput, PlannerTxContext, PriorityFeeBudget, PriorityFeeBudgetInput,
        PrioritySellPlannerOutcome, TxPrepRequestContext,
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
        result: Mutex<Option<std::result::Result<KartalSubmitDirectRawResult, String>>>,
    }

    #[async_trait]
    impl LiveTxSubmitter for FixedSubmitter {
        async fn submit_signal(
            &self,
            _signal: &LiveTraderTxSignal,
        ) -> std::result::Result<KartalSubmitDirectRawResult, String> {
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
                        "profiles": ["balanced", "minimum"],
                        "selected_profile": "balanced",
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

    fn submit_result(status: &str, error: Option<&str>) -> KartalSubmitDirectRawResult {
        KartalSubmitDirectRawResult {
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
    async fn broadcast_result_becomes_submitted_report() {
        let adapter = TxExecutorAdapter::new(
            FixedPlanner {
                signal: signal(Some("attempt-1")),
            },
            FixedSubmitter {
                result: Mutex::new(Some(Ok(submit_result("broadcast", None)))),
            },
        );

        let report = adapter.execute(intent()).await.unwrap();

        assert_eq!(report.order_id, OrderId("attempt-1".to_string()));
        assert_eq!(report.status, ExecutionStatus::Submitted);
        assert!(report.tx_hash.is_some());
        assert_eq!(report.block_number, Some(25_128_246));
        let evidence = report.mined_evidence.expect("submitted evidence");
        assert_eq!(evidence.gas_policy_action.as_deref(), Some("entry_buy"));
        assert_eq!(evidence.gas_policy_profile.as_deref(), Some("balanced"));
        assert_eq!(
            evidence.gas_policy_profiles.as_deref(),
            Some(["balanced".to_string(), "minimum".to_string()].as_slice())
        );
    }

    #[tokio::test]
    async fn dry_run_result_is_cancelled_because_nothing_was_broadcast() {
        let adapter = TxExecutorAdapter::new(
            FixedPlanner {
                signal: signal(Some("attempt-1")),
            },
            FixedSubmitter {
                result: Mutex::new(Some(Ok(submit_result("dry_run", None)))),
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
}
