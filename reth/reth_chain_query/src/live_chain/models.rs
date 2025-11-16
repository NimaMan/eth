use alloy_primitives::B256;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct HeadObservation {
    pub slot: u64,
    pub block_root: B256,
    pub state_root: B256,
    pub arrival_time: DateTime<Utc>,
    pub execution_optimistic: bool,
}

#[derive(Debug, Clone)]
pub struct HeadLatencySample {
    pub observation: HeadObservation,
    pub execution_info: Option<ExecutionInfo>,
    pub block_number: Option<u64>,
    pub block_timestamp: Option<u64>,
    pub rpc_ready_at: Option<DateTime<Utc>>,
    pub rpc_fetch_delay_secs: Option<f64>,
    pub transaction_count: Option<usize>,
    pub block: Value,
    pub receipts: Vec<Value>,
    pub traces: Value,
}

impl HeadLatencySample {
    pub fn new(observation: HeadObservation) -> Self {
        Self {
            observation,
            execution_info: None,
            block_number: None,
            block_timestamp: None,
            rpc_ready_at: None,
            rpc_fetch_delay_secs: None,
            transaction_count: None,
            block: Value::Null,
            receipts: Vec::new(),
            traces: Value::Null,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionInfo {
    pub block_hash: B256,
    pub block_number: u64,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize)]
pub struct LighthouseHeadPayload {
    pub slot: String,
    pub block: String,
    pub state: String,
    #[serde(default)]
    pub execution_optimistic: bool,
}

impl LighthouseHeadPayload {
    pub fn to_observation(&self, arrival_time: DateTime<Utc>) -> eyre::Result<HeadObservation> {
        let slot = parse_u64_hex(&self.slot)?;
        Ok(HeadObservation {
            slot,
            block_root: parse_b256(&self.block)?,
            state_root: parse_b256(&self.state)?,
            arrival_time,
            execution_optimistic: self.execution_optimistic,
        })
    }
}

fn parse_b256(value: &str) -> eyre::Result<B256> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(trimmed)?;
    if bytes.len() != 32 {
        return Err(eyre::eyre!("expected 32-byte root, got {}", value));
    }
    Ok(B256::from_slice(&bytes))
}

fn parse_u64_hex(value: &str) -> eyre::Result<u64> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    u64::from_str_radix(trimmed, 16)
        .map_err(|err| eyre::eyre!("invalid hex int {}: {}", value, err))
}
