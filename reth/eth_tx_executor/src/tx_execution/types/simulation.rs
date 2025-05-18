/*
* Simulation Types
*
* Types related to transaction simulation and its results.
*/

use ethers::types::{Address, U256, Bytes, H256};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Types of state changes that can occur during simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChangeType {
    /// ETH balance change
    Balance,
    /// Storage slot change
    Storage,
    /// Contract code change (deployment)
    Code,
    /// Contract creation
    Creation,
}

/// A single state change that occurred during simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    /// Type of state change
    pub change_type: StateChangeType,
    /// Address affected
    pub address: Address,
    /// Storage slot (if applicable)
    pub slot: Option<H256>,
    /// Old value (None if new)
    pub old_value: Option<Bytes>,
    /// New value (None if deleted)
    pub new_value: Option<Bytes>,
}

/// Errors that can occur during simulation
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum SimulationError {
    /// Transaction reverted with a reason
    #[error("Transaction reverted: {0}")]
    Revert(String),
    
    /// Transaction failed due to out of gas
    #[error("Out of gas: used {used}/{limit}")]
    OutOfGas {
        used: U256,
        limit: U256,
    },
    
    /// Invalid transaction parameters
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    /// Nonce error
    #[error("Nonce error: expected {expected}, got {actual}")]
    NonceError {
        expected: U256,
        actual: U256,
    },
    
    /// Insufficient funds
    #[error("Insufficient funds: need {required}, have {available}")]
    InsufficientFunds {
        required: U256,
        available: U256,
    },
    
    /// Other error
    #[error("Simulation error: {0}")]
    Other(String),
}

/// Result of a transaction simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Success flag
    pub success: bool,
    
    /// Gas used
    pub gas_used: U256,
    
    /// Gas limit used in simulation
    pub gas_limit: U256,
    
    /// Execution result (returned data)
    pub result: Option<Bytes>,
    
    /// Error (if simulation failed)
    pub error: Option<SimulationError>,
    
    /// State changes that occurred during simulation
    pub state_changes: Vec<StateChange>,
    
    /// Contract addresses created (if any)
    pub created_contracts: Vec<Address>,
    
    /// Logs emitted during simulation
    pub logs: Vec<Bytes>,
    
    /// Trace data (if requested)
    pub trace: Option<Bytes>,
    
    /// Gas price used for the simulation
    pub effective_gas_price: U256,
    
    /// Total transaction cost (gas_used * effective_gas_price)
    pub transaction_cost: U256,
}

impl SimulationResult {
    /// Creates a successful simulation result
    pub fn success(
        gas_used: U256,
        gas_limit: U256,
        result: Option<Bytes>,
        state_changes: Vec<StateChange>,
        effective_gas_price: U256,
    ) -> Self {
        let transaction_cost = gas_used * effective_gas_price;
        Self {
            success: true,
            gas_used,
            gas_limit,
            result,
            error: None,
            state_changes,
            created_contracts: Vec::new(),
            logs: Vec::new(),
            trace: None,
            effective_gas_price,
            transaction_cost,
        }
    }
    
    /// Creates a failed simulation result
    pub fn failure(
        error: SimulationError,
        gas_used: U256,
        gas_limit: U256,
        effective_gas_price: U256,
    ) -> Self {
        let transaction_cost = gas_used * effective_gas_price;
        Self {
            success: false,
            gas_used,
            gas_limit,
            result: None,
            error: Some(error),
            state_changes: Vec::new(),
            created_contracts: Vec::new(),
            logs: Vec::new(),
            trace: None,
            effective_gas_price,
            transaction_cost,
        }
    }
    
    /// Check if the simulation suggests the transaction will be successful
    pub fn is_likely_successful(&self) -> bool {
        // Transactions that reverted with "out of gas" might succeed with higher gas limit
        if let Some(SimulationError::OutOfGas { .. }) = &self.error {
            return self.gas_used.as_u64() >= self.gas_limit.as_u64() * 95 / 100;
        }
        
        self.success
    }
} 