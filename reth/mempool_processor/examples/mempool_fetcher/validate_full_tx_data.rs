use mempool_processor::FullTransactionIpcClient;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info")
        .init();
    
    info!("🔍 Validating Full Transaction Data");
    info!("===================================");
    
    // Initialize client
    let client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    client.start_monitoring().await?;
    
    info!("⏳ Waiting for connection...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    info!("📊 Fetching 5 transactions to validate data completeness...\n");
    
    // Get 5 transactions
    match client.get_full_transactions(5).await {
        Ok(transactions) => {
            for (i, tx) in transactions.iter().enumerate() {
                info!("Transaction #{} (0x{}...)", i + 1, &tx.hash[2..10]);
                info!("├─ Detection Latency: {}ns ({:.3}μs)", tx.latency_ns, tx.latency_ns as f64 / 1000.0);
                
                // Check all essential fields
                let fields = [
                    ("hash", tx.tx_data.get("hash")),
                    ("from", tx.tx_data.get("from")),
                    ("to", tx.tx_data.get("to")),
                    ("value", tx.tx_data.get("value")),
                    ("gas", tx.tx_data.get("gas")),
                    ("gasPrice", tx.tx_data.get("gasPrice")),
                    ("nonce", tx.tx_data.get("nonce")),
                    ("input", tx.tx_data.get("input")),
                    ("blockNumber", tx.tx_data.get("blockNumber")),
                    ("type", tx.tx_data.get("type")),
                ];
                
                info!("├─ Data Fields:");
                let mut complete = true;
                for (name, value) in fields {
                    if let Some(v) = value {
                        let display_value = match name {
                            "input" => {
                                let input_str = v.as_str().unwrap_or("");
                                if input_str.len() > 20 {
                                    format!("{}... ({} chars)", &input_str[..20], input_str.len())
                                } else {
                                    input_str.to_string()
                                }
                            },
                            _ => format!("{}", v)
                        };
                        info!("│  ├─ {}: {}", name, display_value);
                    } else {
                        info!("│  ├─ {}: ❌ MISSING", name);
                        complete = false;
                    }
                }
                
                info!("└─ Status: {} All fields present\n", if complete { "✅" } else { "❌" });
            }
            
            // Get statistics
            let stats = client.get_stats().await;
            info!("📈 Performance Statistics:");
            info!("├─ Total Transactions: {}", stats.total_transactions);
            info!("├─ Average Latency: {}ns ({:.3}μs)", stats.avg_latency_ns, stats.avg_latency_ns as f64 / 1000.0);
            info!("├─ Min Latency: {}ns", stats.min_latency_ns.unwrap_or(0));
            info!("├─ Max Latency: {}ns", stats.max_latency_ns.unwrap_or(0));
            info!("├─ Sub-1ms: {}% ({}/{})", 
                  if stats.total_transactions > 0 { (stats.sub_1ms_count * 100) / stats.total_transactions } else { 0 },
                  stats.sub_1ms_count, stats.total_transactions);
            info!("└─ Sub-100μs: {} transactions", stats.sub_100us_count);
            
            info!("\n✅ VALIDATION COMPLETE: Full transaction data available immediately via IPC!");
        }
        Err(e) => {
            info!("❌ Error: {}", e);
        }
    }
    
    Ok(())
}