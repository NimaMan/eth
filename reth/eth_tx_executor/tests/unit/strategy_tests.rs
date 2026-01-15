//! Strategy Engine Unit Tests
//! 
//! Tests for decision logic based on real scam patterns

#[cfg(test)]
mod strategy_tests {
    use eth_kartal::strategy::{DecisionEngine, Strategy, ScamAlert, Severity};
    use eth_kartal::common::config::Config;
    
    fn test_config() -> Config {
        // Load test configuration
        Config::from_file("config/dev.toml").unwrap()
    }
    
    #[test]
    fn test_emergency_sell_on_100_percent_drain() {
        let config = test_config();
        let engine = DecisionEngine::new(config);
        
        // Real scam: 100% drain
        let alert = ScamAlert {
            alert_id: "test_100_drain".to_string(),
            severity: Severity::Critical,
            tx_hash: "0x063f3203750dca1c85764521f4b2330dcab9fdd7f407c868d06ea6a9e7aab5bd".to_string(),
            pool_address: "0x9CC8b3118780faea1136AEfeCaD6F43592c4cdeB".to_string(),
            token_address: "0x3cf343b255c9a7aEcafdbA79b8B55bec585a5C66".to_string(),
            current_eth_reserve: 2.396670,
            simulated_eth_reserve: 0.0,
            eth_change_percent: -100.0,
            confidence_score: 0.99,
            ..Default::default()
        };
        
        let strategy = engine.decide_strategy(&alert);
        
        match strategy {
            Strategy::EmergencySell { max_slippage, gas_multiplier, .. } => {
                assert_eq!(max_slippage, 0.30, "Should accept 30% slippage for emergency");
                assert_eq!(gas_multiplier, 3.0, "Should use 3x gas for speed");
            }
            _ => panic!("Expected EmergencySell for 100% drain, got {:?}", strategy),
        }
    }
    
    #[test]
    fn test_partial_exit_on_54_percent_drain() {
        let config = test_config();
        let engine = DecisionEngine::new(config);
        
        // Real scam: 54.44% drain
        let alert = ScamAlert {
            alert_id: "test_54_drain".to_string(),
            severity: Severity::High,
            tx_hash: "0x25007a38bd93d0eac82a47690b7da956ecda98a080738c0b7a6d07efeda8d61e".to_string(),
            pool_address: "0x2551C712f7A50E7E20c3e571DFFD557292Ce9B52".to_string(),
            token_address: "0xD91b157e31BAcD64F0338b823dfE2A363656d6cC".to_string(),
            current_eth_reserve: 56.880193,
            simulated_eth_reserve: 25.916374,
            eth_change_percent: -54.44,
            confidence_score: 0.95,
            ..Default::default()
        };
        
        let strategy = engine.decide_strategy(&alert);
        
        match strategy {
            Strategy::PartialExit { sell_percentage, max_slippage, .. } => {
                assert_eq!(sell_percentage, 0.75, "Should sell 75% of position");
                assert_eq!(max_slippage, 0.15, "Should accept 15% slippage");
            }
            _ => panic!("Expected PartialExit for 54% drain, got {:?}", strategy),
        }
    }
    
    #[test]
    fn test_monitor_only_for_small_pools() {
        let config = test_config();
        let engine = DecisionEngine::new(config);
        
        // Pool smaller than threshold
        let alert = ScamAlert {
            alert_id: "test_small_pool".to_string(),
            severity: Severity::Low,
            pool_address: "0x123".to_string(),
            token_address: "0x456".to_string(),
            current_eth_reserve: 0.05, // Below 0.1 ETH threshold
            simulated_eth_reserve: 0.0,
            eth_change_percent: -100.0,
            confidence_score: 0.99,
            ..Default::default()
        };
        
        let strategy = engine.decide_strategy(&alert);
        
        match strategy {
            Strategy::MonitorOnly { reason } => {
                assert!(reason.contains("pool size"), "Should skip small pools");
            }
            _ => panic!("Expected MonitorOnly for small pool, got {:?}", strategy),
        }
    }
    
    #[test]
    fn test_strategy_for_various_drain_percentages() {
        let config = test_config();
        let engine = DecisionEngine::new(config);
        
        let test_cases = vec![
            (100.0, "EmergencySell"),
            (85.0, "EmergencySell"),
            (55.0, "PartialExit"),
            (35.0, "PartialExit"),
            (25.0, "MonitorOnly"),
            (15.0, "MonitorOnly"),
        ];
        
        for (drain_percent, expected_strategy) in test_cases {
            let alert = ScamAlert {
                alert_id: format!("test_{}_drain", drain_percent),
                severity: if drain_percent >= 80.0 { Severity::Critical } 
                        else if drain_percent >= 50.0 { Severity::High }
                        else { Severity::Medium },
                current_eth_reserve: 10.0,
                simulated_eth_reserve: 10.0 * (1.0 - drain_percent / 100.0),
                eth_change_percent: -drain_percent,
                confidence_score: 0.9,
                ..Default::default()
            };
            
            let strategy = engine.decide_strategy(&alert);
            let strategy_name = match strategy {
                Strategy::EmergencySell { .. } => "EmergencySell",
                Strategy::PartialExit { .. } => "PartialExit",
                Strategy::MonitorOnly { .. } => "MonitorOnly",
                _ => "Unknown",
            };
            
            assert_eq!(strategy_name, expected_strategy, 
                      "Wrong strategy for {}% drain", drain_percent);
        }
    }
    
    #[test]
    fn test_gas_price_escalation() {
        let config = test_config();
        let engine = DecisionEngine::new(config);
        
        // High severity should get higher gas
        let critical_alert = ScamAlert {
            severity: Severity::Critical,
            eth_change_percent: -90.0,
            current_eth_reserve: 5.0,
            gas_price_gwei: 30.0,
            ..Default::default()
        };
        
        let strategy = engine.decide_strategy(&critical_alert);
        
        match strategy {
            Strategy::EmergencySell { gas_multiplier, .. } => {
                assert!(gas_multiplier >= 2.0, "Critical alerts need high gas multiplier");
            }
            _ => panic!("Expected EmergencySell"),
        }
    }
    
    #[test] 
    fn test_confidence_threshold() {
        let config = test_config();
        let engine = DecisionEngine::new(config);
        
        // Low confidence should not trigger action
        let low_confidence_alert = ScamAlert {
            severity: Severity::Critical,
            eth_change_percent: -90.0,
            current_eth_reserve: 5.0,
            confidence_score: 0.3, // Very low confidence
            ..Default::default()
        };
        
        let strategy = engine.decide_strategy(&low_confidence_alert);
        
        match strategy {
            Strategy::MonitorOnly { reason } => {
                assert!(reason.contains("confidence"), "Should skip low confidence alerts");
            }
            _ => panic!("Expected MonitorOnly for low confidence"),
        }
    }
}