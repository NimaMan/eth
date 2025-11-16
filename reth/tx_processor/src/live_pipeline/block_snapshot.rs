use crate::block_processor::ProcessedBlock;
use eyre::{eyre, Result};
use serde::Serialize;

/// Snapshot payload ready for Redis persistence.
#[derive(Debug, Clone)]
pub struct LiveBlockSnapshot {
    pub block_number: u64,
    pub header_json: Option<String>,
    pub tx_entries: Vec<(String, String)>,
}

/// Build a snapshot that mirrors Python's `build_block_snapshot` output.
pub fn build_live_block_snapshot(block: &ProcessedBlock) -> Result<LiveBlockSnapshot> {
    let header_json = serde_json::to_string(&HeaderPayload::from(&block.header))
        .map_err(|err| eyre!("failed to serialize block header: {}", err))?;

    let tx_entries: Vec<(String, String)> = Vec::new();

    Ok(LiveBlockSnapshot {
        block_number: block.header.number,
        header_json: Some(header_json),
        tx_entries,
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
