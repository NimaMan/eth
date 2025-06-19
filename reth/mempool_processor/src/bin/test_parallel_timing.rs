/*
 * Test Parallel Processing Timing
 * 
 * Simple test to verify parallel processing improves throughput
 */

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use eyre::Result;
use tracing::{info, warn};
use tokio::sync::mpsc;
use tokio::time::Duration;

use mempool_processor::mempool_fetcher::ipc_ipc_variants::FullTxIpcClient;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;
use mempool_processor::pool_subscriber::PoolSubscriber;

use ethers::providers::{Provider, Http};

/// Simple metrics
struct Metrics {
    processed: AtomicU64,
    simulation_time_us: AtomicU64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Testing Parallel Transaction Processing");
    
    // Initialize IPC client
    let ipc_client = Arc::new(FullTxIpcClient::new(Some("/tmp/reth.ipc"))?);
    ipc_client.start_monitoring().await?;
    info!("✅ IPC subscription active");
    
    // Initialize pool subscriber for context
    let mut pool_subscriber = PoolSubscriber::with_endpoint(0.01, "tcp://localhost:5557");
    let _pool_cache = pool_subscriber.get_pool_cache();
    
    tokio::spawn(async move {
        let _ = pool_subscriber.start_listening().await;
    });
    
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    let metrics = Arc::new(Metrics {
        processed: AtomicU64::new(0),
        simulation_time_us: AtomicU64::new(0),
    });
    
    // Create channels for workers
    let (tx1, mut rx1) = mpsc::channel::<mempool_processor::mempool_fetcher::ipc_ipc_variants::FullIpcTransaction>(100);
    let (tx2, mut rx2) = mpsc::channel::<mempool_processor::mempool_fetcher::ipc_ipc_variants::FullIpcTransaction>(100);
    
    // Worker 1
    let metrics1 = metrics.clone();
    tokio::spawn(async move {
        let tx_simulator = DebugTraceCallSimulator::new("http://localhost:8545").await.unwrap();
        
        while let Some(ipc_tx) = rx1.recv().await {
            let sim_start = Instant::now();
            let _ = tx_simulator.process_transaction(&ipc_tx.tx_view, &Default::default()).await;
            let sim_time = sim_start.elapsed().as_micros() as u64;
            
            metrics1.simulation_time_us.fetch_add(sim_time, Ordering::Relaxed);
            metrics1.processed.fetch_add(1, Ordering::Relaxed);
        }
    });
    
    // Worker 2
    let metrics2 = metrics.clone();
    tokio::spawn(async move {
        let tx_simulator = DebugTraceCallSimulator::new("http://localhost:8545").await.unwrap();
        
        while let Some(ipc_tx) = rx2.recv().await {
            let sim_start = Instant::now();
            let _ = tx_simulator.process_transaction(&ipc_tx.tx_view, &Default::default()).await;
            let sim_time = sim_start.elapsed().as_micros() as u64;
            
            metrics2.simulation_time_us.fetch_add(sim_time, Ordering::Relaxed);
            metrics2.processed.fetch_add(1, Ordering::Relaxed);
        }
    });
    
    // Distribution loop
    let start_time = Instant::now();
    let mut current_worker = 0;
    
    loop {
        match ipc_client.get_full_transactions(20).await {
            Ok(txs) if !txs.is_empty() => {
                for tx in txs {
                    // Round-robin distribution
                    if current_worker == 0 {
                        let _ = tx1.try_send(tx);
                        current_worker = 1;
                    } else {
                        let _ = tx2.try_send(tx);
                        current_worker = 0;
                    }
                }
                
                let total = metrics.processed.load(Ordering::Relaxed);
                if total > 0 && total % 100 == 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let rate = total as f64 / elapsed;
                    let avg_sim_us = metrics.simulation_time_us.load(Ordering::Relaxed) / total;
                    
                    info!("📊 Processed: {} | Rate: {:.1} tx/s | Avg sim: {:.1}ms",
                          total, rate, avg_sim_us as f64 / 1000.0);
                }
                
                if total >= 1000 {
                    info!("✅ Test complete!");
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let rate = total as f64 / elapsed;
                    info!("Final rate: {:.1} tx/s with 2 workers", rate);
                    break;
                }
            }
            Ok(_) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => {
                warn!("Failed to get transactions: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    
    Ok(())
}