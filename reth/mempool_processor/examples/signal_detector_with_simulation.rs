/// Enhanced Signal Detector with Optional Simulation
/// 
/// This example shows how to add simulation capabilities to the existing
/// signal detection flow while maintaining high performance.

use std::time::{Duration, Instant};
use std::sync::Arc;
use std::collections::VecDeque;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn};
use tokio::time;

use mempool_processor::mempool_fetcher::{NonBlockingIpcClient, NonBlockingTransaction};
use mempool_processor::signal_engine::FunctionDetector;
use mempool_processor::tx_simulator::{BatchProcessor, SignalDetector, SignalDetectionConfig};
use reth_tx_simulator::{RethDirectTxSimulator, BatchSimulationOptions};

#[derive(Parser, Debug)]
struct Args {
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    /// Reth database path
    #[arg(long, env = "RETH_DB_PATH", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_db_path: String,
    
    /// Enable transaction simulation for detected signals
    #[arg(long)]
    enable_simulation: bool,
    
    /// Maximum simulation queue size
    #[arg(long, default_value = "100")]
    max_queue_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Starting Signal Detection Service");
    if args.enable_simulation {
        info!("   🔬 Transaction simulation enabled for detected signals");
    }
    
    // Initialize IPC client
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("⚡ IPC subscription active!");
    
    // Initialize function detector
    let function_detector = FunctionDetector::new();
    
    // Initialize simulation components if enabled
    let simulation_queue: Arc<tokio::sync::Mutex<VecDeque<NonBlockingTransaction>>> = 
        Arc::new(tokio::sync::Mutex::new(VecDeque::new()));
    
    let sim_handle = if args.enable_simulation {
        info!("🧪 Initializing transaction simulator...");
        let reth_simulator = Arc::new(RethDirectTxSimulator::new(&args.reth_db_path)?);
        let batch_processor = BatchProcessor::new(reth_simulator);
        let signal_detector = SignalDetector::new(SignalDetectionConfig::default());
        let queue = simulation_queue.clone();
        
        // Spawn simulation task
        Some(tokio::spawn(async move {
            simulation_task(batch_processor, signal_detector, queue).await
        }))
    } else {
        None
    };
    
    // Main processing loop
    info!("🎯 Starting main processing loop...");
    let mut total_processed = 0u64;
    let mut signals_detected = 0u64;
    let mut last_report = Instant::now();
    
    loop {
        let new_txs = ipc_client.get_transactions_instant(100).await;
        
        if new_txs.is_empty() {
            time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        for tx in new_txs {
            total_processed += 1;
            
            // Fast function detection
            let mut signal_found = false;
            
            // Check liquidity removal
            if function_detector.is_liquidity_removal(&tx.input).is_some() {
                signal_found = true;
                signals_detected += 1;
            }
            
            // Check trading enabled
            if tx.input.len() >= 4 {
                let selector = hex::encode(&tx.input[0..4]);
                if matches!(selector.as_str(), "c9567bf9" | "8a8c523c" | "a8aa1b31") {
                    signal_found = true;
                    signals_detected += 1;
                }
            }
            
            // Queue for simulation if signal found and simulation enabled
            if signal_found && args.enable_simulation {
                let mut queue = simulation_queue.lock().await;
                if queue.len() < args.max_queue_size {
                    queue.push_back(tx);
                } else {
                    warn!("Simulation queue full, dropping transaction");
                }
            }
            
            // Pass to function detector for logging
            function_detector.detect_from_ipc(&tx);
        }
        
        // Periodic reporting
        if last_report.elapsed() > Duration::from_secs(60) {
            info!("📊 PERFORMANCE REPORT:");
            info!("   Total processed: {}", total_processed);
            info!("   Signals detected: {}", signals_detected);
            
            if args.enable_simulation {
                let queue_size = simulation_queue.lock().await.len();
                info!("   Simulation queue: {} pending", queue_size);
            }
            
            function_detector.log_stats_summary();
            last_report = Instant::now();
        }
    }
}

/// Background task for processing simulation queue
async fn simulation_task(
    processor: BatchProcessor,
    signal_detector: SignalDetector,
    queue: Arc<tokio::sync::Mutex<VecDeque<NonBlockingTransaction>>>,
) -> Result<()> {
    let mut batch = Vec::new();
    let batch_size = 5;
    
    loop {
        // Collect transactions for batch
        {
            let mut queue_guard = queue.lock().await;
            while batch.len() < batch_size && !queue_guard.is_empty() {
                if let Some(tx) = queue_guard.pop_front() {
                    batch.push(tx);
                }
            }
        }
        
        if batch.is_empty() {
            time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        
        info!("🔬 Simulating batch of {} transactions", batch.len());
        
        // Process batch
        match processor.simulate_batch_with_state_changes(batch.clone()).await {
            Ok(results) => {
                for (tx_hash, result) in results {
                    if let Ok(state_changes) = result {
                        // Process state changes
                        // Note: This is simplified - real implementation would parse properly
                        info!("✅ Simulated {}: {} addresses affected", 
                              tx_hash, 
                              state_changes.as_object().map(|o| o.len()).unwrap_or(0));
                    }
                }
            }
            Err(e) => {
                warn!("Batch simulation failed: {}", e);
            }
        }
        
        batch.clear();
        signal_detector.log_stats_summary();
    }
}