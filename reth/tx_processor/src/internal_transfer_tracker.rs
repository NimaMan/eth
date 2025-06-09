use std::collections::HashMap;
use revm_primitives::{Address as RevmAddress, U256 as RevmU256};
use crate::InternalTransfer;
use crate::state_diff_utils::{CalculatedAccountChanges, EthMovement};
use serde_json::Value;

/// Add internal transfers to calculated account changes
pub fn integrate_internal_transfers(
    mut all_changes: HashMap<RevmAddress, CalculatedAccountChanges>,
    internal_transfers: &[InternalTransfer],
) -> HashMap<RevmAddress, CalculatedAccountChanges> {
    
    // WETH contract address
    const WETH_ADDRESS: RevmAddress = RevmAddress::new([
        0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e,
        0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2
    ]);
    
    // Process each internal transfer
    for (i, transfer) in internal_transfers.iter().enumerate() {
        if transfer.success && transfer.value > RevmU256::ZERO {
            
            // Skip internal transfers from/to WETH to match Python behavior
            if transfer.from == WETH_ADDRESS || transfer.to == WETH_ADDRESS {
                continue;
            }
            
            // Ensure both addresses exist in changes map
            if !all_changes.contains_key(&transfer.from) {
                all_changes.insert(transfer.from, CalculatedAccountChanges {
                    address: transfer.from,
                    ..Default::default()
                });
            }
            
            if !all_changes.contains_key(&transfer.to) {
                all_changes.insert(transfer.to, CalculatedAccountChanges {
                    address: transfer.to,
                    ..Default::default()
                });
            }
            
            // Subtract from sender
            if let Some(sender_changes) = all_changes.get_mut(&transfer.from) {
                sender_changes.movements.eth.out_list.push(EthMovement {
                    source_identifier: format!("internal_{}", i),
                    raw_amount: transfer.value,
                });
                sender_changes.eth_net_change = sender_changes.eth_net_change.clone().subtract_positive(transfer.value);
            }
            
            // Add to receiver
            if let Some(receiver_changes) = all_changes.get_mut(&transfer.to) {
                receiver_changes.movements.eth.in_list.push(EthMovement {
                    source_identifier: format!("internal_{}", i),
                    raw_amount: transfer.value,
                });
                receiver_changes.eth_net_change = receiver_changes.eth_net_change.clone().add_positive(transfer.value);
            }
        }
    }
    
    all_changes
}

/// Extract internal transfers from RPC trace (fallback method)
pub async fn extract_internal_transfers_from_rpc(
    tx_hash: &str,
    rpc_url: &str,
) -> Result<Vec<InternalTransfer>, anyhow::Error> {
    use serde_json::{json, Value};
    
    // Call debug_traceTransaction to get internal calls
    let client = reqwest::Client::new();
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "debug_traceTransaction", 
        "params": [tx_hash, {"tracer": "callTracer"}],
        "id": 1
    });
    
    let response = client
        .post(rpc_url)
        .json(&request_body)
        .send()
        .await?;
    
    let response_json: Value = response.json().await?;
    
    if let Some(result) = response_json.get("result") {
        parse_trace_for_internal_transfers(result, 0)
    } else {
        Ok(Vec::new())
    }
}

/// Parse trace JSON to extract internal ETH transfers
fn parse_trace_for_internal_transfers(trace: &Value, depth: usize) -> Result<Vec<InternalTransfer>, anyhow::Error> {
    let mut transfers = Vec::new();
    
    // Check if this call has value
    if let (Some(from), Some(to), Some(value_str)) = (
        trace.get("from").and_then(|v| v.as_str()),
        trace.get("to").and_then(|v| v.as_str()),
        trace.get("value").and_then(|v| v.as_str())
    ) {
        if let Ok(value_u256) = RevmU256::from_str_radix(value_str.trim_start_matches("0x"), 16) {
            if value_u256 > RevmU256::ZERO {
                let from_addr = from.parse::<RevmAddress>().unwrap_or_default();
                let to_addr = to.parse::<RevmAddress>().unwrap_or_default();
                
                let call_type = trace.get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("CALL")
                    .to_string();
                
                let success = trace.get("error").is_none();
                
                transfers.push(InternalTransfer {
                    from: from_addr,
                    to: to_addr,
                    value: value_u256,
                    depth,
                    call_type,
                    success,
                });
            }
        }
    }
    
    // Process child calls recursively
    if let Some(calls) = trace.get("calls").and_then(|v| v.as_array()) {
        for call in calls {
            let mut child_transfers = parse_trace_for_internal_transfers(call, depth + 1)?;
            transfers.append(&mut child_transfers);
        }
    }
    
    Ok(transfers)
}