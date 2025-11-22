//! Utilities for parsing Ethereum block headers from JSON.
//!
//! This module provides functions to deserialize JSON representations of block headers
//! into `reth_primitives::SealedHeader` objects, along with various helper functions
//! for parsing different primitive types (e.g., `B256`, `Address`, `U256`, `Bloom`)
//! from JSON values. It's designed to facilitate the consumption of block data
//! from external sources (like RPC responses) for simulation purposes.
use alloy_consensus::Header as AlloyHeader;
use alloy_primitives::{
    Address as AlloyAddress, Bloom, Bytes as AlloyBytes, B256, B64, U256 as AlloyU256,
};
use eyre::{eyre, Result};
use reth_primitives::SealedHeader;
use serde_json::{Map, Value};
use std::str::FromStr;

pub fn parse_sealed_header_from_json(json: &str) -> Result<SealedHeader> {
    let value: Value = serde_json::from_str(json)?;
    let obj = value
        .as_object()
        .ok_or_else(|| eyre!("block header JSON must be an object"))?;

    let hash = parse_b256(obj, "hash")?;
    let header = AlloyHeader {
        parent_hash: parse_b256(obj, "parentHash")?,
        ommers_hash: parse_b256(obj, "sha3Uncles")?,
        beneficiary: parse_address(obj, "miner")?,
        state_root: parse_b256(obj, "stateRoot")?,
        transactions_root: parse_b256(obj, "transactionsRoot")?,
        receipts_root: parse_b256(obj, "receiptsRoot")?,
        logs_bloom: parse_bloom(obj, "logsBloom")?,
        difficulty: parse_u256(obj, "difficulty")?,
        number: parse_u64(obj, "number")?,
        gas_limit: parse_u64(obj, "gasLimit")?,
        gas_used: parse_u64(obj, "gasUsed")?,
        timestamp: parse_u64(obj, "timestamp")?,
        extra_data: parse_bytes(obj, "extraData")?,
        mix_hash: parse_b256(obj, "mixHash")?,
        nonce: parse_b64(obj, "nonce")?,
        base_fee_per_gas: parse_optional_u64(obj, "baseFeePerGas")?,
        withdrawals_root: parse_optional_b256(obj, "withdrawalsRoot")?,
        blob_gas_used: parse_optional_u64(obj, "blobGasUsed")?,
        excess_blob_gas: parse_optional_u64(obj, "excessBlobGas")?,
        parent_beacon_block_root: parse_optional_b256(obj, "parentBeaconBlockRoot")?,
        requests_hash: None,
    };

    Ok(SealedHeader::new(header, hash))
}

fn parse_b256(obj: &Map<String, Value>, key: &str) -> Result<B256> {
    let value = obj
        .get(key)
        .ok_or_else(|| eyre!("missing {key} in block header"))?;
    let bytes = match value_to_bytes(value)? {
        Some(bytes) => bytes,
        None => return Err(eyre!("{key} cannot be null")),
    };
    if bytes.len() != 32 {
        return Err(eyre!("{key} must be 32 bytes, got {}", bytes.len()));
    }
    Ok(B256::from_slice(&bytes))
}

fn parse_optional_b256(obj: &Map<String, Value>, key: &str) -> Result<Option<B256>> {
    let Some(value) = obj.get(key) else {
        return Ok(None);
    };
    let bytes = match value_to_bytes(value)? {
        Some(bytes) => bytes,
        None => return Ok(None),
    };
    if bytes.len() != 32 {
        return Err(eyre!("{key} must be 32 bytes, got {}", bytes.len()));
    }
    Ok(Some(B256::from_slice(&bytes)))
}

fn parse_address(obj: &Map<String, Value>, key: &str) -> Result<AlloyAddress> {
    let value = obj
        .get(key)
        .ok_or_else(|| eyre!("missing {key} in block header"))?;
    let address_str = match value {
        Value::String(s) => s.as_str(),
        _ => return Err(eyre!("{key} must be a hex string")),
    };
    AlloyAddress::from_str(address_str.trim()).map_err(|e| eyre!("invalid {key}: {e}"))
}

fn parse_bloom(obj: &Map<String, Value>, key: &str) -> Result<Bloom> {
    let value = obj
        .get(key)
        .ok_or_else(|| eyre!("missing {key} in block header"))?;
    let bytes = match value_to_bytes(value)? {
        Some(bytes) => bytes,
        None => return Err(eyre!("{key} cannot be null")),
    };
    if bytes.len() != 256 {
        return Err(eyre!("{key} must be 256 bytes, got {}", bytes.len()));
    }
    Ok(Bloom::from_slice(&bytes))
}

fn parse_bytes(obj: &Map<String, Value>, key: &str) -> Result<AlloyBytes> {
    let value = obj
        .get(key)
        .ok_or_else(|| eyre!("missing {key} in block header"))?;
    let bytes = match value_to_bytes(value)? {
        Some(bytes) => bytes,
        None => return Ok(AlloyBytes::default()),
    };
    Ok(AlloyBytes::from(bytes))
}

fn parse_b64(obj: &Map<String, Value>, key: &str) -> Result<B64> {
    let value = obj
        .get(key)
        .ok_or_else(|| eyre!("missing {key} in block header"))?;
    let bytes = match value_to_bytes(value)? {
        Some(bytes) => bytes,
        None => return Err(eyre!("{key} cannot be null")),
    };
    if bytes.len() != 8 {
        return Err(eyre!("{key} must be 8 bytes, got {}", bytes.len()));
    }
    Ok(B64::from_slice(&bytes))
}

fn parse_u64(obj: &Map<String, Value>, key: &str) -> Result<u64> {
    let value = obj
        .get(key)
        .ok_or_else(|| eyre!("missing {key} in block header"))?;
    match value {
        Value::Number(num) => num.as_u64().ok_or_else(|| eyre!("{key} must fit in u64")),
        Value::String(s) => Ok(parse_string_to_u64(s)?),
        _ => Err(eyre!("{key} must be number or hex string")),
    }
}

fn parse_optional_u64(obj: &Map<String, Value>, key: &str) -> Result<Option<u64>> {
    let Some(value) = obj.get(key) else {
        return Ok(None);
    };
    match value {
        Value::Null => Ok(None),
        Value::Number(num) => Ok(num.as_u64()),
        Value::String(s) if s.is_empty() => Ok(None),
        Value::String(s) => Ok(Some(parse_string_to_u64(s)?)),
        _ => Err(eyre!("{key} must be number or hex string")),
    }
}

fn parse_u256(obj: &Map<String, Value>, key: &str) -> Result<AlloyU256> {
    let value = obj
        .get(key)
        .ok_or_else(|| eyre!("missing {key} in block header"))?;
    match value {
        Value::Number(num) => {
            let as_u64 = num.as_u64().ok_or_else(|| eyre!("{key} must fit in u64"))?;
            Ok(AlloyU256::from(as_u64))
        }
        Value::String(s) => parse_string_to_u256(s),
        _ => Err(eyre!("{key} must be number or hex string")),
    }
}

fn parse_string_to_u64(value: &str) -> Result<u64> {
    if let Some(stripped) = value.strip_prefix("0x") {
        return Ok(u64::from_str_radix(stripped, 16)?);
    }
    Ok(value.parse::<u64>()?)
}

fn parse_string_to_u256(value: &str) -> Result<AlloyU256> {
    if let Some(stripped) = value.strip_prefix("0x") {
        return AlloyU256::from_str_radix(stripped, 16)
            .map_err(|err| eyre!("invalid u256: {}", err));
    }
    value
        .parse::<u128>()
        .map(AlloyU256::from)
        .map_err(|err| eyre!("invalid decimal u256: {}", err))
}

fn value_to_bytes(value: &Value) -> Result<Option<Vec<u8>>> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) => {
            let stripped = s.trim_start_matches("0x");
            Ok(Some(hex::decode(stripped)?))
        }
        Value::Array(arr) => {
            let mut bytes = Vec::with_capacity(arr.len());
            for entry in arr {
                let Value::Number(num) = entry else {
                    return Err(eyre!("byte arrays must contain numbers"));
                };
                let byte = num.as_u64().ok_or_else(|| eyre!("byte must fit in u8"))? as u8;
                bytes.push(byte);
            }
            Ok(Some(bytes))
        }
        _ => Err(eyre!("unsupported value type for byte field")),
    }
}
