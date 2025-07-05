/// State Change Extractor for Direct Reth Simulator
/// 
/// Extracts account state changes from TracingInspector output to provide
/// the same data format as debug_traceCall for compatibility with signal detection.

use std::collections::HashMap;
use eyre::Result;
use alloy_primitives::{Address, U256};
use revm_primitives::{Address as RevmAddress, U256 as RevmU256};

// Import state change types from revm_tx_simulator
use revm_tx_simulator_lib::process_tx::state_diff_utils::{
    CalculatedAccountChanges, SignedAmount, AccountMovements, 
    EthMovementsInOut, EthMovement
};
use crate::common::address::to_checksum_address;

/// Extracts state changes from TracingInspector traces
pub struct StateChangeExtractor;

impl StateChangeExtractor {
    /// Extract state changes from transaction execution
    /// For now, returns a simple ETH transfer implementation
    /// TODO: Parse full TracingInspector output for complete state changes
    pub fn extract_state_changes(
        tx_from: Address,
        tx_to: Option<Address>,
        tx_value: U256,
    ) -> Result<HashMap<String, CalculatedAccountChanges>> {
        let mut state_changes: HashMap<String, CalculatedAccountChanges> = HashMap::new();
        
        // Process the main ETH transfer if value > 0
        if tx_value > U256::ZERO {
            // Convert addresses and values
            let from_revm = RevmAddress::from(tx_from.0.0);
            let value_revm = RevmU256::from_limbs(tx_value.into_limbs());
            
            // Deduct ETH from sender
            let from_checksum = crate::common::address::checksum_address(&format!("{:x}", tx_from));
            let from_changes = state_changes.entry(from_checksum.clone()).or_insert_with(|| {
                CalculatedAccountChanges {
                    address: from_revm,
                    eth_net_change: SignedAmount::default(),
                    token_net_changes: HashMap::new(),
                    token_infos: Vec::new(),
                    movements: AccountMovements::default(),
                }
            });
            
            // Add outgoing ETH movement
            from_changes.movements.eth.out_list.push(EthMovement {
                source_identifier: "tx_value".to_string(),
                raw_amount: value_revm.clone(),
            });
            
            // Update net change (negative for sender)
            from_changes.eth_net_change = SignedAmount::new(value_revm.clone(), true);
            
            // Add ETH to recipient if not a contract creation
            if let Some(to_addr) = tx_to {
                let to_revm = RevmAddress::from(to_addr.0.0);
                let to_checksum = crate::common::address::checksum_address(&format!("{:x}", to_addr));
                let to_changes = state_changes.entry(to_checksum).or_insert_with(|| {
                    CalculatedAccountChanges {
                        address: to_revm,
                        eth_net_change: SignedAmount::default(),
                        token_net_changes: HashMap::new(),
                        token_infos: Vec::new(),
                        movements: AccountMovements::default(),
                    }
                });
                
                // Add incoming ETH movement
                to_changes.movements.eth.in_list.push(EthMovement {
                    source_identifier: "tx_value".to_string(),
                    raw_amount: value_revm.clone(),
                });
                
                // Update net change (positive for recipient)
                to_changes.eth_net_change = SignedAmount::new(value_revm, false);
            }
        }
        
        // TODO: In the future, parse TracingInspector output to extract:
        // - Internal ETH transfers
        // - ERC20 token transfers
        // - Contract creations
        // - Storage changes
        
        Ok(state_changes)
    }
}