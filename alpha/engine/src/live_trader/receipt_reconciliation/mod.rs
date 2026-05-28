use alloy_primitives::{hex, keccak256, Address, B256, U256};
use eth_alpha_core::{
    amount::Amount,
    execution::{ExecutionReport, ExecutionStatus, MinedExecutionEvidence},
    ids::TxHash,
    order::OrderSide,
};
use eth_alpha_store::SubmittedExecutionRecord;
use eyre::{eyre, Result, WrapErr};

mod rpc;

pub(in crate::live_trader) use rpc::JsonRpcReceiptProvider;
use rpc::{indexed_address_topic, ReceiptProvider, ReceiptStatus, RpcTransactionReceipt};

const BOUGHT_V2_SIGNATURE: &str = "BoughtV2(address,uint256,uint256,uint256)";
const EMERGENCY_SOLD_V2_SIGNATURE: &str = "EmergencySoldV2(address,uint256,uint256,uint256)";
const ACCEPTED_CONFIRMATION_DEPTH: u64 = 1;
const RECHECK_CONFIRMATION_DEPTH: u64 = 3;

#[derive(Debug)]
pub(super) struct ReceiptReconciliationBatch {
    pub(super) reports: Vec<ExecutionReport>,
    pub(super) unresolved: Vec<ReceiptReconciliationIssue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ReceiptReconciliationIssue {
    pub(super) order_id: String,
    pub(super) tx_hash: TxHash,
    pub(super) reason: String,
}

pub(super) struct VaultReceiptReconciler<P> {
    provider: P,
    vault_address: Address,
}

impl<P> VaultReceiptReconciler<P> {
    pub(super) fn new(provider: P, vault_address: Address) -> Self {
        Self {
            provider,
            vault_address,
        }
    }
}

impl<P> VaultReceiptReconciler<P>
where
    P: ReceiptProvider,
{
    pub(super) async fn reconcile(
        &self,
        submitted: Vec<SubmittedExecutionRecord>,
    ) -> Result<ReceiptReconciliationBatch> {
        let mut reports = Vec::new();
        let mut unresolved = Vec::new();
        for record in submitted {
            let Some(receipt) = self.provider.transaction_receipt(record.tx_hash).await? else {
                continue;
            };
            let dependency_receipt = match dependency_tx_hash(&record)? {
                Some(tx_hash) => self.provider.transaction_receipt(tx_hash).await?,
                None => None,
            };
            match reconcile_receipt(
                &record,
                &receipt,
                dependency_receipt.as_ref(),
                self.vault_address,
            )? {
                ReceiptReconciliation::Final(report) => reports.push(report),
                ReceiptReconciliation::Unresolved(issue) => unresolved.push(issue),
            }
        }
        Ok(ReceiptReconciliationBatch {
            reports,
            unresolved,
        })
    }

    pub(super) async fn reconcile_after_processed_block(
        &self,
        submitted: Vec<SubmittedExecutionRecord>,
        current_processed_block: Option<u64>,
    ) -> Result<ReceiptReconciliationBatch> {
        let ready = submitted
            .into_iter()
            .filter(|record| ready_for_receipt_reconciliation(record, current_processed_block))
            .collect();
        self.reconcile(ready).await
    }
}

enum ReceiptReconciliation {
    Final(ExecutionReport),
    Unresolved(ReceiptReconciliationIssue),
}

fn reconcile_receipt(
    record: &SubmittedExecutionRecord,
    receipt: &RpcTransactionReceipt,
    dependency_receipt: Option<&RpcTransactionReceipt>,
    vault_address: Address,
) -> Result<ReceiptReconciliation> {
    let block_number = receipt.block_number()?;
    let gas_used = receipt.gas_used()?;
    let gas_cost = receipt.gas_cost()?;
    let receipt_status = receipt.status()?;
    match receipt_status {
        ReceiptStatus::Failed => {
            let mined_evidence = Some(mined_evidence(
                record,
                receipt,
                dependency_receipt,
                &gas_cost,
            )?);
            Ok(ReceiptReconciliation::Final(ExecutionReport {
                order_id: record.order_id.clone(),
                status: ExecutionStatus::Failed,
                tx_hash: Some(record.tx_hash),
                block_number,
                filled_amount: None,
                token_amount: None,
                gas_used,
                gas_cost,
                mined_evidence,
                error: Some("transaction receipt status=0x0".to_string()),
            }))
        }
        ReceiptStatus::Succeeded => {
            let Some(fill) = extract_v2_vault_fill(
                receipt,
                record.order_side,
                vault_address,
                record.token_address,
            )?
            else {
                return Ok(ReceiptReconciliation::Unresolved(
                    ReceiptReconciliationIssue {
                        order_id: record.order_id.0.clone(),
                        tx_hash: record.tx_hash,
                        reason: "successful receipt is missing matching V2 vault fill event"
                            .to_string(),
                    },
                ));
            };

            let mined_evidence = Some(mined_evidence(
                record,
                receipt,
                dependency_receipt,
                &gas_cost,
            )?);
            Ok(ReceiptReconciliation::Final(ExecutionReport {
                order_id: record.order_id.clone(),
                status: ExecutionStatus::Confirmed,
                tx_hash: Some(record.tx_hash),
                block_number,
                filled_amount: Some(fill.filled_amount),
                token_amount: fill.token_amount,
                gas_used,
                gas_cost,
                mined_evidence,
                error: None,
            }))
        }
        ReceiptStatus::Unknown => Ok(ReceiptReconciliation::Unresolved(
            ReceiptReconciliationIssue {
                order_id: record.order_id.0.clone(),
                tx_hash: record.tx_hash,
                reason: "receipt status is missing or unknown".to_string(),
            },
        )),
    }
}

fn mined_evidence(
    record: &SubmittedExecutionRecord,
    receipt: &RpcTransactionReceipt,
    dependency_receipt: Option<&RpcTransactionReceipt>,
    gas_cost: &Option<Amount>,
) -> Result<MinedExecutionEvidence> {
    let block_number = receipt.block_number()?;
    let expected_confirmation_block = record
        .submitted_block_number
        .map(|block| block.saturating_add(1));
    let confirmation_lag_blocks = match (block_number, expected_confirmation_block) {
        (Some(actual), Some(expected)) => Some(actual as i64 - expected as i64),
        _ => None,
    };

    let dependency_ordering = dependency_ordering_evidence(record, receipt, dependency_receipt)?;

    Ok(MinedExecutionEvidence {
        receipt_block_number: block_number,
        simulation_block_number: None,
        block_hash: receipt.block_hash()?,
        transaction_index: receipt.transaction_index()?,
        cumulative_gas_used: receipt.cumulative_gas_used()?,
        receipt_status: receipt.normalized_status(),
        submitted_block_number: record.submitted_block_number,
        expected_confirmation_block,
        confirmation_lag_blocks,
        effective_gas_price_wei: receipt.effective_gas_price_wei()?,
        legacy_gas_price_wei: receipt.legacy_gas_price_wei()?,
        paid_gas_cost_wei: gas_cost.as_ref().map(|amount| amount.raw.to_string()),
        selected_gas_limit: record.selected_gas_limit.clone(),
        selected_max_fee_per_gas_wei: record.selected_max_fee_per_gas_wei.clone(),
        selected_max_priority_fee_per_gas_wei: record.selected_max_priority_fee_per_gas_wei.clone(),
        selected_bribe_priority_fee_per_gas_wei: record
            .selected_bribe_priority_fee_per_gas_wei
            .clone(),
        selected_bribe_max_fee_per_gas_wei: record.selected_bribe_max_fee_per_gas_wei.clone(),
        gas_policy_action: record.gas_policy_action.clone(),
        gas_policy_signal: record.gas_policy_signal.clone(),
        gas_policy_status: record.gas_policy_status.clone(),
        gas_policy_profile: record.gas_policy_profile.clone(),
        gas_policy_profiles: record.gas_policy_profiles.clone(),
        gas_rank_source: record.gas_rank_source.clone(),
        gas_estimated_max_cost_eth: record.gas_estimated_max_cost_eth.clone(),
        gas_estimated_priority_spend_eth: record.gas_estimated_priority_spend_eth.clone(),
        gas_policy_guard: record.gas_policy_guard.clone(),
        private_execution_transport: record.private_execution_transport.clone(),
        bundle_hash: record.bundle_hash.clone(),
        bundle_target_block: record.bundle_target_block,
        bundle_max_block: record.bundle_max_block,
        bundle_ordering_status: dependency_ordering.status,
        bundle_dependency_block_number: dependency_ordering.block_number,
        bundle_dependency_transaction_index: dependency_ordering.transaction_index,
        gas_policy_tail_after_tx_hash: normalized_tail_after_tx_hash(record).map(str::to_string),
        gas_policy_dependency_priority_fee_wei: record
            .gas_policy_dependency_priority_fee_wei
            .clone(),
        gas_policy_dependency_gas_price_wei: record.gas_policy_dependency_gas_price_wei.clone(),
        accepted_confirmation_depth: Some(ACCEPTED_CONFIRMATION_DEPTH),
        recheck_confirmation_depth: Some(RECHECK_CONFIRMATION_DEPTH),
    })
}

#[derive(Default)]
struct DependencyOrderingEvidence {
    status: Option<String>,
    block_number: Option<u64>,
    transaction_index: Option<u64>,
}

fn dependency_tx_hash(record: &SubmittedExecutionRecord) -> Result<Option<TxHash>> {
    let Some(trimmed) = normalized_tail_after_tx_hash(record) else {
        return Ok(None);
    };
    trimmed
        .parse::<TxHash>()
        .map(Some)
        .wrap_err_with(|| format!("invalid tail dependency tx hash {trimmed:?}"))
}

fn normalized_tail_after_tx_hash(record: &SubmittedExecutionRecord) -> Option<&str> {
    let trimmed = record.gas_policy_tail_after_tx_hash.as_deref()?.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return None;
    }
    Some(trimmed)
}

fn dependency_ordering_evidence(
    record: &SubmittedExecutionRecord,
    receipt: &RpcTransactionReceipt,
    dependency_receipt: Option<&RpcTransactionReceipt>,
) -> Result<DependencyOrderingEvidence> {
    if normalized_tail_after_tx_hash(record).is_none() {
        return Ok(DependencyOrderingEvidence::default());
    }
    let Some(dependency_receipt) = dependency_receipt else {
        return Ok(DependencyOrderingEvidence {
            status: Some("dependency_receipt_missing".to_string()),
            ..DependencyOrderingEvidence::default()
        });
    };

    let dependency_block = dependency_receipt.block_number()?;
    let dependency_index = dependency_receipt.transaction_index()?;
    let our_block = receipt.block_number()?;
    let our_index = receipt.transaction_index()?;
    let status = match (dependency_block, dependency_index, our_block, our_index) {
        (Some(dep_block), Some(dep_index), Some(block), Some(index))
            if dep_block == block && dep_index < index =>
        {
            "verified_same_block_after_dependency"
        }
        (Some(dep_block), Some(_), Some(block), Some(_)) if dep_block != block => {
            "dependency_mined_in_different_block"
        }
        (Some(_), Some(dep_index), Some(_), Some(index)) if dep_index >= index => {
            "not_after_dependency"
        }
        _ => "missing_ordering_fields",
    };

    Ok(DependencyOrderingEvidence {
        status: Some(status.to_string()),
        block_number: dependency_block,
        transaction_index: dependency_index,
    })
}

fn ready_for_receipt_reconciliation(
    record: &SubmittedExecutionRecord,
    current_processed_block: Option<u64>,
) -> bool {
    match (record.submitted_block_number, current_processed_block) {
        (Some(submitted_block), Some(current_block)) => {
            current_block >= submitted_block.saturating_add(1)
        }
        (Some(_), None) => false,
        (None, _) => true,
    }
}

struct VaultFill {
    filled_amount: Amount,
    token_amount: Option<Amount>,
}

fn extract_v2_vault_fill(
    receipt: &RpcTransactionReceipt,
    side: OrderSide,
    vault_address: Address,
    token_address: Address,
) -> Result<Option<VaultFill>> {
    let event_topic = match side {
        OrderSide::Buy => event_signature_topic(BOUGHT_V2_SIGNATURE),
        OrderSide::Sell => event_signature_topic(EMERGENCY_SOLD_V2_SIGNATURE),
    };
    let expected_token_topic = indexed_address_topic(token_address);
    for log in &receipt.logs {
        let Ok(log_address) = log.address.parse::<Address>() else {
            continue;
        };
        if log_address != vault_address {
            continue;
        }
        let topics = parse_topics(&log.topics)?;
        if topics.first().copied() != Some(event_topic) {
            continue;
        }
        if topics.get(1).copied() != Some(expected_token_topic) {
            continue;
        }
        let words = decode_event_words(&log.data, 3)?;
        return Ok(Some(match side {
            OrderSide::Buy => VaultFill {
                filled_amount: Amount {
                    raw: words[0],
                    decimals: 18,
                },
                token_amount: Some(Amount {
                    raw: words[1],
                    decimals: 18,
                }),
            },
            OrderSide::Sell => VaultFill {
                filled_amount: Amount {
                    raw: words[1],
                    decimals: 18,
                },
                token_amount: None,
            },
        }));
    }
    Ok(None)
}

fn parse_topics(values: &[String]) -> Result<Vec<B256>> {
    values
        .iter()
        .map(|value| value.parse::<B256>().map_err(|error| eyre!("{error}")))
        .collect()
}

fn decode_event_words(data: &str, expected_words: usize) -> Result<Vec<U256>> {
    let data = data.trim().strip_prefix("0x").unwrap_or(data.trim());
    let bytes = hex::decode(data).wrap_err("invalid log data hex")?;
    let expected_len = expected_words
        .checked_mul(32)
        .ok_or_else(|| eyre!("event word count overflow"))?;
    if bytes.len() < expected_len {
        return Err(eyre!(
            "log data too short: got {} bytes, need {expected_len}",
            bytes.len()
        ));
    }
    Ok((0..expected_words)
        .map(|index| U256::from_be_slice(&bytes[index * 32..(index + 1) * 32]))
        .collect())
}

fn event_signature_topic(signature: &str) -> B256 {
    keccak256(signature.as_bytes())
}

#[cfg(test)]
mod tests;
