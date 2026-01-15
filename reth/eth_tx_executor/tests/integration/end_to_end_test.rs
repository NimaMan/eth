//! End-to-End Integration Tests
//! 
//! Tests the complete flow from alert reception to transaction execution

#[cfg(test)]
mod integration_tests {
    use eth_kartal::{
        alert_processor::{AlertReceiver, ScamAlert},
        strategy::DecisionEngine,
        tx_executor::TransactionExecutor,
        risk::RiskManager,
    };
    use tokio::sync::mpsc;
    use std::time::{Duration, Instant};
    use ethers::prelude::*;
    
    #[tokio::test]
    async fn test_complete_alert_to_execution_flow() {
        // Setup test environment
        let config = Config::from_file("config/dev.toml").unwrap();
        
        // Create channels
        let (alert_tx, mut alert_rx) = mpsc::channel::<ScamAlert>(100);
        let (strategy_tx, mut strategy_rx) = mpsc::channel(100);
        let (execution_tx, mut execution_rx) = mpsc::channel(100);
        
        // Initialize components
        let alert_receiver = AlertReceiver::new("tcp://localhost:5558", alert_tx);
        let decision_engine = DecisionEngine::new(config.clone());
        let risk_manager = RiskManager::new(config.clone());
        let tx_executor = TransactionExecutor::new(config.clone());
        
        // Start alert receiver in background
        tokio::spawn(async move {
            alert_receiver.start_listening().await.unwrap();
        });
        
        // Start decision engine
        tokio::spawn(async move {
            while let Some(alert) = alert_rx.recv().await {
                let start = Instant::now();
                
                // Make strategy decision
                let strategy = decision_engine.decide_strategy(&alert);
                
                // Check risk limits
                match risk_manager.evaluate_strategy(&strategy, &alert) {
                    RiskDecision::Approved => {
                        strategy_tx.send((alert, strategy, start)).await.unwrap();
                    }
                    RiskDecision::Rejected(reason) => {
                        println!("Risk rejected: {}", reason);
                    }
                    RiskDecision::Modified(new_strategy) => {
                        strategy_tx.send((alert, new_strategy, start)).await.unwrap();
                    }
                }
            }
        });
        
        // Start transaction executor
        tokio::spawn(async move {
            while let Some((alert, strategy, start_time)) = strategy_rx.recv().await {
                match tx_executor.execute_strategy(&strategy, &alert).await {
                    Ok(tx_hash) => {
                        let elapsed = start_time.elapsed();
                        execution_tx.send((tx_hash, elapsed)).await.unwrap();
                    }
                    Err(e) => {
                        eprintln!("Execution failed: {}", e);
                    }
                }
            }
        });
        
        // Send test alert via ZMQ
        send_test_alert().await;
        
        // Wait for execution
        let timeout = tokio::time::timeout(
            Duration::from_secs(5),
            execution_rx.recv()
        ).await;
        
        match timeout {
            Ok(Some((tx_hash, latency))) => {
                println!("Transaction executed: {}", tx_hash);
                println!("Total latency: {:?}", latency);
                
                // Assert performance requirements
                assert!(latency.as_millis() < 200, "Latency exceeds 200ms limit");
            }
            Ok(None) => panic!("Channel closed unexpectedly"),
            Err(_) => panic!("Timeout waiting for execution"),
        }
    }
    
    #[tokio::test]
    async fn test_concurrent_alert_handling() {
        let config = Config::from_file("config/dev.toml").unwrap();
        let (alert_tx, mut alert_rx) = mpsc::channel::<ScamAlert>(1000);
        
        // Send 100 alerts concurrently
        let mut handles = vec![];
        for i in 0..100 {
            let tx = alert_tx.clone();
            let handle = tokio::spawn(async move {
                let alert = create_test_alert(i);
                tx.send(alert).await.unwrap();
            });
            handles.push(handle);
        }
        
        // Wait for all sends
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Process all alerts
        let mut processed = 0;
        let start = Instant::now();
        
        while let Ok(Some(alert)) = tokio::time::timeout(
            Duration::from_millis(100),
            alert_rx.recv()
        ).await {
            processed += 1;
        }
        
        let elapsed = start.elapsed();
        let rate = processed as f64 / elapsed.as_secs_f64();
        
        println!("Processed {} alerts in {:?}", processed, elapsed);
        println!("Rate: {:.2} alerts/second", rate);
        
        assert_eq!(processed, 100, "Should process all alerts");
        assert!(rate > 100.0, "Should handle >100 alerts/second");
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_integration() {
        let config = Config::from_file("config/dev.toml").unwrap();
        let risk_manager = RiskManager::new(config);
        
        // Simulate losses to trigger circuit breaker
        for _ in 0..5 {
            risk_manager.record_trade_result(TradeResult {
                success: false,
                loss_eth: 0.15,
                ..Default::default()
            }).await;
        }
        
        // Circuit should be open
        assert!(risk_manager.is_circuit_open(), "Circuit should trip after losses");
        
        // Try to execute trade - should be rejected
        let alert = create_critical_alert();
        let strategy = Strategy::EmergencySell { .. };
        
        match risk_manager.evaluate_strategy(&strategy, &alert) {
            RiskDecision::Rejected(reason) => {
                assert!(reason.contains("Circuit breaker"));
            }
            _ => panic!("Should reject due to circuit breaker"),
        }
    }
    
    #[tokio::test]
    async fn test_mainnet_fork_execution() {
        // Start Anvil mainnet fork
        let anvil = Anvil::new()
            .fork("https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY")
            .spawn();
        
        let provider = Provider::<Http>::try_from(anvil.endpoint()).unwrap();
        
        // Setup test wallet with funds
        let wallet = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse::<LocalWallet>()
            .unwrap()
            .with_chain_id(1u64);
        
        let client = SignerMiddleware::new(provider.clone(), wallet.clone());
        
        // Fund wallet
        anvil.set_balance(wallet.address(), U256::from(10).pow(19.into())).await;
        
        // Create transaction executor
        let tx_executor = TransactionExecutor::new_with_provider(Arc::new(client));
        
        // Test emergency sell execution
        let token = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"; // USDC
        let amount = U256::from(1000).mul(U256::from(10).pow(6.into())); // 1000 USDC
        
        let tx_hash = tx_executor.execute_emergency_sell(
            token.parse().unwrap(),
            amount,
            U256::zero(), // Accept any price
        ).await.unwrap();
        
        // Wait for confirmation
        let receipt = provider.get_transaction_receipt(tx_hash).await.unwrap().unwrap();
        
        assert_eq!(receipt.status.unwrap(), U64::from(1), "Transaction should succeed");
        println!("Transaction confirmed: {:?}", receipt.transaction_hash);
    }
    
    // Helper functions
    async fn send_test_alert() {
        use zmq::Context;
        
        let context = Context::new();
        let socket = context.socket(zmq::PUB).unwrap();
        socket.bind("tcp://localhost:5558").unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let alert = serde_json::json!({
            "alert_id": "test_alert_1",
            "severity": "Critical",
            "tx_hash": "0x123...",
            "pool_address": "0x456...",
            "token_address": "0x789...",
            "current_eth_reserve": 10.0,
            "simulated_eth_reserve": 0.0,
            "eth_change_percent": -100.0,
            "confidence_score": 0.99,
        });
        
        socket.send(&serde_json::to_string(&alert).unwrap(), 0).unwrap();
    }
    
    fn create_test_alert(id: usize) -> ScamAlert {
        ScamAlert {
            alert_id: format!("test_alert_{}", id),
            severity: Severity::Critical,
            current_eth_reserve: 5.0,
            simulated_eth_reserve: 0.0,
            eth_change_percent: -100.0,
            confidence_score: 0.99,
            ..Default::default()
        }
    }
    
    fn create_critical_alert() -> ScamAlert {
        ScamAlert {
            alert_id: "critical_test".to_string(),
            severity: Severity::Critical,
            current_eth_reserve: 10.0,
            simulated_eth_reserve: 0.0,
            eth_change_percent: -100.0,
            confidence_score: 0.99,
            ..Default::default()
        }
    }
}