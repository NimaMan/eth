/*
* Simulation Types
*
* Types related to transaction simulation and its results.
*/

use ethers::types::{Address, U256, Bytes, H256};
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Types of state changes that can occur during simulation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Error, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SimulationError {
    /// Transaction reverted with a reason
    #[error("Transaction reverted: {0}")]
    Revert(String),
    
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
}
