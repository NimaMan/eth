use crate::block_processor::{ProcessedBlock, ProcessedBlockTransactions};
use crate::processed_block_provider::CompactProcessedTransaction;
use alloy_primitives::Address;
use eyre::{eyre, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

/// Snapshot payload ready for Redis persistence.
#[derive(Debug, Clone)]
pub struct LiveBlockSnapshot {
    pub block_number: u64,
    pub block_hash: String,
    pub parent_hash: Option<String>,
    pub timestamp: u64,
    pub header_json: Option<String>,
    pub tx_entries: Vec<LiveTxEntry>,
}

#[derive(Debug, Clone)]
pub struct LiveTxEntry {
    pub hash: String,
    pub tx_index: u64,
    pub payload_json: String,
    pub unique_addresses: Vec<String>,
}

/// Build a snapshot that matches the Redis live-data block schema.
pub fn build_live_block_snapshot(block: &ProcessedBlock) -> Result<LiveBlockSnapshot> {
    let header_json = serde_json::to_string(&HeaderPayload::from(&block.header))
        .map_err(|err| eyre!("failed to serialize block header: {}", err))?;

    let tx_entries = block
        .transactions
        .iter()
        .map(build_transaction_entry)
        .collect::<Result<Vec<_>>>()?;

    Ok(LiveBlockSnapshot {
        block_number: block.header.number,
        block_hash: format!("{:#x}", block.header.hash),
        parent_hash: Some(format!("{:#x}", block.header.parent_hash)),
        timestamp: block.header.timestamp,
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
    #[serde(rename = "withdrawalsRoot", skip_serializing_if = "Option::is_none")]
    withdrawals_root: Option<String>,
    #[serde(rename = "blobGasUsed", skip_serializing_if = "Option::is_none")]
    blob_gas_used: Option<String>,
    #[serde(rename = "excessBlobGas", skip_serializing_if = "Option::is_none")]
    excess_blob_gas: Option<String>,
    #[serde(
        rename = "parentBeaconBlockRoot",
        skip_serializing_if = "Option::is_none"
    )]
    parent_beacon_block_root: Option<String>,
    #[serde(rename = "requestsHash", skip_serializing_if = "Option::is_none")]
    requests_hash: Option<String>,
    #[serde(
        rename = "blockAccessListHash",
        skip_serializing_if = "Option::is_none"
    )]
    block_access_list_hash: Option<String>,
    #[serde(rename = "slotNumber", skip_serializing_if = "Option::is_none")]
    slot_number: Option<String>,
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
            withdrawals_root: header.withdrawals_root.map(|value| format!("{value:#x}")),
            blob_gas_used: header.blob_gas_used.map(to_hex),
            excess_blob_gas: header.excess_blob_gas.map(to_hex),
            parent_beacon_block_root: header
                .parent_beacon_block_root
                .map(|value| format!("{value:#x}")),
            requests_hash: header.requests_hash.map(|value| format!("{value:#x}")),
            block_access_list_hash: header
                .block_access_list_hash
                .map(|value| format!("{value:#x}")),
            slot_number: header.slot_number.map(to_hex),
        }
    }
}

fn to_hex(value: u64) -> String {
    format!("0x{:x}", value)
}

fn build_transaction_entry(tx: &ProcessedBlockTransactions) -> Result<LiveTxEntry> {
    let tx_hash = format!("{:#x}", tx.processed.hash);
    let unique_addresses = address_set_to_strings(&tx.processed.unique_addresses);
    let compact = CompactProcessedTransaction::from_processed(&tx.processed);
    let mut payload = compact.to_sparse_json_value().map_err(|err| {
        eyre!(
            "failed to serialize compact processed transaction {}: {}",
            tx_hash,
            err
        )
    })?;
    let object = payload.as_object_mut().ok_or_else(|| {
        eyre!(
            "compact processed transaction {} did not serialize to an object",
            tx_hash
        )
    })?;
    if let Some(error) = &tx.processing_error {
        object.insert("processing_error".to_string(), Value::String(error.clone()));
    }
    let tx_json = serde_json::to_string(&payload).map_err(|err| {
        eyre!(
            "failed to encode compact processed transaction {}: {}",
            tx_hash,
            err
        )
    })?;
    Ok(LiveTxEntry {
        hash: tx_hash,
        tx_index: tx.processed.tx_index,
        payload_json: tx_json,
        unique_addresses,
    })
}

fn address_set_to_strings(addresses: &HashSet<Address>) -> Vec<String> {
    let mut values: Vec<String> = addresses
        .iter()
        .map(reth_chain_query::to_checksum_address)
        .collect();
    values.sort();
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processed_block_provider::CompactProcessedTransaction;
    use crate::tx_processor::data_models::ProcessedTransaction;
    use alloy_primitives::{B256, U256};
    use reth_chain_query::provider::BlockHeader;

    #[test]
    fn live_snapshot_uses_compact_transaction_payload() {
        let mut tx = ProcessedTransaction::new(
            B256::repeat_byte(0x11),
            42,
            1_700_000_000,
            7,
            Address::repeat_byte(0x22),
            Some(Address::repeat_byte(0x33)),
            U256::from(123),
            true,
            9,
            2,
            vec![0xde, 0xad, 0xbe, 0xef],
        );
        tx.unique_addresses.insert(Address::repeat_byte(0x22));
        let block_tx = CompactProcessedTransaction::from_processed(&tx)
            .into_block_transaction(Some("simulated failure".to_string()));
        let block = ProcessedBlock {
            header: BlockHeader {
                number: 42,
                hash: B256::repeat_byte(0xaa),
                parent_hash: B256::repeat_byte(0xbb),
                timestamp: 1_700_000_000,
                gas_limit: 30_000_000,
                gas_used: 21_000,
                base_fee_per_gas: Some(1),
                withdrawals_root: None,
                blob_gas_used: None,
                excess_blob_gas: None,
                parent_beacon_block_root: None,
                requests_hash: None,
                block_access_list_hash: None,
                slot_number: None,
            },
            transactions: vec![block_tx],
        };

        let snapshot = build_live_block_snapshot(&block).expect("build live snapshot");
        let payload: Value =
            serde_json::from_str(&snapshot.tx_entries[0].payload_json).expect("payload json");
        let object = payload.as_object().expect("payload object");

        assert_eq!(
            object
                .get("processed_tx_schema_version")
                .and_then(Value::as_u64),
            Some(crate::COMPACT_PROCESSED_TRANSACTION_SCHEMA_VERSION as u64)
        );
        assert_eq!(
            object.get("input").and_then(Value::as_str),
            Some("0xdeadbeef")
        );
        assert_eq!(
            object.get("processing_error").and_then(Value::as_str),
            Some("simulated failure")
        );
        assert!(!object.contains_key("actions"));
        assert!(!object.contains_key("bribe_amount"));
    }
}
