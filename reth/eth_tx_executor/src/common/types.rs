//! Common type definitions used throughout the eth_kartal system
//!
//! Provides type aliases and common structures to ensure consistency
//! across all modules.

use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Token address type alias for clarity
pub type TokenAddress = Address;

/// Pool address type alias for clarity
pub type PoolAddress = Address;

/// Transaction hash type alias
pub type TxHash = H256;

/// Gas price in wei
pub type GasPrice = U256;

/// Token amount (raw units)
pub type TokenAmount = U256;

/// Timestamp (Unix epoch seconds)
pub type Timestamp = u64;

/// Result type alias for the entire system
pub type Result<T> = std::result::Result<T, crate::common::errors::KartalError>;

/// Priority levels for transaction execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Normal priority - standard execution
    Normal = 0,
    /// High priority - expedited execution
    High = 1,
    /// Critical priority - maximum urgency
    Critical = 2,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Normal => write!(f, "Normal"),
            Priority::High => write!(f, "High"),
            Priority::Critical => write!(f, "Critical"),
        }
    }
}

/// Action types for trading operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Buy tokens with ETH
    Buy,
    /// Sell tokens for ETH
    Sell,
    /// Add liquidity to pool
    AddLiquidity,
    /// Remove liquidity from pool
    RemoveLiquidity,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Buy => write!(f, "Buy"),
            Action::Sell => write!(f, "Sell"),
            Action::AddLiquidity => write!(f, "AddLiquidity"),
            Action::RemoveLiquidity => write!(f, "RemoveLiquidity"),
        }
    }
}

/// Chain ID enumeration for supported networks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainId {
    /// Ethereum Mainnet
    Mainnet = 1,
    /// Goerli Testnet
    Goerli = 5,
    /// Sepolia Testnet
    Sepolia = 11155111,
}

impl From<ChainId> for u64 {
    fn from(chain: ChainId) -> Self {
        chain as u64
    }
}

/// Performance metrics for tracking execution times
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Time from alert receipt to execution start (ms)
    pub alert_to_start_ms: u64,
    /// Time to check positions (ms)
    pub position_check_ms: u64,
    /// Time to calculate gas ranking (ms)
    pub gas_ranking_ms: u64,
    /// Time to get price quote (ms)
    pub price_quote_ms: u64,
    /// Time to build transaction (ms)
    pub tx_build_ms: u64,
    /// Time to submit transaction (ms)
    pub tx_submit_ms: u64,
    /// Total execution time (ms)
    pub total_ms: u64,
}

impl PerformanceMetrics {
    /// Check if metrics meet performance targets
    pub fn meets_targets(&self) -> bool {
        self.total_ms < 200 // Sub-200ms target
    }

    /// Get a summary of the slowest operation
    pub fn slowest_operation(&self) -> (&'static str, u64) {
        let operations = [
            ("alert_to_start", self.alert_to_start_ms),
            ("position_check", self.position_check_ms),
            ("gas_ranking", self.gas_ranking_ms),
            ("price_quote", self.price_quote_ms),
            ("tx_build", self.tx_build_ms),
            ("tx_submit", self.tx_submit_ms),
        ];

        operations
            .into_iter()
            .max_by_key(|(_, time)| *time)
            .unwrap_or(("unknown", 0))
    }
}

/// Transaction status for tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending in mempool
    Pending,
    /// Transaction was included in a block
    Confirmed { block_number: u64, block_hash: H256 },
    /// Transaction failed or was dropped
    Failed { reason: String },
    /// Transaction was replaced by another
    Replaced { new_tx_hash: TxHash },
}

/// Standard slippage tolerances
pub mod slippage {
    /// Conservative slippage (0.5%)
    pub const CONSERVATIVE: f64 = 0.005;
    /// Standard slippage (1%)
    pub const STANDARD: f64 = 0.01;
    /// Aggressive slippage (3%)
    pub const AGGRESSIVE: f64 = 0.03;
    /// Maximum allowed slippage (5%)
    pub const MAX_ALLOWED: f64 = 0.05;
}

/// Gas limit constants
pub mod gas_limits {
    use ethers::types::U256;

    /// Standard ERC20 transfer
    pub const ERC20_TRANSFER: u64 = 65_000;
    /// Uniswap V2 swap
    pub const UNISWAP_V2_SWAP: u64 = 150_000;
    /// Uniswap V3 swap
    pub const UNISWAP_V3_SWAP: u64 = 200_000;
    /// Safe high gas limit for complex operations
    pub const SAFE_HIGH: u64 = 300_000;

    /// Convert to U256
    pub fn as_u256(limit: u64) -> U256 {
        U256::from(limit)
    }
}
