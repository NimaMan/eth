use crate::block_processor::{ProcessedBlock, ProcessedBlockTransactions};
use alloy_primitives::Address;
use eyre::{eyre, Result};
use serde::Serialize;
use serde_json::{json, Value};
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
    let mut payload = serde_json::to_value(&tx.processed).map_err(|err| {
        eyre!(
            "failed to serialize processed transaction {}: {}",
            tx_hash,
            err
        )
    })?;

    let object = payload.as_object_mut().ok_or_else(|| {
        eyre!(
            "processed transaction {} did not serialize to an object",
            tx_hash
        )
    })?;

    object.insert("hash".to_string(), json!(tx_hash.clone()));
    object.insert(
        "from_address".to_string(),
        json!(reth_chain_query::to_checksum_address(
            &tx.processed.from_address
        )),
    );
    object.insert(
        "to_address".to_string(),
        tx.processed
            .to_address
            .map(|address| json!(reth_chain_query::to_checksum_address(&address)))
            .unwrap_or(Value::Null),
    );
    object.insert(
        "contract_address".to_string(),
        tx.processed
            .contract_address
            .map(|address| json!(reth_chain_query::to_checksum_address(&address)))
            .unwrap_or(Value::Null),
    );
    object.insert(
        "input".to_string(),
        json!(format!("0x{}", hex::encode(&tx.processed.input))),
    );
    object.insert(
        "unique_addresses".to_string(),
        json!(unique_addresses.clone()),
    );
    object.insert(
        "erc20_contracts".to_string(),
        address_set_to_json(&tx.processed.erc20_contracts),
    );
    object.insert(
        "erc721_contracts".to_string(),
        address_set_to_json(&tx.processed.erc721_contracts),
    );
    object.insert(
        "erc1155_contracts".to_string(),
        address_set_to_json(&tx.processed.erc1155_contracts),
    );
    object.insert(
        "processing_error".to_string(),
        tx.processing_error
            .as_ref()
            .map(|error| json!(error))
            .unwrap_or(Value::Null),
    );

    prune_empty_json_fields(&mut payload);
    let tx_json = serde_json::to_string(&payload).map_err(|err| {
        eyre!(
            "failed to encode processed transaction {}: {}",
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

fn address_set_to_json(addresses: &HashSet<Address>) -> Value {
    json!(address_set_to_strings(addresses))
}

fn address_set_to_strings(addresses: &HashSet<Address>) -> Vec<String> {
    let mut values: Vec<String> = addresses
        .iter()
        .map(reth_chain_query::to_checksum_address)
        .collect();
    values.sort();
    values
}

fn prune_empty_json_fields(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for nested in object.values_mut() {
                prune_empty_json_fields(nested);
            }
            object.retain(|_, nested| !is_empty_json_field(nested));
        }
        Value::Array(items) => {
            for nested in items {
                prune_empty_json_fields(nested);
            }
        }
        _ => {}
    }
}

fn is_empty_json_field(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.is_empty(),
        Value::Array(values) => values.is_empty(),
        Value::Object(values) => values.is_empty(),
        _ => false,
    }
}
