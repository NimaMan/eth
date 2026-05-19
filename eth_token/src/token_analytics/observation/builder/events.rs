use serde_json::Value;

use crate::erc20::ERC20Token;

use super::utils::{is_lp_burn_holder, normalize_address};

pub(super) fn event_tx_hash(event: &Value) -> Option<String> {
    event
        .get("tx_hash")
        .or_else(|| event.get("txHash"))
        .or_else(|| event.get("transaction_hash"))
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

pub(super) fn lp_event_counts_at_block(
    token: &ERC20Token,
    pool_address: &str,
    block_number: u64,
) -> (u32, u32, u32) {
    let pool_address = normalize_address(pool_address);
    if let Some(pool) = token.v2_pools.get(&pool_address) {
        let (transfer_count, burn_transfer_count) =
            lp_transfer_counts_at_block(&pool.lp_tracker.transfers, block_number);
        return (
            transfer_count,
            burn_transfer_count,
            event_count_at_block(&pool.lp_tracker.approval_events, block_number),
        );
    }
    if let Some(pool) = token.v3_pools.get(&pool_address) {
        let (transfer_count, burn_transfer_count) =
            lp_transfer_counts_at_block(&pool.liquidity_position_events, block_number);
        return (transfer_count, burn_transfer_count, 0);
    }
    if let Some(pool) = token.v4_pools.get(&pool_address) {
        let (transfer_count, burn_transfer_count) =
            lp_transfer_counts_at_block(&pool.liquidity_position_events, block_number);
        return (
            transfer_count,
            burn_transfer_count,
            event_count_at_block(&pool.lp_approval_events, block_number),
        );
    }
    (0, 0, 0)
}

pub(super) fn lp_transfer_counts_at_block(events: &[Value], block_number: u64) -> (u32, u32) {
    let mut regular = 0_u32;
    let mut burned = 0_u32;
    for event in events
        .iter()
        .filter(|event| event_block_number(event) == Some(block_number))
    {
        if event_transfers_to_burn_address(event) {
            burned = burned.saturating_add(1);
        } else {
            regular = regular.saturating_add(1);
        }
    }
    (regular, burned)
}

pub(super) fn event_count_at_block(events: &[Value], block_number: u64) -> u32 {
    events
        .iter()
        .filter(|event| event_block_number(event) == Some(block_number))
        .count() as u32
}

pub(super) fn event_block_number(event: &Value) -> Option<u64> {
    event
        .get("block")
        .or_else(|| event.get("block_number"))
        .and_then(Value::as_u64)
}

pub(super) fn event_timestamp(event: &Value) -> Option<u64> {
    event
        .get("timestamp")
        .or_else(|| event.get("block_timestamp"))
        .and_then(Value::as_u64)
}

pub(super) fn event_string(event: &Value, key: &str) -> Option<String> {
    event
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
}

pub(super) fn event_transfer_to_address(event: &Value) -> Option<String> {
    event_string(event, "to_address").or_else(|| event_string(event, "to"))
}

pub(super) fn event_transfer_from_address(event: &Value) -> Option<String> {
    event_string(event, "from_address").or_else(|| event_string(event, "from"))
}

pub(super) fn event_transfers_to_burn_address(event: &Value) -> bool {
    event_transfer_to_address(event).is_some_and(|address| is_lp_burn_holder(&address))
}

pub(super) fn event_amount(event: &Value) -> Option<f64> {
    ["amount", "position_liquidity", "liquidity"]
        .into_iter()
        .find_map(|key| event_number(event, key))
        .filter(|amount| amount.is_finite() && *amount > 0.0)
}

pub(super) fn event_number(event: &Value, key: &str) -> Option<f64> {
    let value = event.get(key)?;
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|value| value.parse::<f64>().ok()))
}

pub(super) fn event_bool(event: &Value, key: &str) -> Option<bool> {
    event.get(key).and_then(Value::as_bool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn separates_lp_burn_transfers_from_regular_lp_transfers() {
        let events = vec![
            json!({
                "block_number": 10,
                "tx_hash": "0xregular",
                "from_address": "0x1111111111111111111111111111111111111111",
                "to_address": "0x2222222222222222222222222222222222222222",
                "amount": 3.0
            }),
            json!({
                "block_number": 10,
                "tx_hash": "0xdead",
                "from_address": "0x1111111111111111111111111111111111111111",
                "to_address": "0x000000000000000000000000000000000000dEaD",
                "amount": 7.0
            }),
        ];

        assert_eq!(lp_transfer_counts_at_block(&events, 10), (1, 1));
    }
}
