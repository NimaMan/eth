//! End-to-End Integration Test for Alert Flow
//! 
//! Tests the complete flow from alert reception to decision making

use eth_kartal::{
    alert_processor::{AlertReceiver, ReceiverConfig, ScamAlert},
    strategy::{DecisionEngine, DecisionConfig, TradingDecision},
    wallet::PositionTracker,
};
use ethers::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[path = "./mock_alert_publisher.rs"]
mod mock_alert_publisher;
use mock_alert_publisher::MockAlertPublisher;

#[tokio::test]
async fn test_alert_to_decision_flow() {
    // Setup channels
    let (alert_tx, mut alert_rx) = mpsc::channel::<ScamAlert>(100);
    
    // Setup alert receiver
    let receiver_config = ReceiverConfig {
        endpoint: "tcp://localhost:25559".to_string(), // Use different port for test
        timeout_ms: 1000,
        ..Default::default()
    };
    
    let mut receiver = AlertReceiver::new(receiver_config, alert_tx);
    receiver.start().expect("Failed to start receiver");
    
    // Setup decision engine
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
    let wallet = Address::from([0x42; 20]); // Test wallet
    let position_tracker = Arc::new(PositionTracker::new(provider, wallet).unwrap());
    
    let decision_config = DecisionConfig {
        test_mode: true,
        ..Default::default()
    };
    let decision_engine = DecisionEngine::new(decision_config, position_tracker);
    
    // Start alert publisher
    let mut publisher = MockAlertPublisher::new("tcp://127.0.0.1:25559").unwrap();
    
    // Test 1: Emergency sell alert
    println!("Test 1: Emergency sell alert");
    let alert_id = publisher.send_alert("Critical", -95.0, 0.95).unwrap();
    
    // Wait for alert
    let alert = tokio::time::timeout(Duration::from_secs(2), alert_rx.recv())
        .await
        .expect("Timeout waiting for alert")
        .expect("Channel closed");
    
    assert_eq!(alert.alert_id, alert_id);
    assert_eq!(alert.severity, eth_kartal::alert_processor::Severity::Critical);
    
    // Process alert
    let decision = decision_engine.process_alert(&alert).await.unwrap();
    
    // Should be Skip (no position) or Emergency based on mock
    match decision {
        TradingDecision::Skip { reason, .. } => {
            assert!(reason.contains("No tokens held") || reason.contains("Position check failed"));
        }
        TradingDecision::EmergencySell { .. } => {
            // This would happen if we mocked a position
            assert!(true);
        }
        _ => panic!("Expected Skip or EmergencySell decision"),
    }
    
    // Test 2: Partial sell alert
    println!("Test 2: Partial sell alert");
    let alert_id = publisher.send_alert("High", -55.0, 0.90).unwrap();
    
    let alert = tokio::time::timeout(Duration::from_secs(2), alert_rx.recv())
        .await
        .expect("Timeout waiting for alert")
        .expect("Channel closed");
    
    assert_eq!(alert.severity, eth_kartal::alert_processor::Severity::High);
    
    // Test 3: Monitor alert
    println!("Test 3: Monitor alert");
    let alert_id = publisher.send_alert("Medium", -30.0, 0.85).unwrap();
    
    let alert = tokio::time::timeout(Duration::from_secs(2), alert_rx.recv())
        .await
        .expect("Timeout waiting for alert")
        .expect("Channel closed");
    
    assert_eq!(alert.severity, eth_kartal::alert_processor::Severity::Medium);
    
    println!("✅ All alert flow tests passed!");
}

#[tokio::test]
async fn test_low_confidence_filtering() {
    // Setup
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
    let wallet = Address::from([0x42; 20]);
    let position_tracker = Arc::new(PositionTracker::new(provider, wallet).unwrap());
    
    let decision_config = DecisionConfig::default();
    let decision_engine = DecisionEngine::new(decision_config, position_tracker);
    
    // Create alert with low confidence
    let mut publisher = MockAlertPublisher::new("tcp://127.0.0.1:25560").unwrap();
    
    // Create a mock alert with low confidence
    let alert = eth_kartal::alert_processor::ScamAlert {
        alert_id: "test_low_confidence".to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        severity: eth_kartal::alert_processor::Severity::Critical,
        event_type: eth_kartal::alert_processor::EventType::ScamAlert,
        tx_hash: "0x".to_string() + &"1".repeat(64),
        detected_latency_us: 1000,
        pool_address: Address::random(),
        pool_version: "V2".to_string(),
        token_address: Address::random(),
        token_symbol: "TEST".to_string(),
        token_decimals: 18,
        current_eth_reserve: 100.0,
        simulated_eth_reserve: 5.0,
        eth_change_amount: -95.0,
        eth_change_percent: -95.0,
        current_price: 1.0,
        simulated_price: 0.05,
        price_impact_percent: -95.0,
        confidence_score: 0.5, // Low confidence
        gas_price_gwei: 30.0,
        details: "Low confidence test".to_string(),
        requires_action: true,
    };
    
    // Process alert
    let decision = decision_engine.process_alert(&alert).await.unwrap();
    
    // Should be Monitor due to low confidence
    match decision {
        TradingDecision::Monitor { reason, .. } => {
            assert!(reason.contains("Low confidence"));
        }
        TradingDecision::Skip { .. } => {
            // Also acceptable if no position
            assert!(true);
        }
        _ => panic!("Expected Monitor or Skip decision for low confidence"),
    }
}

#[tokio::test]
async fn test_concurrent_alert_handling() {
    let (alert_tx, mut alert_rx) = mpsc::channel::<ScamAlert>(100);
    
    // Start receiver
    let receiver_config = ReceiverConfig {
        endpoint: "tcp://localhost:25561".to_string(),
        ..Default::default()
    };
    let mut receiver = AlertReceiver::new(receiver_config, alert_tx);
    receiver.start().unwrap();
    
    // Send multiple alerts concurrently
    let mut publisher = MockAlertPublisher::new("tcp://127.0.0.1:25561").unwrap();
    let alert_ids = publisher.send_burst(10).unwrap();
    
    // Collect all alerts
    let mut received_alerts = Vec::new();
    let timeout = Duration::from_secs(3);
    let start = tokio::time::Instant::now();
    
    while received_alerts.len() < alert_ids.len() && start.elapsed() < timeout {
        if let Ok(Some(alert)) = tokio::time::timeout(
            Duration::from_millis(100),
            alert_rx.recv()
        ).await {
            received_alerts.push(alert);
        }
    }
    
    // Verify all alerts received
    assert_eq!(received_alerts.len(), alert_ids.len());
    println!("✅ Received all {} concurrent alerts", alert_ids.len());
}