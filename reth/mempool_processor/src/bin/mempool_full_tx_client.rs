use mempool_processor::FullTransactionIpcClient;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use std::io::Write;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info")
        .init();
    
    info!("🚀 Starting Mempool Full Transaction IPC Client");
    info!("==============================================");
    
    // Initialize timing log file
    let start_time = chrono::Local::now();
    let timestamp = start_time.format("%Y%m%d_%H%M%S");
    let timing_log_path = format!("/home/nima/code/crypto/logs/mempool/full_tx_ipc_{}.log", timestamp);
    let mut timing_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&timing_log_path)?;
    
    info!("📋 Timing logs: {}", timing_log_path);
    
    // Initialize full transaction IPC client
    let ipc_client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start_monitoring().await?;
    
    info!("⏳ Waiting for connection to stabilize...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Performance tracking
    let mut total_transactions = 0u64;
    let mut last_report = Instant::now();
    let process_start = Instant::now();
    
    info!("📊 Starting transaction monitoring...");
    
    loop {
        // Get transactions with full data
        match ipc_client.get_full_transactions(25).await {
            Ok(transactions) => {
                if !transactions.is_empty() {
                    for tx in transactions {
                        total_transactions += 1;
                        
                        // Validate we have full data
                        let has_from = tx.tx_data.get("from").is_some();
                        let has_to = tx.tx_data.get("to").is_some();
                        let has_value = tx.tx_data.get("value").is_some();
                        let has_input = tx.tx_data.get("input").is_some();
                        let has_gas = tx.tx_data.get("gas").is_some();
                        
                        // Log first few transactions
                        if total_transactions <= 5 {
                            info!("✅ TX #{}: 0x{}... ({}ns latency)", 
                                  total_transactions, 
                                  &tx.hash[2..8],
                                  tx.latency_ns);
                            info!("   Full data: from={}, to={}, value={}, input={}, gas={}",
                                  has_from, has_to, has_value, has_input, has_gas);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Error getting transactions: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        
        // Report performance every 1000 transactions
        if total_transactions > 0 && total_transactions % 1000 == 0 && last_report.elapsed() > Duration::from_secs(1) {
            let stats = ipc_client.get_stats().await;
            let throughput = total_transactions as f64 / process_start.elapsed().as_secs_f64();
            
            let report = format!(
                "[{}] ============================================================\n\
                [{}] Full TX IPC Performance Report:\n\
                [{}]    Total Transactions:     {}\n\
                [{}]    Average Latency:        {}ns ({}μs)\n\
                [{}]    Min Latency:           {}ns\n\
                [{}]    Max Latency:           {}ns\n\
                [{}]    Sub-1ms Transactions:   {}% ({}/{})\n\
                [{}]    Sub-100μs Transactions: {}\n\
                [{}]    Sub-10μs Transactions:  {}\n\
                [{}]    Throughput:             {:.1} tx/sec\n\
                [{}] ============================================================\n\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                total_transactions,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                stats.avg_latency_ns,
                stats.avg_latency_ns / 1000,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                stats.min_latency_ns.unwrap_or(0),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                stats.max_latency_ns.unwrap_or(0),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                if stats.total_transactions > 0 { 
                    (stats.sub_1ms_count * 100) / stats.total_transactions 
                } else { 0 },
                stats.sub_1ms_count, 
                stats.total_transactions,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                stats.sub_100us_count,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                stats.sub_10us_count,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), 
                throughput,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f")
            );
            
            // Log to console
            info!("📊 {} transactions, avg: {}μs, throughput: {:.1} tx/sec", 
                  total_transactions, stats.avg_latency_ns / 1000, throughput);
            
            // Write to timing file
            timing_file.write_all(report.as_bytes())?;
            timing_file.flush()?;
            
            last_report = Instant::now();
        }
        
        // Small sleep to avoid busy loop
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}