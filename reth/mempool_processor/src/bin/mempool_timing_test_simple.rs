/*
 * Simple Mempool Timing Test
 * 
 * Logs full pipeline timing: IPC fetch + simulation
 */

// use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error};
use tokio::time;

use mempool_processor::mempool_fetcher::ipc_ipc_variants::{FullTxIpcClient, FullIpcTransaction};
use mempool_processor::mempool_fetcher::TransactionView;
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;

use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    #[arg(long, default_value = "0.01")]
    eth_threshold: f64,
}

fn convert_full_tx_to_transaction_view(ipc_tx: &FullIpcTransaction) -> TransactionView {
    let tx = &ipc_tx.transaction;
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Starting Simple Mempool Timing Test");
    
    // Check RPC connection
    let http_provider = Arc::new(Provider::<Http>::try_from(&args.eth_rpc_url)?);
    let latest_block = http_provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    info!("📦 Current block: #{}", latest_block.number.unwrap_or_default());
    
    // Initialize IPC client
    info!("🔌 Initializing IPC client...");
    let ipc_client = Arc::new(FullTxIpcClient::new(Some(&args.ipc_path))?);
    ipc_client.start_monitoring().await?;
    info!("✅ IPC subscription active");
    
    // Initialize pool subscriber
    info!("🔗 Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber failed: {}", e);
        }
    });
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("📊 Monitoring {} pools", pool_cache_clone.get_pool_count());
    
    // Initialize transaction simulator
    info!("🔧 Initializing transaction simulator...");
    let tx_simulator = DebugTraceCallSimulator::new(&args.eth_rpc_url).await?;
    info!("✅ Transaction simulator ready");
    
    // Timing tracking
    let mut total_processed = 0u64;
    let mut ipc_times_ms: Vec<f64> = Vec::new();
    let mut sim_times_ms: Vec<f64> = Vec::new();
    let mut total_times_ms: Vec<f64> = Vec::new();
    let mut pools_affected_count = 0u64;
    
    info!("🔄 Starting main processing loop...");
    info!("{}", "=".repeat(60));
    
    loop {
        // Get transactions
        let new_txs = match ipc_client.get_full_transactions(10).await {
            Ok(txs) => txs,
            Err(e) => {
                warn!("Failed to get transactions: {}", e);
                time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        
        if new_txs.is_empty() {
            time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        for ipc_tx in new_txs {
            let pipeline_start = Instant::now();
            
            // IPC timing
            let ipc_latency_ms = ipc_tx.latency_us as f64 / 1000.0;
            
            // Convert transaction
            let tx_view = convert_full_tx_to_transaction_view(&ipc_tx);
            
            // Simulate
            let sim_start = Instant::now();
            match tx_simulator.process_transaction(&tx_view, &Default::default()).await {
                Ok(Some(state_changes)) => {
                    let sim_elapsed = sim_start.elapsed().as_secs_f64() * 1000.0;
                    
                    // Check for pool affects
                    let mut pools_affected = 0;
                    for (address_str, changes) in &state_changes {
                        if pool_cache_clone.get_pool(address_str).is_some() {
                            let eth_delta = if changes.eth_net_change.is_negative {
                                -(changes.eth_net_change.absolute_value.to_string()
                                    .parse::<u128>()
                                    .unwrap_or(0) as f64 / 1e18)
                            } else {
                                changes.eth_net_change.absolute_value.to_string()
                                    .parse::<u128>()
                                    .unwrap_or(0) as f64 / 1e18
                            };
                            
                            if eth_delta.abs() > 0.001 {
                                pools_affected += 1;
                            }
                        }
                    }
                    
                    if pools_affected > 0 {
                        pools_affected_count += 1;
                    }
                    
                    // Total timing
                    let total_elapsed = pipeline_start.elapsed().as_secs_f64() * 1000.0;
                    
                    // Record times
                    ipc_times_ms.push(ipc_latency_ms);
                    sim_times_ms.push(sim_elapsed);
                    total_times_ms.push(total_elapsed);
                    
                    // Keep only last 1000
                    if ipc_times_ms.len() > 1000 {
                        ipc_times_ms.remove(0);
                        sim_times_ms.remove(0);
                        total_times_ms.remove(0);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Simulation error: {}", e);
                }
            }
            
            total_processed += 1;
            
            // Report every 100 transactions
            if total_processed % 100 == 0 {
                let avg_ipc = ipc_times_ms.iter().sum::<f64>() / ipc_times_ms.len() as f64;
                let avg_sim = sim_times_ms.iter().sum::<f64>() / sim_times_ms.len() as f64;
                let avg_total = total_times_ms.iter().sum::<f64>() / total_times_ms.len() as f64;
                
                let max_ipc = ipc_times_ms.iter().cloned().fold(f64::MIN, f64::max);
                let max_sim = sim_times_ms.iter().cloned().fold(f64::MIN, f64::max);
                let max_total = total_times_ms.iter().cloned().fold(f64::MIN, f64::max);
                
                info!("⚡ TIMING REPORT (last {} transactions):", total_processed);
                info!("   📡 IPC Detection:   avg={:.2}ms  max={:.2}ms", avg_ipc, max_ipc);
                info!("   🔬 Simulation:      avg={:.2}ms  max={:.2}ms", avg_sim, max_sim);
                info!("   📊 Total Pipeline:  avg={:.2}ms  max={:.2}ms", avg_total, max_total);
                info!("   🎯 Pools Affected:  {} ({:.1}%)", 
                      pools_affected_count, 
                      (pools_affected_count as f64 / total_processed as f64 * 100.0));
                info!("   🚀 Throughput:      {:.1} tx/sec", 
                      1000.0 / avg_total);
                info!("{}", "=".repeat(60));
            }
        }
    }
}