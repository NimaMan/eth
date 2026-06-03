use tx_processor::tx_processor::data_models::InternalTransaction;

use super::model::{MovementSource, RawSourceKind, TxAsset, TxMovement};

pub fn is_balance_moving_internal_eth(row: &InternalTransaction) -> bool {
    if row.value.is_zero() || row.error.is_some() || row.to_address.is_none() {
        return false;
    }

    if is_excluded_call_kind(Some(row.trace_type.as_str()))
        || is_excluded_call_kind(row.call_type.as_deref())
    {
        return false;
    }

    true
}

pub fn movement_from_internal_trace(
    trace_index: usize,
    row: &InternalTransaction,
) -> Option<TxMovement> {
    if !is_balance_moving_internal_eth(row) {
        return None;
    }

    let to = row.to_address?;
    Some(TxMovement::new(
        row.from_address,
        to,
        TxAsset::native_eth(),
        row.value,
        MovementSource::new(RawSourceKind::InternalTrace)
            .with_trace_index(trace_index)
            .with_call_type(row.call_type.clone()),
    ))
}

fn is_excluded_call_kind(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let normalized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    matches!(
        normalized.as_str(),
        "DELEGATECALL" | "STATICCALL" | "CALLCODE"
    )
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, U256};

    use super::*;

    #[test]
    fn internal_eth_predicate_excludes_non_balance_moving_rows() {
        let router = address!("dddddddddddddddddddddddddddddddddddddddd");
        let implementation = address!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");

        let mut row = InternalTransaction {
            from_address: router,
            to_address: Some(implementation),
            value: U256::from(1),
            gas: 0,
            gas_used: 0,
            trace_type: "call".to_string(),
            call_type: Some("DELEGATECALL".to_string()),
            depth: 1,
            error: None,
        };
        assert!(!is_balance_moving_internal_eth(&row));

        row.call_type = Some("CALL".to_string());
        assert!(is_balance_moving_internal_eth(&row));

        row.trace_type = "\"DELEGATECALL\"".to_string();
        row.call_type = None;
        assert!(!is_balance_moving_internal_eth(&row));

        row.trace_type = "call".to_string();
        row.call_type = Some("\"STATICCALL\"".to_string());
        assert!(!is_balance_moving_internal_eth(&row));

        row.call_type = Some("CALL".to_string());
        row.value = U256::ZERO;
        assert!(!is_balance_moving_internal_eth(&row));
    }
}
