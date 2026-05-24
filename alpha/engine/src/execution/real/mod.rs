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
        let cancellation_block = input
            .context
            .tx
            .observed_block
            .or(Some(input.context.current_block));
        match self.planner.plan_priority_sell(input).await {
            Ok(PrioritySellPlannerOutcome::Submit { signal, .. }) => Ok(signal),
            Ok(PrioritySellPlannerOutcome::Reject(reject)) => {
                Err(AlphaCoreError::ExecutionCancelled {
                    reason: format!(
                        "priority sell tx prep rejected: {} {}",
                        reject.reason, reject.metadata
                    ),
                    block_number: cancellation_block,
                })
            }
            Err(error) if error.is_deferrable() => Err(AlphaCoreError::ExecutionDeferred {
                reason: format!("priority sell planner deferred: {error}"),
                block_number: error.deferral_block_number(),
            }),
            Err(error) if error.is_pre_broadcast_reject() => {
                Err(AlphaCoreError::ExecutionCancelled {
                    reason: format!("priority sell planner rejected before broadcast: {error}"),
                    block_number: cancellation_block,
                })
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
            Err(AlphaCoreError::ExecutionDeferred {
                reason,
                block_number,
            }) => {
                return Ok(deferred_report(self.next_order_id(), reason, block_number));
            }
            Err(AlphaCoreError::ExecutionCancelled {
                reason,
                block_number,
            }) => {
                return Ok(cancelled_report(self.next_order_id(), reason, block_number));
            }
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
        "rejected" => ExecutionStatus::Cancelled,
        "broadcast_error" => ExecutionStatus::Failed,
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
        ExecutionStatus::Cancelled if status_key == "rejected" => {
            Some("tx executor rejected before broadcast".to_string())
        }
        ExecutionStatus::Failed => Some(format!("tx executor returned status {status_key}")),
        _ => None,
    }
}

fn cancelled_report(
    order_id: OrderId,
    reason: impl Into<String>,
    block_number: Option<BlockNumber>,
) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Cancelled,
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

fn deferred_report(
    order_id: OrderId,
    reason: impl Into<String>,
    block_number: Option<BlockNumber>,
) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Deferred,
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
mod tests;
