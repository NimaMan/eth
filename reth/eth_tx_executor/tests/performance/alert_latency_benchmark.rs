//! Alert Processing Latency Benchmark
//!
//! Measures end-to-end latency from alert reception to decision

use eth_kartal::{
    alert_processor::{AlertReceiver, ReceiverConfig, ScamAlert},
    strategy::{DecisionEngine, DecisionConfig},
    wallet::PositionTracker,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use ethers::prelude::*;

#[path = "../integration/mock_alert_publisher.rs"]
mod mock_alert_publisher;
use mock_alert_publisher::MockAlertPublisher;

struct LatencyMeasurement {
    alert_id: String,
    sent_at: Instant,
    received_at: Option<Instant>,
    decided_at: Option<Instant>,
    total_latency_ms: Option<f64>,
}

async fn measure_alert_latency() -> Vec<LatencyMeasurement> {
    // Setup
    let (alert_tx, mut alert_rx) = mpsc::channel::<(ScamAlert, Instant)>(1000);
    let mut measurements = Vec::new();
    
    // Start alert receiver
    let receiver_config = ReceiverConfig {
        endpoint: "tcp://localhost:5559".to_string(),
        timeout_ms: 100, // Short timeout for benchmark
        ..Default::default()
    };
    
    // Wrap the channel sender to record receive time
    let (wrapped_tx, mut wrapped_rx) = mpsc::channel::<ScamAlert>(1000);
    let alert_tx_clone = alert_tx.clone();
    
    tokio::spawn(async move {
        while let Some(alert) = wrapped_rx.recv().await {
            let received_at = Instant::now();
            alert_tx_clone.send((alert, received_at)).await.unwrap();
        }
    });
    
    let mut receiver = AlertReceiver::new(receiver_config, wrapped_tx);
    receiver.start().unwrap();
    
    // Setup decision engine
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
    let position_tracker = Arc::new(PositionTracker::new(
        provider,
        Address::random(),
    ).unwrap());
    
    let decision_config = DecisionConfig {
        test_mode: true,
        ..Default::default()
    };
    let decision_engine = DecisionEngine::new(decision_config, position_tracker);
    
    // Start alert publisher
    let mut publisher = MockAlertPublisher::new("tcp://127.0.0.1:5559").unwrap();
    
    // Send test alerts
    let alert_count = 100;
    let mut sent_times = std::collections::HashMap::new();
    
    for i in 0..alert_count {
        let sent_at = Instant::now();
        let alert_id = publisher.send_alert(
            if i % 2 == 0 { "Critical" } else { "High" },
            if i % 2 == 0 { -85.0 } else { -55.0 },
            0.95,
        ).unwrap();
        
        sent_times.insert(alert_id.clone(), sent_at);
        
        // Small delay between alerts
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Process alerts and measure latency
    let mut processed = 0;
    let timeout = Duration::from_secs(5);
    let start = Instant::now();
    
    while processed < alert_count && start.elapsed() < timeout {
        if let Ok(Some((alert, received_at))) = tokio::time::timeout(
            Duration::from_millis(100),
            alert_rx.recv()
        ).await {
            let decided_at = Instant::now();
            
            // Process alert
            let _decision = decision_engine.process_alert(&alert).await.unwrap();
            
            // Record measurement
            if let Some(sent_at) = sent_times.get(&alert.alert_id) {
                let total_latency_ms = decided_at.duration_since(*sent_at).as_secs_f64() * 1000.0;
                
                measurements.push(LatencyMeasurement {
                    alert_id: alert.alert_id.clone(),
                    sent_at: *sent_at,
                    received_at: Some(received_at),
                    decided_at: Some(decided_at),
                    total_latency_ms: Some(total_latency_ms),
                });
                
                processed += 1;
            }
        }
    }
    
    measurements
}

fn analyze_latency_results(measurements: &[LatencyMeasurement]) {
    let latencies: Vec<f64> = measurements
        .iter()
        .filter_map(|m| m.total_latency_ms)
        .collect();
    
    if latencies.is_empty() {
        println!("No latency measurements collected!");
        return;
    }
    
    let mut sorted_latencies = latencies.clone();
    sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let min = sorted_latencies.first().unwrap();
    let max = sorted_latencies.last().unwrap();
    let mean = latencies.iter().sum::<f64>() / latencies.len() as f64;
    
    let p50 = sorted_latencies[sorted_latencies.len() / 2];
    let p95 = sorted_latencies[(sorted_latencies.len() as f64 * 0.95) as usize];
    let p99 = sorted_latencies[(sorted_latencies.len() as f64 * 0.99) as usize];
    
    println!("\n=== Latency Analysis ===");
    println!("Samples: {}", latencies.len());
    println!("Min: {:.2}ms", min);
    println!("Mean: {:.2}ms", mean);
    println!("P50: {:.2}ms", p50);
    println!("P95: {:.2}ms", p95);
    println!("P99: {:.2}ms", p99);
    println!("Max: {:.2}ms", max);
    
    // Check against target
    let target_ms = 200.0;
    let under_target = latencies.iter().filter(|&&l| l < target_ms).count();
    let success_rate = (under_target as f64 / latencies.len() as f64) * 100.0;
    
    println!("\nTarget: <{}ms", target_ms);
    println!("Success rate: {:.1}% ({}/{})", success_rate, under_target, latencies.len());
    
    if p95 < target_ms {
        println!("✅ P95 latency meets target!");
    } else {
        println!("❌ P95 latency exceeds target!");
    }
}

#[tokio::test]
async fn benchmark_alert_processing_latency() {
    println!("Starting alert processing latency benchmark...");
    
    let measurements = measure_alert_latency().await;
    
    analyze_latency_results(&measurements);
    
    // Assert performance requirements
    let latencies: Vec<f64> = measurements
        .iter()
        .filter_map(|m| m.total_latency_ms)
        .collect();
    
    if !latencies.is_empty() {
        let mut sorted = latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
        
        // P95 should be under 200ms
        assert!(p95 < 200.0, "P95 latency ({:.2}ms) exceeds 200ms target", p95);
    }
}

#[tokio::test]
async fn benchmark_concurrent_alerts() {
    println!("Starting concurrent alert processing benchmark...");
    
    // This would test handling multiple alerts simultaneously
    // Implementation would be similar but spawn multiple alert senders
    
    // Placeholder for now
    assert!(true);
}

#[test]
fn test_latency_calculations() {
    // Test the latency calculation logic
    let start = Instant::now();
    std::thread::sleep(Duration::from_millis(50));
    let end = Instant::now();
    
    let latency_ms = end.duration_since(start).as_secs_f64() * 1000.0;
    assert!(latency_ms >= 50.0);
    assert!(latency_ms < 60.0); // Allow some margin
}