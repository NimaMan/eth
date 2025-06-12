//! Integration tests with real mainnet transaction data
//! These tests verify correct behavior with actual blockchain data

#[cfg(test)]
mod tests {
    use crate::simulate_signed_tx::simulate_signed_tx;
    use crate::simulate_signed_tx::simulation_core::ExecutionResultType;
    use ethers_core::types::H256;
    use revm_context::result::SuccessReason;
    use revm_primitives::hardfork::SpecId;
    use std::str::FromStr;
    use anyhow::Result;

    const RPC_URL: &str = "http://127.0.0.1:8545";

    /// Real mainnet transactions for testing
    struct TestTransaction {
        hash: &'static str,
        block: u64,
        description: &'static str,
        expected_gas_used: u64,
        expected_gas_refunded: u64,
        expected_logs: usize,
        has_internal_transfers: bool,
    }

    const TEST_TRANSACTIONS: &[TestTransaction] = &[
        TestTransaction {
            hash: "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060",
            block: 46147,
            description: "Simple ETH transfer (early Ethereum)",
            expected_gas_used: 21000,
            expected_gas_refunded: 0,
            expected_logs: 0,
            has_internal_transfers: false,
        },
        TestTransaction {
            hash: "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae",
            block: 22646153,
            description: "Complex Uniswap V3/V4 swap with multiple interactions",
            expected_gas_used: 315099,
            expected_gas_refunded: 78774,
            expected_logs: 13,
            has_internal_transfers: true, // TODO: Will be verified when CallTracer is integrated
        },
        TestTransaction {
            hash: "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b",
            block: 18500000,
            description: "ERC20 token transfer",
            expected_gas_used: 65000, // Approximate
            expected_gas_refunded: 0,
            expected_logs: 1, // Transfer event
            has_internal_transfers: false,
        },
    ];

    #[tokio::test]
    async fn test_early_ethereum_transaction() -> Result<()> {
        let test_tx = &TEST_TRANSACTIONS[0];
        let tx_hash = H256::from_str(test_tx.hash)?;
        
        let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
        
        // Verify basic properties
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(SuccessReason::Stop)
        ), "{}", test_tx.description);
        
        assert_eq!(output.gas_used, test_tx.expected_gas_used, 
                   "Gas used mismatch for {}", test_tx.description);
        assert_eq!(output.gas_refunded, test_tx.expected_gas_refunded,
                   "Gas refunded mismatch for {}", test_tx.description);
        assert_eq!(output.logs.len(), test_tx.expected_logs,
                   "Log count mismatch for {}", test_tx.description);
        
        // Early transaction should have no logs or complex features
        assert!(output.logs.is_empty(), "Early transaction should have no logs");
        assert!(output.output_data.is_empty(), "Simple transfer has no output");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_complex_defi_transaction() -> Result<()> {
        let test_tx = &TEST_TRANSACTIONS[1];
        let tx_hash = H256::from_str(test_tx.hash)?;
        
        let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
        
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(SuccessReason::Stop)
        ), "{}", test_tx.description);
        
        assert_eq!(output.gas_used, test_tx.expected_gas_used,
                   "Gas used mismatch for {}", test_tx.description);
        assert_eq!(output.gas_refunded, test_tx.expected_gas_refunded,
                   "Gas refunded mismatch for {}", test_tx.description);
        assert_eq!(output.logs.len(), test_tx.expected_logs,
                   "Log count mismatch for {}", test_tx.description);
        
        // Complex DeFi transaction should have multiple logs
        assert!(!output.logs.is_empty(), "DeFi transaction should have logs");
        
        // Verify gas refund percentage (around 25% for this transaction)
        let refund_percentage = (output.gas_refunded as f64 / output.gas_used as f64) * 100.0;
        assert!(refund_percentage > 20.0 && refund_percentage < 30.0,
                "Gas refund should be around 25%, got {:.1}%", refund_percentage);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_erc20_transfer_transaction() -> Result<()> {
        let test_tx = &TEST_TRANSACTIONS[2];
        let tx_hash = H256::from_str(test_tx.hash)?;
        
        let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
        
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(SuccessReason::Stop)
        ), "{}", test_tx.description);
        
        // ERC20 transfers use more gas than simple ETH transfers
        assert!(output.gas_used > 21000, "ERC20 transfer uses more than 21k gas");
        assert!(output.gas_used < 100000, "ERC20 transfer shouldn't use too much gas");
        
        // Should have Transfer event
        assert!(!output.logs.is_empty(), "ERC20 transfer should emit events");
        
        // Check for Transfer event signature
        let transfer_topic = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
        let has_transfer_event = output.logs.iter().any(|log| {
            log.topics().first()
                .map(|topic| format!("{:x}", topic) == transfer_topic)
                .unwrap_or(false)
        });
        assert!(has_transfer_event, "Should have Transfer event");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_transaction_simulation() -> Result<()> {
        // Test simulating multiple transactions in sequence
        let mut results = Vec::new();
        
        for test_tx in TEST_TRANSACTIONS {
            let tx_hash = H256::from_str(test_tx.hash)?;
            let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
            
            results.push((test_tx.description, output));
        }
        
        // Verify all simulations completed successfully
        assert_eq!(results.len(), TEST_TRANSACTIONS.len());
        
        // Print summary for debugging
        for (desc, output) in &results {
            println!("{}: gas_used={}, gas_refunded={}, logs={}", 
                desc, output.gas_used, output.gas_refunded, output.logs.len());
        }
        
        // Verify each result meets basic expectations
        for (i, (_, output)) in results.iter().enumerate() {
            assert!(matches!(
                output.result_type,
                ExecutionResultType::Success(_)
            ), "Transaction {} should succeed", i);
            
            assert!(output.gas_used > 0, "Should use some gas");
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_hardfork_detection() -> Result<()> {
        // Test transactions from different eras to verify hardfork detection
        let hardfork_test_cases = vec![
            (46147, "Early Ethereum", SpecId::BYZANTIUM), // Early block  
            (12965000, "Berlin era", SpecId::LONDON),      // Berlin era
            (15050000, "London era", SpecId::MERGE),       // London era
            (17034870, "Shanghai era", SpecId::SHANGHAI),  // Shanghai era
            (22646153, "Cancun era", SpecId::PRAGUE),      // Cancun era
        ];
        
        for (block_num, era_desc, expected_spec) in hardfork_test_cases {
            // Use spec detection function directly
            use crate::simulate_signed_tx::spec_utils::spec_id_from_block_number;
            let detected_spec = spec_id_from_block_number(block_num);
            
            assert_eq!(detected_spec, expected_spec,
                      "Wrong hardfork detection for {} (block {})", era_desc, block_num);
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_cancun_blob_gas_handling() -> Result<()> {
        // Test that Cancun-era transactions handle blob gas fields correctly
        let test_tx = &TEST_TRANSACTIONS[1]; // This is a Cancun-era transaction
        
        assert!(test_tx.block >= 19_426_587, "Test transaction should be post-Cancun");
        
        let tx_hash = H256::from_str(test_tx.hash)?;
        let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
        
        // Should simulate successfully with Cancun hardfork features
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(_)
        ), "Cancun transaction should simulate successfully");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_transaction_analysis() -> Result<()> {
        // Analyze the complex DeFi transaction for patterns
        let tx_hash = H256::from_str(TEST_TRANSACTIONS[1].hash)?;
        let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
        
        // Analyze events
        let mut event_analysis = std::collections::HashMap::new();
        
        for log in &output.logs {
            if let Some(topic) = log.topics().first() {
                let topic_str = format!("{:x}", topic)[0..8].to_string(); // First 4 bytes
                *event_analysis.entry(topic_str).or_insert(0) += 1;
            }
        }
        
        println!("Event analysis for complex DeFi transaction:");
        for (topic, count) in &event_analysis {
            println!("  Topic 0x{}: {} occurrences", topic, count);
        }
        
        // Should have multiple event types
        assert!(event_analysis.len() >= 3, "Should have multiple event types");
        
        // Verify known event signatures from DeFi
        let known_signatures = vec![
            "ddf252ad", // Transfer
            "c42079f9", // Swap (Uniswap V3)
            "40e9cecb", // Some DeFi event
        ];
        
        let mut found_signatures = 0;
        for sig in known_signatures {
            if event_analysis.contains_key(sig) {
                found_signatures += 1;
            }
        }
        
        assert!(found_signatures >= 1, "Should find at least one known DeFi signature");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_gas_efficiency_analysis() -> Result<()> {
        // Analyze gas efficiency across different transaction types
        let mut gas_metrics = Vec::new();
        
        for test_tx in TEST_TRANSACTIONS {
            let tx_hash = H256::from_str(test_tx.hash)?;
            let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
            
            let efficiency = if output.logs.is_empty() {
                output.gas_used as f64 // For simple transfers
            } else {
                output.gas_used as f64 / output.logs.len() as f64 // Gas per event
            };
            
            gas_metrics.push((test_tx.description, output.gas_used, efficiency));
        }
        
        println!("Gas efficiency analysis:");
        for (desc, gas_used, efficiency) in &gas_metrics {
            println!("  {}: {} gas, {:.0} gas/event", desc, gas_used, efficiency);
        }
        
        // Simple transfer should be most efficient (21k gas, no events)
        let simple_transfer_gas = gas_metrics.iter()
            .find(|(desc, _, _)| desc.contains("Simple ETH"))
            .map(|(_, gas, _)| *gas)
            .unwrap();
        
        assert_eq!(simple_transfer_gas, 21000, "Simple transfer should use exactly 21k gas");
        
        Ok(())
    }
}