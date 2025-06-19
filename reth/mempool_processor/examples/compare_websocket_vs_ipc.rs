/// Compare WebSocket+HTTP vs IPC performance for fetching full transaction data
/// 
/// This example demonstrates the performance difference between:
/// 1. WebSocket (hash only) + HTTP RPC fetch = ~50-200ms per transaction
/// 2. IPC with full transaction data = ~10-20ms per transaction

use std::time::Instant;
use eyre::Result;
use tracing::{info, warn};
use ethers::providers::{Provider, Http};
use ethers::types::H256;

use mempool_processor::mempool_fetcher::{WebSocketClient, FullTxIpcClient};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(true)
        .init();
    
    info!("🚀 Comparing WebSocket+HTTP vs IPC Performance");
    
    // Configuration
    let eth_rpc_url = "http://localhost:8545";
    let eth_ws_url = "ws://localhost:8546";
    let eth_ipc_path = "/tmp/reth.ipc";
    let test_duration_secs = 30;
    
    // Test 1: WebSocket + HTTP approach
    info!("\n📊 Test 1: WebSocket (hash only) + HTTP RPC fetch");
    info!("Expected: ~50-200ms per transaction due to HTTP overhead");
    
    let ws_client = WebSocketClient::new(eth_ws_url, eth_rpc_url)?;
    ws_client.start_monitoring().await?;
    
    let http_provider = Provider::<Http>::try_from(eth_rpc_url)?;
    
    let mut ws_latencies = Vec::new();
    let test_start = Instant::now();
    
    while test_start.elapsed().as_secs() < test_duration_secs {
        let txs = ws_client.get_transactions(10).await?;
        
        for ws_tx in txs {
            let fetch_start = Instant::now();
            
            // Parse hash and fetch full transaction via HTTP
            if let Ok(tx_hash) = ws_tx.hash.parse::<H256>() {
                match http_provider.get_transaction(tx_hash).await {
                    Ok(Some(_tx)) => {
                        let latency = fetch_start.elapsed();
                        ws_latencies.push(latency.as_millis() as u64);
                        
                        if ws_latencies.len() <= 5 {
                            info!("  Transaction {} fetched in {}ms", 
                                  &ws_tx.hash[..10], latency.as_millis());
                        }
                    }
                    Ok(None) => warn!("Transaction not found: {}", &ws_tx.hash[..10]),
                    Err(e) => warn!("Failed to fetch transaction: {}", e),
                }
            }
            
            if ws_latencies.len() >= 50 {
                break;
            }
        }
        
        if ws_latencies.len() >= 50 {
            break;
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // Calculate WebSocket+HTTP statistics
    if !ws_latencies.is_empty() {
        ws_latencies.sort();
        let ws_avg = ws_latencies.iter().sum::<u64>() / ws_latencies.len() as u64;
        let ws_min = ws_latencies[0];
        let ws_p50 = ws_latencies[ws_latencies.len() / 2];
        let ws_p95 = ws_latencies[(ws_latencies.len() as f64 * 0.95) as usize];
        let ws_max = ws_latencies[ws_latencies.len() - 1];
        
        info!("\n📈 WebSocket + HTTP Results:");
        info!("  Transactions measured: {}", ws_latencies.len());
        info!("  Average latency: {}ms", ws_avg);
        info!("  Min latency: {}ms", ws_min);
        info!("  P50 (median): {}ms", ws_p50);
        info!("  P95: {}ms", ws_p95);
        info!("  Max latency: {}ms", ws_max);
    }
    
    // Test 2: IPC with full transaction data
    info!("\n📊 Test 2: IPC with full transaction data");
    info!("Expected: ~10-20ms per transaction (no HTTP overhead)");
    
    let ipc_client = FullTxIpcClient::new(Some(eth_ipc_path))?;
    ipc_client.start_monitoring().await?;
    
    // Give it a moment to connect
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    let mut ipc_latencies = Vec::new();
    let test_start = Instant::now();
    
    while test_start.elapsed().as_secs() < test_duration_secs {
        let txs = ipc_client.get_full_transactions(10).await?;
        
        for ipc_tx in txs {
            // IPC latency already includes full transaction data!
            ipc_latencies.push(ipc_tx.latency_us / 1000); // Convert to milliseconds
            
            if ipc_latencies.len() <= 5 {
                info!("  Transaction {} received in {}ms (includes full data!)", 
                      &ipc_tx.hash[..10], ipc_tx.latency_us / 1000);
            }
            
            if ipc_latencies.len() >= 50 {
                break;
            }
        }
        
        if ipc_latencies.len() >= 50 {
            break;
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // Calculate IPC statistics
    if !ipc_latencies.is_empty() {
        ipc_latencies.sort();
        let ipc_avg = ipc_latencies.iter().sum::<u64>() / ipc_latencies.len() as u64;
        let ipc_min = ipc_latencies[0];
        let ipc_p50 = ipc_latencies[ipc_latencies.len() / 2];
        let ipc_p95 = ipc_latencies[(ipc_latencies.len() as f64 * 0.95) as usize];
        let ipc_max = ipc_latencies[ipc_latencies.len() - 1];
        
        info!("\n📈 IPC Results:");
        info!("  Transactions measured: {}", ipc_latencies.len());
        info!("  Average latency: {}ms", ipc_avg);
        info!("  Min latency: {}ms", ipc_min);
        info!("  P50 (median): {}ms", ipc_p50);
        info!("  P95: {}ms", ipc_p95);
        info!("  Max latency: {}ms", ipc_max);
        
        // Comparison
        if !ws_latencies.is_empty() {
            let ws_avg = ws_latencies.iter().sum::<u64>() / ws_latencies.len() as u64;
            let improvement = ((ws_avg as f64 - ipc_avg as f64) / ws_avg as f64) * 100.0;
            
            info!("\n🎯 Performance Comparison:");
            info!("  WebSocket+HTTP average: {}ms", ws_avg);
            info!("  IPC average: {}ms", ipc_avg);
            info!("  Improvement: {:.1}% faster", improvement);
            info!("  Speedup: {:.1}x", ws_avg as f64 / ipc_avg as f64);
        }
    }
    
    // Get final IPC statistics
    let stats = ipc_client.get_stats().await;
    info!("\n📊 Overall IPC Statistics:");
    info!("  Total transactions: {}", stats.total_transactions);
    info!("  Sub-1ms: {:.1}%", (stats.sub_1ms_count as f64 / stats.total_transactions as f64) * 100.0);
    info!("  Sub-10ms: {:.1}%", (stats.sub_10ms_count as f64 / stats.total_transactions as f64) * 100.0);
    
    Ok(())
}