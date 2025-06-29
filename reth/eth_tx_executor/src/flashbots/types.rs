//! Types and structures for Flashbots integration

use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Result of bundle submission
#[derive(Debug, Clone)]
pub enum BundleResult {
    /// Bundle was included in specified block
    Included {
        block_number: u64,
        block_hash: H256,
        gas_used: U256,
        effective_gas_price: U256,
    },
    /// Bundle was not included (may retry)
    NotIncluded {
        reason: BundleNotIncludedReason,
    },
    /// Bundle submission failed
    Failed {
        error: String,
    },
}

/// Reasons why bundle was not included
#[derive(Debug, Clone)]
pub enum BundleNotIncludedReason {
    /// Block has not been mined yet
    BlockNotMined,
    /// Another bundle had higher priority fee
    Outbid,
    /// Bundle would revert
    WouldRevert,
    /// Account nonce too low
    NonceTooLow,
    /// Account nonce too high  
    NonceTooHigh,
    /// Bundle timestamp invalid
    InvalidTimestamp,
    /// Unknown reason
    Unknown,
}

/// Bundle submission status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleStatus {
    /// Bundle hash
    pub bundle_hash: H256,
    /// Target block number
    pub block_number: u64,
    /// Submission timestamp
    pub submitted_at: u64,
    /// Current status
    pub status: BundleState,
    /// Simulation results if available
    pub simulation: Option<SimulationResult>,
}

/// Bundle state in relay
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BundleState {
    /// Bundle received by relay
    Pending,
    /// Bundle included in block
    Included,
    /// Bundle failed/rejected
    Failed,
    /// Bundle not included in target block
    NotIncluded,
}

/// Bundle simulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationResult {
    /// Whether bundle execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Gas used by bundle
    pub gas_used: U256,
    /// Coinbase payment (tip to validator)
    pub coinbase_diff: U256,
    /// ETH sent to coinbase
    pub eth_sent_to_coinbase: U256,
    /// Gas fees paid
    pub gas_fees: U256,
    /// State changes
    pub state_diffs: Vec<StateDiff>,
    /// Transaction results
    pub results: Vec<TransactionResult>,
}

/// State difference from simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    pub address: Address,
    pub slot: H256,
    pub value: H256,
}

/// Individual transaction result in bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResult {
    /// Transaction hash
    pub tx_hash: H256,
    /// Gas used
    pub gas_used: U256,
    /// Revert reason if failed
    pub revert: Option<String>,
    /// Return value
    pub value: Option<Bytes>,
}

/// Flashbots bundle request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleRequest {
    /// Array of signed transactions (hex encoded)
    pub txs: Vec<String>,
    /// Target block number
    pub block_number: U256,
    /// Minimum timestamp for bundle validity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_timestamp: Option<u64>,
    /// Maximum timestamp for bundle validity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_timestamp: Option<u64>,
    /// Reverting transaction hashes (bundle discarded if these revert)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverting_tx_hashes: Option<Vec<H256>>,
}

/// Flashbots RPC request wrapper
#[derive(Debug, Serialize)]
pub struct FlashbotsRequest<T> {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: T,
    pub id: u64,
}

impl<T> FlashbotsRequest<T> {
    pub fn new(method: &'static str, params: T) -> Self {
        Self {
            jsonrpc: "2.0",
            method,
            params,
            id: 1,
        }
    }
}

/// Flashbots RPC response
#[derive(Debug, Deserialize)]
pub struct FlashbotsResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(flatten)]
    pub data: FlashbotsResponseData<T>,
}

/// Response data (either result or error)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FlashbotsResponseData<T> {
    Success { result: T },
    Error { error: FlashbotsError },
}

/// Flashbots error response
#[derive(Debug, Clone, Deserialize)]
pub struct FlashbotsError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Send bundle parameters
#[derive(Debug, Serialize)]
pub struct SendBundleParams(pub Vec<BundleRequest>);

/// Bundle statistics from relay
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleStats {
    /// Whether bundle is high priority
    pub is_high_priority: bool,
    /// Whether bundle was simulated
    pub is_simulated: bool,
    /// Simulated block number
    pub simulated_at_block_number: Option<u64>,
    /// Bundle hash for tracking
    pub bundle_hash: H256,
}

/// Configuration for bundle submission
#[derive(Debug, Clone)]
pub struct BundleConfig {
    /// Target block offset (current block + offset)
    pub block_offset: u64,
    /// Tip percentage of transaction value
    pub tip_percentage: f64,
    /// Minimum tip amount in wei
    pub min_tip: U256,
    /// Maximum tip amount in wei  
    pub max_tip: U256,
    /// Enable revert protection
    pub revert_protection: bool,
    /// Submission timeout
    pub timeout: Duration,
    /// Number of blocks to try
    pub max_blocks_to_try: u64,
}