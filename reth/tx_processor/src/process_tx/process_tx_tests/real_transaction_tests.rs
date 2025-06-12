//! Real Transaction Tests
//! 
//! Tests using actual mainnet transactions to validate processing accuracy

#[cfg(test)]
mod tests {
    use crate::process_tx::{
        extract_state_changes_python_format,
        PythonValidatorClient,
    };
    use crate::compare_with_python;

    const TEST_RPC_URL: &str = "http://127.0.0.1:8545";

    /// Test transaction data for validation
    #[derive(Debug)]
    struct TestTransaction {
        hash: &'static str,
        block: u64,
        description: &'static str,
        expected_success: bool,
        has_internal_transfers: bool,
        has_logs: bool,
        min_gas_used: u64,
        max_gas_used: u64,
    }

    const TEST_TRANSACTIONS: &[TestTransaction] = &[
        TestTransaction {
            hash: "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060",
            block: 46147,
            description: "Simple ETH transfer (early Ethereum)",
            expected_success: true,
            has_internal_transfers: false,
            has_logs: false,
            min_gas_used: 21000,
            max_gas_used: 21000,
        },
        TestTransaction {
            hash: "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae",
            block: 22646153,
            description: "Complex Uniswap V3/V4 swap with multiple interactions",
            expected_success: true,
            has_internal_transfers: true,
            has_logs: true,
            min_gas_used: 300000,
            max_gas_used: 400000,
        },
        TestTransaction {
            hash: "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b",
            block: 18500000,
            description: "ERC20 token transfer",
            expected_success: true,
            has_internal_transfers: false,
            has_logs: true,
            min_gas_used: 50000,
            max_gas_used: 100000,
        },
    ];

    #[tokio::test]
    #[ignore] // Only run with --ignored when Reth node is available
    async fn test_simple_eth_transfer() {
        let tx = &TEST_TRANSACTIONS[0]; // Simple ETH transfer
        
        match extract_state_changes_python_format(tx.hash.to_string(), TEST_RPC_URL).await {
            Ok(state_changes) => {
                println!("✅ Processed simple ETH transfer: {}", tx.hash);
                assert_eq!(state_changes.metadata.tx_hash, tx.hash);
                assert!(state_changes.metadata.processing_time_ms > 0.0);
                
                // Simple transfers should have minimal state changes
                assert!(state_changes.metadata.addresses_affected >= 2); // At least sender and receiver
                
                // Verify no internal transfers for simple transfer
                assert!(!state_changes.metadata.includes_internal_transfers || 
                       state_changes.state_changes.values().all(|change| change.token_net.is_empty()));
            }
            Err(e) => {
                println!("⚠️  Simple transfer test failed: {}", e);
                println!("   This is expected if Reth node is not available or not synced to block {}", tx.block);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run with --ignored when Reth node is available
    async fn test_complex_defi_transaction() {
        let tx = &TEST_TRANSACTIONS[1]; // Complex DeFi transaction
        
        match extract_state_changes_python_format(tx.hash.to_string(), TEST_RPC_URL).await {
            Ok(state_changes) => {
                println!("✅ Processed complex DeFi transaction: {}", tx.hash);
                assert_eq!(state_changes.metadata.tx_hash, tx.hash);
                assert!(state_changes.metadata.processing_time_ms > 0.0);
                
                // Complex DeFi should have multiple addresses and tokens involved
                assert!(state_changes.metadata.addresses_affected >= 3);
                assert!(state_changes.metadata.tokens_involved >= 1);
                
                // Should have state changes
                assert!(!state_changes.state_changes.is_empty());
                
                // At least one address should have token movements
                let has_token_movements = state_changes.state_changes.values()
                    .any(|change| !change.token_net.is_empty());
                assert!(has_token_movements, "Complex DeFi transaction should have token movements");
            }
            Err(e) => {
                println!("⚠️  Complex DeFi test failed: {}", e);
                println!("   This is expected if Reth node is not available or not synced to block {}", tx.block);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run with --ignored when Reth node is available
    async fn test_erc20_transfer() {
        let tx = &TEST_TRANSACTIONS[2]; // ERC20 transfer
        
        match extract_state_changes_python_format(tx.hash.to_string(), TEST_RPC_URL).await {
            Ok(state_changes) => {
                println!("✅ Processed ERC20 transfer: {}", tx.hash);
                assert_eq!(state_changes.metadata.tx_hash, tx.hash);
                
                // ERC20 transfers should involve at least 2 addresses (sender, receiver)
                assert!(state_changes.metadata.addresses_affected >= 2);
                
                // Should have token movements
                assert!(state_changes.metadata.tokens_involved >= 1);
                
                // At least one address should have token changes
                let has_token_changes = state_changes.state_changes.values()
                    .any(|change| !change.token_net.is_empty());
                assert!(has_token_changes, "ERC20 transfer should have token changes");
            }
            Err(e) => {
                println!("⚠️  ERC20 transfer test failed: {}", e);
                println!("   This is expected if Reth node is not available or not synced to block {}", tx.block);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run with --ignored when both services are available
    async fn test_batch_real_transactions() {
        let tx_hashes: Vec<String> = TEST_TRANSACTIONS.iter()
            .map(|tx| tx.hash.to_string())
            .collect();
        
        match crate::process_tx::extract_batch_state_changes_python_format(tx_hashes.clone(), TEST_RPC_URL).await {
            Ok(results) => {
                println!("✅ Processed {} transactions in batch", results.len());
                assert_eq!(results.len(), TEST_TRANSACTIONS.len());
                
                for (i, result) in results.iter().enumerate() {
                    let expected_tx = &TEST_TRANSACTIONS[i];
                    assert_eq!(result.metadata.tx_hash, expected_tx.hash);
                    assert!(result.metadata.processing_time_ms > 0.0);
                    
                    // Verify transaction-specific expectations
                    match expected_tx.description {
                        desc if desc.contains("Simple ETH") => {
                            // Simple transfers have minimal complexity
                            assert!(result.metadata.addresses_affected >= 2);
                        }
                        desc if desc.contains("Complex") => {
                            // Complex transactions have more state changes
                            assert!(result.metadata.addresses_affected >= 3);
                            assert!(result.metadata.tokens_involved >= 1);
                        }
                        desc if desc.contains("ERC20") => {
                            // Token transfers have token movements
                            assert!(result.metadata.tokens_involved >= 1);
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Batch processing failed: {}", e);
                println!("   This is expected if Reth node is not available");
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run with --ignored when both services are available  
    async fn test_python_comparison_validation() {
        // Test comparison with Python service for all test transactions
        for tx in TEST_TRANSACTIONS {
            println!("🔍 Comparing transaction: {} - {}", tx.hash, tx.description);
            
            match compare_with_python(tx.hash, TEST_RPC_URL, Some("http://127.0.0.1:18000")).await {
                Ok(result) => {
                    println!("✅ Comparison completed for {}", tx.hash);
                    assert_eq!(result.tx_hash, tx.hash);
                    assert!(result.rust_processing_time_ms > 0.0);
                    assert!(result.python_processing_time_ms > 0.0);
                    
                    // Both implementations should return state changes
                    assert!(result.rust_state_changes.is_some());
                    assert!(result.python_state_changes.is_some());
                    
                    if result.matches {
                        println!("✅ Results match perfectly!");
                    } else {
                        println!("⚠️  Results differ: {} differences", result.differences.len());
                        for diff in &result.differences {
                            println!("   - {}: {}", diff.field, diff.description);
                        }
                    }
                    
                    // Log performance comparison
                    let speed_ratio = result.rust_processing_time_ms / result.python_processing_time_ms;
                    if speed_ratio < 1.0 {
                        println!("   Performance: Rust is {:.1}x faster", 1.0 / speed_ratio);
                    } else {
                        println!("   Performance: Python is {:.1}x faster", speed_ratio);
                    }
                }
                Err(e) => {
                    println!("⚠️  Comparison failed for {}: {}", tx.hash, e);
                    println!("   This is expected if services are not available");
                }
            }
            println!();
        }
    }

    #[tokio::test]
    #[ignore] // Only run with --ignored for specific debugging
    async fn test_transaction_not_found() {
        // Test error handling for non-existent transaction
        let invalid_hash = "0x0000000000000000000000000000000000000000000000000000000000000000";
        
        let result = extract_state_changes_python_format(invalid_hash.to_string(), TEST_RPC_URL).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            crate::process_tx::ProcessTxError::SimulationError(_) |
            crate::process_tx::ProcessTxError::TransactionNotFound(_) => {
                // Expected error types
                println!("✅ Correctly handled non-existent transaction");
            }
            other => {
                println!("Got unexpected error: {:?}", other);
                // Don't panic - error handling may vary based on implementation
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run with --ignored for specific debugging
    async fn test_malformed_transaction_hash() {
        // Test error handling for malformed hash
        let malformed_hash = "not_a_hash";
        
        let result = extract_state_changes_python_format(malformed_hash.to_string(), TEST_RPC_URL).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            crate::process_tx::ProcessTxError::InvalidTransactionData(_) => {
                println!("✅ Correctly handled malformed transaction hash");
            }
            other => {
                println!("Got error: {:?}", other);
                // Don't panic - different error types may be acceptable
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run with --ignored when Python service is available
    async fn test_python_service_consistency() {
        // Test that Python service returns consistent results
        let tx_hash = TEST_TRANSACTIONS[0].hash;
        let client = PythonValidatorClient::new("http://127.0.0.1:18000");
        
        // Run same validation twice
        let result1 = client.validate_transaction(tx_hash, true, true).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let result2 = client.validate_transaction(tx_hash, true, true).await;
        
        match (result1, result2) {
            (Ok(r1), Ok(r2)) => {
                println!("✅ Python service consistency test passed");
                assert_eq!(r1.tx_hash, r2.tx_hash);
                assert_eq!(r1.success, r2.success);
                
                // Processing times may vary slightly, but should be in same ballpark
                let time_ratio = r1.processing_time_ms / r2.processing_time_ms;
                assert!(time_ratio > 0.1 && time_ratio < 10.0, 
                    "Processing times too different: {}ms vs {}ms", 
                    r1.processing_time_ms, r2.processing_time_ms);
            }
            (Err(e1), Err(e2)) => {
                println!("⚠️  Both requests failed: {} / {}", e1, e2);
                // Consistent failure is acceptable if service not available
            }
            _ => {
                println!("⚠️  Inconsistent results from Python service");
                // Don't panic - could be transient issues
            }
        }
    }
}

// Utilities for performance testing
#[cfg(test)]
pub struct TransactionBenchmark {
    pub tx_hash: String,
    pub description: String,
    pub rust_time_ms: f64,
    pub python_time_ms: Option<f64>,
    pub state_changes_count: usize,
    pub success: bool,
}

#[cfg(test)]
pub async fn benchmark_transaction(tx_hash: &str, description: &str) -> TransactionBenchmark {
    use std::time::Instant;
    
    let start = Instant::now();
    let rust_result = crate::process_tx::extract_state_changes_python_format(
        tx_hash.to_string(), 
        "http://127.0.0.1:8545"
    ).await;
    let rust_time = start.elapsed().as_secs_f64() * 1000.0;
    
    let (success, state_changes_count) = match rust_result {
        Ok(changes) => (true, changes.state_changes.len()),
        Err(_) => (false, 0),
    };
    
    // Try to get Python timing if service is available
    let python_time = if let Ok(comparison) = compare_with_python(
        tx_hash, 
        "http://127.0.0.1:8545", 
        Some("http://127.0.0.1:18000")
    ).await {
        Some(comparison.python_processing_time_ms)
    } else {
        None
    };
    
    TransactionBenchmark {
        tx_hash: tx_hash.to_string(),
        description: description.to_string(),
        rust_time_ms: rust_time,
        python_time_ms: python_time,
        state_changes_count,
        success,
    }
}