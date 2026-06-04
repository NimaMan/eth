use alloy_primitives::{Address, U256};
use eth_alpha_core::{
    amount::Amount,
    error::AlphaCoreError,
    mempool_entry::{MempoolEntryEvidence, MEMPOOL_ENTRY_EVIDENCE_VERSION},
    order::{OrderIntent, OrderSide},
};
use eth_live_trading::{
    derive_min_output_from_expected_output, LivePrioritySellPlannerInput, PreSubmitSimulation,
    PreparedSellRoute, StrategyGasRankPolicy, TxSubmissionPolicy, TxSubmissionRoute,
};
use serde_json::{json, Value};

use super::{cancelled_execution_at, parse_u256_quantity, planner_error};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct TailEntryOrderingEvidence {
    pub(super) tail_after_tx_hash: Option<String>,
    pub(super) dependency_priority_fee_wei: Option<String>,
    pub(super) dependency_max_fee_per_gas_wei: Option<String>,
    pub(super) dependency_gas_price_wei: Option<String>,
    pub(super) signal_source: Option<String>,
    pub(super) evidence_source: Option<String>,
}

impl TailEntryOrderingEvidence {
    fn is_public_mempool_source(&self) -> bool {
        source_contains(self.signal_source.as_deref(), "mempool")
            || source_contains(self.evidence_source.as_deref(), "mempool_processor")
    }
}

#[derive(Clone, Debug)]
pub(super) struct TailEntryOverlayPlan {
    pub(super) min_output: U256,
    pub(super) simulation: PreSubmitSimulation,
    pub(super) evidence: MempoolEntryEvidence,
}

pub(super) fn tail_entry_overlay_plan(
    input: &LivePrioritySellPlannerInput,
    gas_policy_action: &str,
) -> eth_alpha_core::error::Result<Option<TailEntryOverlayPlan>> {
    if input.intent.side != OrderSide::Buy || gas_policy_action != "tail_entry_buy" {
        return Ok(None);
    }

    let evidence = mempool_entry_evidence_from_intent(&input.intent, input.context.current_block)?;
    if evidence.evidence_version != MEMPOOL_ENTRY_EVIDENCE_VERSION {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry evidence version {} is not supported",
                evidence.evidence_version
            ),
            input.context.current_block,
        ));
    }
    if !evidence.has_successful_exact_vault_buy() {
        return Err(cancelled_execution_at(
            "tail-entry evidence does not contain a successful exact-vault buy simulation",
            input.context.current_block,
        ));
    }
    if evidence.vault_buy_simulation.chain_id != Some(input.context.tx.chain_id) {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry evidence chain_id {:?} does not match configured chain_id {}",
                evidence.vault_buy_simulation.chain_id, input.context.tx.chain_id
            ),
            input.context.current_block,
        ));
    }

    let owner = input.context.tx.from.parse::<Address>().map_err(|error| {
        AlphaCoreError::Execution(format!(
            "invalid live real from address {:?}: {error}",
            input.context.tx.from
        ))
    })?;
    if evidence.vault_buy_simulation.owner_address != Some(owner) {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry evidence owner {:?} does not match configured from {}",
                evidence.vault_buy_simulation.owner_address, owner
            ),
            input.context.current_block,
        ));
    }

    let eth_spent = parse_u256_quantity(
        evidence
            .vault_buy_simulation
            .eth_spent_wei
            .as_deref()
            .unwrap_or_default(),
        "tail-entry evidence eth_spent_wei",
    )?;
    if eth_spent != input.intent.amount.raw {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry evidence buy amount {eth_spent} does not match intent amount {}",
                input.intent.amount.raw
            ),
            input.context.current_block,
        ));
    }

    let expected_tokens = evidence_expected_tokens_raw(&evidence, input.context.current_block)?;
    let expected_min_output =
        derive_min_output_from_expected_output(expected_tokens, input.intent.max_slippage_bps)
            .map_err(planner_error)?;
    if expected_min_output.is_zero() {
        return Err(cancelled_execution_at(
            "tail-entry evidence derived zero min-output",
            input.context.current_block,
        ));
    }
    let evidence_min_output = evidence_min_tokens_out(&evidence, input.context.current_block)?;
    if evidence_min_output != expected_min_output {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry evidence min_tokens_out {evidence_min_output} does not match strategy-derived min_output {expected_min_output}"
            ),
            input.context.current_block,
        ));
    }

    let gas_used = evidence.vault_buy_simulation.gas_used.ok_or_else(|| {
        AlphaCoreError::Execution(
            "tail-entry exact-vault evidence did not report gas_used; fallback gas estimates are not allowed"
                .to_string(),
        )
    })?;
    if gas_used == 0 {
        return Err(cancelled_execution_at(
            "tail-entry exact-vault evidence reported zero gas_used",
            input.context.current_block,
        ));
    }

    let simulation_block = evidence.simulated_block.unwrap_or(evidence.base_block);
    let simulation_metadata = json!({
        "provider": "mempool_entry_exact_vault_overlay",
        "exact_overlay_simulation": true,
        "evidence_version": evidence.evidence_version.clone(),
        "base_block": evidence.base_block,
        "simulated_block": simulation_block,
        "simulated_at": evidence.simulated_at.clone(),
        "route": evidence.vault_buy_simulation.route.clone(),
        "vault_address": evidence.vault_buy_simulation.vault_address.map(|address| address.to_string()),
        "owner_address": evidence.vault_buy_simulation.owner_address.map(|address| address.to_string()),
        "chain_id": evidence.vault_buy_simulation.chain_id,
        "eth_spent_wei": eth_spent.to_string(),
        "expected_tokens_raw": expected_tokens.to_string(),
        "min_tokens_out": expected_min_output.to_string(),
        "gas_used": gas_used,
        "dependency_tx_hashes": evidence.dependency_tx_hashes.clone(),
        "vault_simulation_dependency_tx_hashes": evidence
            .vault_buy_simulation
            .metadata
            .get("dependency_tx_hashes")
            .cloned()
            .unwrap_or(Value::Null),
        "dependency_fee_metadata": evidence.dependency_fee_metadata.clone(),
        "projected_pool": {
            "pool_creation_block": evidence.projected_pool.pool_creation_block,
            "latest_block": evidence.projected_pool.latest_block,
            "protocol": evidence.projected_pool.protocol.clone(),
            "can_buy": evidence.projected_pool.can_buy,
            "can_sell": evidence.projected_pool.can_sell,
        },
        "audit": evidence.audit.clone(),
    });

    let simulation = PreSubmitSimulation {
        block_number: simulation_block,
        block_hash: None,
        state_root: None,
        expected_output_token: Some(input.intent.token_address.to_string()),
        expected_output_amount: Some(expected_tokens.to_string()),
        min_output_amount: Some(expected_min_output.to_string()),
        expected_recovery_eth: Amount {
            raw: eth_spent,
            decimals: 18,
        }
        .to_decimal(),
        gas_used: Some(gas_used),
        would_revert: false,
        metadata: simulation_metadata,
    };

    Ok(Some(TailEntryOverlayPlan {
        min_output: expected_min_output,
        simulation,
        evidence,
    }))
}

pub(super) fn validate_tail_entry_route(
    input: &LivePrioritySellPlannerInput,
    route: &PreparedSellRoute,
    evidence: &MempoolEntryEvidence,
) -> eth_alpha_core::error::Result<()> {
    if route.protocol != evidence.vault_buy_simulation.route {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry route protocol {} does not match exact-vault evidence route {}",
                route.protocol, evidence.vault_buy_simulation.route
            ),
            input.context.current_block,
        ));
    }
    let route_target = route.router_address.parse::<Address>().map_err(|error| {
        AlphaCoreError::Execution(format!(
            "invalid tail-entry route target {:?}: {error}",
            route.router_address
        ))
    })?;
    if evidence.vault_buy_simulation.vault_address != Some(route_target) {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry route target {} does not match exact-vault evidence vault {:?}",
                route_target, evidence.vault_buy_simulation.vault_address
            ),
            input.context.current_block,
        ));
    }
    let route_value = parse_u256_quantity(&route.value_wei, "tail-entry route value_wei")?;
    if route_value != input.intent.amount.raw {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry route value {route_value} does not match intent amount {}",
                input.intent.amount.raw
            ),
            input.context.current_block,
        ));
    }
    Ok(())
}

pub(super) fn buy_submission_policy(
    gas_rank_policy: &StrategyGasRankPolicy,
    gas_policy_action: &str,
    tail_entry_ordering: &Option<TailEntryOrderingEvidence>,
    current_block: u64,
) -> eth_alpha_core::error::Result<TxSubmissionPolicy> {
    let submission_route = gas_rank_policy.submission_route;
    match submission_route {
        TxSubmissionRoute::PublicRpcBroadcast => return Ok(TxSubmissionPolicy::PublicRpcBroadcast),
        TxSubmissionRoute::PublicMempoolTail => {}
    }
    if gas_policy_action != "tail_entry_buy" {
        return Err(AlphaCoreError::ExecutionCancelled {
            reason: format!(
                "gas policy submission_route={} requires tail_entry_buy action; got {gas_policy_action}",
                submission_route.label()
            ),
            block_number: Some(current_block),
        });
    }
    let tail_entry_ordering =
        tail_entry_ordering
            .as_ref()
            .ok_or_else(|| AlphaCoreError::ExecutionCancelled {
                reason: "public mempool tail-entry buy requires tail-entry ordering evidence"
                    .to_string(),
                block_number: Some(current_block),
            })?;
    if !tail_entry_ordering.is_public_mempool_source() {
        return Err(AlphaCoreError::ExecutionCancelled {
            reason: "public_mempool_tail requires public mempool processor source evidence"
                .to_string(),
            block_number: Some(current_block),
        });
    }
    let _tail_after_tx_hash = tail_entry_ordering
        .tail_after_tx_hash
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AlphaCoreError::ExecutionCancelled {
            reason: "public mempool tail-entry buy requires dependency tail_after_tx_hash"
                .to_string(),
            block_number: Some(current_block),
        })?;
    Ok(TxSubmissionPolicy::PublicRpcBroadcast)
}

pub(super) fn tail_entry_ordering_evidence(
    intent: &OrderIntent,
    gas_policy_action: &str,
) -> Option<TailEntryOrderingEvidence> {
    if intent.side != OrderSide::Buy || gas_policy_action != "tail_entry_buy" {
        return None;
    }
    let dependency_fee_metadata = intent
        .decision_reason
        .as_ref()?
        .details
        .get("risk_event_evidence")?
        .get("mempool_entry_evidence")?
        .get("dependency_fee_metadata")?;
    Some(TailEntryOrderingEvidence {
        tail_after_tx_hash: json_string_field(dependency_fee_metadata, "tail_after_tx_hash"),
        dependency_priority_fee_wei: json_string_field(
            dependency_fee_metadata,
            "dependency_priority_fee_wei",
        ),
        dependency_max_fee_per_gas_wei: json_string_field(
            dependency_fee_metadata,
            "dependency_max_fee_per_gas_wei",
        ),
        dependency_gas_price_wei: json_string_field(
            dependency_fee_metadata,
            "dependency_gas_price_wei",
        ),
        signal_source: intent
            .decision_reason
            .as_ref()
            .and_then(|reason| {
                reason
                    .details
                    .get("risk_event_evidence")
                    .and_then(|evidence| json_string_field(evidence, "signal_source"))
            })
            .or_else(|| {
                intent
                    .decision_reason
                    .as_ref()
                    .and_then(|reason| reason.source.clone())
            }),
        evidence_source: intent.decision_reason.as_ref().and_then(|reason| {
            reason
                .details
                .get("risk_event_evidence")
                .and_then(|evidence| evidence.get("mempool_entry_evidence"))
                .and_then(|entry| entry.get("audit"))
                .and_then(|audit| json_string_field(audit, "source"))
        }),
    })
}

fn source_contains(value: Option<&str>, needle: &str) -> bool {
    value
        .map(|value| value.to_ascii_lowercase().contains(needle))
        .unwrap_or(false)
}

fn mempool_entry_evidence_from_intent(
    intent: &OrderIntent,
    current_block: u64,
) -> eth_alpha_core::error::Result<MempoolEntryEvidence> {
    let risk_evidence = intent
        .decision_reason
        .as_ref()
        .and_then(|reason| reason.details.get("risk_event_evidence"))
        .ok_or_else(|| {
            cancelled_execution_at(
                "tail-entry buy requires risk_event_evidence with mempool_entry_evidence",
                current_block,
            )
        })?;
    match MempoolEntryEvidence::from_risk_evidence(risk_evidence) {
        Some(Ok(evidence)) => Ok(evidence),
        Some(Err(error)) => Err(cancelled_execution_at(
            format!("tail-entry mempool_entry_evidence is invalid: {error}"),
            current_block,
        )),
        None => Err(cancelled_execution_at(
            "tail-entry buy requires mempool_entry_evidence",
            current_block,
        )),
    }
}

fn evidence_expected_tokens_raw(
    evidence: &MempoolEntryEvidence,
    current_block: u64,
) -> eth_alpha_core::error::Result<U256> {
    let token_amount = evidence
        .vault_buy_simulation
        .tokens_received_raw
        .as_deref()
        .ok_or_else(|| {
            cancelled_execution_at(
                "tail-entry exact-vault evidence missing tokens_received_raw",
                current_block,
            )
        })?;
    let token_amount =
        parse_u256_quantity(token_amount, "tail-entry evidence tokens_received_raw")?;
    let Some(metadata_amount) = json_string_field(
        &evidence.vault_buy_simulation.metadata,
        "expected_tokens_raw",
    ) else {
        return Ok(token_amount);
    };
    let metadata_amount = parse_u256_quantity(
        &metadata_amount,
        "tail-entry evidence metadata.expected_tokens_raw",
    )?;
    if metadata_amount != token_amount {
        return Err(cancelled_execution_at(
            format!(
                "tail-entry evidence expected_tokens_raw {metadata_amount} does not match tokens_received_raw {token_amount}"
            ),
            current_block,
        ));
    }
    Ok(token_amount)
}

fn evidence_min_tokens_out(
    evidence: &MempoolEntryEvidence,
    current_block: u64,
) -> eth_alpha_core::error::Result<U256> {
    let min_output = json_string_field(&evidence.vault_buy_simulation.metadata, "min_tokens_out")
        .ok_or_else(|| {
        cancelled_execution_at(
            "tail-entry exact-vault evidence missing metadata.min_tokens_out",
            current_block,
        )
    })?;
    let min_output =
        parse_u256_quantity(&min_output, "tail-entry evidence metadata.min_tokens_out")?;
    if min_output.is_zero() {
        return Err(cancelled_execution_at(
            "tail-entry exact-vault evidence has zero metadata.min_tokens_out",
            current_block,
        ));
    }
    Ok(min_output)
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    let value = value.get(key)?;
    if value.is_null() {
        return None;
    }
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| Some(value.to_string()))
}
