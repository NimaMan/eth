/// Real-Time Mempool Simulation Pipeline
/// 
/// This example demonstrates the complete architecture for real-time transaction simulation:
/// WebSocket Mempool → Transaction Queue → Simulation → Results
///
/// Key Features:
/// - Live mempool streaming via WebSocket
/// - Non-blocking queue processing
/// - Concurrent simulation pipeline
/// - Performance monitoring with latency tracking
/// 
/// Run with: cargo run --example realtime_simulation_pipeline

use mempool_processor::mempool_fetcher::{WebSocketFetcher, TimestampedTransaction};
use mempool_processor::tx_simulator::{SimulatorWrapper, FastRpcSimulator};
use mempool_processor::mempool_fetcher::TransactionView;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;
use tracing::{info, warn, error, debug};
use tracing_subscriber;
use revm_context::BlockEnv;
use revm_primitives::hardfork::SpecId;
use ethers::providers::{Provider, Http, Middleware};
use std::sync::atomic::{AtomicU64, Ordering};

/// Performance metrics for the pipeline
#[derive(Default)]
struct PipelineMetrics {
    transactions_received: AtomicU64,
    transactions_simulated: AtomicU64,
    total_mempool_latency_ms: AtomicU64,
    total_simulation_time_ms: AtomicU64,
    simulation_errors: AtomicU64,
}

impl PipelineMetrics {
    fn record_transaction(&self, mempool_latency_ms: f64, simulation_time_ms: f64, success: bool) {
        self.transactions_received.fetch_add(1, Ordering::Relaxed);
        self.total_mempool_latency_ms.fetch_add(mempool_latency_ms as u64, Ordering::Relaxed);
        
        if success {
            self.transactions_simulated.fetch_add(1, Ordering::Relaxed);
            self.total_simulation_time_ms.fetch_add(simulation_time_ms as u64, Ordering::Relaxed);
        } else {
            self.simulation_errors.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    fn print_stats(&self) {
        let received = self.transactions_received.load(Ordering::Relaxed);
        let simulated = self.transactions_simulated.load(Ordering::Relaxed);
        let errors = self.simulation_errors.load(Ordering::Relaxed);
        
        if received > 0 {
            let avg_mempool_latency = self.total_mempool_latency_ms.load(Ordering::Relaxed) as f64 / received as f64;
            let success_rate = (simulated as f64 / received as f64) * 100.0;
            
            let avg_simulation_time = if simulated > 0 {
                self.total_simulation_time_ms.load(Ordering::Relaxed) as f64 / simulated as f64
            } else {
                0.0
            };
            
            info!("📊 Pipeline Statistics:");
            info!("  ├─ Transactions Received: {}", received);
            info!("  ├─ Successfully Simulated: {} ({:.1}%)", simulated, success_rate);
            info!("  ├─ Simulation Errors: {}", errors);
            info!("  ├─ Avg Mempool Latency: {:.1}ms", avg_mempool_latency);
            info!("  └─ Avg Simulation Time: {:.1}ms", avg_simulation_time);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("realtime_simulation_pipeline=info,mempool_processor=debug")
        .init();
    
    info!("🚀 Real-Time Mempool Simulation Pipeline");
    info!("=========================================\n");
    
    // Configuration
    let ws_url = std::env::var("ETH_WS_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8546".to_string());
    let rpc_url = std::env::var("ETH_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    
    info!("Configuration:");
    info!("├─ WebSocket URL: {}", ws_url);
    info!("└─ RPC URL: {}", rpc_url);
    
    // Initialize components
    info!("\n🔧 Initializing components...");
    
    // 1. WebSocket fetcher for live mempool
    let fetcher = WebSocketFetcher::new(&ws_url, &rpc_url).await?;
    info!("✅ WebSocket fetcher initialized");
    
    // 2. Transaction simulator
    let simulator = SimulatorWrapper::new_fast_rpc(&rpc_url).await?;
    info!("✅ Fast RPC simulator initialized");
    
    // 3. Get current block context for simulation
    let provider = Arc::new(Provider::<Http>::try_from(&rpc_url)?);
    let latest_block = provider.get_block(ethers::types::BlockNumber::Latest).await?
        .ok_or("Failed to get latest block")?;
    
    let block_env = BlockEnv {
        number: revm_primitives::U256::from(latest_block.number.unwrap_or_default().as_u64()),
        timestamp: revm_primitives::U256::from(latest_block.timestamp.as_u64()),
        gas_limit: latest_block.gas_limit.as_u64(),
        basefee: latest_block.base_fee_per_gas.unwrap_or_default().as_u64(),
        difficulty: revm_primitives::U256::from(latest_block.difficulty.as_u64()),
        prevrandao: Some(revm_primitives::B256::from_slice(latest_block.mix_hash.unwrap_or_default().as_bytes())),
        beneficiary: revm_primitives::Address::from_slice(latest_block.author.unwrap_or_default().as_bytes()),
        ..Default::default()
    };
    
    info!("✅ Block context ready (block #{})", latest_block.number.unwrap_or_default());
    
    // 4. Performance metrics
    let metrics = Arc::new(PipelineMetrics::default());
    
    // 5. Processing queue (high capacity for burst handling)
    let (queue_tx, mut queue_rx) = mpsc::channel::<TimestampedTransaction>(10000);
    
    info!("✅ Processing queue initialized (10,000 transaction buffer)");
    
    // Start the pipeline
    info!("\n🏃 Starting real-time pipeline...");
    
    // Task 1: Mempool streaming 
    let fetcher_handle = {
        let queue_tx = queue_tx.clone();
        let fetcher = Arc::new(fetcher);
        tokio::spawn(async move {
            info!("📡 Starting mempool streaming...");
            
            // Start WebSocket subscription
            if let Err(e) = fetcher.start_full_capture().await {
                error!("Failed to start mempool capture: {}", e);
                return;
            }
            
            // Get the transaction receiver
            let receiver = fetcher.get_transaction_receiver().await;
            let mut receiver = receiver.lock().await;
            
            info!("🔄 Mempool streaming active - forwarding to simulation queue");
            
            let mut count = 0;
            let start_time = Instant::now();
            
            // Forward transactions to simulation queue
            while let Some(timestamped_tx) = receiver.recv().await {
                count += 1;
                
                // Log progress every 100 transactions
                if count % 100 == 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let tps = count as f64 / elapsed;
                    debug!("📈 Forwarded {} transactions to simulation queue ({:.1} TPS)", count, tps);
                }
                
                // Forward to simulation queue (non-blocking)
                if let Err(e) = queue_tx.try_send(timestamped_tx) {
                    warn!("Simulation queue full, dropping transaction: {}", e);
                }
            }
            
            warn!("Mempool streaming ended");
        })
    };
    
    // Task 2: Simulation processing
    let simulation_handle = {
        let simulator = Arc::new(simulator);
        let metrics = Arc::clone(&metrics);
        let block_env = block_env.clone();
        
        tokio::spawn(async move {
            info!("🧮 Starting simulation processor...");
            
            let mut processed_count = 0;
            let start_time = Instant::now();
            
            while let Some(timestamped_tx) = queue_rx.recv().await {
                let simulation_start = Instant::now();
                
                // Simulate the transaction
                match simulator.process_transaction(&timestamped_tx.tx, &block_env).await {
                    Ok(Some(state_changes)) => {
                        let simulation_time = simulation_start.elapsed().as_millis() as f64;
                        
                        processed_count += 1;
                        
                        // Record metrics
                        metrics.record_transaction(
                            timestamped_tx.latency_ms,
                            simulation_time,
                            true
                        );
                        
                        // Log interesting transactions (with state changes)
                        if !state_changes.is_empty() {
                            debug!("💰 Transaction with {} state changes (mempool: {:.1}ms, sim: {:.1}ms)", 
                                  state_changes.len(), timestamped_tx.latency_ms, simulation_time);
                        }
                        
                        // Log progress
                        if processed_count % 50 == 0 {
                            let elapsed = start_time.elapsed().as_secs_f64();
                            let tps = processed_count as f64 / elapsed;
                            info!("🔄 Simulated {} transactions ({:.1} TPS)", processed_count, tps);
                        }
                    }
                    Ok(None) => {
                        // No state changes, but successful simulation
                        let simulation_time = simulation_start.elapsed().as_millis() as f64;
                        metrics.record_transaction(timestamped_tx.latency_ms, simulation_time, true);
                        processed_count += 1;
                    }
                    Err(e) => {
                        // Simulation error
                        let simulation_time = simulation_start.elapsed().as_millis() as f64;
                        metrics.record_transaction(timestamped_tx.latency_ms, simulation_time, false);
                        debug!("❌ Simulation error: {}", e);
                    }
                }
            }
            
            warn!("Simulation processor ended");
        })
    };
    
    // Task 3: Statistics reporting  
    let stats_handle = {
        let metrics = Arc::clone(&metrics);
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                metrics.print_stats();
            }
        })
    };
    
    info!("✅ All pipeline tasks started");
    info!("\n📊 Pipeline Status:");
    info!("├─ 📡 Mempool Streaming: Active");
    info!("├─ 🧮 Transaction Simulation: Active");
    info!("├─ 📈 Statistics Reporting: Every 30s");
    info!("└─ 🛑 Press Ctrl+C to stop");
    
    // Wait for Ctrl+C
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("\n🛑 Shutdown signal received");
        }
        _ = fetcher_handle => {
            warn!("Mempool fetcher task ended");
        }
        _ = simulation_handle => {
            warn!("Simulation task ended");
        }
        _ = stats_handle => {
            warn!("Stats task ended");
        }
    }
    
    // Final statistics
    info!("\n📊 Final Pipeline Statistics:");
    metrics.print_stats();
    
    info!("✅ Pipeline shutdown complete");
    Ok(())
}