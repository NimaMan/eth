use alloy_primitives::{Address, B256, U256};
use tx_processor::tx_processor::data_models::ProcessedTransaction;

#[test]
fn processed_transaction_deserializes_without_approval_for_all_events() {
    let tx = ProcessedTransaction::new(
        B256::ZERO,
        1,
        0,
        0,
        Address::ZERO,
        None,
        U256::ZERO,
        true,
        0,
        0,
        Vec::new(),
    );
    let mut value = serde_json::to_value(tx).expect("serialize processed tx");
    value
        .as_object_mut()
        .expect("processed tx json object")
        .remove("approval_for_all_events");

    let decoded: ProcessedTransaction =
        serde_json::from_value(value).expect("deserialize older processed tx payload");

    assert!(decoded.approval_for_all_events.is_empty());
}

#[test]
fn processed_transaction_deserializes_balance_changes_without_movements() {
    let mut tx = ProcessedTransaction::new(
        B256::ZERO,
        1,
        0,
        0,
        Address::ZERO,
        None,
        U256::ZERO,
        true,
        0,
        0,
        Vec::new(),
    );
    tx.address_balance_changes.insert(
        Address::ZERO,
        tx_processor::tx_processor::data_models::AddressBalanceChange::default(),
    );
    let mut value = serde_json::to_value(tx).expect("serialize processed tx");
    value["address_balance_changes"]["0x0000000000000000000000000000000000000000"]
        .as_object_mut()
        .expect("balance change object")
        .remove("movements");

    let decoded: ProcessedTransaction =
        serde_json::from_value(value).expect("deserialize older balance change payload");

    let balance_change = decoded
        .address_balance_changes
        .get(&Address::ZERO)
        .expect("zero address balance change");
    assert!(balance_change.movements.tokens.is_empty());
    assert!(balance_change.movements.currencies.is_empty());
}
