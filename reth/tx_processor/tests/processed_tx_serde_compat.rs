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
