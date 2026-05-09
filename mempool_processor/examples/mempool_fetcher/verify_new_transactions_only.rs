/// Verify that we only receive NEW transactions entering the mempool
///
/// This test will:
/// 1. Get current mempool size
/// 2. Start subscription
/// 3. Track first 100 transactions
/// 4. Check if we see any existing transactions or only new ones
use mempool_processor::mempool_fetcher::MempoolFetcherIPCClient;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("Verifying that we only receive NEW transactions...");

    // First, get some existing transactions from mempool
    let rpc_client = reqwest::Client::new();
    let rpc_url = mempool_processor::config::eth_rpc_url_from_env();
    info!("Getting current mempool transactions...");
    let rpc_response = rpc_client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "txpool_content",
            "params": [],
            "id": 1
        }))
        .send()
        .await?
        .error_for_status()?;
    let rpc_body: serde_json::Value = rpc_response.json().await?;
    let mempool_content = rpc_body.get("result").cloned().unwrap_or_default();

    // Extract some transaction hashes from existing mempool
    let mut existing_hashes = HashSet::new();
    if let Some(pending) = mempool_content
        .get("pending")
        .and_then(|value| value.as_object())
    {
        for (_, txs_by_nonce) in pending {
            let Some(txs_by_nonce) = txs_by_nonce.as_object() else {
                continue;
            };
            for (_, tx) in txs_by_nonce {
                if let Some(hash) = tx.get("hash").and_then(|h| h.as_str()) {
                    existing_hashes.insert(hash.to_string());
                }
            }
        }
    }

    info!(
        "Found {} existing transactions in mempool",
        existing_hashes.len()
    );

    // Now start our subscription
    info!("Starting subscription to newPendingTransactions...");
    let ipc_path = mempool_processor::config::reth_ipc_path_from_env();
    info!("IPC path: {}", ipc_path);
    let ipc_client = MempoolFetcherIPCClient::new(Some(&ipc_path))?;
    ipc_client.start().await?;

    // Track what we receive
    let mut received_count = 0;
    let mut new_count = 0;
    let mut existing_count = 0;
    let start = Instant::now();

    info!("Monitoring for 30 seconds...");

    while start.elapsed() < Duration::from_secs(30) {
        let transactions = ipc_client.get_transactions_instant(100).await;
        if !transactions.is_empty() {
            for tx in transactions {
                received_count += 1;

                if existing_hashes.contains(&tx.hash) {
                    existing_count += 1;
                    info!("❌ EXISTING transaction detected: {}", short_hash(&tx.hash));
                } else {
                    new_count += 1;
                    if new_count <= 5 {
                        info!(
                            "✅ NEW transaction: {} ({}μs)",
                            short_hash(&tx.hash),
                            tx.detection_ns / 1000
                        );
                    }
                }

                if received_count % 50 == 0 {
                    info!(
                        "Progress: {} total, {} new, {} existing",
                        received_count, new_count, existing_count
                    );
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    info!("\n=== VERIFICATION RESULTS ===");
    info!("Total transactions received: {}", received_count);
    let received_count_f64 = received_count.max(1) as f64;
    info!(
        "NEW transactions: {} ({:.1}%)",
        new_count,
        (new_count as f64 / received_count_f64) * 100.0
    );
    info!(
        "EXISTING transactions: {} ({:.1}%)",
        existing_count,
        (existing_count as f64 / received_count_f64) * 100.0
    );

    if existing_count == 0 {
        info!("\n✅ SUCCESS: We only receive NEW transactions!");
        info!("The 'newPendingTransactions' subscription works as expected.");
    } else {
        info!("\n❌ UNEXPECTED: We received some existing transactions!");
        info!("This would indicate a problem with our understanding.");
    }

    Ok(())
}

fn short_hash(hash: &str) -> &str {
    hash.get(..10).unwrap_or(hash)
}
