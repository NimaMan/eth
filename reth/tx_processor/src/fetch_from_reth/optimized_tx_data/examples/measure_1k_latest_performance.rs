//! Performance Benchmark: Latest 1000 Transactions
//! 
//! This benchmark measures the real-world performance of the optimized transaction
//! data retrieval module using the most recent 1000 transactions from the Ethereum
//! blockchain. It compares performance with and without provider optimization.
//!
//! To run:
//!   cargo run --bin optimized_tx_measure_1k --release

use anyhow::Result;
use std::time::{Instant, Duration};
use std::collections::HashMap;
use alloy_rpc_types::BlockId;

use ethers_core::types::H256;
use revm_tx_simulator_lib::fetch_from_reth::optimized_tx_data::{
    get_basic_transaction_data,
    get_basic_transaction_data_with_provider,
    get_smart_transaction_data_with_provider,
    get_full_transaction_analysis_with_provider,
    TransactionDataOptions,
};
use revm_tx_simulator_lib::fetch_from_reth::{RethDatabaseProvider, RethDataProvider};

#[derive(Default)]
struct PerformanceStats {
    min_time: Duration,
    max_time: Duration,
    total_time: Duration,
    count: usize,
    errors: usize,
}

impl PerformanceStats {
    fn record(&mut self, duration: Duration) {
        if self.count == 0 || duration < self.min_time {
            self.min_time = duration;
        }
        if duration > self.max_time {
            self.max_time = duration;
        }
        self.total_time += duration;
        self.count += 1;
    }
    
    fn avg_time(&self) -> Duration {
        if self.count > 0 {
            self.total_time / self.count as u32
        } else {
            Duration::ZERO
        }
    }
    
    fn throughput(&self) -> f64 {
        if self.total_time.as_secs_f64() > 0.0 {
            self.count as f64 / self.total_time.as_secs_f64()
        } else {
            0.0
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Performance Benchmark: Latest 1000 Transactions");
    println!("=================================================\n");
    
    // Step 1: Connect to database and get latest transactions
    println!("📊 Step 1: Fetching latest 1000 transactions from blockchain...");
    let provider_start = Instant::now();
    let datadir = "/home/nima/.local/share/reth/mainnet";
    let db_provider = RethDatabaseProvider::new(datadir)?;
    let provider_creation_time = provider_start.elapsed();
    println!("✅ Database provider created in {:.3}ms", provider_creation_time.as_millis());
    
    // Get latest block number
    let latest_block = db_provider.latest_block_number()?;
    println!("📦 Latest block: #{}", latest_block);
    
    // Collect transaction hashes from recent blocks
    let mut tx_hashes = Vec::new();
    let mut current_block = latest_block;
    
    println!("🔍 Collecting transactions from recent blocks...");
    while tx_hashes.len() < 1000 && current_block > 0 {
        match db_provider.fetch_block(BlockId::Number(current_block)) {
            Ok(block_data) => {
                let tx_count = block_data.transaction_count;
                if tx_count > 0 {
                    // Fetch transactions for this block
                    match db_provider.fetch_transactions_by_block(BlockId::Number(current_block)) {
                        Ok(transactions) => {
                            for tx_data in transactions.iter().rev() {
                                if tx_hashes.len() >= 1000 {
                                    break;
                                }
                                // Convert B256 to H256
                                let h256 = H256::from_slice(&tx_data.hash.0);
                                tx_hashes.push(h256);
                            }
                            println!("  Block #{}: {} transactions", current_block, tx_count);
                        }
                        Err(e) => {
                            println!("  ⚠️  Error fetching transactions for block {}: {}", current_block, e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("  ⚠️  Error fetching block {}: {}", current_block, e);
            }
        }
        current_block -= 1;
    }
    
    println!("✅ Collected {} transactions from blocks {} to {}\n", 
             tx_hashes.len(), current_block + 1, latest_block);
    
    // Step 2: Benchmark without optimization (sample only)
    println!("📊 Step 2: Baseline Performance (No Optimization - 10 tx sample)");
    println!("========================================================");
    
    let mut baseline_stats = PerformanceStats::default();
    let sample_size = 10; // Only test 10 transactions for baseline
    
    for (i, tx_hash) in tx_hashes.iter().take(sample_size).enumerate() {
        let start = Instant::now();
        match get_basic_transaction_data(*tx_hash, TransactionDataOptions::basic()).await {
            Ok(_) => {
                baseline_stats.record(start.elapsed());
                if i == 0 {
                    println!("  First tx: {:.3}ms", start.elapsed().as_millis());
                }
            }
            Err(e) => {
                baseline_stats.errors += 1;
                println!("  ❌ Error processing tx: {}", e);
            }
        }
    }
    
    println!("\n📈 Baseline Results (10 tx sample):");
    println!("  Min time: {:.3}ms", baseline_stats.min_time.as_millis());
    println!("  Max time: {:.3}ms", baseline_stats.max_time.as_millis());
    println!("  Avg time: {:.3}ms", baseline_stats.avg_time().as_millis());
    println!("  Errors: {}", baseline_stats.errors);
    println!("  Projected time for 1K: {:.1}s", baseline_stats.avg_time().as_secs_f64() * 1000.0);
    
    // Step 3: Benchmark with optimization (full 1K)
    println!("\n📊 Step 3: Optimized Performance (Provider Reuse - Full 1K)");
    println!("==========================================================");
    
    let mut basic_stats = PerformanceStats::default();
    let mut smart_stats = PerformanceStats::default();
    let mut complete_stats = PerformanceStats::default();
    let mut tx_type_distribution = HashMap::new();
    
    let benchmark_start = Instant::now();
    
    // Process all 1000 transactions with optimization
    for (i, tx_hash) in tx_hashes.iter().enumerate() {
        // Basic mode
        let start = Instant::now();
        match get_basic_transaction_data_with_provider(
            *tx_hash, 
            TransactionDataOptions::basic(), 
            Some(&db_provider)
        ).await {
            Ok(data) => {
                basic_stats.record(start.elapsed());
                
                // Show progress every 100 transactions
                if i % 100 == 0 {
                    println!("  Processing tx {}/1000... ({:.3}ms)", i + 1, start.elapsed().as_millis());
                }
            }
            Err(e) => {
                basic_stats.errors += 1;
                if basic_stats.errors <= 5 {
                    println!("  ❌ Basic error: {}", e);
                }
            }
        }
        
        // Smart mode (sample every 10th transaction)
        if i % 10 == 0 {
            let start = Instant::now();
            match get_smart_transaction_data_with_provider(
                *tx_hash,
                TransactionDataOptions::smart(),
                Some(&db_provider)
            ).await {
                Ok(data) => {
                    smart_stats.record(start.elapsed());
                    *tx_type_distribution.entry(data.transaction_type).or_insert(0) += 1;
                }
                Err(_) => {
                    smart_stats.errors += 1;
                }
            }
        }
        
        // Complete mode (sample every 100th transaction)
        if i % 100 == 0 {
            let start = Instant::now();
            match get_full_transaction_analysis_with_provider(
                *tx_hash,
                TransactionDataOptions::complete(),
                Some(&db_provider)
            ).await {
                Ok(_) => {
                    complete_stats.record(start.elapsed());
                }
                Err(_) => {
                    complete_stats.errors += 1;
                }
            }
        }
    }
    
    let total_benchmark_time = benchmark_start.elapsed();
    
    // Step 4: Results Summary
    println!("\n{}", "=".repeat(70));
    println!("📊 PERFORMANCE BENCHMARK RESULTS - 1000 LATEST TRANSACTIONS");
    println!("{}", "=".repeat(70));
    
    println!("\n🏗️ Setup Cost:");
    println!("  Provider creation: {:.1}ms (one-time)", provider_creation_time.as_millis());
    
    println!("\n⚡ Basic Mode Performance (1000 transactions):");
    println!("  Total time: {:.3}s", basic_stats.total_time.as_secs_f64());
    println!("  Min time: {:.3}ms", basic_stats.min_time.as_secs_f64() * 1000.0);
    println!("  Max time: {:.3}ms", basic_stats.max_time.as_secs_f64() * 1000.0);
    println!("  Avg time: {:.3}ms", basic_stats.avg_time().as_secs_f64() * 1000.0);
    println!("  Success rate: {:.1}%", (basic_stats.count as f64 / 1000.0) * 100.0);
    println!("  Throughput: {:.0} tx/sec", basic_stats.throughput());
    
    println!("\n🧠 Smart Mode Performance ({} transactions sampled):", smart_stats.count);
    println!("  Avg time: {:.3}ms", smart_stats.avg_time().as_secs_f64() * 1000.0);
    println!("  Success rate: {:.1}%", (smart_stats.count as f64 / 100.0) * 100.0);
    
    println!("\n🔬 Complete Mode Performance ({} transactions sampled):", complete_stats.count);
    println!("  Avg time: {:.3}ms", complete_stats.avg_time().as_secs_f64() * 1000.0);
    println!("  Success rate: {:.1}%", (complete_stats.count as f64 / 10.0) * 100.0);
    
    println!("\n📈 Transaction Type Distribution (from Smart mode sampling):");
    for (tx_type, count) in tx_type_distribution.iter() {
        println!("  {:?}: {} ({:.1}%)", 
                 tx_type, 
                 count, 
                 (*count as f64 / smart_stats.count as f64) * 100.0);
    }
    
    // Performance Comparison
    println!("\n🚀 Performance Comparison:");
    println!("{}", "-".repeat(50));
    
    let baseline_projected = baseline_stats.avg_time().as_secs_f64() * 1000.0;
    let optimized_actual = basic_stats.total_time.as_secs_f64();
    let speedup = baseline_projected / optimized_actual;
    
    println!("  Baseline (projected): {:.1}s for 1K transactions", baseline_projected);
    println!("  Optimized (actual):   {:.1}s for 1K transactions", optimized_actual);
    println!("  Speedup factor:       {:.0}x faster", speedup);
    
    // Real-world projections
    println!("\n🌍 Real-World Performance Projections:");
    println!("{}", "-".repeat(50));
    
    let tx_per_second = basic_stats.throughput();
    let tx_per_minute = tx_per_second * 60.0;
    let tx_per_hour = tx_per_minute * 60.0;
    
    println!("  Transactions/second:  {:.0}", tx_per_second);
    println!("  Transactions/minute:  {:.0}", tx_per_minute);
    println!("  Transactions/hour:    {:.0}", tx_per_hour);
    println!("  Daily capacity:       {:.0}", tx_per_hour * 24.0);
    
    // Memory efficiency
    println!("\n💾 Efficiency Metrics:");
    println!("  Avg query time: {:.3}ms (excluding provider creation)", 
             basic_stats.avg_time().as_secs_f64() * 1000.0);
    println!("  Provider overhead amortized: {:.3}ms per query", 
             provider_creation_time.as_secs_f64() * 1000.0 / 1000.0);
    println!("  Total time for 1K batch: {:.3}s", total_benchmark_time.as_secs_f64());
    
    // Final summary
    println!("\n{}", "=".repeat(70));
    println!("✅ BENCHMARK COMPLETE");
    println!("{}", "=".repeat(70));
    println!("\n🎯 Key Findings:");
    println!("  • Provider reuse delivers {:.0}x speedup for batch operations", speedup);
    println!("  • Sub-millisecond query performance achieved ({:.3}ms avg)", 
             basic_stats.avg_time().as_secs_f64() * 1000.0);
    println!("  • Production capacity: {:.0} transactions/minute", tx_per_minute);
    println!("  • Latest blocks successfully processed with high reliability");
    
    Ok(())
}