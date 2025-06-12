//! Comparison Logic Tests
//! 
//! Unit tests for state change comparison logic between Rust and Python

#[cfg(test)]
mod tests {
    use crate::process_tx::python_validator::{
        compare_state_changes, ComparisonResult, ValidationDifference,
    };
    use crate::process_tx::{
        PythonCompatibleStateChanges, AddressStateChange, ProcessingMetadata,
    };
    use std::collections::HashMap;
    use serde_json;

    fn create_test_rust_changes(address: &str, eth_net: &str, tokens: Vec<(&str, &str)>) -> PythonCompatibleStateChanges {
        let mut state_changes = HashMap::new();
        let mut token_net = HashMap::new();
        
        for (token, amount) in tokens {
            token_net.insert(token.to_string(), amount.to_string());
        }
        
        state_changes.insert(address.to_string(), AddressStateChange {
            eth_net: eth_net.to_string(),
            token_net,
        });
        
        PythonCompatibleStateChanges {
            state_changes,
            metadata: ProcessingMetadata {
                tx_hash: "0x1234".to_string(),
                block_number: 18500000,
                processing_time_ms: 25.5,
                addresses_affected: 1,
                tokens_involved: tokens.len(),
            }
        }
    }

    fn create_test_python_tx(state_changes: HashMap<String, serde_json::Value>) -> Option<crate::process_tx::python_validator::PythonProcessedTransaction> {
        use crate::process_tx::python_validator::{PythonProcessedTransaction, PythonEventCounts, PythonFees};
        
        Some(PythonProcessedTransaction {
            hash: "0x1234".to_string(),
            block_number: 18500000,
            block_timestamp: 1234567890,
            txn_index: 0,
            from_address: "0xabcd".to_string(),
            to_address: Some("0xefgh".to_string()),
            contract_address: None,
            value: 0,
            status: 1,
            nonce: 1,
            input: "0x".to_string(),
            txn_type: "transfer".to_string(),
            actions: vec!["transfer".to_string()],
            fees: Some(PythonFees {
                gas_price: 20_000_000_000,
                gas_used: 21000,
                txn_fee: 420_000_000_000_000,
            }),
            bribe_amount: None,
            unique_addresses: vec!["0xabcd".to_string(), "0xefgh".to_string()],
            erc20_contracts: vec![],
            event_counts: PythonEventCounts {
                erc20_transfers: 0,
                erc721_transfers: 0,
                erc1155_transfers: 0,
                internal_transactions: 0,
                uniswap_v2_swaps: 0,
                uniswap_v2_syncs: 0,
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
            },
            erc20_transfers: vec![],
            internal_transactions: vec![],
            uniswap_v2_swaps: vec![],
            uniswap_v4_swaps: vec![],
            state_changes,
        })
    }

    #[test]
    fn test_perfect_match() {
        // Test Case 1: Perfect match between Rust and Python
        let rust_changes = create_test_rust_changes(
            "0x1234",
            "1.5",
            vec![("USDC", "1000.0"), ("WETH", "0.5")]
        );

        let mut python_state_changes = HashMap::new();
        python_state_changes.insert("0x1234".to_string(), serde_json::json!({
            "eth_net": "1.5",
            "token_net": {
                "USDC": "1000.0",
                "WETH": "0.5"
            }
        }));

        let python_tx = create_test_python_tx(python_state_changes);
        let result = compare_state_changes(&rust_changes, &python_tx);

        assert!(result.matches, "Perfect match should return true");
        assert!(result.differences.is_empty(), "Perfect match should have no differences");
    }

    #[test]
    fn test_zero_address_acceptable() {
        // Test Case 2: Zero address differences (acceptable)
        // Rust has address with zero changes, Python omits it
        let rust_changes = create_test_rust_changes(
            "0x5678",
            "0",
            vec![]
        );

        let python_state_changes = HashMap::new(); // Python omits zero-change address
        let python_tx = create_test_python_tx(python_state_changes);
        let result = compare_state_changes(&rust_changes, &python_tx);

        assert!(result.matches, "Zero address omission should be acceptable");
        assert!(result.differences.is_empty(), "Zero address should have no differences");
    }

    #[test]
    fn test_formatting_differences_acceptable() {
        // Test Case 3: Formatting differences (acceptable)
        let rust_changes = create_test_rust_changes(
            "0x1234",
            "1.500000000000000000",
            vec![("USDC", "1000.000000")]
        );

        let mut python_state_changes = HashMap::new();
        python_state_changes.insert("0x1234".to_string(), serde_json::json!({
            "eth_net": "1.5",
            "token_net": {
                "USDC": "1000"
            }
        }));

        let python_tx = create_test_python_tx(python_state_changes);
        let result = compare_state_changes(&rust_changes, &python_tx);

        assert!(result.matches, "Formatting differences should be acceptable");
        assert!(result.differences.is_empty(), "Formatting differences should be normalized");
    }

    #[test]
    fn test_value_mismatch_error() {
        // Test Case 4: Value mismatch (error)
        let rust_changes = create_test_rust_changes(
            "0x1234",
            "1.5",
            vec![("USDC", "1000")]
        );

        let mut python_state_changes = HashMap::new();
        python_state_changes.insert("0x1234".to_string(), serde_json::json!({
            "eth_net": "1.6", // Different value
            "token_net": {
                "USDC": "1000"
            }
        }));

        let python_tx = create_test_python_tx(python_state_changes);
        let result = compare_state_changes(&rust_changes, &python_tx);

        assert!(!result.matches, "Value mismatch should not match");
        assert!(!result.differences.is_empty(), "Value mismatch should have differences");
        
        // Check for ETH difference
        let eth_diff = result.differences.iter().find(|d| d.field.contains("eth_net"));
        assert!(eth_diff.is_some(), "Should have ETH net difference");
    }

    #[test]
    fn test_missing_token_error() {
        // Test Case 5: Missing token (error)
        let rust_changes = create_test_rust_changes(
            "0x1234",
            "0",
            vec![("USDC", "1000"), ("WETH", "0.5")]
        );

        let mut python_state_changes = HashMap::new();
        python_state_changes.insert("0x1234".to_string(), serde_json::json!({
            "eth_net": "0",
            "token_net": {
                "USDC": "1000"
                // WETH missing
            }
        }));

        let python_tx = create_test_python_tx(python_state_changes);
        let result = compare_state_changes(&rust_changes, &python_tx);

        assert!(!result.matches, "Missing token should not match");
        assert!(!result.differences.is_empty(), "Missing token should have differences");
        
        // Check for WETH difference
        let weth_diff = result.differences.iter().find(|d| d.field.contains("WETH"));
        assert!(weth_diff.is_some(), "Should have WETH difference");
    }

    #[test]
    fn test_zero_token_omission_acceptable() {
        // Test Case 6: Zero token omission (acceptable)
        let rust_changes = create_test_rust_changes(
            "0x1234",
            "1.5",
            vec![("USDC", "0"), ("WETH", "0.5")] // Zero USDC
        );

        let mut python_state_changes = HashMap::new();
        python_state_changes.insert("0x1234".to_string(), serde_json::json!({
            "eth_net": "1.5",
            "token_net": {
                "WETH": "0.5"
                // USDC omitted because it's zero
            }
        }));

        let python_tx = create_test_python_tx(python_state_changes);
        let result = compare_state_changes(&rust_changes, &python_tx);

        assert!(result.matches, "Zero token omission should be acceptable");
        assert!(result.differences.is_empty(), "Zero token omission should have no differences");
    }

    #[test]
    fn test_decimal_normalization() {
        // Test decimal normalization logic
        use crate::process_tx::python_validator::normalize_decimal_string;
        
        // Test various decimal formats that should be equivalent
        assert_eq!(normalize_decimal_string("1.5"), normalize_decimal_string("1.500000"));
        assert_eq!(normalize_decimal_string("0"), normalize_decimal_string("0.0"));
        assert_eq!(normalize_decimal_string("0"), normalize_decimal_string("0.000000"));
        assert_eq!(normalize_decimal_string("1000"), normalize_decimal_string("1000.0"));
        
        // Test that they normalize to expected values
        assert_eq!(normalize_decimal_string("1.500000"), "1.5");
        assert_eq!(normalize_decimal_string("0.000000"), "0");
        assert_eq!(normalize_decimal_string("1000.0"), "1000");
    }

    #[test]
    fn test_amounts_equal() {
        // Test the amounts_equal function
        use crate::process_tx::python_validator::amounts_equal;
        
        // Should be equal
        assert!(amounts_equal("1.5", "1.500000"));
        assert!(amounts_equal("0", "0.0"));
        assert!(amounts_equal("1000", "1000.0"));
        assert!(amounts_equal("-1.5", "-1.500000"));
        
        // Should not be equal
        assert!(!amounts_equal("1.5", "1.6"));
        assert!(!amounts_equal("1.5", "-1.5"));
        assert!(!amounts_equal("0", "0.1"));
    }

    #[test]
    fn test_complex_scenario() {
        // Test a complex scenario with multiple addresses and tokens
        let mut rust_state_changes = HashMap::new();
        
        // Address 1: Has ETH and tokens
        rust_state_changes.insert("0x1111".to_string(), AddressStateChange {
            eth_net: "1.5".to_string(),
            token_net: {
                let mut tokens = HashMap::new();
                tokens.insert("USDC".to_string(), "1000.0".to_string());
                tokens.insert("WETH".to_string(), "0.5".to_string());
                tokens
            },
        });
        
        // Address 2: Zero changes (should be omitted by Python)
        rust_state_changes.insert("0x2222".to_string(), AddressStateChange {
            eth_net: "0".to_string(),
            token_net: HashMap::new(),
        });
        
        let rust_changes = PythonCompatibleStateChanges {
            state_changes: rust_state_changes,
            metadata: ProcessingMetadata {
                tx_hash: "0x1234".to_string(),
                block_number: 18500000,
                processing_time_ms: 25.5,
                addresses_affected: 2,
                tokens_involved: 2,
            }
        };
        
        // Python only has non-zero address
        let mut python_state_changes = HashMap::new();
        python_state_changes.insert("0x1111".to_string(), serde_json::json!({
            "eth_net": "1.5",
            "token_net": {
                "USDC": "1000.0",
                "WETH": "0.5"
            }
        }));
        
        let python_tx = create_test_python_tx(python_state_changes);
        let result = compare_state_changes(&rust_changes, &python_tx);
        
        assert!(result.matches, "Complex scenario should match (zero address omission is acceptable)");
        assert!(result.differences.is_empty(), "Complex scenario should have no differences");
    }
}