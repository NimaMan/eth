//! Unit tests for optimized transaction data retrieval module

#[cfg(test)]
mod tests {
    use super::{
        types::{TransactionType, DataLevel, TransactionDataOptions},
        heuristics::{detect_transaction_type, needs_internal_transfers, should_use_simulation},
    };
    use revm_primitives::{Address as RevmAddress, U256 as RevmU256};
    use std::str::FromStr;

    #[test]
    fn test_transaction_type_detection() {
        // Test simple transfer
        let simple_transfer = detect_transaction_type(
            Some(RevmAddress::from_str("0x742d35cc6571c4c8d8e9c8e3a6d3b8a2b4c4d5e6").unwrap()),
            RevmU256::from(1000000000000000000u64), // 1 ETH
            &[], // Empty input
            21000, // Standard gas limit
        );
        assert_eq!(simple_transfer, TransactionType::SimpleTransfer);

        // Test contract deployment
        let contract_deployment = detect_transaction_type(
            None, // No to address
            RevmU256::ZERO,
            &[0x60, 0x80, 0x60, 0x40], // Sample bytecode
            2000000,
        );
        assert_eq!(contract_deployment, TransactionType::ContractDeployment);

        // Test DEX interaction
        let uniswap_router = RevmAddress::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap();
        let dex_interaction = detect_transaction_type(
            Some(uniswap_router),
            RevmU256::ZERO,
            &[0xa9, 0x05, 0x9c, 0xbb], // swapExactTokensForTokens selector
            200000,
        );
        assert_eq!(dex_interaction, TransactionType::DexInteraction);

        // Test contract call
        let contract_call = detect_transaction_type(
            Some(RevmAddress::from_str("0x1234567890123456789012345678901234567890").unwrap()),
            RevmU256::ZERO,
            &[0xa9, 0x05, 0x9c, 0xaa], // Unknown function selector
            150000,
        );
        assert_eq!(contract_call, TransactionType::ContractCall);
    }

    #[test]
    fn test_internal_transfers_heuristics() {
        let basic_options = TransactionDataOptions::basic();
        let smart_options = TransactionDataOptions::smart();
        let complete_options = TransactionDataOptions::complete();
        let force_options = TransactionDataOptions::with_internal_transfers();

        // Test simple transfer - should not need simulation
        assert!(!needs_internal_transfers(&TransactionType::SimpleTransfer, &basic_options));
        assert!(!needs_internal_transfers(&TransactionType::SimpleTransfer, &smart_options));
        assert!(needs_internal_transfers(&TransactionType::SimpleTransfer, &complete_options));
        assert!(needs_internal_transfers(&TransactionType::SimpleTransfer, &force_options));

        // Test DEX interaction - should need simulation in smart/complete modes
        assert!(!needs_internal_transfers(&TransactionType::DexInteraction, &basic_options));
        assert!(needs_internal_transfers(&TransactionType::DexInteraction, &smart_options));
        assert!(needs_internal_transfers(&TransactionType::DexInteraction, &complete_options));
        assert!(needs_internal_transfers(&TransactionType::DexInteraction, &force_options));

        // Test contract call - conservative approach
        assert!(!needs_internal_transfers(&TransactionType::ContractCall, &basic_options));
        assert!(!needs_internal_transfers(&TransactionType::ContractCall, &smart_options));
        assert!(needs_internal_transfers(&TransactionType::ContractCall, &complete_options));
        assert!(needs_internal_transfers(&TransactionType::ContractCall, &force_options));

        // Test contract deployment - should not need simulation
        assert!(!needs_internal_transfers(&TransactionType::ContractDeployment, &basic_options));
        assert!(!needs_internal_transfers(&TransactionType::ContractDeployment, &smart_options));
        assert!(needs_internal_transfers(&TransactionType::ContractDeployment, &complete_options));
        assert!(needs_internal_transfers(&TransactionType::ContractDeployment, &force_options));
    }

    #[test]
    fn test_simulation_heuristics() {
        let basic_options = TransactionDataOptions::basic();
        let smart_options = TransactionDataOptions::smart();
        let complete_options = TransactionDataOptions::complete();

        // Test that simulation heuristics match internal transfer heuristics
        assert_eq!(
            should_use_simulation(&TransactionType::SimpleTransfer, &basic_options),
            needs_internal_transfers(&TransactionType::SimpleTransfer, &basic_options)
        );

        assert_eq!(
            should_use_simulation(&TransactionType::DexInteraction, &smart_options),
            needs_internal_transfers(&TransactionType::DexInteraction, &smart_options)
        );

        assert_eq!(
            should_use_simulation(&TransactionType::ComplexDeFi, &complete_options),
            needs_internal_transfers(&TransactionType::ComplexDeFi, &complete_options)
        );
    }

    #[test]
    fn test_transaction_data_options() {
        // Test basic options
        let basic = TransactionDataOptions::basic();
        assert_eq!(basic.force_level, Some(DataLevel::Basic));
        assert!(!basic.need_internal_transfers);
        assert!(!basic.need_call_trace);
        assert!(!basic.need_state_changes);

        // Test smart options
        let smart = TransactionDataOptions::smart();
        assert_eq!(smart.force_level, Some(DataLevel::Smart));
        assert!(!smart.need_internal_transfers);
        assert!(!smart.need_call_trace);
        assert!(!smart.need_state_changes);

        // Test complete options
        let complete = TransactionDataOptions::complete();
        assert_eq!(complete.force_level, Some(DataLevel::Complete));
        assert!(complete.need_internal_transfers);
        assert!(complete.need_call_trace);
        assert!(complete.need_state_changes);

        // Test with internal transfers
        let with_internal = TransactionDataOptions::with_internal_transfers();
        assert!(with_internal.need_internal_transfers);
    }

    #[test]
    fn test_data_level_enum() {
        assert_eq!(DataLevel::Basic, DataLevel::Basic);
        assert_ne!(DataLevel::Basic, DataLevel::Smart);
        assert_ne!(DataLevel::Smart, DataLevel::Complete);
    }

    #[test]
    fn test_transaction_type_serialization() {
        use serde_json;

        let tx_type = TransactionType::DexInteraction;
        let serialized = serde_json::to_string(&tx_type).unwrap();
        let deserialized: TransactionType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(tx_type, deserialized);
    }

    #[test]
    fn test_function_selector_detection() {
        // Test common DEX function selectors
        let swap_selector = &[0xa9, 0x05, 0x9c, 0xbb]; // swapExactTokensForTokens
        let tx_type = detect_transaction_type(
            Some(RevmAddress::from_str("0x1234567890123456789012345678901234567890").unwrap()),
            RevmU256::ZERO,
            swap_selector,
            200000,
        );
        assert_eq!(tx_type, TransactionType::DexInteraction);

        // Test multicall selector
        let multicall_selector = &[0xac, 0x96, 0x50, 0xd8]; // multicall
        let tx_type = detect_transaction_type(
            Some(RevmAddress::from_str("0x1234567890123456789012345678901234567890").unwrap()),
            RevmU256::ZERO,
            multicall_selector,
            500000,
        );
        assert_eq!(tx_type, TransactionType::ComplexDeFi);
    }

    #[test]
    fn test_gas_limit_heuristics() {
        // Low gas limit should suggest simple transfer
        let low_gas = detect_transaction_type(
            Some(RevmAddress::from_str("0x742d35cc6571c4c8d8e9c8e3a6d3b8a2b4c4d5e6").unwrap()),
            RevmU256::from(1000000000000000000u64),
            &[],
            21000,
        );
        assert_eq!(low_gas, TransactionType::SimpleTransfer);

        // High gas limit should suggest contract call
        let high_gas = detect_transaction_type(
            Some(RevmAddress::from_str("0x742d35cc6571c4c8d8e9c8e3a6d3b8a2b4c4d5e6").unwrap()),
            RevmU256::ZERO,
            &[],
            150000,
        );
        assert_eq!(high_gas, TransactionType::ContractCall);
    }

    #[test] 
    fn test_known_contract_addresses() {
        // Test Uniswap V2 Router
        let uniswap_v2 = RevmAddress::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap();
        let tx_type = detect_transaction_type(Some(uniswap_v2), RevmU256::ZERO, &[], 200000);
        assert_eq!(tx_type, TransactionType::DexInteraction);

        // Test 1inch Router
        let oneinch = RevmAddress::from_str("0x1111111254EEB25477B68fb85Ed929f73A960582").unwrap();
        let tx_type = detect_transaction_type(Some(oneinch), RevmU256::ZERO, &[], 200000);
        assert_eq!(tx_type, TransactionType::DexInteraction);

        // Test unknown address with no special characteristics
        let unknown = RevmAddress::from_str("0x1234567890123456789012345678901234567890").unwrap();
        let tx_type = detect_transaction_type(Some(unknown), RevmU256::ZERO, &[], 50000);
        assert_eq!(tx_type, TransactionType::ContractCall);
    }
}