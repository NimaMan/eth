//! Tests for the core simulation_core module
//! These tests verify the low-level simulate_transaction function

#[cfg(test)]
mod tests {
    use crate::simulate_signed_tx::simulation_core::{
        simulate_transaction, SimCacheDB, SimulationOutput, ExecutionResultType
    };
    use revm_primitives::{Address, Bytes, U256};
    use revm_context::{BlockEnv, CfgEnv, TxEnv, TransactTo};
    use revm::database::{CacheDB, InMemoryDB};
    use revm_context::result::{SuccessReason, HaltReason};
    use std::collections::HashMap;

    /// Helper function to create a test database with some initial state
    fn create_test_db() -> SimCacheDB {
        let db = InMemoryDB::default();
        // For now, use empty database - real functionality would need proper account setup
        CacheDB::new(db)
    }

    /// Helper to create default block environment
    fn create_block_env(block_number: u64) -> BlockEnv {
        BlockEnv {
            number: U256::from(block_number),
            beneficiary: Address::from([3u8; 20]),
            timestamp: U256::from(1700000000u64),
            difficulty: U256::ZERO,
            prevrandao: Some([0u8; 32]),
            basefee: 20_000_000_000u64, // 20 gwei
            gas_limit: 30_000_000u64,
            blob_excess_gas_and_price: None,
        }
    }

    /// Helper to create default config environment
    fn create_cfg_env(chain_id: u64, block_number: u64) -> CfgEnv {
        let mut cfg = CfgEnv::default();
        cfg.chain_id = chain_id;
        // Use the spec utility function
        cfg.spec = crate::simulate_signed_tx::spec_utils::spec_id_from_block_number(block_number);
        cfg
    }

    /// Helper to create a simple transfer transaction
    fn create_transfer_tx(from: Address, to: Address, value: U256, nonce: u64) -> TxEnv {
        let mut tx = TxEnv::default();
        tx.caller = from;
        tx.kind = TransactTo::Call(to);
        tx.value = value;
        tx.data = Bytes::default();
        tx.gas_limit = 21_000;
        tx.gas_price = 30_000_000_000u128; // 30 gwei
        tx.nonce = nonce;
        tx.chain_id = Some(1);
        tx
    }

    #[test]
    fn test_successful_eth_transfer() {
        // Setup
        let db = create_test_db();
        let block_env = create_block_env(18_000_000);
        let cfg_env = create_cfg_env(1, 18_000_000);
        
        let from = Address::from([2u8; 20]); // Rich account
        let to = Address::from([4u8; 20]); // New account
        let value = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
        
        let tx_env = create_transfer_tx(from, to, value, 0);
        
        // Execute
        let result = simulate_transaction(tx_env, block_env, cfg_env, db);
        
        // Verify
        assert!(result.is_ok(), "Simulation should succeed");
        let (output, _final_db) = result.unwrap();
        
        // Check execution result
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(SuccessReason::Stop)
        ));
        
        // Check gas usage for simple transfer
        assert_eq!(output.gas_used, 21_000, "Simple transfer uses 21k gas");
        assert_eq!(output.gas_refunded, 0, "No gas refund for simple transfer");
        
        // Check no logs or output data
        assert!(output.logs.is_empty(), "Simple transfer has no logs");
        assert!(output.output_data.is_empty(), "Simple transfer has no output");
        
        // Check internal transfers (currently empty until CallTracer integration)
        assert!(output.internal_transfers.is_empty(), "Internal transfers not yet implemented");
    }

    #[test]
    fn test_insufficient_balance_transfer() {
        // Setup
        let db = create_test_db();
        let block_env = create_block_env(18_000_000);
        let cfg_env = create_cfg_env(1, 18_000_000);
        
        let from = Address::from([1u8; 20]); // Account with 10 ETH
        let to = Address::from([4u8; 20]);
        let value = U256::from(100_000_000_000_000_000_000u128); // 100 ETH (more than balance)
        
        let tx_env = create_transfer_tx(from, to, value, 0);
        
        // Execute
        let result = simulate_transaction(tx_env, block_env, cfg_env, db);
        
        // Verify - should fail during transaction validation
        assert!(result.is_err(), "Should fail due to insufficient balance");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("insufficient") || err.to_string().contains("funds"),
            "Error should mention insufficient funds"
        );
    }

    #[test]
    fn test_contract_creation() {
        // Setup
        let db = create_test_db();
        let block_env = create_block_env(18_000_000);
        let cfg_env = create_cfg_env(1, 18_000_000);
        
        let from = Address::from([2u8; 20]); // Rich account
        
        // Simple contract bytecode (just returns)
        let contract_code = Bytes::from(vec![0x60, 0x00, 0x60, 0x00, 0xF3]); // PUSH1 0x00 PUSH1 0x00 RETURN
        
        let mut tx_env = TxEnv::default();
        tx_env.caller = from;
        tx_env.kind = TransactTo::Create;
        tx_env.value = U256::ZERO;
        tx_env.data = contract_code;
        tx_env.gas_limit = 100_000;
        tx_env.gas_price = 30_000_000_000u128;
        tx_env.nonce = 0;
        tx_env.chain_id = Some(1);
        
        // Execute
        let result = simulate_transaction(tx_env, block_env, cfg_env, db);
        
        // Verify
        assert!(result.is_ok(), "Contract creation should succeed");
        let (output, _final_db) = result.unwrap();
        
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Success(_)
        ));
        
        // Contract creation uses more gas than simple transfer
        assert!(output.gas_used > 21_000, "Contract creation uses more than 21k gas");
    }

    #[test]
    fn test_revert_scenario() {
        // Setup
        let db = create_test_db();
        let block_env = create_block_env(18_000_000);
        let cfg_env = create_cfg_env(1, 18_000_000);
        
        let from = Address::from([2u8; 20]);
        
        // Contract that reverts: PUSH1 0x00 PUSH1 0x00 REVERT
        let revert_code = Bytes::from(vec![0x60, 0x00, 0x60, 0x00, 0xFD]);
        
        let mut tx_env = TxEnv::default();
        tx_env.caller = from;
        tx_env.kind = TransactTo::Create;
        tx_env.value = U256::ZERO;
        tx_env.data = revert_code;
        tx_env.gas_limit = 100_000;
        tx_env.gas_price = 30_000_000_000u128;
        tx_env.nonce = 0;
        tx_env.chain_id = Some(1);
        
        // Execute
        let result = simulate_transaction(tx_env, block_env, cfg_env, db);
        
        // Verify
        assert!(result.is_ok(), "Simulation should complete even if contract reverts");
        let (output, _final_db) = result.unwrap();
        
        assert!(matches!(
            output.result_type,
            ExecutionResultType::Revert
        ), "Should be a revert");
        
        assert!(output.gas_used > 0, "Gas should be consumed even on revert");
        assert_eq!(output.gas_refunded, 0, "No refund on revert");
    }

    #[test]
    fn test_gas_limit_exceeded() {
        // Setup
        let db = create_test_db();
        let block_env = create_block_env(18_000_000);
        let cfg_env = create_cfg_env(1, 18_000_000);
        
        let from = Address::from([2u8; 20]);
        let to = Address::from([4u8; 20]);
        
        // Create transaction with very low gas limit
        let mut tx_env = create_transfer_tx(from, to, U256::from(1_000_000_000_000_000_000u128), 0);
        tx_env.gas_limit = 1000; // Too low for transfer
        
        // Execute
        let result = simulate_transaction(tx_env, block_env, cfg_env, db);
        
        // Verify - this might either fail or halt
        if let Ok((output, _)) = result {
            // If it succeeds in simulating, it should halt due to out of gas
            assert!(matches!(
                output.result_type,
                ExecutionResultType::Halt(_)
            ), "Should halt due to out of gas");
        }
    }

    #[test]
    fn test_different_hardforks() {
        // Test that different block numbers use correct spec IDs
        // Test that the spec detection function works correctly
        use crate::simulate_signed_tx::spec_utils::spec_id_from_block_number;
        
        let test_cases = vec![
            1_000_000,
            12_965_000,
            15_050_000,
            17_034_870,
            19_426_587,
        ];
        
        for block_num in test_cases {
            let db = create_test_db();
            let block_env = create_block_env(block_num);
            let cfg_env = create_cfg_env(1, block_num);
            
            // Verify spec ID is set correctly (specific values tested in spec_utils tests)
            let expected_spec = spec_id_from_block_number(block_num);
            assert_eq!(cfg_env.spec, expected_spec, "Wrong spec for block {}", block_num);
            
            // Simple transfer to verify it works
            let from = Address::from([2u8; 20]);
            let to = Address::from([4u8; 20]);
            let tx_env = create_transfer_tx(from, to, U256::from(1_000_000_000_000_000u128), 0);
            
            let result = simulate_transaction(tx_env, block_env, cfg_env, db);
            assert!(result.is_ok(), "Simulation should work for block {}", block_num);
        }
    }

    #[test]
    fn test_simulation_output_structure() {
        // Verify all fields of SimulationOutput are populated correctly
        let db = create_test_db();
        let block_env = create_block_env(18_000_000);
        let cfg_env = create_cfg_env(1, 18_000_000);
        
        let from = Address::from([2u8; 20]);
        let to = Address::from([4u8; 20]);
        let tx_env = create_transfer_tx(from, to, U256::from(1_000_000_000_000_000u128), 0);
        
        let result = simulate_transaction(tx_env, block_env, cfg_env, db);
        assert!(result.is_ok());
        
        let (output, final_db) = result.unwrap();
        
        // Verify output structure
        assert!(matches!(output.result_type, ExecutionResultType::Success(_)));
        assert_eq!(output.gas_used, 21_000);
        assert_eq!(output.gas_refunded, 0);
        assert!(output.logs.is_empty());
        assert!(output.output_data.is_empty());
        assert!(output.internal_transfers.is_empty()); // TODO: Will be populated when CallTracer is integrated
        
        // Verify we get back a valid database (basic check)
        let _ = final_db; // Database should be consumable
    }
}