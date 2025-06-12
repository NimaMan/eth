//! State Extraction Unit Tests
//! 
//! Tests for core state change extraction functionality

#[cfg(test)]
mod tests {
    use revm_primitives::{Address as RevmAddress, U256 as RevmU256, Log as RevmLog, B256};
    use crate::process_tx::{
        format_token_amount, format_eth_amount, 
        ProcessTxError, PythonCompatibleStateChanges,
        convert_to_python_format, extract_event_counts,
        CalculatedAccountChanges, TokenInfo,
    };
    use crate::process_tx::state_diff_utils::SignedAmount;
    use std::collections::HashMap;

    #[test]
    fn test_format_token_amount_usdc() {
        // USDC has 6 decimals
        let amount = RevmU256::from(1_000_000u64); // 1 USDC
        let formatted = format_token_amount(&amount, 6, false);
        assert_eq!(formatted, "1");

        let amount = RevmU256::from(1_500_000u64); // 1.5 USDC
        let formatted = format_token_amount(&amount, 6, false);
        assert_eq!(formatted, "1.5");

        let amount = RevmU256::from(1_500_000u64); // -1.5 USDC
        let formatted = format_token_amount(&amount, 6, true);
        assert_eq!(formatted, "-1.5");
    }

    #[test]
    fn test_format_token_amount_weth() {
        // WETH has 18 decimals (like ETH)
        let amount = RevmU256::from(1_000_000_000_000_000_000u64); // 1 WETH
        let formatted = format_token_amount(&amount, 18, false);
        assert_eq!(formatted, "1");

        let amount = RevmU256::from(500_000_000_000_000_000u64); // 0.5 WETH
        let formatted = format_token_amount(&amount, 18, false);
        assert_eq!(formatted, "0.5");
    }

    #[test]
    fn test_format_eth_amount() {
        let signed_amount = SignedAmount::new(RevmU256::from(1_000_000_000_000_000_000u64), false);
        let formatted = format_eth_amount(&signed_amount);
        assert_eq!(formatted, "1");

        let signed_amount = SignedAmount::new(RevmU256::from(500_000_000_000_000_000u64), true);
        let formatted = format_eth_amount(&signed_amount);
        assert_eq!(formatted, "-0.5");

        let signed_amount = SignedAmount::zero();
        let formatted = format_eth_amount(&signed_amount);
        assert_eq!(formatted, "0");
    }

    #[test]
    fn test_signed_amount_arithmetic() {
        let amount1 = SignedAmount::new(RevmU256::from(100u64), false); // +100
        let amount2 = SignedAmount::new(RevmU256::from(50u64), true);   // -50
        
        let result = amount1.clone() + amount2;
        assert_eq!(result.absolute_value, RevmU256::from(50u64));
        assert!(!result.is_negative); // +50

        let amount3 = SignedAmount::new(RevmU256::from(200u64), true);  // -200
        let result2 = amount1 + amount3;
        assert_eq!(result2.absolute_value, RevmU256::from(100u64));
        assert!(result2.is_negative); // -100
    }

    #[test]
    fn test_convert_to_python_format() {
        let mut changes = HashMap::new();
        let address = RevmAddress::from([1u8; 20]);
        
        let mut account_changes = CalculatedAccountChanges {
            address,
            eth_net_change: SignedAmount::new(RevmU256::from(500_000_000_000_000_000u64), true), // -0.5 ETH
            ..Default::default()
        };

        // Add a token change
        account_changes.token_infos.push(TokenInfo {
            address: RevmAddress::from([2u8; 20]),
            symbol: "USDC".to_string(),
            decimals: 6,
            net_change: SignedAmount::new(RevmU256::from(1_000_000u64), false), // +1 USDC
        });

        changes.insert(address, account_changes);

        let python_format = convert_to_python_format(
            &changes,
            "0x1234".to_string(),
            12345,
            25.5,
        );

        assert_eq!(python_format.metadata.tx_hash, "0x1234");
        assert_eq!(python_format.metadata.block_number, 12345);
        assert_eq!(python_format.metadata.processing_time_ms, 25.5);
        assert_eq!(python_format.metadata.addresses_affected, 1);
        assert_eq!(python_format.metadata.tokens_involved, 1);

        let address_str = format!("0x{:x}", address);
        let address_change = python_format.state_changes.get(&address_str).unwrap();
        assert_eq!(address_change.eth_net, "-0.5");
        assert_eq!(address_change.token_net.get("USDC").unwrap(), "1");
    }

    #[test]
    fn test_extract_event_counts() {
        // Create mock logs
        let mut logs = Vec::new();
        
        // ERC20 Transfer event signature
        let transfer_topic = B256::from([
            0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 
            0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
            0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 
            0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
        ]);

        // Add ERC20 transfer logs
        for _ in 0..3 {
            logs.push(RevmLog {
                address: RevmAddress::from([1u8; 20]),
                data: vec![0u8; 32].into(),
                topics: vec![transfer_topic, B256::ZERO, B256::ZERO],
            });
        }

        let counts = extract_event_counts(&logs);
        assert_eq!(counts.erc20_transfers, 3);
        assert_eq!(counts.erc721_transfers, 0);
        assert_eq!(counts.internal_transactions, 0);
    }

    #[test]
    fn test_process_tx_error_display() {
        let error = ProcessTxError::TransactionNotFound("0x1234".to_string());
        assert!(error.to_string().contains("Transaction not found"));

        let error = ProcessTxError::SimulationError("Test error".to_string());
        assert!(error.to_string().contains("Transaction simulation failed"));

        let error = ProcessTxError::RpcError("Connection failed".to_string());
        assert!(error.to_string().contains("RPC error"));
    }

    #[test]
    fn test_python_compatible_state_changes_serialization() {
        use serde_json;
        
        let mut state_changes = HashMap::new();
        state_changes.insert("0x1234".to_string(), crate::process_tx::AddressStateChange {
            eth_net: "0.5".to_string(),
            token_net: {
                let mut tokens = HashMap::new();
                tokens.insert("USDC".to_string(), "1000".to_string());
                tokens
            },
        });

        let python_changes = PythonCompatibleStateChanges {
            state_changes,
            metadata: crate::process_tx::ProcessingMetadata {
                tx_hash: "0x1234".to_string(),
                block_number: 12345,
                processing_time_ms: 25.5,
                includes_internal_transfers: true,
                addresses_affected: 1,
                tokens_involved: 1,
            },
        };

        // Test serialization
        let json = serde_json::to_string(&python_changes).unwrap();
        assert!(json.contains("0x1234"));
        assert!(json.contains("0.5"));
        assert!(json.contains("USDC"));
        assert!(json.contains("1000"));

        // Test deserialization
        let deserialized: PythonCompatibleStateChanges = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata.tx_hash, "0x1234");
        assert_eq!(deserialized.metadata.addresses_affected, 1);
    }

    #[test]
    fn test_large_numbers_formatting() {
        // Test very large amounts (common in DeFi)
        let large_amount = RevmU256::from_str_radix("1000000000000000000000000", 10).unwrap(); // 1M tokens with 18 decimals
        let formatted = format_token_amount(&large_amount, 18, false);
        assert_eq!(formatted, "1000000");

        // Test very small amounts
        let small_amount = RevmU256::from(1u64); // 1 wei
        let formatted = format_token_amount(&small_amount, 18, false);
        assert_eq!(formatted, "0.000000000000000001");
    }

    #[test]
    fn test_zero_amounts() {
        let zero = RevmU256::ZERO;
        
        assert_eq!(format_token_amount(&zero, 18, false), "0");
        assert_eq!(format_token_amount(&zero, 6, false), "0");
        assert_eq!(format_token_amount(&zero, 18, true), "0"); // Sign doesn't matter for zero
        
        let signed_zero = SignedAmount::zero();
        assert_eq!(format_eth_amount(&signed_zero), "0");
    }

    #[test]
    fn test_edge_case_decimals() {
        // Test tokens with unusual decimal counts
        let amount = RevmU256::from(123456789u64);
        
        // 2 decimals (like EURS)
        let formatted = format_token_amount(&amount, 2, false);
        assert_eq!(formatted, "1234567.89");
        
        // 8 decimals (like WBTC)
        let formatted = format_token_amount(&amount, 8, false);
        assert_eq!(formatted, "1.23456789");
        
        // 0 decimals (theoretical)
        let formatted = format_token_amount(&amount, 0, false);
        assert_eq!(formatted, "123456789");
    }
}

// Helper functions for other test modules
#[cfg(test)]
pub fn create_test_signed_amount(value: u64, negative: bool) -> crate::process_tx::state_diff_utils::SignedAmount {
    crate::process_tx::state_diff_utils::SignedAmount::new(RevmU256::from(value), negative)
}

#[cfg(test)]
pub fn create_test_address(value: u8) -> RevmAddress {
    RevmAddress::from([value; 20])
}