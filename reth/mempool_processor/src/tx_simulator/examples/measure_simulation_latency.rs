/// Measure Transaction Simulation Latency
/// 
/// This example connects to the mempool, fetches new transactions,
/// simulates them, and measures the total time from:
/// - Transaction arrival at Reth node
/// - Detection via WebSocket
/// - Full simulation completion
///
/// Usage:
///   cargo run --example measure_simulation_latency
///
/// Expected output:
///   - Detection latency: ~30ms (from Reth to our detection)
///   - Simulation latency: ~50-200ms (depending on transaction complexity)
///   - Total latency: ~80-250ms (from Reth arrival to simulation complete)

use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use eyre::Result;
use tracing::{info, debug, error, warn};
use mempool_processor::mempool_fetcher::{WebSocketClient, TransactionView};
use mempool_processor::tx_simulator::SimulatorWrapper;
use revm_primitives::hardfork::SpecId;
use revm_context::BlockEnv;

#[derive(Debug, Clone)]
struct SimulationMetrics {
    tx_hash: String,
    detection_latency_ms: f64,
    simulation_start: Instant,
    simulation_end: Option<Instant>,
    simulation_latency_ms: Option<f64>,
    total_latency_ms: Option<f64>,
    simulation_success: bool,
    error_message: Option<String>,
}

#[derive(Debug, Default)]
struct PerformanceStats {
    total_transactions: u64,
    successful_simulations: u64,
    failed_simulations: u64,
    avg_detection_latency_ms: f64,
    avg_simulation_latency_ms: f64,
    avg_total_latency_ms: f64,
    min_total_latency_ms: f64,
    max_total_latency_ms: f64,
    transactions_under_100ms: u64,
    transactions_under_250ms: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,measure_simulation_latency=info")
        .init();

    info!("🚀 Transaction Simulation Latency Measurement");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Configuration
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    let measurement_duration = Duration::from_secs(60); // Measure for 1 minute
    
    info!("Configuration:");
    info!("  WebSocket URL: {}", ws_url);
    info!("  HTTP URL: {}", http_url);
    info!("  Measurement duration: {:?}", measurement_duration);
    
    // Connect to providers
    info!("\n📡 Connecting to Ethereum node...");
    let http_provider = Provider::<Http>::try_from(http_url)?;
    let http_provider = Arc::new(http_provider);
    
    // Verify connection
    let block_number = http_provider.get_block_number().await?;
    info!("✅ Connected! Current block: {}", block_number);
    
    // Get current block for simulation context
    let latest_block = http_provider.get_block(ethers::types::BlockNumber::Latest).await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
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
    
    // Initialize transaction simulator
    info!("\n🔧 Initializing transaction simulator...");
    let simulator = SimulatorWrapper::new_revm(http_url, 1, SpecId::CANCUN).await?;
    let simulator = Arc::new(simulator);
    
    // Initialize WebSocket client for mempool monitoring
    info!("\n📊 Starting mempool monitoring...");
    let ws_client = WebSocketClient::new(ws_url, http_url)?;
    ws_client.start_monitoring().await?;
    
    // Performance tracking
    let stats = Arc::new(RwLock::new(PerformanceStats::default()));
    let all_metrics = Arc::new(RwLock::new(Vec::<SimulationMetrics>::new()));
    
    // Start measurement
    let start_time = Instant::now();
    info!("\n⏱️  Starting measurement at block {}...\n", block_number);
    
    // Process transactions
    let mut transaction_count = 0;
    
    while start_time.elapsed() < measurement_duration {
        // Get batch of transactions (with timeout to check duration)
        match tokio::time::timeout(
            Duration::from_millis(100),
            ws_client.get_transactions(10)
        ).await {
            Ok(Ok(transactions)) => {
                for ws_tx in transactions {
                    transaction_count += 1;
                    let tx_hash = ws_tx.hash.clone();
                    let detection_latency_ms = ws_tx.latency_ms;
                    
                    // Start simulation
                    let simulation_start = Instant::now();
                    let mut metrics = SimulationMetrics {
                        tx_hash: tx_hash.clone(),
                        detection_latency_ms,
                        simulation_start,
                        simulation_end: None,
                        simulation_latency_ms: None,
                        total_latency_ms: None,
                        simulation_success: false,
                        error_message: None,
                    };
                    
                    // Fetch full transaction data
                    let tx_hash_h256 = match tx_hash.parse::<H256>() {
                        Ok(h) => h,
                        Err(e) => {
                            warn!("Invalid transaction hash {}: {}", tx_hash, e);
                            continue;
                        }
                    };
                    
                    match http_provider.get_transaction(tx_hash_h256).await {
                        Ok(Some(tx)) => {
                            // Convert to TransactionView for simulator
                            let tx_view = TransactionView {
                                hash: tx.hash.as_bytes().to_vec(),
                                from: tx.from.as_bytes().to_vec(),
                                to: tx.to.map(|addr| addr.as_bytes().to_vec()),
                                value: tx.value,
                                gas_price: tx.gas_price,
                                gas_limit: Some(tx.gas),
                                nonce: Some(U256::from(tx.nonce.as_u64())),
                                input_data: Some(tx.input.to_vec()),
                            };
                            
                            // Simulate transaction
                            match simulator.process_transaction(&tx_view, &block_env).await {
                                Ok(Some(state_changes)) => {
                                    let simulation_end = Instant::now();
                                    let simulation_latency = simulation_end.duration_since(simulation_start);
                                    let simulation_latency_ms = simulation_latency.as_secs_f64() * 1000.0;
                                    let total_latency_ms = detection_latency_ms + simulation_latency_ms;
                                    
                                    metrics.simulation_end = Some(simulation_end);
                                    metrics.simulation_latency_ms = Some(simulation_latency_ms);
                                    metrics.total_latency_ms = Some(total_latency_ms);
                                    metrics.simulation_success = true;
                                    
                                    // Log interesting transactions
                                    if transaction_count <= 5 || total_latency_ms < 100.0 {
                                        info!("TX #{}: {}", transaction_count, tx_hash);
                                        info!("  Detection latency: {:.2}ms", detection_latency_ms);
                                        info!("  Simulation latency: {:.2}ms", simulation_latency_ms);
                                        info!("  Total latency: {:.2}ms", total_latency_ms);
                                        info!("  State changes: {} accounts affected", state_changes.len());
                                    }
                                    
                                    // Update stats
                                    let mut stats_guard = stats.write().await;
                                    stats_guard.total_transactions += 1;
                                    stats_guard.successful_simulations += 1;
                                    
                                    // Update averages
                                    let n = stats_guard.successful_simulations as f64;
                                    stats_guard.avg_detection_latency_ms = 
                                        (stats_guard.avg_detection_latency_ms * (n - 1.0) + detection_latency_ms) / n;
                                    stats_guard.avg_simulation_latency_ms = 
                                        (stats_guard.avg_simulation_latency_ms * (n - 1.0) + simulation_latency_ms) / n;
                                    stats_guard.avg_total_latency_ms = 
                                        (stats_guard.avg_total_latency_ms * (n - 1.0) + total_latency_ms) / n;
                                    
                                    // Update min/max
                                    if stats_guard.min_total_latency_ms == 0.0 || total_latency_ms < stats_guard.min_total_latency_ms {
                                        stats_guard.min_total_latency_ms = total_latency_ms;
                                    }
                                    if total_latency_ms > stats_guard.max_total_latency_ms {
                                        stats_guard.max_total_latency_ms = total_latency_ms;
                                    }
                                    
                                    // Update distribution
                                    if total_latency_ms < 100.0 {
                                        stats_guard.transactions_under_100ms += 1;
                                    }
                                    if total_latency_ms < 250.0 {
                                        stats_guard.transactions_under_250ms += 1;
                                    }
                                }
                                Ok(None) => {
                                    // Transaction didn't produce state changes (e.g., failed transaction)
                                    metrics.simulation_success = true;
                                    metrics.error_message = Some("No state changes".to_string());
                                    debug!("Transaction {} produced no state changes", tx_hash);
                                }
                                Err(e) => {
                                    metrics.simulation_success = false;
                                    metrics.error_message = Some(e.to_string());
                                    
                                    let mut stats_guard = stats.write().await;
                                    stats_guard.total_transactions += 1;
                                    stats_guard.failed_simulations += 1;
                                    
                                    debug!("Simulation failed for {}: {}", tx_hash, e);
                                }
                            }
                        }
                        Ok(None) => {
                            warn!("Transaction {} not found", tx_hash);
                        }
                        Err(e) => {
                            error!("Error fetching transaction {}: {}", tx_hash, e);
                        }
                    }
                    
                    // Store metrics
                    all_metrics.write().await.push(metrics);
                    
                    // Progress update
                    if transaction_count % 10 == 0 {
                        let elapsed = start_time.elapsed().as_secs();
                        let stats_guard = stats.read().await;
                        info!("\nProgress: {}s elapsed, {} transactions processed", elapsed, transaction_count);
                        info!("  Success rate: {:.1}%", 
                              stats_guard.successful_simulations as f64 / stats_guard.total_transactions as f64 * 100.0);
                        info!("  Avg total latency: {:.2}ms", stats_guard.avg_total_latency_ms);
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("Error getting transactions: {}", e);
            }
            Err(_) => {
                // Timeout - no transactions available, continue
            }
        }
    }
    
    // Display final results
    let final_stats = stats.read().await;
    let final_metrics = all_metrics.read().await;
    display_results(&*final_stats, &*final_metrics).await;
    
    Ok(())
}

async fn display_results(stats: &PerformanceStats, all_metrics: &[SimulationMetrics]) {
    println!("\n\n📊 FINAL SIMULATION LATENCY RESULTS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    println!("\n📈 TRANSACTION STATISTICS:");
    println!("  Total transactions: {}", stats.total_transactions);
    println!("  Successful simulations: {} ({:.1}%)", 
             stats.successful_simulations,
             stats.successful_simulations as f64 / stats.total_transactions as f64 * 100.0);
    println!("  Failed simulations: {}", stats.failed_simulations);
    
    println!("\n⏱️  LATENCY BREAKDOWN:");
    println!("  Average detection latency: {:.2}ms", stats.avg_detection_latency_ms);
    println!("  Average simulation latency: {:.2}ms", stats.avg_simulation_latency_ms);
    println!("  Average total latency: {:.2}ms", stats.avg_total_latency_ms);
    
    println!("\n📊 LATENCY DISTRIBUTION:");
    println!("  Minimum total latency: {:.2}ms", stats.min_total_latency_ms);
    println!("  Maximum total latency: {:.2}ms", stats.max_total_latency_ms);
    println!("  Transactions <100ms: {} ({:.1}%)", 
             stats.transactions_under_100ms,
             stats.transactions_under_100ms as f64 / stats.successful_simulations as f64 * 100.0);
    println!("  Transactions <250ms: {} ({:.1}%)", 
             stats.transactions_under_250ms,
             stats.transactions_under_250ms as f64 / stats.successful_simulations as f64 * 100.0);
    
    // Calculate percentiles from successful simulations
    let mut successful_latencies: Vec<f64> = all_metrics.iter()
        .filter(|m| m.simulation_success && m.total_latency_ms.is_some())
        .map(|m| m.total_latency_ms.unwrap())
        .collect();
    
    if !successful_latencies.is_empty() {
        successful_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let p50 = successful_latencies[successful_latencies.len() / 2];
        let p95 = successful_latencies[(successful_latencies.len() as f64 * 0.95) as usize];
        let p99 = successful_latencies[(successful_latencies.len() as f64 * 0.99) as usize];
        
        println!("\n📈 LATENCY PERCENTILES:");
        println!("  P50 (median): {:.2}ms", p50);
        println!("  P95: {:.2}ms", p95);
        println!("  P99: {:.2}ms", p99);
    }
    
    println!("\n✅ CONCLUSION:");
    println!("  From TX arrival at Reth → Simulation complete: {:.2}ms average", stats.avg_total_latency_ms);
    println!("  Detection accounts for: {:.1}% of total latency", 
             stats.avg_detection_latency_ms / stats.avg_total_latency_ms * 100.0);
    println!("  Simulation accounts for: {:.1}% of total latency", 
             stats.avg_simulation_latency_ms / stats.avg_total_latency_ms * 100.0);
}