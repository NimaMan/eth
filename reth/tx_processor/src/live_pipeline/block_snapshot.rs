use crate::block_processor::ProcessedBlock;
use eyre::{eyre, Result};
use serde::Serialize;
use serde_json::json;

/// Snapshot payload ready for Redis persistence.
#[derive(Debug, Clone)]
pub struct LiveBlockSnapshot {
    pub block_number: u64,
    pub payload: String,
    pub header_json: Option<String>,
    pub tx_entries: Vec<(String, String)>,
    pub tx_count: usize,
}

/// Build a snapshot that mirrors Python's `build_block_snapshot` output.
pub fn build_live_block_snapshot(block: &ProcessedBlock) -> Result<LiveBlockSnapshot> {
    let header_json = serde_json::to_string(&HeaderPayload::from(&block.header))
        .map_err(|err| eyre!("failed to serialize block header: {}", err))?;

    let tx_values: Vec<serde_json::Value> = Vec::new();
    let payload_value = json!({
        "block_number": block.header.number,
        "header": header_json,
        "tx_count": tx_values.len(),
        "transactions": tx_values,
    });
    let payload = serde_json::to_string(&payload_value)
        .map_err(|err| eyre!("failed to serialize block snapshot payload: {}", err))?;

    let tx_entries: Vec<(String, String)> = Vec::new();

    Ok(LiveBlockSnapshot {
        block_number: block.header.number,
        payload,
        header_json: Some(header_json),
        tx_entries,
        tx_count: tx_values.len(),
    })
}

#[derive(Serialize)]
struct HeaderPayload {
    #[serde(rename = "hash")]
    hash: String,
    #[serde(rename = "parentHash")]
    parent_hash: String,
    #[serde(rename = "number")]
    number: String,
    #[serde(rename = "gasLimit")]
    gas_limit: String,
    #[serde(rename = "gasUsed")]
    gas_used: String,
    #[serde(rename = "timestamp")]
    timestamp: String,
    #[serde(rename = "baseFeePerGas", skip_serializing_if = "Option::is_none")]
    base_fee_per_gas: Option<String>,
}

impl From<&reth_chain_query::provider::BlockHeader> for HeaderPayload {
    fn from(header: &reth_chain_query::provider::BlockHeader) -> Self {
        Self {
            hash: format!("{:#x}", header.hash),
            parent_hash: format!("{:#x}", header.parent_hash),
            number: to_hex(header.number),
            gas_limit: to_hex(header.gas_limit),
            gas_used: to_hex(header.gas_used),
            timestamp: to_hex(header.timestamp),
            base_fee_per_gas: header.base_fee_per_gas.map(to_hex),
        }
    }
}

fn to_hex(value: u64) -> String {
    format!("0x{:x}", value)
}
