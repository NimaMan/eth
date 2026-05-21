use alloy_primitives::{hex, keccak256, Address, B256, U256};
use async_trait::async_trait;
use eth_alpha_core::{
    amount::Amount,
    execution::{ExecutionReport, ExecutionStatus},
    ids::TxHash,
    order::OrderSide,
};
use eth_alpha_store::SubmittedExecutionRecord;
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use serde_json::json;

const BOUGHT_V2_SIGNATURE: &str = "BoughtV2(address,uint256,uint256,uint256)";
const EMERGENCY_SOLD_V2_SIGNATURE: &str = "EmergencySoldV2(address,uint256,uint256,uint256)";

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
    match receipt.status()? {
        ReceiptStatus::Failed => Ok(ReceiptReconciliation::Final(ExecutionReport {
            order_id: record.order_id.clone(),
            status: ExecutionStatus::Failed,
            tx_hash: Some(record.tx_hash),
            block_number,
            filled_amount: None,
            token_amount: None,
            gas_used,
            gas_cost,
            error: Some("transaction receipt status=0x0".to_string()),
        })),
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

            Ok(ReceiptReconciliation::Final(ExecutionReport {
                order_id: record.order_id.clone(),
                status: ExecutionStatus::Confirmed,
                tx_hash: Some(record.tx_hash),
                block_number,
                filled_amount: Some(fill.filled_amount),
                token_amount: fill.token_amount,
                gas_used,
                gas_cost,
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

#[derive(Clone, Debug, Deserialize)]
pub(super) struct RpcTransactionReceipt {
    #[serde(rename = "blockNumber")]
    block_number: Option<String>,
    status: Option<String>,
    #[serde(rename = "gasUsed")]
    gas_used: Option<String>,
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
    fn status(&self) -> Result<ReceiptStatus> {
        match self.status.as_deref().map(str::trim) {
            Some("0x1") => Ok(ReceiptStatus::Succeeded),
            Some("0x0") => Ok(ReceiptStatus::Failed),
            Some(value) if value.is_empty() => Ok(ReceiptStatus::Unknown),
            Some(value) => Err(eyre!("unknown receipt status {value:?}")),
            None => Ok(ReceiptStatus::Unknown),
        }
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

#[cfg(test)]
mod tests {
    use eth_alpha_core::{
        ids::{OrderId, PositionId, TradeId},
        order::OrderSide,
    };
    use serde_json::json;

    use super::*;

    fn submitted(side: OrderSide, token: Address) -> SubmittedExecutionRecord {
        SubmittedExecutionRecord {
            order_id: OrderId("order-1".to_string()),
            tx_hash: "0x1111111111111111111111111111111111111111111111111111111111111111"
                .parse()
                .unwrap(),
            position_id: PositionId("pos-1".to_string()),
            trade_id: Some(TradeId("trade-1".to_string())),
            order_side: side,
            token_address: token,
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
            "status": status,
            "gasUsed": "0x5208",
            "effectiveGasPrice": "0x3b9aca00",
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
    fn successful_buy_receipt_confirms_from_bought_event() {
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
    }

    #[test]
    fn successful_sell_receipt_confirms_from_emergency_sell_event() {
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
    }

    #[test]
    fn successful_receipt_without_vault_event_stays_unresolved() {
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
}
