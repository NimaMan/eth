use eyre::Result;
use reth_chain_query::provider::{CallFrame, TransactionTrace};
use serde_json::{json, Value};
use tx_processor::ProcessedBlock;

pub(crate) fn normalize_processed_block(block: &ProcessedBlock) -> Result<Value> {
    let txs = block
        .transactions
        .iter()
        .map(|tx| -> Result<Value> {
            let mut processed = serde_json::to_value(&tx.processed)?;
            canonicalize_processed_value(&mut processed);
            Ok(json!({
                "metadata": serde_json::to_value(&tx.metadata)?,
                "receipt": serde_json::to_value(&tx.receipt)?,
                "processed": processed,
                "trace": tx.trace.as_ref().map(normalize_trace).transpose()?,
                "processing_error": tx.processing_error,
            }))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(json!({
        "header": {
            "number": block.header.number,
            "hash": format!("{:#x}", block.header.hash),
            "parent_hash": format!("{:#x}", block.header.parent_hash),
            "timestamp": block.header.timestamp,
            "gas_limit": block.header.gas_limit,
            "gas_used": block.header.gas_used,
            "base_fee_per_gas": block.header.base_fee_per_gas,
            "withdrawals_root": block.header.withdrawals_root.map(|value| format!("{value:#x}")),
            "blob_gas_used": block.header.blob_gas_used,
            "excess_blob_gas": block.header.excess_blob_gas,
            "parent_beacon_block_root": block.header.parent_beacon_block_root.map(|value| format!("{value:#x}")),
            "requests_hash": block.header.requests_hash.map(|value| format!("{value:#x}")),
            "block_access_list_hash": block.header.block_access_list_hash.map(|value| format!("{value:#x}")),
            "slot_number": block.header.slot_number,
        },
        "transactions": txs,
    }))
}

fn normalize_trace(trace: &TransactionTrace) -> Result<Value> {
    Ok(json!({
        "call_frame": normalize_call_frame(&trace.call_frame),
        "gas_used": trace.gas_used,
        "output": format!("0x{}", hex::encode(&trace.output)),
        "error": trace.error,
    }))
}

fn normalize_call_frame(frame: &CallFrame) -> Value {
    json!({
        "from": format!("{:#x}", frame.from),
        "to": frame.to.map(|address| format!("{address:#x}")),
        "value": frame.value.to_string(),
        "input": format!("0x{}", hex::encode(&frame.input)),
        "output": format!("0x{}", hex::encode(&frame.output)),
        "gas_used": frame.gas_used,
        "gas_limit": frame.gas_limit,
        "depth": frame.depth,
        "call_type": frame.call_type.to_string(),
        "subcalls": frame.subcalls.iter().map(normalize_call_frame).collect::<Vec<_>>(),
    })
}

fn canonicalize_processed_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if matches!(
                    key.as_str(),
                    "unique_addresses"
                        | "erc20_contracts"
                        | "erc721_contracts"
                        | "erc1155_contracts"
                ) {
                    sort_json_array(child);
                } else {
                    canonicalize_processed_value(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                canonicalize_processed_value(item);
            }
        }
        _ => {}
    }
}

fn sort_json_array(value: &mut Value) {
    if let Value::Array(items) = value {
        items.sort_by(|left, right| left.to_string().cmp(&right.to_string()));
    }
}

pub(crate) fn first_json_diff(path: &str, left: &Value, right: &Value) -> Option<String> {
    if left == right {
        return None;
    }

    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            let mut keys = left_map.keys().chain(right_map.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
                let child_path = format!("{path}.{key}");
                match (left_map.get(key), right_map.get(key)) {
                    (Some(left_value), Some(right_value)) => {
                        if let Some(diff) = first_json_diff(&child_path, left_value, right_value) {
                            return Some(diff);
                        }
                    }
                    (Some(_), None) => return Some(format!("{child_path}: missing on right")),
                    (None, Some(_)) => return Some(format!("{child_path}: missing on left")),
                    (None, None) => {}
                }
            }
            Some(format!("{path}: object values differ"))
        }
        (Value::Array(left_items), Value::Array(right_items)) => {
            if left_items.len() != right_items.len() {
                return Some(format!(
                    "{path}: array length {} != {}",
                    left_items.len(),
                    right_items.len()
                ));
            }
            for (index, (left_value, right_value)) in
                left_items.iter().zip(right_items.iter()).enumerate()
            {
                let child_path = format!("{path}[{index}]");
                if let Some(diff) = first_json_diff(&child_path, left_value, right_value) {
                    return Some(diff);
                }
            }
            Some(format!("{path}: array values differ"))
        }
        _ => Some(format!("{path}: left={left} right={right}")),
    }
}
