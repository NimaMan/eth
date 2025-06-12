//! Data types for optimized transaction data retrieval

use std::collections::HashMap;
use ethers_core::types::H256;
use revm_primitives::{Address as RevmAddress, U256 as RevmU256, Log as RevmLog};
use serde::{Serialize, Deserialize};

/// Level of data detail requested
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataLevel {
    /// Basic transaction info from database only (fastest ~1ms)
    Basic,
    /// Smart detection - uses database + simulation only if needed
    Smart,
    /// Full analysis with simulation always (slowest ~80-800ms)
    Complete,
}

/// Transaction type classification for optimization heuristics
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionType {
    /// Simple ETH transfer to EOA (no internal transfers expected)
    SimpleTransfer,
    /// Contract call that might have internal transfers
    ContractCall,
    /// Known DEX transaction (high chance of internal transfers)
    DexInteraction,
    /// Known bridge/multi-hop (high chance of internal transfers)  
    ComplexDeFi,
    /// Contract deployment
    ContractDeployment,
}

/// Performance metrics for optimization tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub data_source: String,
    pub retrieval_time_ms: f64,
    pub database_time_ms: Option<f64>,
    pub simulation_time_ms: Option<f64>,
    pub data_level: String,
    pub optimization_applied: bool,
    pub internal_transfers_found: Option<usize>,
}

/// Basic transaction data from database only (fastest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicTxData {
    // Core transaction data
    pub hash: H256,
    pub from: RevmAddress,
    pub to: Option<RevmAddress>,
    pub value: RevmU256,
    pub gas_limit: u64,
    pub gas_price: RevmU256,
    pub gas_used: u64,
    pub nonce: u64,
    pub input_data: Vec<u8>,
    
    // Receipt data
    pub block_number: u64,
    pub block_hash: H256,
    pub transaction_index: u64,
    pub status: bool,
    
    // Event logs (from receipt)
    pub logs: Vec<RevmLog>,
    pub log_count: usize,
    
    // Basic analysis
    pub is_contract_call: bool,
    pub erc20_transfers: Vec<Erc20Transfer>,
    
    // Performance
    pub performance: PerformanceMetrics,
}

/// Smart transaction data with conditional simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartTxData {
    // All basic data
    pub basic: BasicTxData,
    
    // Conditional simulation data (only if heuristics suggest it's needed)
    pub internal_transfers: Option<Vec<InternalTransfer>>,
    pub simulation_performed: bool,
    pub transaction_type: TransactionType,
    
    // Enhanced analysis
    pub complex_interactions: bool,
    pub dex_swaps_detected: usize,
    pub multi_hop_detected: bool,
}

/// Full transaction analysis with complete simulation
#[derive(Debug, Clone, Serialize, Deserialize)]  
pub struct FullTxData {
    // All smart data
    pub smart: SmartTxData,
    
    // Complete simulation results
    pub internal_transfers: Vec<InternalTransfer>,
    pub call_trace: Option<String>, // Simplified call trace
    pub gas_refunded: u64,
    pub output_data: Vec<u8>,
    pub simulation_success: bool,
    pub revert_reason: Option<String>,
    
    // State changes
    pub addresses_affected: usize,
    pub eth_movements: HashMap<RevmAddress, i128>, // Net ETH changes in wei
    pub token_movements: HashMap<RevmAddress, HashMap<RevmAddress, i128>>, // address -> token -> net change
}

/// ERC20 transfer data extracted from logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc20Transfer {
    pub token_address: RevmAddress,
    pub from: RevmAddress,
    pub to: RevmAddress,
    pub amount: RevmU256,
    pub log_index: usize,
}

/// Internal transfer data from simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalTransfer {
    pub from: RevmAddress,
    pub to: RevmAddress,
    pub value: RevmU256,
    pub call_type: String,
    pub depth: u32,
}

/// Options for transaction data retrieval
#[derive(Debug, Clone)]
pub struct TransactionDataOptions {
    /// Force specific data level (overrides smart detection)
    pub force_level: Option<DataLevel>,
    /// Always include internal transfers (forces simulation)
    pub need_internal_transfers: bool,
    /// Include detailed call traces
    pub need_call_trace: bool,
    /// Include state change analysis
    pub need_state_changes: bool,
    /// Reth database directory (optional, uses default if None)
    pub reth_datadir: Option<String>,
    /// RPC URL for state access
    pub rpc_url: String,
    /// Enable performance tracking
    pub track_performance: bool,
}

impl Default for TransactionDataOptions {
    fn default() -> Self {
        Self {
            force_level: None,
            need_internal_transfers: false,
            need_call_trace: false,
            need_state_changes: false,
            reth_datadir: None,
            rpc_url: "http://127.0.0.1:8545".to_string(),
            track_performance: true,
        }
    }
}

impl TransactionDataOptions {
    /// Create options for basic data retrieval (fastest)
    pub fn basic() -> Self {
        Self {
            force_level: Some(DataLevel::Basic),
            ..Default::default()
        }
    }
    
    /// Create options for smart data retrieval (auto-detection)
    pub fn smart() -> Self {
        Self {
            force_level: Some(DataLevel::Smart),
            ..Default::default()
        }
    }
    
    /// Create options for complete data retrieval (always simulate)
    pub fn complete() -> Self {
        Self {
            force_level: Some(DataLevel::Complete),
            need_internal_transfers: true,
            need_call_trace: true,
            need_state_changes: true,
            ..Default::default()
        }
    }
    
    /// Create options that require internal transfers (forces simulation)
    pub fn with_internal_transfers() -> Self {
        Self {
            need_internal_transfers: true,
            ..Default::default()
        }
    }
}