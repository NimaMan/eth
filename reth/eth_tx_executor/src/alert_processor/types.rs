//! Alert types for transaction execution
//!
//! Defines the structure of alerts that trigger transaction execution

use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};

/// Alert that triggers transaction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique identifier for the alert
    pub id: String,

    /// Unix timestamp when alert was generated
    pub timestamp: u64,

    /// Token address to execute transaction on
    pub token_address: Address,

    /// Pool address where action should be taken
    pub pool_address: Address,

    /// Action to take
    pub action: Action,

    /// Execution parameters
    pub params: ExecutionParams,
}

// Re-export Action from common module
pub use crate::common::Action;

/// Parameters for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParams {
    /// Amount to execute (in token units)
    pub amount: U256,

    /// Slippage tolerance (e.g., 0.05 for 5%)
    pub slippage: f64,

    /// Maximum gas price willing to pay (in wei)
    pub max_gas_price: Option<U256>,

    /// Deadline for execution (seconds from now)
    pub deadline_seconds: u64,

    /// Priority level for MEV protection
    pub priority: Priority,
}

// Re-export Priority from common module
pub use crate::common::Priority;

impl Alert {
    /// Calculate deadline timestamp
    pub fn deadline_timestamp(&self) -> U256 {
        U256::from(self.timestamp + self.params.deadline_seconds)
    }

    /// Check if alert is still valid
    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        now < (self.timestamp + self.params.deadline_seconds)
    }
}
