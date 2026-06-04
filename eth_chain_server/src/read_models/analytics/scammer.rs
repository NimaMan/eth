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
    let total_cex_terminals = cases
        .iter()
        .map(|case| value_i64(&case["summary"], "cashout_cex_terminal_count"))
        .sum::<i64>();

    // Scammer-centric, per-exchange, and recoverability rollups (DESIGN §5.2, §8).
    let scammers = build_scammers(&cases);
    let exchanges = build_exchanges(&cases);
    let recoverability = build_recoverability_rollup(&cases);

    Ok(json!({
        "root": root,
        "summary": {
            "case_count": cases.len(),
            "transaction_count": total_transactions,
            "forwarder_trace_count": total_forwarder_traces,
            "total_forwarded_eth": total_forwarded_eth,
            "buyer_count": total_buyers,
            "confiscated_buyer_count": total_confiscated_buyers,
            "cex_terminal_count": total_cex_terminals,
            "scammer_count": scammers.len(),
            "exchange_count": exchanges.len(),
        },
        "scammers": scammers,
        "exchanges": exchanges,
        "recoverability": recoverability,
        "cases": cases,
    }))
}

/// Recoverability ETH-eq split (DESIGN §4.1) read from a case's cashout-trace
/// `summary.recoverability` object. Defaults to all-zero when absent.
fn case_recoverability(cashout_summary: &Value) -> (f64, f64, f64, f64) {
    let recover = cashout_summary
        .get("recoverability")
        .cloned()
        .unwrap_or_else(|| json!({}));
    (
        value_f64(&recover, "at_exchange_eth"),
        value_f64(&recover, "in_wallet_eth"),
        value_f64(&recover, "bridged_eth"),
        value_f64(&recover, "destroyed_eth"),
    )
}

/// The cashout-trace summary object for a case (already nested in the case JSON).
fn cashout_summary(case: &Value) -> Value {
    case.get("suspect_cashout_trace")
        .and_then(|trace| trace.get("summary"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

/// Scammer-centric rollup (DESIGN §8): group cases by operator wallet — the
/// case `suspect_address`, plus the trace `control_address` recorded as an
/// associated control wallet. Rolls up case counts, victim/confiscated counts,
/// the cashout-trace out-traced / to-CEX totals, and the recoverability split.
fn build_scammers(cases: &[Value]) -> Vec<Value> {
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Acc {
        suspect_address: String,
        control_addresses: BTreeSet<String>,
        case_ids: Vec<String>,
        case_count: i64,
        buyer_count: i64,
        confiscated_buyer_count: i64,
        out_traced_eth: f64,
        to_cex_eth: f64,
        at_exchange_eth: f64,
        in_wallet_eth: f64,
        bridged_eth: f64,
        destroyed_eth: f64,
    }

    let mut by_operator: BTreeMap<String, Acc> = BTreeMap::new();
    for case in cases {
        let suspect = value_string(case, "suspect_address");
        if suspect.is_empty() {
            continue;
        }
        let summary = cashout_summary(case);
        let control = value_string(&summary, "control_address");
        let (at_exchange, in_wallet, bridged, destroyed) = case_recoverability(&summary);

        let acc = by_operator.entry(suspect.clone()).or_default();
        acc.suspect_address = suspect;
        if !control.is_empty() {
            acc.control_addresses.insert(control);
        }
        acc.case_ids.push(value_string(case, "case_id"));
        acc.case_count += 1;
        acc.buyer_count += value_i64(&case["summary"], "buyer_count");
        acc.confiscated_buyer_count += value_i64(&case["summary"], "confiscated_buyer_count");
        acc.out_traced_eth += value_f64(&summary, "total_value_out_traced_eth");
        acc.to_cex_eth += value_f64(&summary, "total_value_to_cex_eth");
        acc.at_exchange_eth += at_exchange;
        acc.in_wallet_eth += in_wallet;
        acc.bridged_eth += bridged;
        acc.destroyed_eth += destroyed;
    }

    by_operator
        .into_values()
        .map(|acc| {
            json!({
                "suspect_address": acc.suspect_address,
                "control_addresses": acc.control_addresses.into_iter().collect::<Vec<_>>(),
                "case_ids": acc.case_ids,
                "case_count": acc.case_count,
                "buyer_count": acc.buyer_count,
                "confiscated_buyer_count": acc.confiscated_buyer_count,
                "total_value_out_traced_eth": acc.out_traced_eth,
                "total_value_to_cex_eth": acc.to_cex_eth,
                "recoverability": {
                    "at_exchange_eth": acc.at_exchange_eth,
                    "in_wallet_eth": acc.in_wallet_eth,
                    "bridged_eth": acc.bridged_eth,
                    "destroyed_eth": acc.destroyed_eth,
                },
            })
        })
        .collect()
}

/// Per-exchange aggregate exposure (DESIGN §5.2): iterate every case's cashout
/// `terminals[]`, group by `exchange`, sum `total_value_received_eth`, and count
/// distinct scammers / cases reaching that exchange. This is the compliance
/// contact list. May be empty when no case reaches a CEX (that is valid).
fn build_exchanges(cases: &[Value]) -> Vec<Value> {
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Acc {
        exchange: String,
        total_value_received_eth: f64,
        terminal_count: i64,
        scammers: BTreeSet<String>,
        cases: BTreeSet<String>,
    }

    let mut by_exchange: BTreeMap<String, Acc> = BTreeMap::new();
    for case in cases {
        let suspect = value_string(case, "suspect_address");
        let case_id = value_string(case, "case_id");
        let terminals = case
            .get("suspect_cashout_trace")
            .and_then(|trace| trace.get("terminals"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for terminal in &terminals {
            let exchange = terminal
                .get("exchange")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .unwrap_or("unknown")
                .to_string();
            let acc = by_exchange.entry(exchange.clone()).or_default();
            acc.exchange = exchange;
            acc.total_value_received_eth += value_f64(terminal, "total_value_received_eth");
            acc.terminal_count += 1;
            if !suspect.is_empty() {
                acc.scammers.insert(suspect.clone());
            }
            if !case_id.is_empty() {
                acc.cases.insert(case_id.clone());
            }
        }
    }

    let mut rows: Vec<Value> = by_exchange
        .into_values()
        .map(|acc| {
            json!({
                "exchange": acc.exchange,
                "total_value_received_eth": acc.total_value_received_eth,
                "terminal_count": acc.terminal_count,
                "scammer_count": acc.scammers.len(),
                "case_count": acc.cases.len(),
            })
        })
        .collect();
    // Compliance-target list: largest exposure first.
    rows.sort_by(|left, right| {
        value_f64(right, "total_value_received_eth")
            .partial_cmp(&value_f64(left, "total_value_received_eth"))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

/// Top-level recoverability rollup (DESIGN §4.1, §8): sum the per-case cashout
/// `summary.recoverability` splits into one landscape-level actionability split.
fn build_recoverability_rollup(cases: &[Value]) -> Value {
    let mut at_exchange = 0.0;
    let mut in_wallet = 0.0;
    let mut bridged = 0.0;
    let mut destroyed = 0.0;
    for case in cases {
        let summary = cashout_summary(case);
        let (a, w, b, d) = case_recoverability(&summary);
        at_exchange += a;
        in_wallet += w;
        bridged += b;
        destroyed += d;
    }
    json!({
        "at_exchange_eth": at_exchange,
        "in_wallet_eth": in_wallet,
        "bridged_eth": bridged,
        "destroyed_eth": destroyed,
    })
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
    let suspect_cashout_trace = read_json_object_or_default(
        case_dir
            .join("artifacts")
            .join("tx_fund_flow")
            .join("suspect_cashout_trace.json"),
    );
    let cashout_summary = suspect_cashout_trace
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
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
            "cashout_node_count": value_i64(&cashout_summary, "node_count"),
            "cashout_edge_count": value_i64(&cashout_summary, "edge_count"),
            "cashout_cex_terminal_count": value_i64(&cashout_summary, "cex_terminal_count"),
            "cashout_unlabeled_lead_count": value_i64(&cashout_summary, "unlabeled_lead_count"),
            "cashout_total_eth_to_cex": value_f64(&cashout_summary, "total_value_to_cex_eth"),
            "cashout_funding_source_count": value_i64(&cashout_summary, "funding_source_count"),
        },
        "key_txs": case_config.get("key_txs").cloned().unwrap_or(Value::Null),
        "links": case_config.get("links").cloned().unwrap_or(Value::Null),
        "transactions": transaction_rows,
        "forwarder_traces": trace_rows,
        "buyer_outcomes": buyer_outcomes,
        "buyer_fund_flow_edges": buyer_edge_rows,
        "address_clusters": cluster_rows,
        "suspect_cashout_trace": suspect_cashout_trace,
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

/// Like [`read_json_or_default`] but defaults to an empty object, for artifacts
/// whose top level is a JSON object (e.g. `suspect_cashout_trace.json`).
fn read_json_object_or_default(path: PathBuf) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_else(|| json!({}))
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
