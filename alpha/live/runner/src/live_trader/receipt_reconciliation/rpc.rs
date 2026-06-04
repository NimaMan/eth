use alloy_primitives::{Address, B256, U256};
use async_trait::async_trait;
use eth_alpha_core::{amount::Amount, ids::TxHash};
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use serde_json::json;

#[async_trait]
pub(in crate::live_trader) trait ReceiptProvider: Send + Sync {
    async fn transaction_receipt(&self, tx_hash: TxHash) -> Result<Option<RpcTransactionReceipt>>;
}

#[derive(Clone)]
pub(in crate::live_trader) struct JsonRpcReceiptProvider {
    rpc_url: String,
    http: reqwest::Client,
}

impl JsonRpcReceiptProvider {
    pub(in crate::live_trader) fn new(rpc_url: impl Into<String>) -> Self {
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

#[derive(Clone, Debug, Deserialize)]
pub(in crate::live_trader) struct RpcTransactionReceipt {
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
    pub(super) logs: Vec<RpcLog>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct RpcLog {
    pub(super) address: String,
    #[serde(default)]
    pub(super) topics: Vec<String>,
    pub(super) data: String,
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
pub(super) enum ReceiptStatus {
    Succeeded,
    Failed,
    Unknown,
}

impl RpcTransactionReceipt {
    pub(super) fn normalized_status(&self) -> Option<String> {
        self.status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    pub(super) fn status(&self) -> Result<ReceiptStatus> {
        match self.status.as_deref().map(str::trim) {
            Some("0x1") => Ok(ReceiptStatus::Succeeded),
            Some("0x0") => Ok(ReceiptStatus::Failed),
            Some(value) if value.is_empty() => Ok(ReceiptStatus::Unknown),
            Some(value) => Err(eyre!("unknown receipt status {value:?}")),
            None => Ok(ReceiptStatus::Unknown),
        }
    }

    pub(super) fn block_hash(&self) -> Result<Option<B256>> {
        self.block_hash
            .as_deref()
            .map(parse_b256)
            .transpose()
            .wrap_err("invalid receipt blockHash")
    }

    pub(super) fn block_number(&self) -> Result<Option<u64>> {
        self.block_number
            .as_deref()
            .map(parse_hex_u64)
            .transpose()
            .wrap_err("invalid receipt blockNumber")
    }

    pub(super) fn gas_used(&self) -> Result<Option<u64>> {
        self.gas_used
            .as_deref()
            .map(parse_hex_u64)
            .transpose()
            .wrap_err("invalid receipt gasUsed")
    }

    pub(super) fn transaction_index(&self) -> Result<Option<u64>> {
        self.transaction_index
            .as_deref()
            .map(parse_hex_u64)
            .transpose()
            .wrap_err("invalid receipt transactionIndex")
    }

    pub(super) fn cumulative_gas_used(&self) -> Result<Option<u64>> {
        self.cumulative_gas_used
            .as_deref()
            .map(parse_hex_u64)
            .transpose()
            .wrap_err("invalid receipt cumulativeGasUsed")
    }

    pub(super) fn effective_gas_price_wei(&self) -> Result<Option<String>> {
        self.effective_gas_price
            .as_deref()
            .map(parse_hex_u256)
            .transpose()
            .map(|value| value.map(|value| value.to_string()))
            .wrap_err("invalid receipt effectiveGasPrice")
    }

    pub(super) fn legacy_gas_price_wei(&self) -> Result<Option<String>> {
        self.gas_price
            .as_deref()
            .map(parse_hex_u256)
            .transpose()
            .map(|value| value.map(|value| value.to_string()))
            .wrap_err("invalid receipt gasPrice")
    }

    pub(super) fn gas_cost(&self) -> Result<Option<Amount>> {
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

pub(super) fn indexed_address_topic(address: Address) -> B256 {
    let mut topic = [0u8; 32];
    topic[12..].copy_from_slice(address.as_slice());
    B256::from(topic)
}
