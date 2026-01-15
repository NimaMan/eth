use crate::single_tx::unsigned::UnsignedTransaction;
use alloy_eips::eip2930::AccessListItem;
use alloy_primitives::{Address, Bytes, B256, U256};
use eyre::{bail, eyre, Result};
use serde_json::{Map, Value};
use std::str::FromStr;

/// Build an unsigned transaction directly from the JSON/dict representation
/// of a processed transaction stored in our live snapshots.
pub fn build_unsigned_transaction_from_processed_tx_json(
    value: &Value,
) -> Result<UnsignedTransaction> {
    let obj = value
        .as_object()
        .ok_or_else(|| eyre!("processed transaction must be a JSON object"))?;

    let mut tx = UnsignedTransaction::default();

    let from = obj
        .get("from_address")
        .and_then(Value::as_str)
        .ok_or_else(|| eyre!("processed transaction missing from_address"))?;
    tx.from = Some(parse_address(from)?);

    if let Some(to_str) = obj.get("to_address").and_then(Value::as_str) {
        if !to_str.is_empty() {
            tx.to = Some(parse_address(to_str)?);
        }
    }

    let value_field = obj
        .get("value")
        .ok_or_else(|| eyre!("processed transaction missing value"))?;
    tx.value = Some(parse_u256(value_field)?);

    let nonce_field = obj
        .get("nonce")
        .ok_or_else(|| eyre!("processed transaction missing nonce"))?;
    tx.nonce = Some(parse_u64(nonce_field)?);

    if let Some(input) = obj.get("input").and_then(Value::as_str) {
        if !input.is_empty() {
            tx.data = Some(Bytes::from(hex_to_bytes(input)?));
        }
    }

    if let Some(access_list) = obj.get("access_list").and_then(Value::as_array) {
        tx.access_list = build_access_list(access_list)?;
    }

    if let Some(hashes) = obj.get("blob_versioned_hashes").and_then(Value::as_array) {
        tx.blob_versioned_hashes = build_hashes(hashes)?;
    }

    let raw_type_value = obj
        .get("raw_tx_type")
        .ok_or_else(|| eyre!("processed transaction missing raw_tx_type"))?;
    let tx_type = parse_u64(raw_type_value)?;

    let fees_obj = obj
        .get("fees")
        .and_then(Value::as_object)
        .ok_or_else(|| eyre!("processed transaction missing fees object"))?;

    assign_fee_fields(tx_type, fees_obj, &mut tx)?;

    let max_blob_fee = obj
        .get("max_fee_per_blob_gas")
        .filter(|value| !value.is_null())
        .or_else(|| {
            fees_obj
                .get("max_fee_per_blob_gas")
                .filter(|value| !value.is_null())
        });
    if let Some(max_blob_fee_value) = max_blob_fee {
        tx.max_fee_per_blob_gas = Some(parse_u128(max_blob_fee_value)?);
    } else if tx_type == 3 {
        return Err(eyre!("blob transaction missing max_fee_per_blob_gas"));
    }

    Ok(tx)
}

fn assign_fee_fields(
    tx_type: u64,
    fees_obj: &Map<String, Value>,
    tx: &mut UnsignedTransaction,
) -> Result<()> {
    let gas_limit_value = fees_obj
        .get("gas_limit")
        .ok_or_else(|| eyre!("fees missing gas_limit"))?;
    tx.gas = Some(parse_u64(gas_limit_value)?);

    match tx_type {
        0 | 1 => {
            let gas_price_value = fees_obj
                .get("gas_price")
                .ok_or_else(|| eyre!("fees missing gas_price for legacy-like transaction"))?;
            tx.gas_price = Some(parse_u128(gas_price_value)?);
        }
        // Treat 2 (EIP-1559), 3 (EIP-4844 blob), and 4 (EIP-7702 auth list) with the same fee fields.
        2 | 3 | 4 => {
            let max_fee_value = fees_obj
                .get("max_fee_per_gas")
                .ok_or_else(|| eyre!("fees missing max_fee_per_gas for EIP-1559 transaction"))?;
            let max_priority_value = fees_obj
                .get("max_priority_fee")
                .ok_or_else(|| eyre!("fees missing max_priority_fee for EIP-1559 transaction"))?;
            tx.max_fee_per_gas = Some(parse_u128(max_fee_value)?);
            tx.max_priority_fee_per_gas = Some(parse_u128(max_priority_value)?);
        }
        _ => {
            bail!("unsupported transaction type {}", tx_type);
        }
    }

    Ok(())
}

fn build_access_list(entries: &[Value]) -> Result<Vec<AccessListItem>> {
    let mut list = Vec::with_capacity(entries.len());
    for entry in entries {
        let obj = entry
            .as_object()
            .ok_or_else(|| eyre!("access list entry must be object"))?;
        let address = obj
            .get("address")
            .and_then(Value::as_str)
            .ok_or_else(|| eyre!("access list entry missing address"))?;
        let storage_keys = obj
            .get("storage_keys")
            .and_then(Value::as_array)
            .ok_or_else(|| eyre!("access list entry missing storage_keys"))?;
        let mut keys = Vec::with_capacity(storage_keys.len());
        for key in storage_keys {
            if let Some(key_str) = key.as_str() {
                keys.push(parse_b256(key_str)?);
            }
        }
        list.push(AccessListItem {
            address: parse_address(address)?,
            storage_keys: keys,
        });
    }
    Ok(list)
}

fn build_hashes(entries: &[Value]) -> Result<Vec<B256>> {
    let mut hashes = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some(value) = entry.as_str() {
            hashes.push(parse_b256(value)?);
        }
    }
    Ok(hashes)
}

fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|err| eyre!("invalid address {}: {}", value, err))
}

fn parse_b256(value: &str) -> Result<B256> {
    let bytes = hex_to_bytes(value)?;
    if bytes.len() != 32 {
        bail!("expected 32-byte value, got {}", bytes.len());
    }
    Ok(B256::from_slice(&bytes))
}

fn parse_u64(value: &Value) -> Result<u64> {
    match value {
        Value::Number(num) => num.as_u64().ok_or_else(|| eyre!("invalid u64 field")),
        Value::String(s) => parse_u64_str(s),
        _ => Err(eyre!("invalid u64 field type {}", value)),
    }
}

fn parse_u64_str(s: &str) -> Result<u64> {
    if let Some(stripped) = s.strip_prefix("0x") {
        Ok(u64::from_str_radix(stripped, 16)?)
    } else {
        Ok(s.parse::<u64>()?)
    }
}

fn parse_u128(value: &Value) -> Result<u128> {
    match value {
        Value::Number(num) => num
            .as_u64()
            .map(|v| v as u128)
            .ok_or_else(|| eyre!("invalid u128 field")),
        Value::String(s) => parse_u128_str(s),
        _ => Err(eyre!("invalid u128 field type {}", value)),
    }
}

fn parse_u128_str(s: &str) -> Result<u128> {
    if let Some(stripped) = s.strip_prefix("0x") {
        Ok(u128::from_str_radix(stripped, 16)?)
    } else {
        Ok(s.parse::<u128>()?)
    }
}

fn parse_u256(value: &Value) -> Result<U256> {
    match value {
        Value::Number(num) => {
            if let Some(u) = num.as_u64() {
                Ok(U256::from(u))
            } else if let Some(i) = num.as_i64() {
                Ok(U256::from(i as u64))
            } else {
                Err(eyre!("invalid numeric value for u256"))
            }
        }
        Value::String(s) => parse_u256_str(s),
        _ => Err(eyre!("invalid u256 field type {}", value)),
    }
}

fn parse_u256_str(s: &str) -> Result<U256> {
    if let Some(stripped) = s.strip_prefix("0x") {
        U256::from_str_radix(stripped, 16).map_err(|err| eyre!("invalid hex u256: {}", err))
    } else {
        U256::from_str_radix(s, 10).map_err(|err| eyre!("invalid decimal u256: {}", err))
    }
}

fn hex_to_bytes(value: &str) -> Result<Vec<u8>> {
    let stripped = value.trim().trim_start_matches("0x");
    if stripped.is_empty() {
        return Ok(Vec::new());
    }
    let bytes = hex::decode(stripped)?;
    Ok(bytes)
}
