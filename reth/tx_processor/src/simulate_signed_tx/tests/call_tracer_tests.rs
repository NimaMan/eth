//! Tests for CallTracer functionality
//! These tests verify the internal transfer tracking capabilities

#[cfg(test)]
mod tests {
    use crate::simulate_signed_tx::call_tracer::{CallTracer, CallType, CallTrace};
    use revm_primitives::{Address, U256};
    use alloy_primitives::Bytes;

    #[test]
    fn test_call_tracer_creation() {
        let tracer = CallTracer::new();
        
        // Should start with empty transfers
        assert!(tracer.get_internal_transfers().is_empty());
        assert!(tracer.get_traces().is_empty());
    }

    #[test]
    fn test_manual_internal_transfer_recording() {
        let mut tracer = CallTracer::new();
        
        let from = Address::from([1u8; 20]);
        let to = Address::from([2u8; 20]);
        let value = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
        
        // Record a transfer
        tracer.record_internal_transfer(from, to, value);
        
        let transfers = tracer.get_internal_transfers();
        assert_eq!(transfers.len(), 1);
        
        let transfer = &transfers[0];
        assert_eq!(transfer.from, from);
        assert_eq!(transfer.to, to);
        assert_eq!(transfer.value, value);
        assert_eq!(transfer.depth, 0); // Default depth
        assert_eq!(transfer.call_type, CallType::Call); // Default type
    }

    #[test]
    fn test_multiple_internal_transfers() {
        let mut tracer = CallTracer::new();
        
        // Record multiple transfers
        let transfers_data = vec![
            (Address::from([1u8; 20]), Address::from([2u8; 20]), U256::from(1_000_000_000_000_000_000u128)),
            (Address::from([2u8; 20]), Address::from([3u8; 20]), U256::from(500_000_000_000_000_000u128)),
            (Address::from([3u8; 20]), Address::from([4u8; 20]), U256::from(250_000_000_000_000_000u128)),
        ];
        
        for (from, to, value) in &transfers_data {
            tracer.record_internal_transfer(*from, *to, *value);
        }
        
        let transfers = tracer.get_internal_transfers();
        assert_eq!(transfers.len(), 3);
        
        // Verify all transfers recorded correctly
        for (i, (expected_from, expected_to, expected_value)) in transfers_data.iter().enumerate() {
            assert_eq!(transfers[i].from, *expected_from);
            assert_eq!(transfers[i].to, *expected_to);
            assert_eq!(transfers[i].value, *expected_value);
        }
    }

    #[test]
    fn test_internal_transfer_structure() {
        let mut tracer = CallTracer::new();
        
        let from = Address::from([10u8; 20]);
        let to = Address::from([20u8; 20]);
        let value = U256::from(2_500_000_000_000_000_000u128); // 2.5 ETH
        
        tracer.record_internal_transfer(from, to, value);
        
        let transfers = tracer.get_internal_transfers();
        let transfer = &transfers[0];
        
        // Test the InternalTransfer structure
        assert_eq!(transfer.from, from);
        assert_eq!(transfer.to, to);
        assert_eq!(transfer.value, value);
        assert_eq!(transfer.depth, 0);
        assert_eq!(transfer.call_type, CallType::Call);
        
        // Test that it implements Debug and Clone
        let _debug_str = format!("{:?}", transfer);
        let _cloned_transfer = transfer.clone();
    }

    #[test]
    fn test_call_type_display() {
        // Test the Display implementation for CallType
        assert_eq!(format!("{}", CallType::Call), "CALL");
        assert_eq!(format!("{}", CallType::DelegateCall), "DELEGATECALL");
        assert_eq!(format!("{}", CallType::StaticCall), "STATICCALL");
        assert_eq!(format!("{}", CallType::Create), "CREATE");
        assert_eq!(format!("{}", CallType::Create2), "CREATE2");
    }

    #[test]
    fn test_call_type_equality() {
        // Test PartialEq and Eq for CallType
        assert_eq!(CallType::Call, CallType::Call);
        assert_ne!(CallType::Call, CallType::DelegateCall);
        assert_eq!(CallType::Create, CallType::Create);
        assert_ne!(CallType::Create, CallType::Create2);
    }

    #[test]
    fn test_call_trace_structure() {
        // Test CallTrace structure (even if not used in simulation yet)
        let trace = CallTrace {
            from: Address::from([1u8; 20]),
            to: Some(Address::from([2u8; 20])),
            value: U256::from(1_000_000_000_000_000_000u128),
            input: Bytes::from(vec![0x60, 0x80, 0x60, 0x40]),
            output: Bytes::from(vec![0x00]),
            gas_used: 21000,
            call_type: CallType::Call,
            status: true,
            error: None,
            calls: Vec::new(),
        };
        
        // Verify fields
        assert_eq!(trace.from, Address::from([1u8; 20]));
        assert_eq!(trace.to, Some(Address::from([2u8; 20])));
        assert_eq!(trace.value, U256::from(1_000_000_000_000_000_000u128));
        assert_eq!(trace.gas_used, 21000);
        assert_eq!(trace.call_type, CallType::Call);
        assert!(trace.status);
        // Note: depth is tracked in CallFrame, not CallTrace
        assert!(trace.calls.is_empty());
        
        // Test that it implements Debug and Clone
        let _debug_str = format!("{:?}", trace);
        let _cloned_trace = trace.clone();
    }

    #[test]
    fn test_zero_value_transfers() {
        let mut tracer = CallTracer::new();
        
        // Test recording zero-value transfers (should still be recorded)
        let from = Address::from([5u8; 20]);
        let to = Address::from([6u8; 20]);
        let zero_value = U256::ZERO;
        
        tracer.record_internal_transfer(from, to, zero_value);
        
        let transfers = tracer.get_internal_transfers();
        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].value, U256::ZERO);
    }

    #[test]
    fn test_same_address_transfers() {
        let mut tracer = CallTracer::new();
        
        // Test self-transfers (from == to)
        let addr = Address::from([7u8; 20]);
        let value = U256::from(1_000_000_000_000_000_000u128);
        
        tracer.record_internal_transfer(addr, addr, value);
        
        let transfers = tracer.get_internal_transfers();
        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].from, addr);
        assert_eq!(transfers[0].to, addr);
    }

    #[test]
    fn test_large_value_transfers() {
        let mut tracer = CallTracer::new();
        
        // Test with very large values
        let from = Address::from([8u8; 20]);
        let to = Address::from([9u8; 20]);
        let large_value = U256::MAX;
        
        tracer.record_internal_transfer(from, to, large_value);
        
        let transfers = tracer.get_internal_transfers();
        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].value, U256::MAX);
    }

    #[test]
    fn test_tracer_state_isolation() {
        // Test that different tracers maintain separate state
        let mut tracer1 = CallTracer::new();
        let mut tracer2 = CallTracer::new();
        
        let addr1 = Address::from([10u8; 20]);
        let addr2 = Address::from([11u8; 20]);
        let value = U256::from(1_000_000_000_000_000_000u128);
        
        tracer1.record_internal_transfer(addr1, addr2, value);
        
        // tracer2 should still be empty
        assert_eq!(tracer1.get_internal_transfers().len(), 1);
        assert_eq!(tracer2.get_internal_transfers().len(), 0);
        
        // Add to tracer2
        tracer2.record_internal_transfer(addr2, addr1, value * U256::from(2));
        
        // Both should have different values
        assert_eq!(tracer1.get_internal_transfers().len(), 1);
        assert_eq!(tracer2.get_internal_transfers().len(), 1);
        assert_eq!(tracer1.get_internal_transfers()[0].value, value);
        assert_eq!(tracer2.get_internal_transfers()[0].value, value * U256::from(2));
    }

    #[test]
    fn test_tracer_clone() {
        let mut tracer = CallTracer::new();
        
        let from = Address::from([12u8; 20]);
        let to = Address::from([13u8; 20]);
        let value = U256::from(1_000_000_000_000_000_000u128);
        
        tracer.record_internal_transfer(from, to, value);
        
        // Clone the tracer
        let cloned_tracer = tracer.clone();
        
        // Both should have the same data
        assert_eq!(tracer.get_internal_transfers().len(), cloned_tracer.get_internal_transfers().len());
        assert_eq!(tracer.get_internal_transfers()[0].value, cloned_tracer.get_internal_transfers()[0].value);
        
        // But should be independent (modifying one doesn't affect the other)
        tracer.record_internal_transfer(to, from, value * U256::from(2));
        
        assert_eq!(tracer.get_internal_transfers().len(), 2);
        assert_eq!(cloned_tracer.get_internal_transfers().len(), 1);
    }

    #[test]
    fn test_empty_traces_list() {
        let tracer = CallTracer::new();
        
        // Should start with empty traces
        let traces = tracer.get_traces();
        assert!(traces.is_empty());
    }

    /// Integration test note: The CallTracer is designed to be used as an Inspector
    /// during EVM execution. These tests verify the data structures and basic functionality.
    /// Full integration with REVM execution is tested in the simulation_core_tests.rs file.
    #[test]
    fn test_integration_readiness() {
        let tracer = CallTracer::new();
        
        // Verify the tracer implements the necessary traits for REVM integration
        // This is more of a compile-time check, but we include it for completeness
        
        // Should be cloneable (required for REVM inspector pattern)
        let _cloned = tracer.clone();
        
        // Should have the expected methods
        let _transfers = tracer.get_internal_transfers();
        let _traces = tracer.get_traces();
        
        // The tracer should be ready to implement Inspector trait
        // when integrated with REVM execution
    }
}