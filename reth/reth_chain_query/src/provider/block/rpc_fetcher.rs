use alloy_eips::{eip2930::AccessListItem, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, B256, U256};
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use serde_json::Value;
use tracing::warn;

use crate::provider::{
    CallFrame, CallType, Log, RawBlockData, TransactionData, TransactionReceipt, TransactionTrace,
};
use eyre::Result;
use std::str::FromStr;

pub struct RpcBlockDataFetcher {
    http: HttpClient,
    debug: HttpClient,
}

impl RpcBlockDataFetcher {
    pub fn new(http_endpoint: &str) -> Result<Self> {
        let http = HttpClientBuilder::default().build(http_endpoint)?;
        let debug = HttpClientBuilder::default().build(http_endpoint)?;
        Ok(Self { http, debug })
    }

    pub fn with_debug_endpoint(mut self, endpoint: &str) -> Result<Self> {
        self.debug = HttpClientBuilder::default().build(endpoint)?;
        Ok(self)
    }

    pub async fn fetch_block(&self, block_hash: B256) -> Result<Option<Value>> {
        let params = rpc_params![format!("{block_hash:#x}"), true];
        let block = self
            .http
            .request::<Option<Value>, _>("eth_getBlockByHash", params)
            .await?;
        Ok(block)
    }

    pub async fn fetch_receipts(&self, block_hash: B256) -> Result<Option<Vec<Value>>> {
        let params = rpc_params![format!("{block_hash:#x}")];
        let receipts = self
            .http
            .request::<Option<Vec<Value>>, _>("eth_getBlockReceipts", params)
            .await?;
        Ok(receipts)
    }

    pub async fn trace_transaction(&self, tx_hash: B256) -> Result<Value> {
        let params = rpc_params![
            format!("{tx_hash:#x}"),
            serde_json::json!({"tracer": "callTracer", "timeout": "60s"})
        ];
        let trace = self
            .debug
            .request::<Value, _>("debug_traceTransaction", params)
            .await?;
        Ok(trace)
    }

    pub async fn trace_transactions(&self, tx_hashes: &[B256]) -> Result<Vec<Value>> {
        let mut traces = Vec::with_capacity(tx_hashes.len());
        for hash in tx_hashes {
            match self.trace_transaction(*hash).await {
                Ok(value) => traces.push(value),
                Err(err) => {
                    warn!("trace for {hash:#x} failed: {}", err);
                    return Err(err);
                }
            }
        }
        Ok(traces)
    }

    pub async fn trace_block_by_number(&self, block_number: u64) -> Result<Vec<Value>> {
        let params = rpc_params![
            format!("0x{block_number:x}"),
            serde_json::json!({"tracer": "callTracer", "timeout": "60s"})
        ];
        let traces = self
            .debug
            .request::<Vec<Value>, _>("debug_traceBlockByNumber", params)
            .await?;
        Ok(traces)
    }

    pub async fn fetch_raw_block_data(
        &self,
        block_hash: B256,
        block_number: u64,
        include_traces: bool,
    ) -> Result<RawBlockData> {
        let block_value = self
            .fetch_block(block_hash)
            .await?
            .ok_or_else(|| eyre::eyre!("block {:?} missing from RPC", block_hash))?;
        let header = parse_block_header(&block_value)?;
        let transactions = parse_transactions(&block_value, &header)?;
        let receipts_value = self
            .fetch_receipts(block_hash)
            .await?
            .ok_or_else(|| eyre::eyre!("receipts for {:?} missing from RPC", block_hash))?;
        let receipts = parse_receipts(&receipts_value, header.number)?;

        let traces = if include_traces {
            let trace_entries = self.trace_block_by_number(block_number).await?;
            Some(parse_block_traces(trace_entries)?)
        } else {
            None
        };

        Ok(RawBlockData {
            header,
            transactions,
            receipts,
            traces,
        })
    }
}

fn parse_block_header(block: &Value) -> Result<crate::provider::BlockHeader> {
    Ok(crate::provider::BlockHeader {
        number: parse_u64(required_field(block, "number")?)?,
        hash: parse_b256(required_field(block, "hash")?)?,
        parent_hash: parse_b256(required_field(block, "parentHash")?)?,
        timestamp: parse_u64(required_field(block, "timestamp")?)?,
        gas_limit: parse_u64(required_field(block, "gasLimit")?)?,
        gas_used: parse_u64(required_field(block, "gasUsed")?)?,
        base_fee_per_gas: block
            .get("baseFeePerGas")
            .map(|v| parse_u64(v))
            .transpose()?,
        withdrawals_root: parse_b256_opt(block.get("withdrawalsRoot"))?,
        blob_gas_used: parse_u64_opt(block.get("blobGasUsed"))?,
        excess_blob_gas: parse_u64_opt(block.get("excessBlobGas"))?,
        parent_beacon_block_root: parse_b256_opt(block.get("parentBeaconBlockRoot"))?,
        requests_hash: parse_b256_opt(block.get("requestsHash"))?,
        block_access_list_hash: parse_b256_opt(block.get("blockAccessListHash"))?,
        slot_number: parse_u64_opt(block.get("slotNumber"))?,
    })
}

fn parse_transactions(
    block: &Value,
    header: &crate::provider::BlockHeader,
) -> Result<Vec<TransactionData>> {
    let txs = block
        .get("transactions")
        .and_then(Value::as_array)
        .ok_or_else(|| eyre::eyre!("block missing transactions array"))?;
    let mut results = Vec::with_capacity(txs.len());
    for (idx, tx) in txs.iter().enumerate() {
        let tx_index = parse_u64_opt(tx.get("transactionIndex"))?.unwrap_or(idx as u64);
        let hash = parse_b256(required_field(tx, "hash")?)?;
        let from = parse_address(required_field(tx, "from")?)?;
        let to = parse_address_optional(tx.get("to"))?;
        let value = parse_u256(required_field(tx, "value")?)?;
        let input = parse_bytes(Some(required_field(tx, "input")?))?;
        let gas_limit = parse_u64(required_field(tx, "gas")?)?;
        let gas_price = parse_u256_opt(tx.get("gasPrice"))?
            .unwrap_or_else(|| header.base_fee_per_gas.map(U256::from).unwrap_or_default());
        let nonce = parse_u64(required_field(tx, "nonce")?)?;
        let transaction_type = parse_u64_opt(tx.get("type"))?.unwrap_or(0) as u8;
        let max_fee_per_gas = parse_u256_opt(tx.get("maxFeePerGas"))?;
        let max_priority_fee_per_gas = parse_u256_opt(tx.get("maxPriorityFeePerGas"))?;
        let access_list = parse_access_list(tx.get("accessList"))?;
        let blob_hashes = parse_blob_hashes(tx.get("blobVersionedHashes"))?;
        let max_fee_per_blob_gas = parse_u256_opt(tx.get("maxFeePerBlobGas"))?;
        let signed_authorizations = parse_authorization_list(tx.get("authorizationList"))?;

        results.push(TransactionData {
            hash,
            block_number: header.number,
            block_timestamp: header.timestamp,
            tx_index,
            tx_number: tx_index,
            from,
            to,
            value,
            input,
            gas_price,
            gas_limit,
            nonce,
            transaction_type,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            access_list,
            blob_versioned_hashes: blob_hashes,
            max_fee_per_blob_gas,
            signed_authorizations,
        });
    }
    Ok(results)
}

fn parse_receipts(receipts: &[Value], block_number: u64) -> Result<Vec<TransactionReceipt>> {
    let mut result = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        let tx_hash = parse_b256(required_field(receipt, "transactionHash")?)?;
        let status = match receipt.get("status") {
            Some(Value::Bool(b)) => *b,
            Some(value) => parse_u64(value)? != 0,
            None => true,
        };
        let gas_used = parse_u64(required_field(receipt, "gasUsed")?)?;
        let cumulative_gas_used = parse_u64(required_field(receipt, "cumulativeGasUsed")?)?;
        let effective_gas_price = parse_u256(required_field(receipt, "effectiveGasPrice")?)?;
        let contract_address = parse_address_optional(receipt.get("contractAddress"))?;
        let blob_gas_used = parse_u64_opt(receipt.get("blobGasUsed"))?;
        let logs = parse_logs(receipt, block_number)?;

        result.push(TransactionReceipt {
            tx_hash,
            status,
            gas_used,
            logs,
            cumulative_gas_used,
            effective_gas_price,
            contract_address,
            blob_gas_used,
        });
    }
    Ok(result)
}

fn parse_logs(receipt: &Value, block_number: u64) -> Result<Vec<Log>> {
    let logs = receipt
        .get("logs")
        .and_then(Value::as_array)
        .ok_or_else(|| eyre::eyre!("receipt missing logs"))?;
    let mut result = Vec::with_capacity(logs.len());
    for log in logs {
        let address = parse_address(required_field(log, "address")?)?;
        let topics = log
            .get("topics")
            .and_then(Value::as_array)
            .ok_or_else(|| eyre::eyre!("log missing topics"))?
            .iter()
            .map(|topic| parse_b256(topic))
            .collect::<Result<Vec<_>>>()?;
        let data = parse_bytes(Some(required_field(log, "data")?))?;
        let log_index = parse_u64(required_field(log, "logIndex")?)?;
        let transaction_index = parse_u64(required_field(log, "transactionIndex")?)?;

        result.push(Log {
            address,
            topics,
            data,
            log_index,
            transaction_index,
            block_number,
        });
    }
    Ok(result)
}

#[allow(dead_code)]
fn parse_traces(values: &[Value]) -> Result<Vec<TransactionTrace>> {
    values.iter().map(parse_trace).collect()
}

fn parse_block_traces(entries: Vec<Value>) -> Result<Vec<TransactionTrace>> {
    let mut traces = Vec::with_capacity(entries.len());
    for entry in entries {
        let trace_value = match entry {
            Value::Object(mut map) => map.remove("result").unwrap_or(Value::Object(map)),
            other => other,
        };
        traces.push(parse_trace(&trace_value)?);
    }
    Ok(traces)
}

fn parse_trace(value: &Value) -> Result<TransactionTrace> {
    let call_frame = parse_call_frame(value, 0)?;
    let gas_used = parse_u64(required_field(value, "gasUsed")?)?;
    let output = parse_bytes(value.get("output"))?;
    let error = value
        .get("error")
        .and_then(Value::as_str)
        .map(|s| s.to_string());
    Ok(TransactionTrace {
        call_frame,
        gas_used,
        output,
        error,
    })
}

fn parse_call_frame(value: &Value, depth: u32) -> Result<CallFrame> {
    let from = parse_address(required_field(value, "from")?)?;
    let to = parse_address_optional(value.get("to"))?;
    let call_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("CALL")
        .to_ascii_uppercase();
    let call_type = match call_type.as_str() {
        "CALL" => CallType::Call,
        "DELEGATECALL" => CallType::DelegateCall,
        "STATICCALL" => CallType::StaticCall,
        "CREATE" => CallType::Create,
        "CREATE2" => CallType::Create2,
        "SELFDESTRUCT" => CallType::Call,
        other => return Err(eyre::eyre!("unsupported call type {}", other)),
    };
    let gas_limit = parse_u64(required_field(value, "gas")?)?;
    let gas_used = parse_u64(required_field(value, "gasUsed")?)?;
    let input = parse_bytes(Some(required_field(value, "input")?))?;
    let output = parse_bytes(value.get("output"))?;
    let subcalls = value
        .get("calls")
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .map(|call| parse_call_frame(call, depth + 1))
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let value_field = parse_u256_opt(value.get("value"))?.unwrap_or_default();

    Ok(CallFrame {
        from,
        to,
        value: value_field,
        input,
        output,
        gas_used,
        gas_limit,
        depth,
        call_type,
        subcalls,
    })
}

fn parse_access_list(value: Option<&Value>) -> Result<Vec<AccessListItem>> {
    let Some(entries) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut list = Vec::with_capacity(entries.len());
    for entry in entries {
        let address = parse_address(required_field(entry, "address")?)?;
        let storage_keys = entry
            .get("storageKeys")
            .and_then(Value::as_array)
            .ok_or_else(|| eyre::eyre!("access list entry missing storageKeys"))?
            .iter()
            .map(|key| parse_b256(key))
            .collect::<Result<Vec<_>>>()?;
        list.push(AccessListItem {
            address,
            storage_keys,
        });
    }
    Ok(list)
}

fn parse_blob_hashes(value: Option<&Value>) -> Result<Vec<B256>> {
    let Some(entries) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    entries.iter().map(|v| parse_b256(v)).collect()
}

fn parse_authorization_list(value: Option<&Value>) -> Result<Vec<SignedAuthorization>> {
    match value {
        Some(Value::Array(_)) => serde_json::from_value(value.cloned().unwrap_or(Value::Null))
            .map_err(|err| eyre::eyre!("invalid authorizationList: {}", err)),
        _ => Ok(Vec::new()),
    }
}

fn required_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value> {
    value
        .get(key)
        .ok_or_else(|| eyre::eyre!("field {} missing", key))
}

fn parse_u64(value: &Value) -> Result<u64> {
    match value {
        Value::String(s) => parse_u64_hex_str(s),
        Value::Number(num) => num
            .as_u64()
            .ok_or_else(|| eyre::eyre!("invalid numeric value {}", value)),
        _ => Err(eyre::eyre!("invalid type for u64 field: {:?}", value)),
    }
}

fn parse_u64_opt(value: Option<&Value>) -> Result<Option<u64>> {
    match value {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(value) => parse_u64(value).map(Some),
    }
}

fn parse_u64_hex_str(value: &str) -> Result<u64> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        if hex.is_empty() {
            return Ok(0);
        }
        u64::from_str_radix(hex, 16).map_err(|err| eyre::eyre!("invalid hex {}: {}", value, err))
    } else {
        trimmed
            .parse::<u64>()
            .map_err(|err| eyre::eyre!("invalid decimal {}: {}", value, err))
    }
}

fn parse_u256(value: &Value) -> Result<U256> {
    match value {
        Value::String(s) => parse_u256_hex_str(s),
        Value::Number(num) => Ok(U256::from(num.as_u64().unwrap_or(0))),
        _ => Err(eyre::eyre!("invalid type for U256: {:?}", value)),
    }
}

fn parse_u256_opt(value: Option<&Value>) -> Result<Option<U256>> {
    value.map(parse_u256).transpose()
}

fn parse_u256_hex_str(value: &str) -> Result<U256> {
    let trimmed = value.trim();
    let digits = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    U256::from_str_radix(digits, 16).map_err(|err| eyre::eyre!("invalid hex {}: {}", value, err))
}

fn parse_address(value: &Value) -> Result<Address> {
    let s = value
        .as_str()
        .ok_or_else(|| eyre::eyre!("invalid address value {:?}", value))?;
    Address::from_str(s).map_err(|err| eyre::eyre!("invalid address {}: {}", s, err))
}

fn parse_address_optional(value: Option<&Value>) -> Result<Option<Address>> {
    match value {
        Some(Value::String(s)) if !s.is_empty() => Address::from_str(s)
            .map(Some)
            .map_err(|err| eyre::eyre!("invalid address {}: {}", s, err)),
        _ => Ok(None),
    }
}

fn parse_b256(value: &Value) -> Result<B256> {
    let s = value
        .as_str()
        .ok_or_else(|| eyre::eyre!("invalid hash value {:?}", value))?;
    B256::from_str(s).map_err(|err| eyre::eyre!("invalid hash {}: {}", s, err))
}

fn parse_b256_opt(value: Option<&Value>) -> Result<Option<B256>> {
    match value {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(value) => parse_b256(value).map(Some),
    }
}

fn parse_bytes(value: Option<&Value>) -> Result<Bytes> {
    match value {
        Some(Value::String(s)) => {
            let stripped = s.strip_prefix("0x").unwrap_or(s);
            let bytes = hex::decode(stripped)
                .map_err(|err| eyre::eyre!("invalid hex bytes {}: {}", s, err))?;
            Ok(Bytes::from(bytes))
        }
        Some(Value::Null) | None => Ok(Bytes::new()),
        Some(other) => Err(eyre::eyre!("invalid bytes field {:?}", other)),
    }
}
