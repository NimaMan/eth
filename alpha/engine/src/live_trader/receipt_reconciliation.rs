use alloy_primitives::{Address, B256, U256, hex, keccak256};
use async_trait::async_trait;
use eth_alpha_core::{
    amount::Amount,
    execution::{ExecutionReport, ExecutionStatus, MinedExecutionEvidence},
    ids::TxHash,
    order::OrderSide,
};
use eth_alpha_store::SubmittedExecutionRecord;
use eyre::{Result, WrapErr, eyre};
use serde::Deserialize;
use serde_json::json;

const BOUGHT_V2_SIGNATURE: &str = "BoughtV2(address,uint256,uint256,uint256)";
const EMERGENCY_SOLD_V2_SIGNATURE: &str = "EmergencySoldV2(address,uint256,uint256,uint256)";
const ACCEPTED_CONFIRMATION_DEPTH: u64 = 1;
const RECHECK_CONFIRMATION_DEPTH: u64 = 3;

#[async_trait]
pub(super) trait ReceiptProvider: Send + Sync {
    async fn transaction_receipt(&self, tx_hash: TxHash) -> Result<Option<RpcTransactionReceipt>>;
}

#[derive(Clone)]
pub(super) struct JsonRpcReceiptProvider {
    rpc_url: String,
    http: reqwest::Client,
}

impl JsonRpcReceiptProvider {
    pub(super) fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ReceiptProvider for JsonRpcReceiptProvider {
    async fn transaction_receipt(&self, tx_hash: TxHash) -> Result<Option<RpcTransactionReceipt>> {
        let response = self
            .http
            .post(&self.rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "eth_getTransactionReceipt",
                "params": [tx_hash.to_string()],
            }))
            .send()
            .await
            .wrap_err("failed to call eth_getTransactionReceipt")?
            .error_for_status()
            .wrap_err("eth_getTransactionReceipt returned HTTP error")?;
        let response: JsonRpcResponse<RpcTransactionReceipt> = response
            .json()
            .await
            .wrap_err("failed to decode eth_getTransactionReceipt response")?;
        if let Some(error) = response.error {
            return Err(eyre!(
                "eth_getTransactionReceipt RPC error {}: {}",
                error.code,
                error.message
            ));
        }
        Ok(response.result)
    }
}

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
            match reconcile_receipt(&record, &receipt, self.vault_address)? {
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
    vault_address: Address,
) -> Result<ReceiptReconciliation> {
    let block_number = receipt.block_number()?;
    let gas_used = receipt.gas_used()?;
    let gas_cost = receipt.gas_cost()?;
    let receipt_status = receipt.status()?;
    match receipt_status {
        ReceiptStatus::Failed => {
            let mined_evidence = Some(mined_evidence(record, receipt, &gas_cost)?);
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

            let mined_evidence = Some(mined_evidence(record, receipt, &gas_cost)?);
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

    Ok(MinedExecutionEvidence {
        receipt_block_number: block_number,
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
        accepted_confirmation_depth: Some(ACCEPTED_CONFIRMATION_DEPTH),
        recheck_confirmation_depth: Some(RECHECK_CONFIRMATION_DEPTH),
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

#[derive(Clone, Debug, Deserialize)]
pub(super) struct RpcTransactionReceipt {
    #[serde(rename = "blockNumber")]
    block_number: Option<String>,
    #[serde(rename = "blockHash")]
    block_hash: Option<String>,
    #[serde(rename = "transactionIndex")]
    transaction_index: Option<String>,
    status: Option<String>,
    #[serde(rename = "gasUsed")]
    gas_used: Option<String>,
    #[serde(rename = "cumulativeGasUsed")]
    cumulative_gas_used: Option<String>,
    #[serde(rename = "effectiveGasPrice")]
    effective_gas_price: Option<String>,
    #[serde(rename = "gasPrice")]
    gas_price: Option<String>,
    #[serde(default)]
    logs: Vec<RpcLog>,
}

#[derive(Clone, Debug, Deserialize)]
struct RpcLog {
    address: String,
    #[serde(default)]
    topics: Vec<String>,
    data: String,
}

#[derive(Clone, Debug, Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Clone, Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReceiptStatus {
    Succeeded,
    Failed,
    Unknown,
}

impl RpcTransactionReceipt {
    fn normalized_status(&self) -> Option<String> {
        self.status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn status(&self) -> Result<ReceiptStatus> {
        match self.status.as_deref().map(str::trim) {
            Some("0x1") => Ok(ReceiptStatus::Succeeded),
            Some("0x0") => Ok(ReceiptStatus::Failed),
            Some(value) if value.is_empty() => Ok(ReceiptStatus::Unknown),
            Some(value) => Err(eyre!("unknown receipt status {value:?}")),
            None => Ok(ReceiptStatus::Unknown),
        }
    }

    fn block_hash(&self) -> Result<Option<B256>> {
        self.block_hash
            .as_deref()
            .map(parse_b256)
            .transpose()
            .wrap_err("invalid receipt blockHash")
    }

    fn block_number(&self) -> Result<Option<u64>> {
        self.block_number
            .as_deref()
            .map(parse_hex_u64)
            .transpose()
            .wrap_err("invalid receipt blockNumber")
    }

    fn gas_used(&self) -> Result<Option<u64>> {
        self.gas_used
            .as_deref()
            .map(parse_hex_u64)
            .transpose()
            .wrap_err("invalid receipt gasUsed")
    }

    fn transaction_index(&self) -> Result<Option<u64>> {
        self.transaction_index
            .as_deref()
            .map(parse_hex_u64)
            .transpose()
            .wrap_err("invalid receipt transactionIndex")
    }

    fn cumulative_gas_used(&self) -> Result<Option<u64>> {
        self.cumulative_gas_used
            .as_deref()
            .map(parse_hex_u64)
            .transpose()
            .wrap_err("invalid receipt cumulativeGasUsed")
    }

    fn effective_gas_price_wei(&self) -> Result<Option<String>> {
        self.effective_gas_price
            .as_deref()
            .map(parse_hex_u256)
            .transpose()
            .map(|value| value.map(|value| value.to_string()))
            .wrap_err("invalid receipt effectiveGasPrice")
    }

    fn legacy_gas_price_wei(&self) -> Result<Option<String>> {
        self.gas_price
            .as_deref()
            .map(parse_hex_u256)
            .transpose()
            .map(|value| value.map(|value| value.to_string()))
            .wrap_err("invalid receipt gasPrice")
    }

    fn gas_cost(&self) -> Result<Option<Amount>> {
        let Some(gas_used) = self.gas_used.as_deref() else {
            return Ok(None);
        };
        let Some(price) = self
            .effective_gas_price
            .as_deref()
            .or(self.gas_price.as_deref())
        else {
            return Ok(None);
        };
        let gas_used = parse_hex_u256(gas_used).wrap_err("invalid receipt gasUsed")?;
        let gas_price = parse_hex_u256(price).wrap_err("invalid receipt gas price")?;
        Ok(Some(Amount {
            raw: gas_used * gas_price,
            decimals: 18,
        }))
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

fn indexed_address_topic(address: Address) -> B256 {
    let mut topic = [0u8; 32];
    topic[12..].copy_from_slice(address.as_slice());
    B256::from(topic)
}

fn parse_hex_u64(value: &str) -> Result<u64> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    u64::from_str_radix(value, 16).map_err(|error| eyre!("{error}"))
}

fn parse_hex_u256(value: &str) -> Result<U256> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    U256::from_str_radix(value, 16).map_err(|error| eyre!("{error}"))
}

fn parse_b256(value: &str) -> Result<B256> {
    value.parse::<B256>().map_err(|error| eyre!("{error}"))
}

#[cfg(test)]
mod tests {
    use eth_alpha_core::{
        ids::{OrderId, PositionId, TradeId},
        order::OrderSide,
    };
    use serde_json::json;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    fn submitted(side: OrderSide, token: Address) -> SubmittedExecutionRecord {
        SubmittedExecutionRecord {
            order_id: OrderId("order-1".to_string()),
            tx_hash: "0x1111111111111111111111111111111111111111111111111111111111111111"
                .parse()
                .unwrap(),
            submitted_block_number: Some(99),
            position_id: PositionId("pos-1".to_string()),
            trade_id: Some(TradeId("trade-1".to_string())),
            order_side: side,
            token_address: token,
            selected_gas_limit: Some("500000".to_string()),
            selected_max_fee_per_gas_wei: Some("100000000000".to_string()),
            selected_max_priority_fee_per_gas_wei: Some("50000000000".to_string()),
            selected_bribe_priority_fee_per_gas_wei: Some("40000000000".to_string()),
            selected_bribe_max_fee_per_gas_wei: Some("100000000000".to_string()),
        }
    }

    fn receipt(
        status: &str,
        vault: Address,
        token: Address,
        signature: &str,
        words: [U256; 3],
    ) -> RpcTransactionReceipt {
        serde_json::from_value(json!({
            "blockNumber": "0x64",
            "blockHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "transactionIndex": "0x7",
            "status": status,
            "gasUsed": "0x5208",
            "cumulativeGasUsed": "0xa410",
            "effectiveGasPrice": "0x3b9aca00",
            "gasPrice": "0x3b9aca00",
            "logs": [{
                "address": vault.to_string(),
                "topics": [
                    event_signature_topic(signature).to_string(),
                    indexed_address_topic(token).to_string()
                ],
                "data": format!(
                    "0x{}{}{}",
                    hex::encode(words[0].to_be_bytes::<32>()),
                    hex::encode(words[1].to_be_bytes::<32>()),
                    hex::encode(words[2].to_be_bytes::<32>())
                )
            }]
        }))
        .unwrap()
    }

    #[test]
    fn gate3_a5_successful_buy_receipt_records_vault_event_amounts_and_gas() {
        let vault = Address::repeat_byte(0x22);
        let token = Address::repeat_byte(0x33);
        let receipt = receipt(
            "0x1",
            vault,
            token,
            BOUGHT_V2_SIGNATURE,
            [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        );

        let report =
            match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, vault).unwrap() {
                ReceiptReconciliation::Final(report) => report,
                ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
            };

        assert_eq!(report.status, ExecutionStatus::Confirmed);
        assert_eq!(report.block_number, Some(100));
        assert_eq!(report.filled_amount.unwrap().raw, U256::from(10u64));
        assert_eq!(report.token_amount.unwrap().raw, U256::from(20u64));
        assert_eq!(report.gas_used, Some(21_000));
        assert_eq!(
            report.gas_cost.as_ref().map(|amount| amount.raw),
            Some(U256::from(21_000_000_000_000u64))
        );
    }

    #[test]
    fn gate3_a3_receipt_evidence_records_inclusion_position_and_backtest_lag() {
        let vault = Address::repeat_byte(0x22);
        let token = Address::repeat_byte(0x33);
        let receipt = receipt(
            "0x1",
            vault,
            token,
            BOUGHT_V2_SIGNATURE,
            [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        );

        let report =
            match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, vault).unwrap() {
                ReceiptReconciliation::Final(report) => report,
                ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
            };
        let evidence = report.mined_evidence.expect("mined evidence");

        assert_eq!(evidence.receipt_block_number, Some(100));
        assert_eq!(
            evidence.block_hash,
            Some(
                "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .parse()
                    .unwrap()
            )
        );
        assert_eq!(evidence.transaction_index, Some(7));
        assert_eq!(evidence.cumulative_gas_used, Some(42_000));
        assert_eq!(evidence.submitted_block_number, Some(99));
        assert_eq!(evidence.expected_confirmation_block, Some(100));
        assert_eq!(evidence.confirmation_lag_blocks, Some(0));
    }

    #[test]
    fn gate3_a6_receipt_evidence_records_actual_paid_gas_cost() {
        let vault = Address::repeat_byte(0x22);
        let token = Address::repeat_byte(0x33);
        let receipt = receipt(
            "0x1",
            vault,
            token,
            BOUGHT_V2_SIGNATURE,
            [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        );

        let report =
            match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, vault).unwrap() {
                ReceiptReconciliation::Final(report) => report,
                ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
            };
        let evidence = report.mined_evidence.expect("mined evidence");

        assert_eq!(
            evidence.effective_gas_price_wei.as_deref(),
            Some("1000000000")
        );
        assert_eq!(evidence.legacy_gas_price_wei.as_deref(), Some("1000000000"));
        assert_eq!(
            evidence.paid_gas_cost_wei.as_deref(),
            Some("21000000000000")
        );
        assert_eq!(evidence.selected_gas_limit.as_deref(), Some("500000"));
        assert_eq!(
            evidence.selected_max_fee_per_gas_wei.as_deref(),
            Some("100000000000")
        );
        assert_eq!(
            evidence.selected_max_priority_fee_per_gas_wei.as_deref(),
            Some("50000000000")
        );
        assert_eq!(
            evidence.selected_bribe_priority_fee_per_gas_wei.as_deref(),
            Some("40000000000")
        );
        assert_eq!(
            evidence.selected_bribe_max_fee_per_gas_wei.as_deref(),
            Some("100000000000")
        );
        assert_eq!(
            report
                .gas_cost
                .as_ref()
                .map(|amount| amount.raw.to_string()),
            evidence.paid_gas_cost_wei
        );
    }

    #[test]
    fn gate3_a7_receipt_evidence_records_finality_policy() {
        let vault = Address::repeat_byte(0x22);
        let token = Address::repeat_byte(0x33);
        let receipt = receipt(
            "0x1",
            vault,
            token,
            BOUGHT_V2_SIGNATURE,
            [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        );

        let report =
            match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, vault).unwrap() {
                ReceiptReconciliation::Final(report) => report,
                ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
            };
        let evidence = report.mined_evidence.expect("mined evidence");

        assert_eq!(evidence.receipt_status.as_deref(), Some("0x1"));
        assert_eq!(evidence.accepted_confirmation_depth, Some(1));
        assert_eq!(evidence.recheck_confirmation_depth, Some(3));
    }

    #[test]
    fn gate3_a5_successful_sell_receipt_records_vault_event_amounts_and_gas() {
        let vault = Address::repeat_byte(0x22);
        let token = Address::repeat_byte(0x33);
        let receipt = receipt(
            "0x1",
            vault,
            token,
            EMERGENCY_SOLD_V2_SIGNATURE,
            [U256::from(20u64), U256::from(9u64), U256::from(1u64)],
        );

        let report =
            match reconcile_receipt(&submitted(OrderSide::Sell, token), &receipt, vault).unwrap() {
                ReceiptReconciliation::Final(report) => report,
                ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
            };

        assert_eq!(report.status, ExecutionStatus::Confirmed);
        assert_eq!(report.filled_amount.unwrap().raw, U256::from(9u64));
        assert!(report.token_amount.is_none());
        assert_eq!(
            report.gas_cost.as_ref().map(|amount| amount.raw),
            Some(U256::from(21_000_000_000_000u64))
        );
    }

    #[test]
    fn gate3_a4_successful_receipt_without_vault_event_stays_unresolved() {
        let vault = Address::repeat_byte(0x22);
        let token = Address::repeat_byte(0x33);
        let wrong_token = Address::repeat_byte(0x44);
        let receipt = receipt(
            "0x1",
            vault,
            wrong_token,
            BOUGHT_V2_SIGNATURE,
            [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        );

        let result = reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, vault).unwrap();

        assert!(matches!(result, ReceiptReconciliation::Unresolved(_)));
    }

    #[test]
    fn failed_receipt_marks_order_failed() {
        let vault = Address::repeat_byte(0x22);
        let token = Address::repeat_byte(0x33);
        let receipt = receipt(
            "0x0",
            vault,
            token,
            BOUGHT_V2_SIGNATURE,
            [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        );

        let report =
            match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, vault).unwrap() {
                ReceiptReconciliation::Final(report) => report,
                ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
            };

        assert_eq!(report.status, ExecutionStatus::Failed);
        assert!(report.error.unwrap().contains("status=0x0"));
    }

    #[derive(Clone)]
    struct CountingReceiptProvider {
        calls: Arc<AtomicUsize>,
        receipt: Option<RpcTransactionReceipt>,
    }

    #[async_trait]
    impl ReceiptProvider for CountingReceiptProvider {
        async fn transaction_receipt(
            &self,
            _tx_hash: TxHash,
        ) -> Result<Option<RpcTransactionReceipt>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.receipt.clone())
        }
    }

    #[tokio::test]
    async fn reconciliation_waits_until_next_processed_block() {
        let vault = Address::repeat_byte(0x22);
        let token = Address::repeat_byte(0x33);
        let calls = Arc::new(AtomicUsize::new(0));
        let reconciler = VaultReceiptReconciler::new(
            CountingReceiptProvider {
                calls: calls.clone(),
                receipt: Some(receipt(
                    "0x1",
                    vault,
                    token,
                    BOUGHT_V2_SIGNATURE,
                    [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
                )),
            },
            vault,
        );

        let early = reconciler
            .reconcile_after_processed_block(vec![submitted(OrderSide::Buy, token)], Some(99))
            .await
            .unwrap();

        assert!(early.reports.is_empty());
        assert!(early.unresolved.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let ready = reconciler
            .reconcile_after_processed_block(vec![submitted(OrderSide::Buy, token)], Some(100))
            .await
            .unwrap();

        assert_eq!(ready.reports.len(), 1);
        assert_eq!(ready.reports[0].status, ExecutionStatus::Confirmed);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
