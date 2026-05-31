use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_executor::{
    BroadcastMode, DirectRawTransactionRequest, EthTxExecutor, EthTxExecutorError, ExecutionStatus,
    SubmitDirectRawResult,
};

use crate::error::EthTxServiceError;
use crate::repository::{policy_decision_json, EthTxPolicyRepository};

use super::config::{EthTxExecutorServiceConfig, EthTxSignerBackendConfig};
use super::policy::{evaluate_eth_tx_policy, EthTxPolicyEvaluation};

#[derive(Clone)]
pub struct EthTxExecutorAppState {
    config: Arc<EthTxExecutorServiceConfig>,
    executor: Option<Arc<EthTxExecutor>>,
    policy_repository: Arc<EthTxPolicyRepository>,
    rpc_http: reqwest::Client,
}

impl EthTxExecutorAppState {
    pub fn from_config(
        config: EthTxExecutorServiceConfig,
        policy_repository: EthTxPolicyRepository,
    ) -> Result<Self, EthTxServiceError> {
        let executor = match &config.signer_backend {
            EthTxSignerBackendConfig::Env => {
                let private_key_available = std::env::var(&config.private_key_env)
                    .ok()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false);
                if private_key_available {
                    Some(Arc::new(
                        EthTxExecutor::from_env(config.executor.clone(), &config.private_key_env)
                            .map_err(map_executor_error)?,
                    ))
                } else {
                    None
                }
            }
            EthTxSignerBackendConfig::UnixSocket {
                socket_path,
                signer_address,
            } => {
                let signer_address = signer_address.parse().map_err(|err| {
                    EthTxServiceError::validation(format!(
                        "invalid ETH tx signer address {signer_address:?}: {err}"
                    ))
                })?;
                Some(Arc::new(
                    EthTxExecutor::from_unix_socket(
                        config.executor.clone(),
                        socket_path.clone(),
                        signer_address,
                    )
                    .map_err(map_executor_error)?,
                ))
            }
        };
        Ok(Self {
            config: Arc::new(config),
            executor,
            policy_repository: Arc::new(policy_repository),
            rpc_http: reqwest::Client::new(),
        })
    }

    fn signer_available(&self) -> bool {
        self.executor.is_some()
    }

    fn api_token_configured(&self) -> bool {
        self.config
            .api_token
            .as_deref()
            .map(|token| !token.trim().is_empty())
            .unwrap_or(false)
    }

    fn execution_enabled(&self) -> bool {
        !self.config.execution_disabled
    }

    fn require_auth(&self, headers: &HeaderMap) -> Result<(), EthTxServiceError> {
        let expected = self
            .config
            .api_token
            .as_deref()
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| {
                EthTxServiceError::unavailable(
                    "no ETH tx executor bearer token is configured; set ETH_TX_EXECUTOR_API_TOKEN",
                )
            })?;

        let header = headers.get(header::AUTHORIZATION).ok_or_else(|| {
            EthTxServiceError::Auth("missing Authorization bearer token".to_string())
        })?;
        let header = header
            .to_str()
            .map_err(|_| EthTxServiceError::Auth("invalid Authorization header".to_string()))?;

        if header == format!("Bearer {expected}") {
            Ok(())
        } else {
            Err(EthTxServiceError::Auth(
                "invalid Authorization bearer token".to_string(),
            ))
        }
    }

    async fn evaluate_and_record_policy(
        &self,
        request: &DirectRawTransactionRequest,
        route: &str,
    ) -> Result<EthTxPolicyEvaluation, EthTxServiceError> {
        let latest_block = if self.config.policy.require_simulation {
            match fetch_latest_block_number(&self.rpc_http, &self.config.executor.rpc_url).await {
                Ok(block) => Some(block),
                Err(error) => {
                    let evaluation = EthTxPolicyEvaluation::rejected_from(
                        request
                            .attempt_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        self.config.policy.version.clone(),
                        format!("failed to fetch latest block for policy freshness check: {error}"),
                    );
                    let evaluation = self
                        .policy_repository
                        .record_evaluation(
                            evaluation,
                            request,
                            self.config.policy.max_daily_cost_wei,
                            route,
                        )
                        .await
                        .map_err(|error| EthTxServiceError::internal(error.to_string()))?;
                    return Ok(evaluation);
                }
            }
        } else {
            None
        };

        let evaluation = evaluate_eth_tx_policy(&self.config.policy, request, latest_block);
        self.policy_repository
            .record_evaluation(
                evaluation,
                request,
                self.config.policy.max_daily_cost_wei,
                route,
            )
            .await
            .map_err(|error| EthTxServiceError::internal(error.to_string()))
    }
}

pub fn build_eth_tx_executor_routes(state: EthTxExecutorAppState) -> Router {
    Router::new()
        .route("/eth/tx/status", get(status))
        .route("/eth/tx/policy/decisions", get(list_policy_decisions))
        .route(
            "/eth/tx/policy/decisions/{attempt_id}",
            get(list_policy_decisions_for_attempt),
        )
        .route("/eth/tx/direct-raw", post(submit_direct_raw))
        .route("/eth/tx/submit", post(submit_transaction))
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct EthTxExecutorStatus {
    service: &'static str,
    enabled: bool,
    api_token_configured: bool,
    signer_available: bool,
    signer_backend: &'static str,
    execution_disabled: bool,
    broadcast_mode: BroadcastMode,
    broadcast_mode_label: &'static str,
    submission_policy_kinds: Vec<&'static str>,
    chain_id: u64,
    rpc_url: String,
    journal_path: Option<String>,
    direct_raw_endpoint: &'static str,
    submit_endpoint: &'static str,
    policy: EthTxPolicyStatus,
}

#[derive(Debug, Serialize)]
struct EthTxPolicyStatus {
    version: String,
    allowed_from_count: usize,
    allowed_target_count: usize,
    allowed_selector_count: usize,
    max_value_wei: String,
    max_gas_limit: u64,
    max_fee_per_gas_wei: String,
    max_priority_fee_per_gas_wei: String,
    max_transaction_cost_wei: String,
    max_daily_cost_wei: String,
    daily_spend_cap_enabled: bool,
    daily_spend: EthTxDailySpendStatus,
    require_simulation: bool,
    max_simulation_age_blocks: u64,
    required_metadata_fields: Vec<String>,
}

#[derive(Debug, Serialize)]
struct EthTxDailySpendStatus {
    spend_day: String,
    spent_wei: String,
    remaining_daily_cost_wei: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PolicyDecisionQuery {
    limit: Option<i64>,
}

async fn status(
    State(state): State<EthTxExecutorAppState>,
    headers: HeaderMap,
) -> Result<Json<EthTxExecutorStatus>, EthTxServiceError> {
    state.require_auth(&headers)?;
    let config = &state.config;
    let daily_spend = state
        .policy_repository
        .daily_spend_snapshot()
        .await
        .map_err(|error| EthTxServiceError::internal(error.to_string()))?;
    let daily_spend_cap_enabled = config.policy.max_daily_cost_wei > 0;
    let remaining_daily_cost_wei = if daily_spend_cap_enabled {
        daily_spend.spent_wei.parse::<u128>().ok().map(|spent_wei| {
            config
                .policy
                .max_daily_cost_wei
                .saturating_sub(spent_wei)
                .to_string()
        })
    } else {
        None
    };

    let broadcast_mode = config.executor.broadcast_mode;
    Ok(Json(EthTxExecutorStatus {
        service: "eth_tx_executor",
        enabled: state.execution_enabled()
            && state.api_token_configured()
            && state.signer_available(),
        api_token_configured: state.api_token_configured(),
        signer_available: state.signer_available(),
        signer_backend: signer_backend_name(&config.signer_backend),
        execution_disabled: config.execution_disabled,
        broadcast_mode,
        broadcast_mode_label: broadcast_mode_label(broadcast_mode),
        submission_policy_kinds: submission_policy_kinds(),
        chain_id: config.executor.chain_id,
        rpc_url: config.executor.rpc_url.clone(),
        journal_path: config
            .executor
            .local_journal_path
            .as_ref()
            .map(|path| path.display().to_string()),
        direct_raw_endpoint: "/eth/tx/direct-raw",
        submit_endpoint: "/eth/tx/submit",
        policy: EthTxPolicyStatus {
            version: config.policy.version.clone(),
            allowed_from_count: config.policy.allowed_from_addresses.len(),
            allowed_target_count: config.policy.allowed_targets.len(),
            allowed_selector_count: config.policy.allowed_selectors.len(),
            max_value_wei: config.policy.max_value_wei.to_string(),
            max_gas_limit: config.policy.max_gas_limit,
            max_fee_per_gas_wei: config.policy.max_fee_per_gas_wei.to_string(),
            max_priority_fee_per_gas_wei: config.policy.max_priority_fee_per_gas_wei.to_string(),
            max_transaction_cost_wei: config.policy.max_transaction_cost_wei.to_string(),
            max_daily_cost_wei: config.policy.max_daily_cost_wei.to_string(),
            daily_spend_cap_enabled,
            daily_spend: EthTxDailySpendStatus {
                spend_day: daily_spend.spend_day,
                spent_wei: daily_spend.spent_wei,
                remaining_daily_cost_wei,
            },
            require_simulation: config.policy.require_simulation,
            max_simulation_age_blocks: config.policy.max_simulation_age_blocks,
            required_metadata_fields: config.policy.required_metadata_fields.clone(),
        },
    }))
}

fn broadcast_mode_label(mode: BroadcastMode) -> &'static str {
    match mode {
        BroadcastMode::DryRun => "dry run",
        BroadcastMode::Broadcast => "broadcast",
    }
}

fn submission_policy_kinds() -> Vec<&'static str> {
    vec!["public_rpc_broadcast"]
}

fn signer_backend_name(config: &EthTxSignerBackendConfig) -> &'static str {
    match config {
        EthTxSignerBackendConfig::Env => "env",
        EthTxSignerBackendConfig::UnixSocket { .. } => "unix_socket",
    }
}

async fn list_policy_decisions(
    State(state): State<EthTxExecutorAppState>,
    headers: HeaderMap,
    Query(query): Query<PolicyDecisionQuery>,
) -> Result<Json<Value>, EthTxServiceError> {
    state.require_auth(&headers)?;
    let decisions = state
        .policy_repository
        .list_decisions(query.limit.unwrap_or(100))
        .await
        .map_err(|error| EthTxServiceError::internal(error.to_string()))?;

    Ok(Json(json!({
        "decisions": decisions,
    })))
}

async fn list_policy_decisions_for_attempt(
    State(state): State<EthTxExecutorAppState>,
    headers: HeaderMap,
    Path(attempt_id): Path<String>,
    Query(query): Query<PolicyDecisionQuery>,
) -> Result<Json<Value>, EthTxServiceError> {
    state.require_auth(&headers)?;
    let decisions = state
        .policy_repository
        .list_decisions_for_attempt(&attempt_id, query.limit.unwrap_or(100))
        .await
        .map_err(|error| EthTxServiceError::internal(error.to_string()))?;

    Ok(Json(json!({
        "attempt_id": attempt_id,
        "decisions": decisions,
    })))
}

async fn submit_direct_raw(
    State(state): State<EthTxExecutorAppState>,
    headers: HeaderMap,
    Json(request): Json<DirectRawTransactionRequest>,
) -> Result<Json<SubmitDirectRawResult>, EthTxServiceError> {
    state.require_auth(&headers)?;
    let result = submit_broadcast_request(&state, request, "/eth/tx/direct-raw").await?;
    Ok(Json(result))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EthTxSubmitRequest {
    pub transaction: DirectRawTransactionRequest,
    #[serde(default)]
    pub submission_policy: EthTxSubmissionPolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EthTxSubmissionPolicy {
    PublicRpcBroadcast,
}

impl Default for EthTxSubmissionPolicy {
    fn default() -> Self {
        Self::PublicRpcBroadcast
    }
}

#[derive(Debug, Serialize)]
struct EthTxSubmitResult {
    pub attempt_id: String,
    pub status: String,
    pub tx_hash: Option<ethers_core::types::H256>,
    pub error: Option<String>,
    pub bundle_hash: Option<String>,
    pub tail_after_tx_hash: Option<String>,
    pub target_block: Option<u64>,
    pub max_block: Option<u64>,
    pub from: ethers_core::types::Address,
    pub to: ethers_core::types::Address,
    pub nonce: ethers_core::types::U256,
    pub gas_limit: ethers_core::types::U256,
    pub max_fee_per_gas: ethers_core::types::U256,
    pub max_priority_fee_per_gas: ethers_core::types::U256,
    pub elapsed_ms: u128,
}

async fn submit_transaction(
    State(state): State<EthTxExecutorAppState>,
    headers: HeaderMap,
    Json(mut request): Json<EthTxSubmitRequest>,
) -> Result<Json<EthTxSubmitResult>, EthTxServiceError> {
    state.require_auth(&headers)?;
    let route = "/eth/tx/submit";
    annotate_submission_policy(&mut request.transaction, &request.submission_policy);
    let result = submit_broadcast_request(&state, request.transaction, route)
        .await?
        .into();
    Ok(Json(result))
}

async fn submit_broadcast_request(
    state: &EthTxExecutorAppState,
    mut request: DirectRawTransactionRequest,
    route: &str,
) -> Result<SubmitDirectRawResult, EthTxServiceError> {
    let attempt_id = request
        .attempt_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    request.attempt_id = Some(attempt_id.clone());

    if !state.execution_enabled() {
        let evaluation = EthTxPolicyEvaluation::disabled_from(
            attempt_id.clone(),
            state.config.policy.version.clone(),
            "ETH tx execution kill switch is active",
        );
        let evaluation = state
            .policy_repository
            .record_evaluation(
                evaluation,
                &request,
                state.config.policy.max_daily_cost_wei,
                route,
            )
            .await
            .map_err(|error| EthTxServiceError::internal(error.to_string()))?;
        return Err(EthTxServiceError::unavailable(format!(
            "ETH tx execution kill switch is active for attempt {}: {}",
            attempt_id,
            policy_decision_json(&evaluation)
        )));
    }

    let evaluation = state.evaluate_and_record_policy(&request, route).await?;
    if !evaluation.is_accepted() {
        return Err(EthTxServiceError::validation(format!(
            "ETH tx policy rejected attempt {}: {}",
            attempt_id,
            policy_decision_json(&evaluation)
        )));
    }

    let executor = match state.executor.as_ref() {
        Some(executor) => executor,
        None => {
            release_spend_reservation(&state, &attempt_id, "signer is not configured").await;
            return Err(EthTxServiceError::unavailable(format!(
                "{} is not configured, so the executor cannot sign transactions",
                state.config.private_key_env
            )));
        }
    };

    let result = match executor.submit_direct_raw(request).await {
        Ok(result) => result,
        Err(error) => {
            if let Err(release_error) = state
                .policy_repository
                .release_spend_reservation(&attempt_id)
                .await
            {
                tracing::error!(
                    attempt_id,
                    error = %release_error,
                    "failed to release ETH tx spend reservation after executor error"
                );
            }
            return Err(map_executor_error(error));
        }
    };
    if releases_spend_reservation_after_result(result.status) {
        if let Err(release_error) = state
            .policy_repository
            .release_spend_reservation(&attempt_id)
            .await
        {
            tracing::error!(
                attempt_id,
                status = ?result.status,
                error = %release_error,
                "failed to release ETH tx spend reservation after non-broadcast ETH tx result"
            );
        }
    }
    Ok(result)
}

impl From<SubmitDirectRawResult> for EthTxSubmitResult {
    fn from(result: SubmitDirectRawResult) -> Self {
        Self {
            attempt_id: result.attempt_id,
            status: execution_status_key(result.status).to_string(),
            tx_hash: result.tx_hash,
            error: result.error,
            bundle_hash: None,
            tail_after_tx_hash: None,
            target_block: None,
            max_block: None,
            from: result.from,
            to: result.to,
            nonce: result.nonce,
            gas_limit: result.gas_limit,
            max_fee_per_gas: result.max_fee_per_gas,
            max_priority_fee_per_gas: result.max_priority_fee_per_gas,
            elapsed_ms: result.elapsed_ms,
        }
    }
}

fn annotate_submission_policy(
    transaction: &mut DirectRawTransactionRequest,
    submission_policy: &EthTxSubmissionPolicy,
) {
    let Ok(policy_value) = serde_json::to_value(submission_policy) else {
        return;
    };
    match &mut transaction.metadata {
        Value::Object(metadata) => {
            metadata.insert("submission_policy".to_string(), policy_value);
        }
        other => {
            let mut metadata = serde_json::Map::new();
            metadata.insert("source_metadata".to_string(), std::mem::take(other));
            metadata.insert("submission_policy".to_string(), policy_value);
            transaction.metadata = Value::Object(metadata);
        }
    }
}

fn releases_spend_reservation_after_result(status: ExecutionStatus) -> bool {
    matches!(
        status,
        ExecutionStatus::DryRun | ExecutionStatus::BroadcastError
    )
}

fn execution_status_key(status: ExecutionStatus) -> &'static str {
    match status {
        ExecutionStatus::Received => "received",
        ExecutionStatus::Rejected => "rejected",
        ExecutionStatus::Signed => "signed",
        ExecutionStatus::DryRun => "dry_run",
        ExecutionStatus::Broadcast => "broadcast",
        ExecutionStatus::BroadcastError => "broadcast_error",
    }
}

async fn release_spend_reservation(state: &EthTxExecutorAppState, attempt_id: &str, reason: &str) {
    if let Err(release_error) = state
        .policy_repository
        .release_spend_reservation(attempt_id)
        .await
    {
        tracing::error!(
            attempt_id,
            reason,
            error = %release_error,
            "failed to release ETH tx spend reservation"
        );
    }
}

fn map_executor_error(err: EthTxExecutorError) -> EthTxServiceError {
    match err {
        EthTxExecutorError::Validation(message) => EthTxServiceError::Validation(message),
        EthTxExecutorError::Config(message) => EthTxServiceError::Validation(message),
        EthTxExecutorError::Nonce(message)
        | EthTxExecutorError::Broadcast(message)
        | EthTxExecutorError::Rpc(message) => EthTxServiceError::Network(message),
        EthTxExecutorError::Signer(message)
        | EthTxExecutorError::Repository(message)
        | EthTxExecutorError::Serialization(message) => EthTxServiceError::Internal(message),
    }
}

async fn fetch_latest_block_number(client: &reqwest::Client, rpc_url: &str) -> Result<u64, String> {
    let response = client
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_blockNumber",
            "params": [],
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("RPC returned HTTP {status}: {payload}"));
    }
    if let Some(error) = payload.get("error") {
        return Err(format!("RPC error: {error}"));
    }
    let result = payload
        .get("result")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("RPC response missing result: {payload}"))?;
    let hex = result.strip_prefix("0x").unwrap_or(result);
    u64::from_str_radix(hex, 16).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spend_reservation_is_kept_only_for_potentially_broadcast_results() {
        assert!(releases_spend_reservation_after_result(
            ExecutionStatus::DryRun
        ));
        assert!(releases_spend_reservation_after_result(
            ExecutionStatus::BroadcastError
        ));
        assert!(!releases_spend_reservation_after_result(
            ExecutionStatus::Broadcast
        ));
        assert!(!releases_spend_reservation_after_result(
            ExecutionStatus::Received
        ));
        assert!(!releases_spend_reservation_after_result(
            ExecutionStatus::Signed
        ));
        assert!(!releases_spend_reservation_after_result(
            ExecutionStatus::Rejected
        ));
    }
}
