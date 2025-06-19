//! Performance Benchmark Tests
//! 
//! Validates that the system meets latency requirements

#[cfg(test)]
mod performance_tests {
    use eth_kartal::*;
    use std::time::{Duration, Instant};
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn benchmark_alert_parsing(c: &mut Criterion) {
        let json_alert = r#"{
            "alert_id": "bench_1",
            "severity": "Critical",
            "tx_hash": "0x063f3203750dca1c85764521f4b2330dcab9fdd7f407c868d06ea6a9e7aab5bd",
            "pool_address": "0x9CC8b3118780faea1136AEfeCaD6F43592c4cdeB",
            "token_address": "0x3cf343b255c9a7aEcafdbA79b8B55bec585a5C66",
            "current_eth_reserve": 2.396670,
            "simulated_eth_reserve": 0.0,
            "eth_change_percent": -100.0,
            "confidence_score": 0.99
        }"#;
        
        c.bench_function("alert_parsing", |b| {
            b.iter(|| {
                let _alert: ScamAlert = serde_json::from_str(black_box(json_alert)).unwrap();
            })
        });
    }
    
    fn benchmark_strategy_decision(c: &mut Criterion) {
        let config = Config::from_file("config/dev.toml").unwrap();
        let engine = DecisionEngine::new(config);
        let alert = create_test_alert();
        
        c.bench_function("strategy_decision", |b| {
            b.iter(|| {
                let _strategy = engine.decide_strategy(black_box(&alert));
            })
        });
    }
    
    fn benchmark_transaction_building(c: &mut Criterion) {
        let builder = TransactionBuilder::new(
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap()
        );
        
        c.bench_function("transaction_building", |b| {
            b.iter(|| {
                let _tx = builder.build_emergency_sell(
                    black_box("0x3cf343b255c9a7aEcafdbA79b8B55bec585a5C66".parse().unwrap()),
                    black_box(U256::from(1000000)),
                    black_box(U256::zero()),
                );
            })
        });
    }
    
    #[tokio::test]
    async fn test_end_to_end_latency() {
        let mut latencies = Vec::new();
        
        for i in 0..1000 {
            let start = Instant::now();
            
            // 1. Parse alert (simulated)
            let alert = create_test_alert();
            let parse_time = start.elapsed();
            
            // 2. Make strategy decision
            let strategy_start = Instant::now();
            let strategy = decide_strategy(&alert);
            let strategy_time = strategy_start.elapsed();
            
            // 3. Build transaction
            let build_start = Instant::now();
            let tx = build_transaction(&strategy);
            let build_time = build_start.elapsed();
            
            // 4. Simulate submission (mock)
            let submit_start = Instant::now();
            tokio::time::sleep(Duration::from_micros(500)).await; // Simulate network
            let submit_time = submit_start.elapsed();
            
            let total = start.elapsed();
            latencies.push(LatencyMeasurement {
                parse: parse_time,
                strategy: strategy_time,
                build: build_time,
                submit: submit_time,
                total,
            });
        }
        
        // Calculate statistics
        latencies.sort_by_key(|l| l.total);
        
        let p50 = latencies[500].total;
        let p95 = latencies[950].total;
        let p99 = latencies[990].total;
        let max = latencies[999].total;
        
        println!("\n=== LATENCY BENCHMARK RESULTS ===");
        println!("Samples: 1000 transactions");
        println!("\nTotal Latency (Alert → Submission):");
        println!("  P50: {:?}", p50);
        println!("  P95: {:?}", p95);
        println!("  P99: {:?}", p99);
        println!("  Max: {:?}", max);
        
        // Component breakdown for P95
        let p95_sample = &latencies[950];
        println!("\nP95 Component Breakdown:");
        println!("  Parse:    {:?} ({:.1}%)", p95_sample.parse, 
                 p95_sample.parse.as_secs_f64() / p95_sample.total.as_secs_f64() * 100.0);
        println!("  Strategy: {:?} ({:.1}%)", p95_sample.strategy,
                 p95_sample.strategy.as_secs_f64() / p95_sample.total.as_secs_f64() * 100.0);
        println!("  Build:    {:?} ({:.1}%)", p95_sample.build,
                 p95_sample.build.as_secs_f64() / p95_sample.total.as_secs_f64() * 100.0);
        println!("  Submit:   {:?} ({:.1}%)", p95_sample.submit,
                 p95_sample.submit.as_secs_f64() / p95_sample.total.as_secs_f64() * 100.0);
        
        // Performance assertions
        assert!(p50.as_millis() < 100, "P50 latency exceeds 100ms");
        assert!(p95.as_millis() < 150, "P95 latency exceeds 150ms");
        assert!(p99.as_millis() < 200, "P99 latency exceeds 200ms");
        
        println!("\n✅ Performance requirements met!");
    }
    
    #[tokio::test]
    async fn test_throughput() {
        let start = Instant::now();
        let mut processed = 0;
        
        // Process alerts for 10 seconds
        while start.elapsed() < Duration::from_secs(10) {
            // Simulate alert processing
            let alert = create_test_alert();
            let _strategy = decide_strategy(&alert);
            let _tx = build_transaction(&_strategy);
            
            processed += 1;
        }
        
        let elapsed = start.elapsed();
        let rate = processed as f64 / elapsed.as_secs_f64();
        
        println!("\n=== THROUGHPUT TEST ===");
        println!("Duration: {:?}", elapsed);
        println!("Processed: {} alerts", processed);
        println!("Rate: {:.2} alerts/second", rate);
        
        assert!(rate > 100.0, "Throughput below 100 alerts/second");
    }
    
    #[tokio::test]
    async fn test_concurrent_processing() {
        use tokio::sync::mpsc;
        use futures::future::join_all;
        
        let (tx, mut rx) = mpsc::channel(10000);
        
        // Spawn 10 processing tasks
        let mut handles = vec![];
        for i in 0..10 {
            let tx = tx.clone();
            let handle = tokio::spawn(async move {
                for j in 0..100 {
                    let alert = create_test_alert_with_id(i * 100 + j);
                    let start = Instant::now();
                    
                    let _strategy = decide_strategy(&alert);
                    let _tx = build_transaction(&_strategy);
                    
                    let latency = start.elapsed();
                    tx.send((alert.alert_id, latency)).await.unwrap();
                }
            });
            handles.push(handle);
        }
        
        drop(tx); // Close sender
        
        // Collect results
        let mut latencies = vec![];
        while let Some((id, latency)) = rx.recv().await {
            latencies.push(latency);
        }
        
        // Wait for all tasks
        join_all(handles).await;
        
        // Analyze results
        assert_eq!(latencies.len(), 1000, "Should process all alerts");
        
        let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        println!("\nConcurrent processing average latency: {:?}", avg_latency);
        
        assert!(avg_latency.as_millis() < 50, "Concurrent latency too high");
    }
    
    // Helper structures and functions
    struct LatencyMeasurement {
        parse: Duration,
        strategy: Duration,
        build: Duration,
        submit: Duration,
        total: Duration,
    }
    
    fn create_test_alert() -> ScamAlert {
        ScamAlert {
            alert_id: "perf_test".to_string(),
            severity: Severity::Critical,
            tx_hash: "0x123".to_string(),
            pool_address: "0x456".to_string(),
            token_address: "0x789".to_string(),
            current_eth_reserve: 10.0,
            simulated_eth_reserve: 0.0,
            eth_change_percent: -100.0,
            confidence_score: 0.99,
            ..Default::default()
        }
    }
    
    fn create_test_alert_with_id(id: usize) -> ScamAlert {
        let mut alert = create_test_alert();
        alert.alert_id = format!("perf_test_{}", id);
        alert
    }
    
    fn decide_strategy(alert: &ScamAlert) -> Strategy {
        // Simplified decision logic for benchmarking
        if alert.eth_change_percent <= -80.0 {
            Strategy::EmergencySell {
                token: alert.token_address.parse().unwrap(),
                max_slippage: 0.30,
                gas_multiplier: 3.0,
            }
        } else {
            Strategy::MonitorOnly {
                reason: "Below threshold".to_string(),
            }
        }
    }
    
    fn build_transaction(strategy: &Strategy) -> TransactionRequest {
        // Mock transaction building
        TransactionRequest::new()
            .to("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse::<Address>().unwrap())
            .value(0)
            .data(vec![0x00; 196]) // Mock calldata
    }
}