use std::{
    io::{self, Read},
    process,
};

use eth_pool_classification::{
    classify_pool_with_config, PoolClassification, PoolClassificationConfig,
    PoolClassificationInput,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct BatchResponse {
    decisions: Vec<PoolClassification>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut raw = String::new();
    io::stdin()
        .read_to_string(&mut raw)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    let value: Value =
        serde_json::from_str(&raw).map_err(|error| format!("invalid JSON input: {error}"))?;
    let config = PoolClassificationConfig::default();

    if let Some(pools) = value.get("pools").and_then(Value::as_array) {
        return write_batch(pools, &config);
    }
    if let Some(items) = value.as_array() {
        return write_batch(items, &config);
    }

    let input = PoolClassificationInput::from_json_value(&value)
        .map_err(|error| format!("invalid pool classification object: {error}"))?;
    let decision = classify_pool_with_config(&input, &config);
    serde_json::to_writer_pretty(io::stdout(), &decision)
        .map_err(|error| format!("failed to write JSON output: {error}"))?;
    println!();
    Ok(())
}

fn write_batch(items: &[Value], config: &PoolClassificationConfig) -> Result<(), String> {
    let mut decisions = Vec::with_capacity(items.len());
    for item in items {
        let input = PoolClassificationInput::from_json_value(item)
            .map_err(|error| format!("invalid pool classification object: {error}"))?;
        decisions.push(classify_pool_with_config(&input, config));
    }
    serde_json::to_writer_pretty(io::stdout(), &BatchResponse { decisions })
        .map_err(|error| format!("failed to write JSON output: {error}"))?;
    println!();
    Ok(())
}
