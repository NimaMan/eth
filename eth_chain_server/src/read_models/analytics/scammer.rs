use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

const DEFAULT_ROOT: &str = "/home/nima/code/crypto/blockchains/eth/risk_atlas/scammer_analytics";

pub fn payload() -> Value {
    match payload_result() {
        Ok(value) => value,
        Err(error) => json!({
            "error": error.to_string(),
            "cases": [],
            "summary": {},
        }),
    }
}

fn payload_result() -> Result<Value, Box<dyn Error>> {
    let root = scammer_analytics_root();
    let cases_dir = root.join("cases");
    let mut cases = Vec::new();

    for entry in fs::read_dir(&cases_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            cases.push(case_payload(&entry.path())?);
        }
    }
    cases.sort_by(|left, right| {
        value_string(left, "case_id")
            .cmp(&value_string(right, "case_id"))
            .then_with(|| value_string(left, "title").cmp(&value_string(right, "title")))
    });

    let total_transactions = cases
        .iter()
        .map(|case| value_i64(&case["summary"], "transaction_count"))
        .sum::<i64>();
    let total_forwarder_traces = cases
        .iter()
        .map(|case| value_i64(&case["summary"], "forwarder_trace_count"))
        .sum::<i64>();
    let total_forwarded_eth = cases
        .iter()
        .map(|case| value_f64(&case["summary"], "total_forwarded_eth"))
        .sum::<f64>();
    let total_buyers = cases
        .iter()
        .map(|case| value_i64(&case["summary"], "buyer_count"))
        .sum::<i64>();
    let total_confiscated_buyers = cases
        .iter()
        .map(|case| value_i64(&case["summary"], "confiscated_buyer_count"))
        .sum::<i64>();

    Ok(json!({
        "root": root,
        "summary": {
            "case_count": cases.len(),
            "transaction_count": total_transactions,
            "forwarder_trace_count": total_forwarder_traces,
            "total_forwarded_eth": total_forwarded_eth,
            "buyer_count": total_buyers,
            "confiscated_buyer_count": total_confiscated_buyers,
        },
        "cases": cases,
    }))
}

fn scammer_analytics_root() -> PathBuf {
    std::env::var("RISK_ATLAS_SCAMMER_ANALYTICS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_ROOT))
}

fn case_payload(case_dir: &Path) -> Result<Value, Box<dyn Error>> {
    let case_config = read_toml(case_dir.join("case.toml"))?;
    let transactions = read_json_or_default(
        case_dir
            .join("artifacts")
            .join("tx_fund_flow")
            .join("transactions.json"),
    );
    let buyer_outcomes =
        read_json_or_default(case_dir.join("artifacts").join("buyer_token_outcomes.json"));
    let buyer_fund_flow_edges = read_json_or_default(
        case_dir
            .join("artifacts")
            .join("tx_fund_flow")
            .join("buyer_eth_edges.json"),
    );
    let address_clusters = read_json_or_default(
        case_dir
            .join("artifacts")
            .join("tx_fund_flow")
            .join("address_clusters.json"),
    );
    let traces = read_json_or_default(
        case_dir
            .join("artifacts")
            .join("traces")
            .join("forwarder_traces.json"),
    );
    let readme_path = case_dir.join("README.md");
    let readme = fs::read_to_string(&readme_path).unwrap_or_default();
    let report_path = case_dir.join("reports").join("evidence_packet.md");
    let report = fs::read_to_string(&report_path).unwrap_or_default();
    let buyer_report_path = case_dir.join("reports").join("buyer_outcome_report.md");
    let buyer_report = fs::read_to_string(&buyer_report_path).unwrap_or_default();
    let transaction_rows = transactions.as_array().cloned().unwrap_or_default();
    let trace_rows = traces.as_array().cloned().unwrap_or_default();
    let buyer_rows = buyer_outcomes
        .get("buyers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let buyer_summary = buyer_outcomes
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let buyer_edge_rows = buyer_fund_flow_edges
        .as_array()
        .cloned()
        .unwrap_or_default();
    let cluster_rows = address_clusters.as_array().cloned().unwrap_or_default();

    let mut sinks = BTreeSet::new();
    let mut token_contracts = BTreeSet::new();
    let mut blocks = Vec::new();
    for tx in &transaction_rows {
        if let Some(block) = tx.get("block_number").and_then(Value::as_u64) {
            blocks.push(block);
        }
        for movement in tx
            .get("token_movements")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(token) = movement.get("token_address").and_then(Value::as_str) {
                token_contracts.insert(token.to_string());
            }
        }
    }
    for trace in &trace_rows {
        if let Some(sink) = trace.get("sink_address").and_then(Value::as_str) {
            sinks.insert(sink.to_string());
        }
    }
    blocks.sort_unstable();

    let total_forwarded_eth = trace_rows
        .iter()
        .map(|trace| {
            trace
                .get("amount_eth")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        })
        .sum::<f64>();

    Ok(json!({
        "case_id": toml_string(&case_config, "case_id"),
        "title": toml_string(&case_config, "title"),
        "status": toml_string(&case_config, "status"),
        "chain": toml_string(&case_config, "chain"),
        "suspect_address": toml_string(&case_config, "suspect_address"),
        "token_address": toml_optional_string(&case_config, "token_address"),
        "pool_address": toml_optional_string(&case_config, "pool_address"),
        "forwarder_address": toml_optional_string(&case_config, "forwarder_address"),
        "summary": {
            "transaction_count": transaction_rows.len(),
            "forwarder_trace_count": trace_rows.len(),
            "buyer_count": buyer_rows.len(),
            "buyer_fund_flow_edge_count": buyer_edge_rows.len(),
            "address_cluster_count": cluster_rows.len(),
            "confiscated_buyer_count": value_i64(&buyer_summary, "confiscated_count"),
            "still_holding_buyer_count": value_i64(&buyer_summary, "still_holding_count"),
            "unique_forwarder_sinks": sinks.len(),
            "unique_token_contracts_touched": token_contracts.len(),
            "total_forwarded_eth": total_forwarded_eth,
            "first_block": blocks.first().copied(),
            "last_block": blocks.last().copied(),
        },
        "key_txs": case_config.get("key_txs").cloned().unwrap_or(Value::Null),
        "links": case_config.get("links").cloned().unwrap_or(Value::Null),
        "transactions": transaction_rows,
        "forwarder_traces": trace_rows,
        "buyer_outcomes": buyer_outcomes,
        "buyer_fund_flow_edges": buyer_edge_rows,
        "address_clusters": cluster_rows,
        "readme_excerpt": readme.lines().take(180).collect::<Vec<_>>().join("\n"),
        "report_excerpt": report.lines().take(80).collect::<Vec<_>>().join("\n"),
        "buyer_report_excerpt": buyer_report.lines().take(120).collect::<Vec<_>>().join("\n"),
        "paths": {
            "case_dir": case_dir,
            "readme": readme_path,
            "report": report_path,
            "buyer_report": buyer_report_path,
        },
    }))
}

fn read_toml(path: PathBuf) -> Result<Value, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&contents)?;
    Ok(serde_json::to_value(value)?)
}

fn read_json_or_default(path: PathBuf) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_else(|| json!([]))
}

fn toml_string(value: &Value, key: &str) -> String {
    value_string(value, key)
}

fn toml_optional_string(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|text| json!(text))
        .unwrap_or(Value::Null)
}

fn value_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn value_i64(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or_default()
}

fn value_f64(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or_default()
}
