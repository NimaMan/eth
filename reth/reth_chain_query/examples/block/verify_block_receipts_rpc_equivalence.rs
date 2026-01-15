use std::{collections::HashMap, env, time::Instant};

use alloy_primitives::{hex, Address, B256};
use eyre::Result;
use jsonrpsee::{
    core::client::{ClientT, Error as RpcError},
    http_client::HttpClientBuilder,
    rpc_params,
};
use reth_chain_query::RethQueryProvider;
use serde_json::Value;

fn hash_hex(hash: &B256) -> String {
    format!("0x{}", hex::encode(hash.as_slice()))
}

fn addr_hex(address: &Address) -> String {
    format!("0x{}", hex::encode(address.as_slice()))
}

fn topic_hex(topic: &B256) -> String {
    format!("0x{}", hex::encode(topic.as_slice()))
}

fn parse_hex_u64(value: &Value) -> Option<u64> {
    value
        .as_str()
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
}

#[tokio::main]
async fn main() -> Result<()> {
    let reth_datadir = env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());

    let provider = RethQueryProvider::new(&reth_datadir)?;
    let latest_block = provider.get_latest_block()?;

    let block_number: u64 = env::args()
        .nth(1)
        .map(|s| s.parse().expect("invalid block number"))
        .unwrap_or_else(|| latest_block.saturating_sub(10));

    println!("🔍 Verifying block {}", block_number);

    // --- Receipts from DB -------------------------------------------------
    let start_db = Instant::now();
    let db_receipts = provider.fetch_block_receipts_only(block_number).await?;
    let db_time = start_db.elapsed();
    println!(
        "📦 Loaded {} receipts from DB in {:.3}s",
        db_receipts.len(),
        db_time.as_secs_f64()
    );

    // Block metadata from DB
    let db_header = provider.fetch_block_header_only(block_number).await?;

    // --- Receipts from RPC ------------------------------------------------
    let client = HttpClientBuilder::default().build(&rpc_url)?;
    let block_hex = format!("0x{:x}", block_number);

    // Block metadata via RPC
    let rpc_block: Value = client
        .request(
            "eth_getBlockByNumber",
            rpc_params![block_hex.clone(), false],
        )
        .await?;

    // Try batch receipt API first
    let rpc_receipts_value: Result<Value, RpcError> = client
        .request("eth_getBlockReceipts", rpc_params![block_hex.clone()])
        .await;

    let mut rpc_receipt_map: HashMap<String, Value> = HashMap::new();

    match rpc_receipts_value {
        Ok(value) if value.is_array() => {
            for receipt in value.as_array().unwrap() {
                if let Some(tx_hash) = receipt.get("transactionHash").and_then(|v| v.as_str()) {
                    rpc_receipt_map.insert(tx_hash.to_string(), receipt.clone());
                }
            }
        }
        _ => {
            println!(
                "⚠️ eth_getBlockReceipts unavailable; falling back to per-transaction receipts"
            );
            let metadata = provider.fetch_block_tx_metadata_only(block_number).await?;
            for tx in metadata {
                let tx_hash_hex = hash_hex(&tx.hash);
                let receipt: Value = client
                    .request(
                        "eth_getTransactionReceipt",
                        rpc_params![tx_hash_hex.clone()],
                    )
                    .await?;
                rpc_receipt_map.insert(tx_hash_hex, receipt);
            }
        }
    }

    println!("🌐 Retrieved {} receipts from RPC", rpc_receipt_map.len());

    // --- Compare receipts -------------------------------------------------
    let mut receipt_mismatches = Vec::new();

    for receipt in &db_receipts {
        let tx_hash = hash_hex(&receipt.tx_hash);
        match rpc_receipt_map.get(&tx_hash) {
            Some(rpc_receipt) => {
                let status = rpc_receipt
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim_start_matches("0x") == "1");
                if status != Some(receipt.status) {
                    receipt_mismatches.push(format!(
                        "status mismatch for {} (db: {}, rpc: {:?})",
                        tx_hash, receipt.status, status
                    ));
                }

                let rpc_gas_used =
                    parse_hex_u64(rpc_receipt.get("gasUsed").unwrap_or(&Value::Null));
                if rpc_gas_used != Some(receipt.gas_used) {
                    receipt_mismatches.push(format!(
                        "gasUsed mismatch for {} (db: {}, rpc: {:?})",
                        tx_hash, receipt.gas_used, rpc_gas_used
                    ));
                }

                let rpc_cum_gas =
                    parse_hex_u64(rpc_receipt.get("cumulativeGasUsed").unwrap_or(&Value::Null));
                if rpc_cum_gas != Some(receipt.cumulative_gas_used) {
                    receipt_mismatches.push(format!(
                        "cumulativeGasUsed mismatch for {} (db: {}, rpc: {:?})",
                        tx_hash, receipt.cumulative_gas_used, rpc_cum_gas
                    ));
                }

                if let Some(rpc_logs) = rpc_receipt.get("logs").and_then(|v| v.as_array()) {
                    if rpc_logs.len() != receipt.logs.len() {
                        receipt_mismatches.push(format!(
                            "log count mismatch for {} (db: {}, rpc: {})",
                            tx_hash,
                            receipt.logs.len(),
                            rpc_logs.len()
                        ));
                    } else {
                        for (idx, (db_log, rpc_log)) in
                            receipt.logs.iter().zip(rpc_logs.iter()).enumerate()
                        {
                            let rpc_address = rpc_log
                                .get("address")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_lowercase();
                            if rpc_address != addr_hex(&db_log.address).to_lowercase() {
                                receipt_mismatches
                                    .push(format!("log {} address mismatch for {}", idx, tx_hash));
                            }

                            let rpc_topics = rpc_log
                                .get("topics")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let db_topics: Vec<String> =
                                db_log.topics.iter().map(topic_hex).collect();
                            let rpc_topic_strings: Vec<String> = rpc_topics
                                .iter()
                                .filter_map(|topic| topic.as_str().map(|s| s.to_string()))
                                .collect();
                            if db_topics != rpc_topic_strings {
                                receipt_mismatches
                                    .push(format!("log {} topics mismatch for {}", idx, tx_hash));
                            }

                            let rpc_data = rpc_log
                                .get("data")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0x")
                                .to_string();
                            let db_data = format!("0x{}", hex::encode(db_log.data.as_ref()));
                            if rpc_data.to_lowercase() != db_data.to_lowercase() {
                                receipt_mismatches
                                    .push(format!("log {} data mismatch for {}", idx, tx_hash));
                            }
                        }
                    }
                }
            }
            None => {
                receipt_mismatches.push(format!("missing RPC receipt for {}", tx_hash));
            }
        }
    }

    if receipt_mismatches.is_empty() {
        println!("✅ All receipts match between DB and RPC");
    } else {
        println!("❌ Found {} receipt mismatches", receipt_mismatches.len());
        for mismatch in receipt_mismatches.iter().take(20) {
            println!("  - {}", mismatch);
        }
        if receipt_mismatches.len() > 20 {
            println!("  … and {} more", receipt_mismatches.len() - 20);
        }
        std::process::exit(1);
    }

    // --- Compare block metadata ------------------------------------------
    let mut metadata_mismatches = Vec::new();

    let rpc_timestamp = parse_hex_u64(rpc_block.get("timestamp").unwrap_or(&Value::Null));
    if rpc_timestamp != Some(db_header.timestamp) {
        metadata_mismatches.push(format!(
            "timestamp mismatch (db: {}, rpc: {:?})",
            db_header.timestamp, rpc_timestamp
        ));
    }

    let rpc_gas_used = parse_hex_u64(rpc_block.get("gasUsed").unwrap_or(&Value::Null));
    if rpc_gas_used != Some(db_header.gas_used) {
        metadata_mismatches.push(format!(
            "gasUsed mismatch (db: {}, rpc: {:?})",
            db_header.gas_used, rpc_gas_used
        ));
    }

    let rpc_gas_limit = parse_hex_u64(rpc_block.get("gasLimit").unwrap_or(&Value::Null));
    if rpc_gas_limit != Some(db_header.gas_limit) {
        metadata_mismatches.push(format!(
            "gasLimit mismatch (db: {}, rpc: {:?})",
            db_header.gas_limit, rpc_gas_limit
        ));
    }

    let rpc_base_fee = parse_hex_u64(rpc_block.get("baseFeePerGas").unwrap_or(&Value::Null));
    if rpc_base_fee != db_header.base_fee_per_gas {
        metadata_mismatches.push(format!(
            "baseFeePerGas mismatch (db: {:?}, rpc: {:?})",
            db_header.base_fee_per_gas, rpc_base_fee
        ));
    }

    if metadata_mismatches.is_empty() {
        println!("✅ Block header metadata matches RPC");
    } else {
        println!("❌ Block metadata mismatches detected");
        for mismatch in metadata_mismatches {
            println!("  - {}", mismatch);
        }
        std::process::exit(1);
    }

    println!("🎉 Verification successful for block {}", block_number);

    Ok(())
}
