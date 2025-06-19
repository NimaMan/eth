/// Mempool Transaction Fetching Pipeline Analysis
/// 
/// Comprehensive test to measure the complete mempool transaction fetching pipeline:
/// 1. Transaction arrival at Reth node (mempool entry)
/// 2. WebSocket detection and delivery to our application  
/// 3. Transaction data fetching via HTTP RPC (get_transaction)
/// 4. Optional: State diff fetching via debug_traceCall RPC
/// 5. Data conversion to internal format
///
/// This measures RPC-based transaction processing, NOT REVM simulation.
/// For REVM simulation timing, see tools/performance/revm_performance_monitor.rs
///
/// Run with: cargo run --example mempool_fetch_timing_analysis

use mempool_processor::mempool_fetcher::websocket::WebSocketTransaction;
use mempool_processor::mempool_fetcher::TransactionView;
use mempool_processor::performance_metrics::PerformanceTracker;

use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{Transaction, H256};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, warn, debug, error};

/// Detailed timing measurements for mempool transaction fetching
#[derive(Debug, Clone)]
struct MempoolTxFetchTiming {
    /// Transaction hash
    tx_hash: String,
    
    /// When the transaction arrived at Reth node (from WebSocket)
    mempool_arrival_time: Instant,
    
    /// When we detected it via WebSocket
    websocket_detection_time: Instant,
    
    /// When we started RPC transaction fetch
    rpc_fetch_start_time: Option<Instant>,
    
    /// When RPC transaction fetch completed
    rpc_fetch_end_time: Option<Instant>,
    
    /// When we started data conversion
    data_conversion_start_time: Option<Instant>,
    
    /// When data conversion completed
    data_conversion_end_time: Option<Instant>,
    
    /// Whether the transaction was successfully fetched
    fetch_successful: bool,
    
    /// Whether data conversion was successful
    conversion_successful: bool,
}

impl MempoolTxFetchTiming {
    fn new(tx_hash: String, arrival_time: Instant, detection_time: Instant) -> Self {
        Self {
            tx_hash,
            mempool_arrival_time: arrival_time,
            websocket_detection_time: detection_time,
            rpc_fetch_start_time: None,
            rpc_fetch_end_time: None,
            data_conversion_start_time: None,
            data_conversion_end_time: None,
            fetch_successful: false,
            conversion_successful: false,
        }
    }
    
    fn websocket_detection_latency_ms(&self) -> f64 {
        let duration = self.websocket_detection_time.duration_since(self.mempool_arrival_time);
        duration.as_secs_f64() * 1000.0
    }
    
    fn rpc_fetch_duration_ms(&self) -> Option<f64> {
        if let (Some(start), Some(end)) = (self.rpc_fetch_start_time, self.rpc_fetch_end_time) {
            let duration = end.duration_since(start);
            Some(duration.as_secs_f64() * 1000.0)
        } else {
            None
        }
    }
    
    fn data_conversion_duration_ms(&self) -> Option<f64> {
        if let (Some(start), Some(end)) = (self.data_conversion_start_time, self.data_conversion_end_time) {
            let duration = end.duration_since(start);
            Some(duration.as_secs_f64() * 1000.0)
        } else {
            None
        }
    }
    
    fn total_mempool_to_ready_duration_ms(&self) -> Option<f64> {
        if let Some(end) = self.data_conversion_end_time {
            let duration = end.duration_since(self.mempool_arrival_time);
            Some(duration.as_secs_f64() * 1000.0)
        } else {
            None
        }
    }
    
    fn queue_time_ms(&self) -> Option<f64> {
        if let Some(fetch_start) = self.rpc_fetch_start_time {
            let duration = fetch_start.duration_since(self.websocket_detection_time);
            Some(duration.as_secs_f64() * 1000.0)
        } else {
            None
        }
    }
}

fn convert_ethers_to_transaction_view(tx: &Transaction) -> TransactionView {
    TransactionView {
        hash: tx.hash.as_bytes().to_vec(),
        from: tx.from.as_bytes().to_vec(),
        to: tx.to.map(|addr| addr.as_bytes().to_vec()),
        value: tx.value,
        gas_price: tx.gas_price,
        gas_limit: Some(tx.gas),
        nonce: Some(tx.nonce),
        input_data: Some(tx.input.to_vec()),
    }
}

/// Direct transaction processor that handles transactions as they arrive
struct DirectTransactionProcessor {
    http_provider: Arc<Provider<Http>>,
    performance_tracker: Arc<PerformanceTracker>,
    timings: Arc<Mutex<Vec<MempoolTxFetchTiming>>>,
    processed_count: Arc<Mutex<usize>>,
    max_transactions: usize,
}

impl DirectTransactionProcessor {
    fn new(http_provider: Arc<Provider<Http>>, max_transactions: usize) -> Self {
        Self {
            http_provider,
            performance_tracker: Arc::new(PerformanceTracker::new_optimized(10)),
            timings: Arc::new(Mutex::new(Vec::new())),
            processed_count: Arc::new(Mutex::new(0)),
            max_transactions,
        }
    }
    
    async fn process_transaction(&self, ws_tx: WebSocketTransaction) -> bool {
        let mut count = self.processed_count.lock().await;
        *count += 1;
        let current_count = *count;
        
        if current_count > self.max_transactions {
            return false; // Stop processing
        }
        
        info!("📦 TX #{}: {} (WebSocket latency: {:.2}ms)", 
              current_count, 
              &ws_tx.hash[..16], 
              ws_tx.latency_ms);
        
        // Create detailed timing measurement
        let mut timing = MempoolTxFetchTiming::new(
            ws_tx.hash.clone(),
            ws_tx.arrival_time,
            ws_tx.detection_time,
        );
        
        // Start performance tracking
        let tx_timing = self.performance_tracker.start_transaction_with_arrival(
            ws_tx.hash.clone(), 
            ws_tx.arrival_time
        ).await;
        
        // Parse transaction hash for HTTP fetch
        match ws_tx.hash.parse::<H256>() {
            Ok(tx_hash) => {
                // Mark start of RPC fetch
                timing.rpc_fetch_start_time = Some(Instant::now());
                tx_timing.write().await.mark_processing_start();
                
                // Fetch full transaction via HTTP
                match self.http_provider.get_transaction(tx_hash).await {
                    Ok(Some(eth_tx)) => {
                        timing.rpc_fetch_end_time = Some(Instant::now());
                        timing.fetch_successful = true;
                        
                        // Mark start of data conversion
                        timing.data_conversion_start_time = Some(Instant::now());
                        
                        // Convert to internal format
                        let _tx_view = convert_ethers_to_transaction_view(&eth_tx);
                        
                        // Mark end of data conversion
                        timing.data_conversion_end_time = Some(Instant::now());
                        timing.conversion_successful = true;
                        
                        // Complete performance tracking
                        self.performance_tracker.complete_transaction(tx_timing).await;
                        
                        // Log detailed timing breakdown
                        info!("   ├─ WebSocket detection: {:.2}ms", timing.websocket_detection_latency_ms());
                        if let Some(queue_time) = timing.queue_time_ms() {
                            info!("   ├─ Queue time: {:.2}ms", queue_time);
                        }
                        if let Some(fetch_time) = timing.rpc_fetch_duration_ms() {
                            info!("   ├─ RPC fetch: {:.2}ms", fetch_time);
                        }
                        if let Some(prep_time) = timing.data_conversion_duration_ms() {
                            info!("   ├─ Data conversion: {:.2}ms", prep_time);
                        }
                        if let Some(total_time) = timing.total_mempool_to_ready_duration_ms() {
                            info!("   └─ Total pipeline: {:.2}ms", total_time);
                        }
                        
                        debug!("   Transaction details:");
                        debug!("     From: {:?}", eth_tx.from);
                        debug!("     To: {:?}", eth_tx.to);
                        debug!("     Value: {} wei", eth_tx.value);
                        debug!("     Gas: {}", eth_tx.gas);
                    }
                    Ok(None) => {
                        timing.rpc_fetch_end_time = Some(Instant::now());
                        warn!("   ⚠️  Transaction not found in node");
                        self.performance_tracker.complete_transaction(tx_timing).await;
                    }
                    Err(e) => {
                        timing.rpc_fetch_end_time = Some(Instant::now());
                        warn!("   ❌ RPC fetch failed: {}", e);
                        self.performance_tracker.complete_transaction(tx_timing).await;
                    }
                }
            }
            Err(e) => {
                warn!("   ❌ Invalid transaction hash: {}", e);
                self.performance_tracker.complete_transaction(tx_timing).await;
            }
        }
        
        // Store timing
        self.timings.lock().await.push(timing);
        
        true // Continue processing
    }
    
    async fn get_results(&self) -> Vec<MempoolTxFetchTiming> {
        self.timings.lock().await.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();
    
    info!("🚀 Mempool Transaction Fetching Pipeline Analysis");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 Measuring mempool-to-ready transaction fetching pipeline (RPC-based)");
    info!("   Phase 1: Mempool arrival → WebSocket detection");
    info!("   Phase 2: WebSocket detection → RPC fetch start");
    info!("   Phase 3: RPC transaction fetch duration");
    info!("   Phase 4: Data conversion duration");
    info!("   Total: Mempool arrival → Processing ready");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    
    info!("🔌 Connecting to WebSocket: {}", ws_url);
    info!("🌐 HTTP provider: {}", http_url);
    
    // Create HTTP provider for transaction fetching
    let http_provider = Arc::new(Provider::<Http>::try_from(http_url)?);
    
    // Create direct transaction processor
    let max_transactions = 20;
    let processor = Arc::new(DirectTransactionProcessor::new(http_provider, max_transactions));
    
    // Create a custom WebSocket connection for direct processing
    info!("✅ Starting direct transaction processing...\n");
    
    // We'll implement a simple direct WebSocket approach
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
    use futures_util::{SinkExt, StreamExt};
    use url::Url;
    use serde_json::Value;
    
    let url = Url::parse(ws_url)?;
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();
    
    // Subscribe to pending transactions
    let subscribe_request = json!({
        "jsonrpc": "2.0",
        "method": "eth_subscribe",
        "params": ["newPendingTransactions"],
        "id": 1
    });
    
    write.send(Message::Text(subscribe_request.to_string())).await?;
    
    // Read subscription confirmation
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let response: Value = serde_json::from_str(&response)?;
        let subscription_id = response["result"].as_str()
            .ok_or("Failed to get subscription ID")?;
            
        info!("📡 Subscribed to pending transactions: {}", subscription_id);
    }
    
    info!("⏱️  Processing transactions as they arrive (max {})...", max_transactions);
    info!("🔍 Waiting for mempool transactions...");
    
    let start_time = Instant::now();
    let mut processed_count = 0;
    
    // Process transactions as they arrive
    while let Some(message) = read.next().await {
        let arrival_time = Instant::now();
        
        if start_time.elapsed() > Duration::from_secs(120) {
            warn!("⏰ Timeout reached (120 seconds)");
            break;
        }
        
        // Show periodic status
        if start_time.elapsed().as_secs() % 10 == 0 && start_time.elapsed().as_millis() % 10000 < 100 {
            info!("⏱️  Still waiting... {:.0}s elapsed", start_time.elapsed().as_secs_f64());
        }
        
        match message {
            Ok(Message::Text(text)) => {
                debug!("📥 WebSocket message received: {}", text);
                if let Ok(notification) = serde_json::from_str::<Value>(&text) {
                    if let Some(params) = notification.get("params") {
                        if let Some(tx_hash) = params["result"].as_str() {
                            let detection_time = Instant::now();
                            let latency_ms = detection_time.duration_since(arrival_time).as_secs_f64() * 1000.0;
                            
                            let ws_tx = WebSocketTransaction {
                                hash: tx_hash.to_string(),
                                arrival_time,
                                detection_time,
                                latency_ms,
                                tx_data: None,
                            };
                            
                            // Process transaction directly
                            if !processor.process_transaction(ws_tx).await {
                                break; // Stop processing when max reached
                            }
                            
                            processed_count += 1;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {} // Ignore other message types
        }
    }
    
    // Get results and display analysis
    let mempool_fetch_timings = processor.get_results().await;
    
    info!("\n📊 MEMPOOL FETCHING PIPELINE RESULTS:");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    if mempool_fetch_timings.is_empty() {
        warn!("❌ No transactions processed during test period");
        return Ok(());
    }
    
    // Filter successful transactions
    let successful: Vec<&MempoolTxFetchTiming> = mempool_fetch_timings
        .iter()
        .filter(|t| t.conversion_successful)
        .collect();
    
    info!("📈 Transactions processed: {} total, {} successful", mempool_fetch_timings.len(), successful.len());
    
    if !successful.is_empty() {
        // Calculate WebSocket latency statistics
        let websocket_latencies: Vec<f64> = successful.iter().map(|t| t.websocket_detection_latency_ms()).collect();
        let ws_avg = websocket_latencies.iter().sum::<f64>() / websocket_latencies.len() as f64;
        let ws_min = websocket_latencies.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let ws_max = websocket_latencies.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        info!("\n🔌 Phase 1 - WebSocket Detection Latency:");
        info!("   Average: {:.2}ms", ws_avg);
        info!("   Min: {:.2}ms", ws_min);
        info!("   Max: {:.2}ms", ws_max);
        
        // Calculate queue time statistics
        let queue_times: Vec<f64> = successful.iter().filter_map(|t| t.queue_time_ms()).collect();
        if !queue_times.is_empty() {
            let queue_avg = queue_times.iter().sum::<f64>() / queue_times.len() as f64;
            let queue_min = queue_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let queue_max = queue_times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            info!("\n⏳ Phase 2 - Queue Time (Detection → Fetch):");
            info!("   Average: {:.2}ms", queue_avg);
            info!("   Min: {:.2}ms", queue_min);
            info!("   Max: {:.2}ms", queue_max);
        }
        
        // Calculate RPC fetch statistics
        let fetch_times: Vec<f64> = successful.iter().filter_map(|t| t.rpc_fetch_duration_ms()).collect();
        if !fetch_times.is_empty() {
            let fetch_avg = fetch_times.iter().sum::<f64>() / fetch_times.len() as f64;
            let fetch_min = fetch_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let fetch_max = fetch_times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            info!("\n🌐 Phase 3 - RPC Transaction Fetch Duration:");
            info!("   Average: {:.2}ms", fetch_avg);
            info!("   Min: {:.2}ms", fetch_min);
            info!("   Max: {:.2}ms", fetch_max);
        }
        
        // Calculate data conversion statistics
        let prep_times: Vec<f64> = successful.iter().filter_map(|t| t.data_conversion_duration_ms()).collect();
        if !prep_times.is_empty() {
            let prep_avg = prep_times.iter().sum::<f64>() / prep_times.len() as f64;
            let prep_min = prep_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let prep_max = prep_times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            info!("\n⚙️  Phase 4 - Data Conversion:");
            info!("   Average: {:.2}ms", prep_avg);
            info!("   Min: {:.2}ms", prep_min);
            info!("   Max: {:.2}ms", prep_max);
        }
        
        // Calculate total pipeline statistics
        let total_times: Vec<f64> = successful.iter().filter_map(|t| t.total_mempool_to_ready_duration_ms()).collect();
        if !total_times.is_empty() {
            let total_avg = total_times.iter().sum::<f64>() / total_times.len() as f64;
            let total_min = total_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let total_max = total_times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            info!("\n🏁 Total Mempool-to-Ready (Arrival → Processing Ready):");
            info!("   Average: {:.2}ms", total_avg);
            info!("   Min: {:.2}ms", total_min);
            info!("   Max: {:.2}ms", total_max);
            info!("   Throughput: {:.0} transactions/second", 1000.0 / total_avg);
        }
    }
    
    info!("\n✅ Mempool fetch timing analysis completed!");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(())
}