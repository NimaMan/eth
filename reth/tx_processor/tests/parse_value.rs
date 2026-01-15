use alloy_primitives::U256;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Fees {
    max_fee_per_gas: Option<U256>,
    gas_price: U256,
}

#[derive(Deserialize, Debug)]
struct Tx {
    value: U256,
    fees: Fees,
}

#[test]
fn parse_floats() {
    let data = r#"{"value":1.8,"fees":{"gas_price":368581891,"max_fee_per_gas":368581891}}"#;
    let tx: Tx = serde_json::from_str(data).unwrap();
    println!("value {}", tx.value);
    println!("max_fee {:?}", tx.fees.max_fee_per_gas);
}
