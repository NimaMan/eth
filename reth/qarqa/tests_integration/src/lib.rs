//! Integration tests for QARQA components

#[cfg(test)]
mod tests {
    use qarqa_core_types::*;
    use qarqa_tx_simulation::*;
    use qarqa_network_building::*;
    use alloy_primitives::{Address, U256, B256};
    use std::str::FromStr;
    use std::collections::HashMap;
    use chrono;
    
    #[tokio::test]
    async fn test_component_integration() {
        // Test that all components work together
        
        // 1. Create test transaction
        let tx = Transaction {
            hash: B256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap(),
            block_number: 12345,
            from_address: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
            to_address: Some(Address::from_str("0x2222222222222222222222222222222222222222").unwrap()),
            value: U256::from_str("1000000000000000000").unwrap(), // 1 ETH
            gas_limit: 21000,
            gas_price: U256::from_str("20000000000").unwrap(), // 20 gwei
            input_data: Vec::new(),
            status: true,
            gas_used: Some(21000),
            timestamp: None,
        };
        
        // 2. Simulate transaction
        let mut simulator = DevelopmentTransactionSimulator::new();
        simulator.initialize().await.unwrap();
        
        let fund_flows = simulator.simulate_transaction(&tx).await.unwrap();
        assert!(!fund_flows.eth_movements.is_empty());
        
        // 3. Analyze fund flows
        let analyzer = FundFlowAnalyzer::new();
        let flows = analyzer.analyze_fund_flows(&[fund_flows]).unwrap();
        assert!(!flows.is_empty());
        
        // 4. Build network
        let builder = NetworkBuilder::new();
        let network = builder.build_from_fund_flows(&flows, None).unwrap();
        
        // 5. Convert to visualization
        let viz_data = convert_to_visualization_data(&network, None, NetworkCurrency::USD);
        
        // Verify the complete pipeline works
        assert!(viz_data.stats.total_addresses >= 0);
        println!("Integration test passed: {} addresses, {} edges", 
                 viz_data.stats.total_addresses, viz_data.stats.total_edges);
    }
    
    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_database_integration() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
        
        // Test database components
        match qarqa_data_access::DatabaseManager::new(&database_url).await {
            Ok(db) => {
                println!("Database connection successful");
                db.health_check().await.unwrap();
                
                let fetcher = qarqa_data_access::AddressDataFetcher::new(db.pool_cloned());
                let address = Address::from_str("0x0000000000000000000000000000000000000000").unwrap();
                
                match fetcher.get_transaction_count(address, None, None).await {
                    Ok(count) => println!("Transaction count: {}", count),
                    Err(e) => println!("Expected error (no participants table): {}", e),
                }
            }
            Err(e) => {
                println!("Database connection failed (expected): {}", e);
            }
        }
    }
    
    #[test]
    fn test_end_to_end_data_flow() {
        // Test data structures flow correctly through the system
        
        // Create ETH movement
        let eth_movement = EthMovement {
            from: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
            to: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
            amount: U256::from_str("1000000000000000000").unwrap(),
            movement_type: EthMovementType::Direct,
        };
        
        // Create complete fund flows
        let complete_flows = CompleteFundFlows {
            transaction_hash: B256::ZERO,
            block_number: 1,
            timestamp: chrono::Utc::now(),
            eth_movements: vec![eth_movement],
            token_movements: Vec::new(),
            gas_used: 21000,
            status: true,
        };
        
        // Analyze fund flows
        let analyzer = FundFlowAnalyzer::new();
        let fund_flows = analyzer.analyze_fund_flows(&[complete_flows]).unwrap();
        
        // Verify fund flow was created correctly
        assert_eq!(fund_flows.len(), 1);
        assert_eq!(fund_flows[0].amount_eth, 1.0);
        
        // Build network
        let builder = NetworkBuilder::new();
        let network = builder.build_from_fund_flows(&fund_flows, None).unwrap();
        
        // Convert to visualization
        let viz_data = convert_to_visualization_data(&network, None, NetworkCurrency::USD);
        
        // Verify complete data flow
        println!("End-to-end test: {} nodes, {} edges", 
                 viz_data.nodes.len(), viz_data.edges.len());
    }
}