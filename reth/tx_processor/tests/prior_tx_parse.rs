use std::fs;
use tx_processor::tx_processor::data_models::ProcessedTransaction;

fn sanitize(mut value: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("address_balance_changes");
        obj.remove("latest_states");
        if let Some(fees) = obj.get_mut("fees").and_then(|v| v.as_object_mut()) {
            if let Some(tx_fee) = fees.get("tx_fee") {
                if tx_fee.is_f64() {
                    let as_u256 = (tx_fee.as_f64().unwrap() * 1e18).round() as u128;
                    fees.insert(
                        "tx_fee".to_string(),
                        serde_json::Value::String(as_u256.to_string()),
                    );
                }
            }
            for key in ["gas_price", "max_fee_per_gas", "max_priority_fee"] {
                if let Some(val) = fees.get(key) {
                    if val.is_number() {
                        let num = val.as_u64().unwrap();
                        fees.insert(key.to_string(), serde_json::Value::String(num.to_string()));
                    }
                }
            }
        }
        if let Some(val) = obj.get("value") {
            if val.is_f64() {
                let as_u256 = (val.as_f64().unwrap() * 1e18).round() as u128;
                obj.insert(
                    "value".to_string(),
                    serde_json::Value::String(as_u256.to_string()),
                );
            }
        }
        if let Some(internals) = obj
            .get_mut("internal_transactions")
            .and_then(|v| v.as_array_mut())
        {
            for tx in internals {
                if let Some(map) = tx.as_object_mut() {
                    if let Some(val) = map.get("value") {
                        if val.is_f64() {
                            let as_u256 = (val.as_f64().unwrap()).round() as u128;
                            map.insert(
                                "value".to_string(),
                                serde_json::Value::String(as_u256.to_string()),
                            );
                        }
                    }
                }
            }
        }
    }
    value
}

#[test]
fn parse_prior_tx_from_json() {
    let path = std::env::var("PRIOR_TX_JSON").unwrap_or_else(|_| "/tmp/tx.json".to_string());
    let data = fs::read_to_string(&path).expect("read json");
    let value: serde_json::Value = serde_json::from_str(&data).expect("json parse");
    let value = sanitize(value);
    let tx: ProcessedTransaction = serde_json::from_value(value).expect("processed tx");
    println!("gas_price {}", tx.fees.gas_price);
    println!("max_fee {:?}", tx.fees.max_fee_per_gas);
    println!("tx_fee {}", tx.fees.tx_fee);
    println!("value {}", tx.value);
}
