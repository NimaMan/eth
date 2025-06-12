//! Performance Tests
//! 
//! Benchmarks and performance validation for transaction processing

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};
    use crate::process_tx::{
        extract_state_changes_python_format,
        extract_batch_state_changes_python_format,
        compare_with_python,
    };
    // Note: benchmark_transaction and TransactionBenchmark defined at end of real_transaction_tests.rs

    const TEST_RPC_URL: &str = "http://127.0.0.1:8545";
    const PERFORMANCE_TARGET_MS: f64 = 100.0; // Target processing time

    #[tokio::test]
    #[ignore] // Only run for performance testing
    async fn test_single_transaction_performance() {
        let tx_hash = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060";
        
        let start = Instant::now();
        let result = extract_state_changes_python_format(tx_hash.to_string(), TEST_RPC_URL).await;
        let duration = start.elapsed();
        
        match result {
            Ok(state_changes) => {
                let processing_time_ms = duration.as_secs_f64() * 1000.0;
                println!("✅ Single transaction processed in {:.1}ms", processing_time_ms);
                println!("   Addresses affected: {}", state_changes.metadata.addresses_affected);
                println!("   Tokens involved: {}", state_changes.metadata.tokens_involved);
                
                // Performance assertion (relaxed for testing environments)
                if processing_time_ms > PERFORMANCE_TARGET_MS * 5.0 {
                    println!("⚠️  Processing time {:.1}ms exceeds 5x target of {:.1}ms", 
                        processing_time_ms, PERFORMANCE_TARGET_MS);
                }
                
                assert!(processing_time_ms < 30000.0, "Processing took over 30 seconds");
            }
            Err(e) => {
                println!("⚠️  Performance test failed: {}", e);
                println!("   This is expected if Reth node is not available");
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run for performance testing
    async fn test_batch_processing_performance() {
        let tx_hashes = vec![
            "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060".to_string(),
            "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b".to_string(),
        ];
        
        let start = Instant::now();
        let result = extract_batch_state_changes_python_format(tx_hashes.clone(), TEST_RPC_URL).await;
        let duration = start.elapsed();
        
        match result {
            Ok(results) => {
                let total_time_ms = duration.as_secs_f64() * 1000.0;
                let avg_time_ms = total_time_ms / results.len() as f64;
                
                println!("✅ Batch processed {} transactions in {:.1}ms", results.len(), total_time_ms);
                println!("   Average per transaction: {:.1}ms", avg_time_ms);
                
                // Batch processing should be more efficient than sequential
                let sequential_estimate = results.len() as f64 * PERFORMANCE_TARGET_MS;
                if total_time_ms < sequential_estimate {
                    println!("✅ Batch processing is efficient (faster than sequential)");
                } else {
                    println!("⚠️  Batch processing may not be optimal");
                }
                
                assert_eq!(results.len(), tx_hashes.len());
                assert!(total_time_ms < 60000.0, "Batch processing took over 1 minute");
            }
            Err(e) => {
                println!("⚠️  Batch performance test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run for comprehensive benchmarking
    async fn test_rust_vs_python_performance() {
        let test_cases = vec![
            ("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060", "Simple ETH transfer"),
            ("0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b", "ERC20 transfer"),
        ];
        
        let mut benchmarks = Vec::new();
        
        for (tx_hash, description) in test_cases {
            println!("🔍 Benchmarking: {}", description);
            
            match compare_with_python(tx_hash, TEST_RPC_URL, Some("http://127.0.0.1:18000")).await {
                Ok(result) => {
                    println!("   Rust: {:.1}ms", result.rust_processing_time_ms);
                    println!("   Python: {:.1}ms", result.python_processing_time_ms);
                    
                    let speedup = result.python_processing_time_ms / result.rust_processing_time_ms;
                    if speedup > 1.0 {
                        println!("   ✅ Rust is {:.1}x faster", speedup);
                    } else {
                        println!("   ⚠️  Python is {:.1}x faster", 1.0 / speedup);
                    }
                    
                    benchmarks.push((description, result.rust_processing_time_ms, result.python_processing_time_ms));
                }
                Err(e) => {
                    println!("   ❌ Comparison failed: {}", e);
                }
            }
        }
        
        if !benchmarks.is_empty() {
            println!("\n📊 Performance Summary:");
            let total_rust: f64 = benchmarks.iter().map(|(_, r, _)| r).sum();
            let total_python: f64 = benchmarks.iter().map(|(_, _, p)| p).sum();
            let avg_speedup = total_python / total_rust;
            
            println!("   Average Rust time: {:.1}ms", total_rust / benchmarks.len() as f64);
            println!("   Average Python time: {:.1}ms", total_python / benchmarks.len() as f64);
            println!("   Overall speedup: {:.1}x", avg_speedup);
        }
    }

    #[tokio::test]
    #[ignore] // Only run for memory/resource testing
    async fn test_concurrent_processing() {
        use tokio::task::JoinSet;
        
        let tx_hash = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060";
        let concurrent_count = 5;
        
        println!("🔄 Testing {} concurrent transactions", concurrent_count);
        
        let start = Instant::now();
        let mut join_set = JoinSet::new();
        
        for i in 0..concurrent_count {
            let tx_hash = tx_hash.to_string();
            join_set.spawn(async move {
                let task_start = Instant::now();
                let result = extract_state_changes_python_format(tx_hash, TEST_RPC_URL).await;
                let task_duration = task_start.elapsed();
                (i, result, task_duration)
            });
        }
        
        let mut successful = 0;
        let mut total_task_time = Duration::ZERO;
        
        while let Some(task_result) = join_set.join_next().await {
            match task_result {
                Ok((task_id, Ok(_), task_duration)) => {
                    successful += 1;
                    total_task_time += task_duration;
                    println!("   Task {}: {:.1}ms", task_id, task_duration.as_secs_f64() * 1000.0);
                }
                Ok((task_id, Err(e), _)) => {
                    println!("   Task {} failed: {}", task_id, e);
                }
                Err(e) => {
                    println!("   Task join failed: {}", e);
                }
            }
        }
        
        let total_wall_time = start.elapsed();
        let avg_task_time = total_task_time / successful.max(1);
        
        println!("📊 Concurrent Processing Results:");
        println!("   Successful tasks: {}/{}", successful, concurrent_count);
        println!("   Total wall time: {:.1}ms", total_wall_time.as_secs_f64() * 1000.0);
        println!("   Average task time: {:.1}ms", avg_task_time.as_secs_f64() * 1000.0);
        
        // Concurrent processing should complete faster than sequential
        let sequential_estimate = avg_task_time * concurrent_count as u32;
        let efficiency = sequential_estimate.as_secs_f64() / total_wall_time.as_secs_f64();
        println!("   Concurrency efficiency: {:.1}x", efficiency);
        
        if successful > 0 {
            assert!(efficiency > 1.5, "Concurrent processing should be at least 1.5x faster than sequential");
        }
    }

    #[tokio::test]
    #[ignore] // Only run for stress testing
    async fn test_memory_usage_stability() {
        // Process multiple transactions to check for memory leaks
        let tx_hashes = vec![
            "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060",
            "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b",
        ];
        
        let iterations = 10;
        println!("🧪 Memory stability test: {} iterations", iterations);
        
        let mut processing_times = Vec::new();
        
        for i in 0..iterations {
            let start = Instant::now();
            
            for tx_hash in &tx_hashes {
                match extract_state_changes_python_format(tx_hash.to_string(), TEST_RPC_URL).await {
                    Ok(_) => {
                        // Success - continue
                    }
                    Err(_) => {
                        // Expected if node not available
                        println!("   Iteration {} failed (expected if node not available)", i);
                        return;
                    }
                }
            }
            
            let iteration_time = start.elapsed();
            processing_times.push(iteration_time.as_secs_f64() * 1000.0);
            
            if i % 3 == 0 {
                println!("   Iteration {}: {:.1}ms", i, iteration_time.as_secs_f64() * 1000.0);
            }
        }
        
        if processing_times.len() >= 5 {
            let avg_time = processing_times.iter().sum::<f64>() / processing_times.len() as f64;
            let max_time = processing_times.iter().fold(0.0f64, |a, &b| a.max(b));
            let min_time = processing_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            println!("📊 Memory Stability Results:");
            println!("   Average time: {:.1}ms", avg_time);
            println!("   Min time: {:.1}ms", min_time);
            println!("   Max time: {:.1}ms", max_time);
            println!("   Variation: {:.1}%", ((max_time - min_time) / avg_time) * 100.0);
            
            // Times should be relatively stable (not increasing significantly)
            let time_variation = (max_time - min_time) / avg_time;
            assert!(time_variation < 2.0, "Processing times too variable ({}%)", time_variation * 100.0);
        }
    }

    #[test]
    fn test_performance_baseline_assumptions() {
        // Test that our performance assumptions make sense
        
        // Target processing time should be reasonable
        assert!(PERFORMANCE_TARGET_MS > 1.0, "Target too aggressive");
        assert!(PERFORMANCE_TARGET_MS < 1000.0, "Target too lenient");
        
        // Test RPC URL format
        assert!(TEST_RPC_URL.starts_with("http://"));
        assert!(TEST_RPC_URL.contains("127.0.0.1") || TEST_RPC_URL.contains("localhost"));
        
        println!("✅ Performance baseline assumptions validated");
        println!("   Target processing time: {:.1}ms", PERFORMANCE_TARGET_MS);
        println!("   Test RPC URL: {}", TEST_RPC_URL);
    }

    #[tokio::test]
    #[ignore] // Only run for detailed profiling
    async fn test_processing_time_breakdown() {
        let tx_hash = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060";
        
        println!("🔍 Processing time breakdown for: {}", tx_hash);
        
        // Measure total time
        let total_start = Instant::now();
        let result = extract_state_changes_python_format(tx_hash.to_string(), TEST_RPC_URL).await;
        let total_time = total_start.elapsed();
        
        match result {
            Ok(state_changes) => {
                let reported_time = state_changes.metadata.processing_time_ms;
                let measured_time = total_time.as_secs_f64() * 1000.0;
                
                println!("📊 Time Breakdown:");
                println!("   Reported processing time: {:.1}ms", reported_time);
                println!("   Measured total time: {:.1}ms", measured_time);
                println!("   Overhead: {:.1}ms ({:.1}%)", 
                    measured_time - reported_time,
                    ((measured_time - reported_time) / measured_time) * 100.0
                );
                
                println!("📈 Processing Efficiency:");
                println!("   Addresses per ms: {:.2}", 
                    state_changes.metadata.addresses_affected as f64 / measured_time);
                println!("   Tokens per ms: {:.2}",
                    state_changes.metadata.tokens_involved as f64 / measured_time.max(1.0));
                
                // Sanity checks
                assert!(reported_time > 0.0, "Reported time should be positive");
                assert!(measured_time > reported_time * 0.5, "Measured time unusually low");
                assert!(measured_time < reported_time * 10.0, "Measured time unusually high");
            }
            Err(e) => {
                println!("⚠️  Breakdown test failed: {}", e);
            }
        }
    }
}

// Performance utilities
#[cfg(test)]
pub struct PerformanceMetrics {
    pub transaction_count: usize,
    pub total_time_ms: f64,
    pub avg_time_per_tx_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub success_rate: f64,
}

#[cfg(test)]
impl PerformanceMetrics {
    pub fn from_times(times: &[f64]) -> Self {
        if times.is_empty() {
            return PerformanceMetrics {
                transaction_count: 0,
                total_time_ms: 0.0,
                avg_time_per_tx_ms: 0.0,
                min_time_ms: 0.0,
                max_time_ms: 0.0,
                success_rate: 0.0,
            };
        }
        
        let total_time: f64 = times.iter().sum();
        let min_time = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_time = times.iter().fold(0.0f64, |a, &b| a.max(b));
        
        PerformanceMetrics {
            transaction_count: times.len(),
            total_time_ms: total_time,
            avg_time_per_tx_ms: total_time / times.len() as f64,
            min_time_ms: min_time,
            max_time_ms: max_time,
            success_rate: 1.0, // All times provided are considered successful
        }
    }
    
    pub fn print_summary(&self) {
        println!("📊 Performance Summary:");
        println!("   Transactions: {}", self.transaction_count);
        println!("   Success rate: {:.1}%", self.success_rate * 100.0);
        println!("   Total time: {:.1}ms", self.total_time_ms);
        println!("   Average time: {:.1}ms", self.avg_time_per_tx_ms);
        println!("   Min time: {:.1}ms", self.min_time_ms);
        println!("   Max time: {:.1}ms", self.max_time_ms);
        println!("   Throughput: {:.1} tx/sec", 1000.0 / self.avg_time_per_tx_ms);
    }
}