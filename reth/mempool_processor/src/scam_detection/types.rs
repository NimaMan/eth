// scam_detection/types.rs
//
// Type definitions for scam detection data structures.

use std::collections::HashMap;
use ethers::types::H256;
use serde::{Serialize, Deserialize};

/// Reasons a transaction might be flagged as suspicious
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScamAlertReason {
    /// ETH reserve would drop below critical threshold
    EthReserveDepleted,
    
    /// More than X% of ETH reserves would be removed
    LargeEthWithdrawal,
    
    /// Transaction matches a known scam pattern
    KnownScamPattern,
    
    /// Other unspecified reason
    Other,
}

/// Result of simulating the effect of a pending transaction on pool ETH levels
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Transaction hash
    pub tx_hash: String,
    
    /// Simulation result per affected pool
    pub affected_pools: HashMap<String, PoolEffect>,
    
    /// Whether simulation was successful
    pub simulation_successful: bool,
    
    /// Error message if simulation failed
    pub error_message: Option<String>,
}

/// The effect a transaction would have on a specific pool
#[derive(Debug, Clone)]
pub struct PoolEffect {
    /// Pool address
    pub pool_address: String,
    
    /// Current ETH reserve
    pub current_eth_reserve: f64,
    
    /// Predicted ETH reserve after transaction
    pub simulated_eth_reserve: f64,
    
    /// Change in ETH (negative for withdrawals)
    pub eth_delta: f64,
    
    /// Percentage change (negative for withdrawals)
    pub percentage_change: f64,
}

/// A detected potential scam transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScamAlert {
    /// Transaction hash
    pub tx_hash: String,
    
    /// Sender address
    pub from_address: String,
    
    /// Pool address affected
    pub pool_address: String,
    
    /// Associated token address
    pub token_address: String,
    
    /// ETH reserve before transaction
    pub current_eth_reserve: f64,
    
    /// Predicted ETH reserve after transaction
    pub simulated_eth_reserve: f64,
    
    /// Reason for flagging this transaction
    pub reason: ScamAlertReason,
    
    /// Threshold used for detection
    pub eth_threshold: f64,
    
    /// Block number when scam was detected
    pub detection_block: u64,
    
    /// Unix timestamp when detection occurred
    pub detection_time: f64,
} 