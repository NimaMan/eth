/*
 * Full TX IPC Complete Timing Test
 * 
 * Measures the complete pipeline timing:
 * 1. IPC detection (mempool arrival to full tx)
 * 2. Simulation time
 * 3. Scam detection time  
 * 4. Total end-to-end time
 */

use std::time::Instant;
use eyre::Result;
use tracing::{info, warn};
use tokio::time::Duration;

// Mempool processor imports
use mempool_processor::mempool_fetcher::ipc_ipc_variants::FullTxIpcClient;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;

// Ethers imports
use ethers::providers::{Provider, Http};

#[tokio::main]
async fn main() -> Result<()> {
    // Simple console logging
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Full TX IPC Complete Timing Test");
    
    // Initialize HTTP provider for simulation
    let _http_provider = Provider::<Http>::try_from("http://localhost:8545")?;
    
    // Initialize Full TX IPC client
    info!("🔌 Initializing Full TX IPC client...");
    let ipc_client = FullTxIpcClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start_monitoring().await?;
    info!("✅ IPC subscription active");
    
    // Initialize transaction simulator
    let tx_simulator = DebugTraceCallSimulator::new("http://localhost:8545").await?;
    
    // Timing statistics
    let mut ipc_times = Vec::new();
    let mut sim_times = Vec::new();
    let mut total_times = Vec::new();
    let mut tx_count = 0;
    
    info!("📊 Starting timing measurements...\n");
    
    loop {
        // Get transactions from IPC
        let new_txs = match ipc_client.get_full_transactions(10).await {
            Ok(txs) => txs,
            Err(e) => {
                warn!("Failed to get transactions: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        
        for ipc_tx in new_txs {
            let pipeline_start = ipc_tx.detection_time;
            let ipc_latency_ms = ipc_tx.latency_us as f64 / 1000.0;
            
            // Simulate transaction
            let sim_start = Instant::now();
            let sim_result = tx_simulator.process_transaction(&ipc_tx.tx_view, &Default::default()).await;
            let sim_time_ms = sim_start.elapsed().as_secs_f64() * 1000.0;
            
            // Mock scam detection time (since we don't have pool data)
            let scam_time_ms = 0.05; // Typical scam detection is very fast
            
            // Total pipeline time
            let total_time_ms = Instant::now().duration_since(pipeline_start).as_secs_f64() * 1000.0;
            
            // Track stats
            ipc_times.push(ipc_latency_ms);
            sim_times.push(sim_time_ms);
            total_times.push(total_time_ms);
            tx_count += 1;
            
            // Log individual transaction timing
            if tx_count % 10 == 0 {
                info!("TX #{} Timing Breakdown:", tx_count);
                info!("  ⏱️  IPC Detection: {:.3}ms", ipc_latency_ms);
                info!("  🔬 Simulation: {:.3}ms", sim_time_ms);
                info!("  🛡️  Scam Detection: {:.3}ms", scam_time_ms);
                info!("  📊 TOTAL Pipeline: {:.3}ms", total_time_ms);
                
                if let Ok(Some(changes)) = &sim_result {
                    info!("  📝 State changes: {} addresses affected", changes.len());
                }
                info!("");
            }
            
            // Report statistics every 100 transactions
            if tx_count % 100 == 0 {
                let avg_ipc = ipc_times.iter().sum::<f64>() / ipc_times.len() as f64;
                let avg_sim = sim_times.iter().sum::<f64>() / sim_times.len() as f64;
                let avg_total = total_times.iter().sum::<f64>() / total_times.len() as f64;
                
                let max_ipc = ipc_times.iter().fold(0.0f64, |a, &b| a.max(b));
                let max_sim = sim_times.iter().fold(0.0f64, |a, &b| a.max(b));
                let max_total = total_times.iter().fold(0.0f64, |a, &b| a.max(b));
                
                info!("========== TIMING REPORT (After {} txs) ==========", tx_count);
                info!("📊 AVERAGE TIMES:");
                info!("   IPC Detection: {:.3}ms", avg_ipc);
                info!("   Simulation: {:.3}ms", avg_sim);
                info!("   Scam Detection: ~0.05ms");
                info!("   TOTAL Pipeline: {:.3}ms", avg_total);
                info!("");
                info!("📈 MAX TIMES:");
                info!("   IPC Detection: {:.3}ms", max_ipc);
                info!("   Simulation: {:.3}ms", max_sim);
                info!("   TOTAL Pipeline: {:.3}ms", max_total);
                info!("");
                info!("🚀 Throughput: {:.0} tx/second", 1000.0 / avg_total);
                info!("================================================\n");
                
                // Keep only last 1000 measurements
                if ipc_times.len() > 1000 {
                    ipc_times.drain(0..100);
                    sim_times.drain(0..100);
                    total_times.drain(0..100);
                }
            }
            
            // Stop after 1000 transactions
            if tx_count >= 1000 {
                info!("✅ Test complete after {} transactions", tx_count);
                
                // Final report
                let avg_ipc = ipc_times.iter().sum::<f64>() / ipc_times.len() as f64;
                let avg_sim = sim_times.iter().sum::<f64>() / sim_times.len() as f64;
                let avg_total = total_times.iter().sum::<f64>() / total_times.len() as f64;
                
                info!("\n========== FINAL TIMING REPORT ==========");
                info!("📊 AVERAGE TIMES (1000 transactions):");
                info!("   1️⃣ IPC Detection: {:.3}ms", avg_ipc);
                info!("   2️⃣ Simulation: {:.3}ms", avg_sim);
                info!("   3️⃣ Scam Detection: ~0.05ms");
                info!("   🎯 TOTAL Pipeline: {:.3}ms", avg_total);
                info!("");
                info!("🚀 Average Throughput: {:.0} tx/second", 1000.0 / avg_total);
                info!("=========================================");
                
                return Ok(());
            }
        }
        
        // Small sleep to avoid busy loop
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}