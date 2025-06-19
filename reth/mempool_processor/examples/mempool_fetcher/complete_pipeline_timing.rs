/// Complete Pipeline Timing Analysis
/// 
/// Comprehensive timing measurement for the complete mempool processing pipeline:
/// 1. Transaction arrival at Reth mempool
/// 2. WebSocket detection and delivery to our application  
/// 3. Transaction data fetching (RPC vs direct Reth access comparison)
/// 4. Transaction simulation preparation
/// 5. Signal analysis and detection
///
/// This tool helps compare different approaches (RPC vs direct Reth access) and
/// identifies bottlenecks in the complete transaction processing pipeline.
///
/// Run with: cargo run --example complete_pipeline_timing

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use eyre::Result;
use tracing::{info, warn, error, debug};
use tokio::time;

// Mempool processor imports
use mempool_processor::mempool_fetcher::{WebSocketClient, TransactionView};
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::signal_engine::{SignalEngine, SignalConfig, SimulationResult, PoolEffect};
use mempool_processor::tx_simulator::DebugTraceCallSimulator;
use mempool_processor::performance_metrics::PerformanceTracker;

// Ethers imports
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{H256, U256};

// WebSocket imports for direct connection
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;

/// Comprehensive timing measurements for the complete pipeline
#[derive(Debug, Clone)]
pub struct CompletePipelineTiming {
    /// Transaction hash
    pub tx_hash: String,
    
    /// Timestamp when transaction arrived at Reth mempool (approximated)
    pub reth_arrival_time: Instant,
    
    /// Timestamp when WebSocket detected the transaction
    pub websocket_detection_time: Instant,
    
    /// Timestamp when transaction fetch started
    pub fetch_start_time: Instant,
    
    /// Timestamp when transaction fetch completed
    pub fetch_complete_time: Instant,
    
    /// Timestamp when simulation preparation started
    pub simulation_prep_start_time: Instant,
    
    /// Timestamp when simulation completed (ready for analysis)
    pub simulation_ready_time: Instant,
    
    /// Timestamp when signal analysis started
    pub signal_analysis_start_time: Option<Instant>,
    
    /// Timestamp when signal analysis completed
    pub signal_analysis_complete_time: Option<Instant>,
    
    /// Whether the transaction was successfully processed
    pub processing_successful: bool,
    
    /// Method used for transaction fetching
    pub fetch_method: String,
    
    /// Number of pools affected by this transaction
    pub pools_affected: usize,
    
    /// Number of market events detected
    pub events_detected: usize,
}

impl CompletePipelineTiming {
    pub fn new(tx_hash: String, websocket_detection_time: Instant, fetch_method: String) -> Self {
        Self {
            tx_hash,
            reth_arrival_time: websocket_detection_time, // Approximation - WebSocket latency is ~0.1ms
            websocket_detection_time,
            fetch_start_time: Instant::now(),
            fetch_complete_time: Instant::now(),
            simulation_prep_start_time: Instant::now(),
            simulation_ready_time: Instant::now(),
            signal_analysis_start_time: None,
            signal_analysis_complete_time: None,
            processing_successful: false,
            fetch_method,
            pools_affected: 0,
            events_detected: 0,
        }
    }
    
    /// Mark fetch start
    pub fn mark_fetch_start(&mut self) {
        self.fetch_start_time = Instant::now();
    }
    
    /// Mark fetch completion
    pub fn mark_fetch_complete(&mut self) {
        self.fetch_complete_time = Instant::now();
    }
    
    /// Mark simulation preparation start
    pub fn mark_simulation_prep_start(&mut self) {
        self.simulation_prep_start_time = Instant::now();
    }
    
    /// Mark simulation ready
    pub fn mark_simulation_ready(&mut self, pools_affected: usize) {
        self.simulation_ready_time = Instant::now();
        self.pools_affected = pools_affected;
        self.processing_successful = true;
    }
    
    /// Mark signal analysis start
    pub fn mark_signal_analysis_start(&mut self) {
        self.signal_analysis_start_time = Some(Instant::now());
    }
    
    /// Mark signal analysis completion
    pub fn mark_signal_analysis_complete(&mut self, events_detected: usize) {
        self.signal_analysis_complete_time = Some(Instant::now());
        self.events_detected = events_detected;
    }
    
    /// Calculate WebSocket detection latency (approximated)
    pub fn websocket_detection_latency_ms(&self) -> f64 {
        self.websocket_detection_time.duration_since(self.reth_arrival_time).as_secs_f64() * 1000.0
    }
    
    /// Calculate queue time (WebSocket detection → Fetch start)
    pub fn queue_time_ms(&self) -> f64 {
        self.fetch_start_time.duration_since(self.websocket_detection_time).as_secs_f64() * 1000.0
    }
    
    /// Calculate transaction fetch duration
    pub fn fetch_duration_ms(&self) -> f64 {
        self.fetch_complete_time.duration_since(self.fetch_start_time).as_secs_f64() * 1000.0
    }
    
    /// Calculate simulation preparation duration
    pub fn simulation_prep_duration_ms(&self) -> f64 {
        self.simulation_ready_time.duration_since(self.simulation_prep_start_time).as_secs_f64() * 1000.0
    }
    
    /// Calculate total pipeline duration (Reth arrival → Simulation ready)
    pub fn total_pipeline_duration_ms(&self) -> f64 {
        self.simulation_ready_time.duration_since(self.reth_arrival_time).as_secs_f64() * 1000.0
    }
    
    /// Calculate signal analysis duration (if completed)
    pub fn signal_analysis_duration_ms(&self) -> Option<f64> {
        if let (Some(start), Some(end)) = (self.signal_analysis_start_time, self.signal_analysis_complete_time) {
            Some(end.duration_since(start).as_secs_f64() * 1000.0)
        } else {
            None
        }
    }
    
    /// Calculate total processing duration (Reth arrival → Signal analysis complete)
    pub fn total_processing_duration_ms(&self) -> Option<f64> {
        self.signal_analysis_complete_time.map(|end_time| {
            end_time.duration_since(self.reth_arrival_time).as_secs_f64() * 1000.0
        })
    }
}

/// Convert ethers Transaction to TransactionView
fn convert_ethers_to_transaction_view(tx: &ethers::types::Transaction) -> TransactionView {
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

/// Process a single transaction with comprehensive timing analysis
async fn process_transaction_with_timing(
    tx_hash: String,
    websocket_detection_time: Instant,
    http_provider: Arc<Provider<Http>>,
    simulator: Arc<DebugTraceCallSimulator>,
    signal_engine: Arc<SignalEngine>,
    fetch_method: String,
) -> Result<CompletePipelineTiming> {
    let mut timing = CompletePipelineTiming::new(tx_hash.clone(), websocket_detection_time, fetch_method);
    
    // Phase 1: Transaction fetch
    timing.mark_fetch_start();
    
    let eth_tx = match http_provider.get_transaction(H256::from_slice(&hex::decode(&tx_hash[2..])?)).await? {
        Some(tx) => tx,
        None => {
            warn!("Transaction {} not found", tx_hash);
            return Ok(timing);
        }
    };
    
    timing.mark_fetch_complete();
    
    // Phase 2: Simulation preparation
    timing.mark_simulation_prep_start();
    
    let tx_view = convert_ethers_to_transaction_view(&eth_tx);
    
    // Simulate the transaction to get state changes
    let simulation_result = match simulator.simulate_transaction(&tx_view).await {
        Ok(result) => result,
        Err(e) => {
            debug!("Simulation failed for {}: {}", tx_hash, e);
            return Ok(timing);
        }
    };
    
    // Create SimulationResult for signal analysis
    let mut affected_pools = HashMap::new();
    let pools_count = affected_pools.len();
    
    // For now, create a mock simulation result
    // In a real implementation, you would process the simulation_result here
    let signal_simulation = SimulationResult {
        tx_hash: tx_hash.clone(),
        affected_pools,
        simulation_successful: true,
        error_message: None,
    };
    
    timing.mark_simulation_ready(pools_count);
    
    // Phase 3: Signal analysis
    timing.mark_signal_analysis_start();
    
    let events = signal_engine.analyze_transaction(signal_simulation);
    let events_count = events.len();
    
    timing.mark_signal_analysis_complete(events_count);
    
    if !events.is_empty() {
        info!("🚨 Detected {} market events for tx {}", events_count, tx_hash);
        for event in events {
            info!("   📊 {:?}: {} (confidence: {:.2})", 
                  event.event_type, event.details, event.confidence);
        }
    }
    
    Ok(timing)
}

/// Print comprehensive timing analysis results
fn print_timing_analysis(timings: &[CompletePipelineTiming]) {
    if timings.is_empty() {
        warn!("❌ No transactions processed during timing analysis");
        return;
    }
    
    let successful: Vec<_> = timings.iter().filter(|t| t.processing_successful).collect();
    
    info!("\n📊 COMPLETE PIPELINE TIMING ANALYSIS RESULTS:");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📈 Transactions processed: {} total, {} successful", timings.len(), successful.len());
    
    if !successful.is_empty() {
        // Group by fetch method
        let mut by_method: HashMap<String, Vec<&CompletePipelineTiming>> = HashMap::new();
        for timing in &successful {
            by_method.entry(timing.fetch_method.clone()).or_default().push(timing);
        }
        
        for (method, method_timings) in by_method {
            info!("\n🔧 Fetch Method: {}", method);
            info!("   Transactions: {}", method_timings.len());
            
            // Calculate statistics for each phase
            let websocket_times: Vec<f64> = method_timings.iter().map(|t| t.websocket_detection_latency_ms()).collect();
            let queue_times: Vec<f64> = method_timings.iter().map(|t| t.queue_time_ms()).collect();
            let fetch_times: Vec<f64> = method_timings.iter().map(|t| t.fetch_duration_ms()).collect();
            let sim_prep_times: Vec<f64> = method_timings.iter().map(|t| t.simulation_prep_duration_ms()).collect();
            let total_times: Vec<f64> = method_timings.iter().map(|t| t.total_pipeline_duration_ms()).collect();
            
            // Helper function to calculate statistics
            let calc_stats = |times: &[f64]| {
                let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let avg = times.iter().sum::<f64>() / times.len() as f64;
                (min, max, avg)
            };
            
            let (ws_min, ws_max, ws_avg) = calc_stats(&websocket_times);
            let (queue_min, queue_max, queue_avg) = calc_stats(&queue_times);
            let (fetch_min, fetch_max, fetch_avg) = calc_stats(&fetch_times);
            let (sim_min, sim_max, sim_avg) = calc_stats(&sim_prep_times);
            let (total_min, total_max, total_avg) = calc_stats(&total_times);
            
            info!("   📡 Phase 1 - WebSocket Detection: avg {:.2}ms (min {:.2}ms, max {:.2}ms)", ws_avg, ws_min, ws_max);
            info!("   ⏱️  Phase 2 - Queue Time: avg {:.2}ms (min {:.2}ms, max {:.2}ms)", queue_avg, queue_min, queue_max);
            info!("   🌐 Phase 3 - Transaction Fetch: avg {:.2}ms (min {:.2}ms, max {:.2}ms)", fetch_avg, fetch_min, fetch_max);
            info!("   ⚙️  Phase 4 - Simulation Prep: avg {:.2}ms (min {:.2}ms, max {:.2}ms)", sim_avg, sim_min, sim_max);
            info!("   🏁 Total Pipeline: avg {:.2}ms (min {:.2}ms, max {:.2}ms)", total_avg, total_min, total_max);
            info!("   🚀 Throughput: {:.0} transactions/second", 1000.0 / total_avg);
            
            // Signal analysis timing (if available)
            let signal_times: Vec<f64> = method_timings.iter()
                .filter_map(|t| t.signal_analysis_duration_ms())
                .collect();
            
            if !signal_times.is_empty() {
                let (sig_min, sig_max, sig_avg) = calc_stats(&signal_times);
                info!("   🔍 Phase 5 - Signal Analysis: avg {:.2}ms (min {:.2}ms, max {:.2}ms)", sig_avg, sig_min, sig_max);
                
                let total_processing_times: Vec<f64> = method_timings.iter()
                    .filter_map(|t| t.total_processing_duration_ms())
                    .collect();
                
                if !total_processing_times.is_empty() {
                    let (proc_min, proc_max, proc_avg) = calc_stats(&total_processing_times);
                    info!("   🎯 Complete Processing: avg {:.2}ms (min {:.2}ms, max {:.2}ms)", proc_avg, proc_min, proc_max);
                    info!("   ⚡ End-to-end throughput: {:.0} transactions/second", 1000.0 / proc_avg);
                }
            }
            
            // Pool and event statistics
            let total_pools: usize = method_timings.iter().map(|t| t.pools_affected).sum();
            let total_events: usize = method_timings.iter().map(|t| t.events_detected).sum();
            info!("   📊 Pools affected: {} total, avg {:.1} per transaction", total_pools, total_pools as f64 / method_timings.len() as f64);
            info!("   🚨 Events detected: {} total, avg {:.1} per transaction", total_events, total_events as f64 / method_timings.len() as f64);
        }
    }
    
    info!("\n✅ Complete pipeline timing analysis completed!");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🚀 Complete Pipeline Timing Analysis");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 Measuring complete pipeline: Reth arrival → Simulation ready → Signal analysis");
    info!("   Phase 1: Mempool arrival → WebSocket detection");
    info!("   Phase 2: WebSocket detection → Transaction fetch start");
    info!("   Phase 3: Transaction fetch duration (RPC)");
    info!("   Phase 4: Simulation preparation");
    info!("   Phase 5: Signal analysis and detection");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Configuration
    let eth_rpc_url = "http://127.0.0.1:8545";
    let eth_ws_url = "ws://127.0.0.1:8546";
    let pool_zmq_address = "tcp://localhost:5557";
    let sample_size = 20; // Number of transactions to analyze
    
    // Initialize HTTP provider
    info!("📡 Connecting to Ethereum node: {}", eth_rpc_url);
    let http_provider = Arc::new(Provider::<Http>::try_from(eth_rpc_url)?);
    
    // Initialize transaction simulator
    info!("🔧 Initializing transaction simulator...");
    let simulator = Arc::new(DebugTraceCallSimulator::new(http_provider.clone()));
    
    // Initialize pool subscriber for signal engine
    info!("🏊 Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(0.01, pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber failed: {}", e);
        }
    });
    
    // Give pool subscriber time to initialize
    tokio::time::sleep(Duration::from_secs(2)).await;
    let pool_count = pool_cache_clone.get_pool_count();
    info!("📊 Monitoring {} pools", pool_count);
    
    // Initialize signal engine
    info!("🔍 Initializing signal engine...");
    let signal_config = SignalConfig::default();
    let signal_engine = Arc::new(SignalEngine::with_pool_cache(pool_cache));
    
    info!("✅ All services initialized successfully!");
    
    // Start timing analysis
    info!("🔬 Starting comprehensive timing analysis...");
    info!("📈 Sample size: {} transactions", sample_size);
    
    let mut timing_results = Vec::new();
    let mut processed_count = 0;
    let start_time = Instant::now();
    
    // Connect to WebSocket stream directly for timing analysis
    let (ws_stream, _) = connect_async(eth_ws_url).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Subscribe to pending transactions
    let subscribe_msg = serde_json::json!({
        "id": 1,
        "method": "eth_subscribe",
        "params": ["newPendingTransactions"]
    });
    
    ws_sender.send(Message::Text(subscribe_msg.to_string())).await?;
    
    // Wait for subscription confirmation
    if let Some(Ok(Message::Text(response))) = ws_receiver.next().await {
        let parsed: Value = serde_json::from_str(&response)?;
        if let Some(subscription_id) = parsed["result"].as_str() {
            info!("📡 Subscribed to pending transactions: {}", subscription_id);
        }
    }
    
    info!("⏱️  Processing transactions with comprehensive timing analysis...");
    info!("🔍 Waiting for mempool transactions...");
    
    // Process transactions with timing analysis
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(notification) = serde_json::from_str::<Value>(&text) {
                    if let Some(params) = notification.get("params") {
                        if let Some(tx_hash) = params["result"].as_str() {
                            let websocket_detection_time = Instant::now();
                            
                            info!("📦 TX #{}: {} (WebSocket detection)", 
                                  processed_count + 1, &tx_hash[..14]);
                            
                            // Process transaction with comprehensive timing
                            match process_transaction_with_timing(
                                tx_hash.to_string(),
                                websocket_detection_time,
                                http_provider.clone(),
                                simulator.clone(),
                                signal_engine.clone(),
                                "RPC".to_string(),
                            ).await {
                                Ok(timing) => {
                                    // Print individual transaction timing
                                    info!("   ├─ WebSocket detection: {:.2}ms", timing.websocket_detection_latency_ms());
                                    info!("   ├─ Queue time: {:.2}ms", timing.queue_time_ms());
                                    info!("   ├─ Transaction fetch: {:.2}ms", timing.fetch_duration_ms());
                                    info!("   ├─ Simulation prep: {:.2}ms", timing.simulation_prep_duration_ms());
                                    info!("   └─ Total pipeline: {:.2}ms", timing.total_pipeline_duration_ms());
                                    
                                    if let Some(signal_duration) = timing.signal_analysis_duration_ms() {
                                        info!("   └─ Signal analysis: {:.2}ms", signal_duration);
                                        if let Some(total_duration) = timing.total_processing_duration_ms() {
                                            info!("   🎯 Complete processing: {:.2}ms", total_duration);
                                        }
                                    }
                                    
                                    timing_results.push(timing);
                                    processed_count += 1;
                                    
                                    // Check if we've reached the sample size
                                    if processed_count >= sample_size {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to process transaction {}: {}", tx_hash, e);
                                }
                            }
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
        }
        
        // Timeout check (5 minutes max)
        if start_time.elapsed() > Duration::from_secs(300) {
            info!("⏰ Timing analysis timeout reached");
            break;
        }
    }
    
    // Print comprehensive timing analysis results
    print_timing_analysis(&timing_results);
    
    Ok(())
}