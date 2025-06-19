/// Measure timing from Reth mempool arrival to data fetch completion
/// This shows the current HTTP RPC performance vs what DevP2P would achieve

use std::time::{Duration, Instant};
use ethers::providers::{Provider, Http, Ws, Middleware, StreamExt};
use ethers::types::{H256, U256};
use tokio::sync::Mutex;
use std::sync::Arc;
use tracing::{info, warn, error};

#[derive(Debug)]
struct TimingMeasurement {
    tx_hash: H256,
    ws_notification_time: Instant,
    rpc_fetch_start: Instant,
    rpc_fetch_end: Instant,
    total_latency_ms: f64,
    rpc_latency_ms: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("📊 Measuring Reth → Fetch timing (HTTP RPC vs DevP2P)");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    
    info!("🔌 Connecting to Reth WebSocket: {}", ws_url);
    let ws_provider = Provider::<Ws>::connect(ws_url).await?;
    
    info!("🌐 HTTP provider: {}", http_url);
    let http_provider = Arc::new(Provider::<Http>::try_from(http_url)?);
    
    // Check connection
    let block_number = http_provider.get_block_number().await?;
    info!("✅ Connected to Reth at block #{}", block_number);
    
    let measurements = Arc::new(Mutex::new(Vec::new()));
    let measurements_clone = measurements.clone();
    
    // Subscribe to pending transactions
    let mut stream = ws_provider.subscribe_pending_txs().await?;
    info!("📡 Subscribed to mempool");
    info!("");
    info!("⏱️  Measuring transaction timing (press Ctrl+C to stop)...");
    
    let start_time = Instant::now();
    let mut count = 0;
    
    while count < 10 {
        if let Some(tx_hash) = stream.next().await {
            let ws_notification_time = Instant::now();
            count += 1;
            
            // Start RPC fetch
            let rpc_fetch_start = Instant::now();
            
            match http_provider.get_transaction(tx_hash).await {
                Ok(Some(tx)) => {
                    let rpc_fetch_end = Instant::now();
                    
                    let rpc_latency_ms = (rpc_fetch_end - rpc_fetch_start).as_secs_f64() * 1000.0;
                    let total_latency_ms = (rpc_fetch_end - ws_notification_time).as_secs_f64() * 1000.0;
                    
                    info!("📦 TX #{}: 0x{}...", count, hex::encode(&tx_hash.as_bytes()[..4]));
                    info!("   HTTP RPC fetch: {:.2}ms", rpc_latency_ms);
                    info!("   Total latency: {:.2}ms", total_latency_ms);
                    info!("   Gas: {} gwei", tx.gas_price.unwrap_or(U256::zero()) / 1_000_000_000);
                    
                    measurements_clone.lock().await.push(TimingMeasurement {
                        tx_hash,
                        ws_notification_time,
                        rpc_fetch_start,
                        rpc_fetch_end,
                        total_latency_ms,
                        rpc_latency_ms,
                    });
                }
                Ok(None) => {
                    warn!("   Transaction not found");
                }
                Err(e) => {
                    error!("   RPC error: {}", e);
                }
            }
            
            if count >= 10 {
                break;
            }
        }
        
        if start_time.elapsed() > Duration::from_secs(30) {
            warn!("Timeout waiting for transactions");
            break;
        }
    }
    
    // Calculate statistics
    let measurements = measurements.lock().await;
    
    if measurements.is_empty() {
        error!("No transactions measured!");
        return Ok(());
    }
    
    let avg_rpc = measurements.iter().map(|m| m.rpc_latency_ms).sum::<f64>() / measurements.len() as f64;
    let min_rpc = measurements.iter().map(|m| m.rpc_latency_ms).fold(f64::MAX, f64::min);
    let max_rpc = measurements.iter().map(|m| m.rpc_latency_ms).fold(f64::MIN, f64::max);
    
    info!("");
    info!("📊 TIMING RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Transactions measured: {}", measurements.len());
    info!("");
    info!("HTTP RPC Fetch Time:");
    info!("  Average: {:.2}ms", avg_rpc);
    info!("  Min: {:.2}ms", min_rpc);
    info!("  Max: {:.2}ms", max_rpc);
    info!("");
    info!("DevP2P Expected Performance:");
    info!("  Average: <0.1ms");
    info!("  Improvement: {}x faster", (avg_rpc / 0.1) as u32);
    info!("");
    info!("What this means:");
    info!("  - HTTP RPC: Transaction data available ~{:.1}ms after mempool entry", avg_rpc);
    info!("  - DevP2P: Transaction data available <0.1ms after mempool entry");
    info!("  - For MEV/trading: {:.1}ms advantage per transaction", avg_rpc - 0.1);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(())
}