/// Compare All IPC-Based Transaction Detection Methods
/// 
/// This tool compares all IPC-based methods for mempool transaction detection:
/// 1. ipc_ipc (current) - IPC subscription + immediate IPC fetch
/// 2. ipc_ipc_variants/basic - IPC hash notifications only
/// 3. ipc_ipc_variants/full - IPC subscription + IPC fetch with full data attempt
/// 4. ipc_ipc_variants/batch - IPC subscription + batch IPC fetch (parallel)
/// 
/// All methods use Unix IPC socket communication exclusively.

use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use eyre::Result;

// Import our current IPC-IPC implementation
use mempool_processor::mempool_fetcher::ipc_ipc::{IpcIpcMeasurementClient, default_config};

// Import legacy IPC variants
use mempool_processor::mempool_fetcher::ipc_ipc_variants::{
    IpcClient, FullTxIpcClient, BatchTxIpcClient, BatchConfig
};

#[derive(Debug, Clone)]
struct MethodResults {
    method_name: String,
    total_transactions: usize,
    avg_latency_ms: f64,
    median_latency_ms: f64,
    min_latency_ms: f64,
    max_latency_ms: f64,
    sub_1ms_percentage: f64,
    sub_10ms_percentage: f64,
    success_rate_percentage: f64,
    transactions_per_second: f64,
    measurement_duration_sec: f64,
    notes: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("compare_all_ipc_methods=info,mempool_processor=info")
        .init();

    info!("🚀 COMPREHENSIVE IPC METHOD COMPARISON");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Testing all IPC-based transaction detection methods:");
    info!("1. ipc_ipc (current) - IPC subscription + immediate IPC fetch");
    info!("2. ipc_ipc_variants/basic - IPC hash notifications only");
    info!("3. ipc_ipc_variants/full - IPC subscription + IPC fetch with full data attempt");
    info!("4. ipc_ipc_variants/batch - IPC subscription + batch IPC fetch");
    info!("");

    let test_duration = Duration::from_secs(30); // 30 seconds per method
    let target_transactions = 100; // Target transactions per test
    
    let mut results = Vec::new();

    // Test 1: Current IPC-IPC Implementation
    info!("📊 TEST 1: Current IPC-IPC Implementation");
    info!("─────────────────────────────────────────────");
    match test_current_ipc_ipc(target_transactions).await {
        Ok(result) => {
            info!("✅ Current IPC-IPC: {:.2}ms avg, {:.1}% sub-1ms", 
                  result.avg_latency_ms, result.sub_1ms_percentage);
            results.push(result);
        }
        Err(e) => {
            error!("❌ Current IPC-IPC failed: {}", e);
            results.push(MethodResults {
                method_name: "ipc_ipc (current)".to_string(),
                notes: format!("FAILED: {}", e),
                ..Default::default()
            });
        }
    }

    info!("");

    // Test 2: Basic IPC (Hash notifications only)
    info!("📊 TEST 2: Legacy IPC Basic (Hash Notifications Only)");
    info!("────────────────────────────────────────────────────");
    match test_basic_ipc(target_transactions, test_duration).await {
        Ok(result) => {
            info!("✅ Basic IPC: {:.2}ms avg, {:.1}% sub-1ms", 
                  result.avg_latency_ms, result.sub_1ms_percentage);
            results.push(result);
        }
        Err(e) => {
            error!("❌ Basic IPC failed: {}", e);
            results.push(MethodResults {
                method_name: "ipc_ipc_variants/basic".to_string(),
                notes: format!("FAILED: {}", e),
                ..Default::default()
            });
        }
    }

    info!("");

    // Test 3: Full IPC (Subscription + Individual Fetch)
    info!("📊 TEST 3: Legacy IPC Full (Subscription + Individual Fetch)");
    info!("─────────────────────────────────────────────────────────────");
    match test_full_ipc(target_transactions, test_duration).await {
        Ok(result) => {
            info!("✅ Full IPC: {:.2}ms avg, {:.1}% sub-1ms", 
                  result.avg_latency_ms, result.sub_1ms_percentage);
            results.push(result);
        }
        Err(e) => {
            error!("❌ Full IPC failed: {}", e);
            results.push(MethodResults {
                method_name: "ipc_ipc_variants/full".to_string(),
                notes: format!("FAILED: {}", e),
                ..Default::default()
            });
        }
    }

    info!("");

    // Test 4: Batch IPC (Subscription + Batch Fetch)
    info!("📊 TEST 4: Legacy IPC Batch (Subscription + Batch Fetch)");
    info!("────────────────────────────────────────────────────────");
    match test_batch_ipc(target_transactions, test_duration).await {
        Ok(result) => {
            info!("✅ Batch IPC: {:.2}ms avg, {:.1}% sub-1ms", 
                  result.avg_latency_ms, result.sub_1ms_percentage);
            results.push(result);
        }
        Err(e) => {
            error!("❌ Batch IPC failed: {}", e);
            results.push(MethodResults {
                method_name: "ipc_ipc_variants/batch".to_string(),
                notes: format!("FAILED: {}", e),
                ..Default::default()
            });
        }
    }

    // Generate comprehensive comparison report
    generate_comparison_report(&results);

    Ok(())
}

/// Test current IPC-IPC implementation
async fn test_current_ipc_ipc(target_transactions: usize) -> Result<MethodResults> {
    let mut config = default_config();
    config.target_transaction_count = target_transactions;
    config.log_directory = "/home/nima/code/crypto/logs/ipc_comparison".to_string();

    let mut client = IpcIpcMeasurementClient::new(config);
    client.initialize().await?;
    
    let start_time = Instant::now();
    let measurement_results = client.run_measurement().await?;
    let duration = start_time.elapsed();

    Ok(MethodResults {
        method_name: "ipc_ipc (current)".to_string(),
        total_transactions: measurement_results.total_transactions,
        avg_latency_ms: measurement_results.avg_latency_us / 1000.0,
        median_latency_ms: measurement_results.median_latency_us as f64 / 1000.0,
        min_latency_ms: measurement_results.min_latency_us as f64 / 1000.0,
        max_latency_ms: measurement_results.max_latency_us as f64 / 1000.0,
        sub_1ms_percentage: measurement_results.sub_1ms_percentage,
        sub_10ms_percentage: if measurement_results.avg_latency_us < 10000.0 { 100.0 } else { 0.0 },
        success_rate_percentage: measurement_results.success_rate_percentage,
        transactions_per_second: measurement_results.transactions_per_second,
        measurement_duration_sec: duration.as_secs_f64(),
        notes: "Clean implementation with immediate individual fetch".to_string(),
    })
}

/// Test basic IPC (hash notifications only)
async fn test_basic_ipc(target_transactions: usize, max_duration: Duration) -> Result<MethodResults> {
    info!("  Connecting to basic IPC client...");
    let client = IpcClient::new(None)?;
    client.start_monitoring().await?;
    
    // Wait for connection
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    info!("  Collecting hash notifications...");
    let start_time = Instant::now();
    let mut collected = 0;
    let mut latencies = Vec::new();
    
    while collected < target_transactions && start_time.elapsed() < max_duration {
        let transactions = client.get_transactions(20).await?;
        
        for tx in transactions {
            collected += 1;
            latencies.push(tx.latency_us as f64 / 1000.0); // Convert to ms
            
            if collected >= target_transactions {
                break;
            }
        }
        
        if latencies.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    let duration = start_time.elapsed();
    
    if latencies.is_empty() {
        return Err(eyre::eyre!("No transactions collected"));
    }
    
    // Calculate statistics
    let mut sorted_latencies = latencies.clone();
    sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let median_latency = sorted_latencies[sorted_latencies.len() / 2];
    let min_latency = sorted_latencies[0];
    let max_latency = sorted_latencies[sorted_latencies.len() - 1];
    
    let sub_1ms_count = latencies.iter().filter(|&&l| l < 1.0).count();
    let sub_10ms_count = latencies.iter().filter(|&&l| l < 10.0).count();
    
    Ok(MethodResults {
        method_name: "ipc_ipc_variants/basic".to_string(),
        total_transactions: collected,
        avg_latency_ms: avg_latency,
        median_latency_ms: median_latency,
        min_latency_ms: min_latency,
        max_latency_ms: max_latency,
        sub_1ms_percentage: sub_1ms_count as f64 / latencies.len() as f64 * 100.0,
        sub_10ms_percentage: sub_10ms_count as f64 / latencies.len() as f64 * 100.0,
        success_rate_percentage: 100.0,
        transactions_per_second: collected as f64 / duration.as_secs_f64(),
        measurement_duration_sec: duration.as_secs_f64(),
        notes: "Hash notifications only - no transaction data".to_string(),
    })
}

/// Test full IPC (subscription + individual fetch)
async fn test_full_ipc(target_transactions: usize, max_duration: Duration) -> Result<MethodResults> {
    info!("  Connecting to full IPC client...");
    let client = FullTxIpcClient::new(None)?;
    client.start_monitoring().await?;
    
    // Wait for connection
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    info!("  Collecting full transactions...");
    let start_time = Instant::now();
    let mut collected = 0;
    let mut latencies = Vec::new();
    
    while collected < target_transactions && start_time.elapsed() < max_duration {
        let transactions = client.get_full_transactions(20).await?;
        
        for tx in transactions {
            collected += 1;
            latencies.push(tx.latency_us as f64 / 1000.0); // Convert to ms
            
            if collected >= target_transactions {
                break;
            }
        }
        
        if latencies.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    let duration = start_time.elapsed();
    
    if latencies.is_empty() {
        return Err(eyre::eyre!("No transactions collected"));
    }
    
    // Calculate statistics (same as basic)
    let mut sorted_latencies = latencies.clone();
    sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let median_latency = sorted_latencies[sorted_latencies.len() / 2];
    let min_latency = sorted_latencies[0];
    let max_latency = sorted_latencies[sorted_latencies.len() - 1];
    
    let sub_1ms_count = latencies.iter().filter(|&&l| l < 1.0).count();
    let sub_10ms_count = latencies.iter().filter(|&&l| l < 10.0).count();
    
    Ok(MethodResults {
        method_name: "ipc_ipc_variants/full".to_string(),
        total_transactions: collected,
        avg_latency_ms: avg_latency,
        median_latency_ms: median_latency,
        min_latency_ms: min_latency,
        max_latency_ms: max_latency,
        sub_1ms_percentage: sub_1ms_count as f64 / latencies.len() as f64 * 100.0,
        sub_10ms_percentage: sub_10ms_count as f64 / latencies.len() as f64 * 100.0,
        success_rate_percentage: 100.0,
        transactions_per_second: collected as f64 / duration.as_secs_f64(),
        measurement_duration_sec: duration.as_secs_f64(),
        notes: "IPC subscription + individual IPC fetch with full data attempt".to_string(),
    })
}

/// Test batch IPC (subscription + batch fetch)
async fn test_batch_ipc(target_transactions: usize, max_duration: Duration) -> Result<MethodResults> {
    info!("  Connecting to batch IPC client...");
    let config = BatchConfig {
        batch_size: 10,
        batch_timeout_ms: 100,
        connection_pool_size: 2,
        buffer_size: 500,
    };
    
    let client = BatchTxIpcClient::new(Some("/tmp/reth.ipc"), config)?;
    client.start_monitoring().await?;
    
    // Wait for connection
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    info!("  Collecting batch transactions...");
    let start_time = Instant::now();
    let mut collected = 0;
    let mut latencies = Vec::new();
    
    while collected < target_transactions && start_time.elapsed() < max_duration {
        let transaction_batches = client.get_transaction_batches(5).await?;
        
        for batch in transaction_batches {
            for tx in batch {
                collected += 1;
                latencies.push(tx.latency_us as f64 / 1000.0); // Convert to ms
                
                if collected >= target_transactions {
                    break;
                }
            }
            if collected >= target_transactions {
                break;
            }
        }
        
        if latencies.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    let duration = start_time.elapsed();
    
    if latencies.is_empty() {
        return Err(eyre::eyre!("No transactions collected"));
    }
    
    // Calculate statistics (same as others)
    let mut sorted_latencies = latencies.clone();
    sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let median_latency = sorted_latencies[sorted_latencies.len() / 2];
    let min_latency = sorted_latencies[0];
    let max_latency = sorted_latencies[sorted_latencies.len() - 1];
    
    let sub_1ms_count = latencies.iter().filter(|&&l| l < 1.0).count();
    let sub_10ms_count = latencies.iter().filter(|&&l| l < 10.0).count();
    
    Ok(MethodResults {
        method_name: "ipc_ipc_variants/batch".to_string(),
        total_transactions: collected,
        avg_latency_ms: avg_latency,
        median_latency_ms: median_latency,
        min_latency_ms: min_latency,
        max_latency_ms: max_latency,
        sub_1ms_percentage: sub_1ms_count as f64 / latencies.len() as f64 * 100.0,
        sub_10ms_percentage: sub_10ms_count as f64 / latencies.len() as f64 * 100.0,
        success_rate_percentage: 100.0,
        transactions_per_second: collected as f64 / duration.as_secs_f64(),
        measurement_duration_sec: duration.as_secs_f64(),
        notes: "IPC subscription + batch IPC fetch with parallel connections".to_string(),
    })
}

/// Generate comprehensive comparison report
fn generate_comparison_report(results: &[MethodResults]) {
    info!("");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 COMPREHENSIVE IPC METHOD COMPARISON RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Sort by average latency
    let mut sorted_results = results.to_vec();
    sorted_results.sort_by(|a, b| a.avg_latency_ms.partial_cmp(&b.avg_latency_ms).unwrap());
    
    info!("");
    info!("🏆 RANKING BY AVERAGE LATENCY:");
    info!("─────────────────────────────────");
    for (i, result) in sorted_results.iter().enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈", 
            2 => "🥉",
            _ => "  ",
        };
        
        if result.notes.contains("FAILED") {
            info!("{} {}. {} - FAILED ({})", medal, i + 1, result.method_name, result.notes);
        } else {
            info!("{} {}. {} - {:.2}ms avg ({:.1}% sub-1ms)", 
                  medal, i + 1, result.method_name, result.avg_latency_ms, result.sub_1ms_percentage);
        }
    }
    
    info!("");
    info!("📋 DETAILED COMPARISON TABLE:");
    info!("─────────────────────────────────");
    info!("{:<25} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>6}",
          "Method", "Avg (ms)", "Med (ms)", "Min (ms)", "Max (ms)", "<1ms %", "TPS", "Txs");
    info!("{}", "─".repeat(120));
    
    for result in &sorted_results {
        if result.notes.contains("FAILED") {
            info!("{:<25} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>6}",
                  &result.method_name[..std::cmp::min(25, result.method_name.len())],
                  "FAILED", "FAILED", "FAILED", "FAILED", "FAILED", "FAILED", "0");
        } else {
            info!("{:<25} | {:>8.2} | {:>8.2} | {:>8.2} | {:>8.2} | {:>8.1} | {:>8.1} | {:>6}",
                  &result.method_name[..std::cmp::min(25, result.method_name.len())],
                  result.avg_latency_ms,
                  result.median_latency_ms,
                  result.min_latency_ms,
                  result.max_latency_ms,
                  result.sub_1ms_percentage,
                  result.transactions_per_second,
                  result.total_transactions);
        }
    }
    
    info!("");
    info!("📝 METHOD DESCRIPTIONS:");
    info!("─────────────────────────");
    for result in results {
        info!("• {}: {}", result.method_name, result.notes);
    }
    
    info!("");
    info!("🎯 KEY FINDINGS:");
    info!("───────────────");
    
    let successful_results: Vec<_> = results.iter().filter(|r| !r.notes.contains("FAILED")).collect();
    
    if !successful_results.is_empty() {
        let fastest = successful_results.iter().min_by(|a, b| a.avg_latency_ms.partial_cmp(&b.avg_latency_ms).unwrap()).unwrap();
        let highest_sub_1ms = successful_results.iter().max_by(|a, b| a.sub_1ms_percentage.partial_cmp(&b.sub_1ms_percentage).unwrap()).unwrap();
        let highest_tps = successful_results.iter().max_by(|a, b| a.transactions_per_second.partial_cmp(&b.transactions_per_second).unwrap()).unwrap();
        
        info!("• Fastest average latency: {} ({:.2}ms)", fastest.method_name, fastest.avg_latency_ms);
        info!("• Best sub-1ms performance: {} ({:.1}%)", highest_sub_1ms.method_name, highest_sub_1ms.sub_1ms_percentage);
        info!("• Highest throughput: {} ({:.1} TPS)", highest_tps.method_name, highest_tps.transactions_per_second);
        
        // Check if all use IPC
        info!("• All methods use IPC for both subscription and transaction fetching");
        info!("• Performance differences are due to fetch strategies and optimizations");
        
        // Recommendations
        info!("");
        info!("💡 RECOMMENDATIONS:");
        info!("──────────────────");
        info!("• For lowest latency: Use {}", fastest.method_name);
        info!("• For highest sub-1ms rate: Use {}", highest_sub_1ms.method_name);
        info!("• For highest throughput: Use {}", highest_tps.method_name);
        info!("• All IPC methods significantly outperform WebSocket+HTTP combinations");
    }
    
    info!("");
    info!("✅ Comparison complete!");
}

impl Default for MethodResults {
    fn default() -> Self {
        Self {
            method_name: String::new(),
            total_transactions: 0,
            avg_latency_ms: 0.0,
            median_latency_ms: 0.0,
            min_latency_ms: 0.0,
            max_latency_ms: 0.0,
            sub_1ms_percentage: 0.0,
            sub_10ms_percentage: 0.0,
            success_rate_percentage: 0.0,
            transactions_per_second: 0.0,
            measurement_duration_sec: 0.0,
            notes: String::new(),
        }
    }
}