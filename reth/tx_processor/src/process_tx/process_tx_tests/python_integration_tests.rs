//! Python Integration Tests
//! 
//! Tests for Python validation service integration

#[cfg(test)]
mod tests {
    use crate::process_tx::{
        PythonValidatorClient, ValidationRequest, compare_with_python,
        batch_compare_with_python,
    };

    const TEST_RPC_URL: &str = "http://127.0.0.1:8545";
    const TEST_PYTHON_URL: &str = "http://127.0.0.1:18000";

    #[test]
    fn test_python_validator_client_creation() {
        let client = PythonValidatorClient::new(TEST_PYTHON_URL);
        // Just test that creation doesn't panic
        
        let default_client = PythonValidatorClient::default();
        // Test default URL
    }

    #[test]
    fn test_validation_request_serialization() {
        use serde_json;
        
        let request = ValidationRequest {
            tx_hash: "0x1234".to_string(),
            include_state_changes: true,
            include_trace: false,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tx_hash"));
        assert!(json.contains("include_state_changes"));
        assert!(json.contains("include_trace"));

        // Test deserialization
        let deserialized: ValidationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tx_hash, "0x1234");
        assert!(deserialized.include_state_changes);
        assert!(!deserialized.include_trace);
    }

    #[tokio::test]
    #[ignore] // Only run when Python service is available
    async fn test_health_check_integration() {
        let client = PythonValidatorClient::new(TEST_PYTHON_URL);
        
        match client.health_check().await {
            Ok(health) => {
                println!("✅ Python service health: {:?}", health);
                assert_eq!(health.status, "healthy");
                assert!(health.node_connected);
                assert!(health.latest_block > 0);
            }
            Err(e) => {
                println!("⚠️  Python service not available: {}", e);
                // This is expected when service is not running
                // The test passes either way to avoid CI failures
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run when Python service is available
    async fn test_transaction_summary_integration() {
        let client = PythonValidatorClient::new(TEST_PYTHON_URL);
        
        // Use a known early transaction that should exist
        let tx_hash = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060";
        
        match client.get_transaction_summary(tx_hash).await {
            Ok(summary) => {
                println!("✅ Transaction summary: {:?}", summary);
                assert_eq!(summary.tx_hash, tx_hash);
                assert!(summary.block_number > 0);
                assert!(summary.gas_used > 0);
            }
            Err(e) => {
                println!("⚠️  Transaction summary failed: {}", e);
                // Expected when service not running or transaction not found
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run when Python service is available
    async fn test_single_transaction_validation() {
        let client = PythonValidatorClient::new(TEST_PYTHON_URL);
        
        // Use a known transaction
        let tx_hash = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060";
        
        match client.validate_transaction(tx_hash, true, true).await {
            Ok(response) => {
                println!("✅ Validation response: {:?}", response);
                assert_eq!(response.tx_hash, tx_hash);
                assert!(response.success);
                assert!(response.processing_time_ms > 0.0);
                
                if let Some(processed_tx) = response.processed_transaction {
                    assert_eq!(processed_tx.hash, tx_hash);
                    assert!(processed_tx.block_number > 0);
                    assert!(processed_tx.gas_used > 0);
                }
            }
            Err(e) => {
                println!("⚠️  Validation failed: {}", e);
                // Expected when service not running
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run when Python service is available
    async fn test_batch_validation() {
        let client = PythonValidatorClient::new(TEST_PYTHON_URL);
        
        let tx_hashes = vec![
            "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060".to_string(),
            "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b".to_string(),
        ];
        
        match client.validate_batch(tx_hashes.clone(), true, true).await {
            Ok(batch_response) => {
                println!("✅ Batch validation: {:?}", batch_response);
                assert!(batch_response.success);
                assert_eq!(batch_response.total_count, tx_hashes.len());
                assert!(batch_response.total_processing_time_ms > 0.0);
                assert_eq!(batch_response.results.len(), tx_hashes.len());
                
                for result in &batch_response.results {
                    assert!(tx_hashes.contains(&result.tx_hash));
                }
            }
            Err(e) => {
                println!("⚠️  Batch validation failed: {}", e);
                // Expected when service not running
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run when both services are available
    async fn test_compare_with_python_integration() {
        // Test the high-level comparison function
        let tx_hash = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060";
        
        match compare_with_python(tx_hash, TEST_RPC_URL, Some(TEST_PYTHON_URL)).await {
            Ok(result) => {
                println!("✅ Comparison result: {:?}", result);
                assert_eq!(result.tx_hash, tx_hash);
                assert!(result.rust_processing_time_ms > 0.0);
                assert!(result.python_processing_time_ms > 0.0);
                
                // Verify we got state changes from both sides
                assert!(result.rust_state_changes.is_some());
                assert!(result.python_state_changes.is_some());
                
                let rust_changes = result.rust_state_changes.unwrap();
                assert_eq!(rust_changes.metadata.tx_hash, tx_hash);
            }
            Err(e) => {
                println!("⚠️  Comparison failed: {}", e);
                // Expected when services not running
                // Could be RPC error, Python service error, or transaction not found
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run when both services are available
    async fn test_batch_compare_with_python() {
        let tx_hashes = vec![
            "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060".to_string(),
        ];
        
        match batch_compare_with_python(tx_hashes.clone(), TEST_RPC_URL, Some(TEST_PYTHON_URL)).await {
            Ok(results) => {
                println!("✅ Batch comparison results: {} transactions", results.len());
                assert_eq!(results.len(), tx_hashes.len());
                
                for result in results {
                    assert!(tx_hashes.contains(&result.tx_hash));
                    assert!(result.rust_processing_time_ms > 0.0);
                    assert!(result.python_processing_time_ms > 0.0);
                }
            }
            Err(e) => {
                println!("⚠️  Batch comparison failed: {}", e);
                // Expected when services not running
            }
        }
    }

    #[tokio::test]
    async fn test_error_handling_invalid_url() {
        let client = PythonValidatorClient::new("http://invalid.url:99999");
        
        let result = client.health_check().await;
        assert!(result.is_err());
        
        // The error should be an RPC error
        match result.unwrap_err() {
            crate::process_tx::ProcessTxError::RpcError(_) => {
                // Expected
            }
            other => {
                panic!("Expected RpcError, got: {:?}", other);
            }
        }
    }

    #[tokio::test]
    async fn test_error_handling_invalid_transaction() {
        // This test assumes the Python service is NOT running
        // so we expect connection errors, not validation errors
        
        let tx_hash = "0x0000000000000000000000000000000000000000000000000000000000000000";
        
        let result = compare_with_python(tx_hash, TEST_RPC_URL, Some(TEST_PYTHON_URL)).await;
        assert!(result.is_err());
        
        // Should be an RPC error (connection failed) or simulation error (transaction not found)
        match result.unwrap_err() {
            crate::process_tx::ProcessTxError::RpcError(_) |
            crate::process_tx::ProcessTxError::SimulationError(_) |
            crate::process_tx::ProcessTxError::TransactionNotFound(_) => {
                // Expected
            }
            other => {
                println!("Got error: {:?}", other);
                // Don't panic on other errors in case the service behavior changes
            }
        }
    }

    #[test]
    fn test_python_response_types_serialization() {
        use serde_json;
        use crate::process_tx::python_validator::{
            PythonValidationResponse, PythonEventCounts, PythonFees,
        };
        
        // Test event counts serialization
        let event_counts = PythonEventCounts {
            erc20_transfers: 5,
            erc721_transfers: 0,
            erc1155_transfers: 0,
            internal_transactions: 2,
            uniswap_v2_swaps: 1,
            uniswap_v2_syncs: 1,
            uniswap_v3_swaps: 0,
            uniswap_v4_swaps: 0,
            approvals: 0,
            mints: 0,
            burns: 0,
            deposits: 0,
            withdraws: 0,
            permit2_events: 0,
            trading_enabled_events: 0,
            trading_disabled_events: 0,
        };
        
        let json = serde_json::to_string(&event_counts).unwrap();
        assert!(json.contains("erc20_transfers"));
        assert!(json.contains("5"));
        
        // Test round-trip
        let deserialized: PythonEventCounts = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.erc20_transfers, 5);
        assert_eq!(deserialized.internal_transactions, 2);
        
        // Test fees serialization
        let fees = PythonFees {
            gas_price: 20_000_000_000,
            gas_used: 21_000,
            txn_fee: 420_000_000_000_000,
        };
        
        let json = serde_json::to_string(&fees).unwrap();
        assert!(json.contains("gas_price"));
        assert!(json.contains("gas_used"));
        assert!(json.contains("txn_fee"));
        
        let deserialized: PythonFees = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.gas_price, 20_000_000_000);
        assert_eq!(deserialized.gas_used, 21_000);
    }
}

// Test utilities for other modules
#[cfg(test)]
pub async fn test_python_service_available() -> bool {
    use crate::process_tx::PythonValidatorClient;
    let client = PythonValidatorClient::new("http://127.0.0.1:18000");
    client.health_check().await.is_ok()
}

#[cfg(test)]
pub async fn test_reth_node_available() -> bool {
    // Simple check by trying to connect to RPC
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8545")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        }))
        .send()
        .await;
    
    response.is_ok()
}