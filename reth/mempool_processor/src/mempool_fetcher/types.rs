use ethers::prelude::*;
use std::collections::HashSet;

/// Transaction status in mempool
#[derive(Clone, Debug, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

/// Mempool transaction with metadata
#[derive(Clone, Debug)]
pub struct MempoolTransaction {
    pub hash: H256,
    pub transaction: TransactionView,
    pub status: TransactionStatus,
    pub first_seen: std::time::Instant,
}

/// Simple struct to hold transaction data
#[derive(Clone, Debug)]
pub struct TransactionView {
    pub hash: Vec<u8>,
    pub from: Vec<u8>,
    pub to: Option<Vec<u8>>,
    pub value: U256,
    pub gas_price: Option<U256>,
    pub gas_limit: Option<U256>,
    pub nonce: Option<U256>,
    pub input_data: Option<Vec<u8>>,
}

/// Criteria for filtering interesting transactions
#[derive(Clone, Debug)]
pub struct TransactionFilter {
    pub min_value: U256,                // Minimum value in wei
    pub watched_addresses: HashSet<String>, // Addresses of interest (hex strings without 0x)
    pub min_gas_price: Option<U256>,    // Minimum gas price for gas price anomalies
}

impl Default for TransactionFilter {
    fn default() -> Self {
        Self {
            // Default to 0.1 ETH for lower threshold than original
            min_value: U256::from(100000000000000000u64),
            watched_addresses: HashSet::new(),
            min_gas_price: None,
        }
    }
}

/// Represents the reason a transaction was flagged as interesting
#[derive(Clone, Debug, PartialEq)]
pub enum AlertReason {
    HighValue,
    WatchedAddress,
    HighGasPrice,
    /// Detected a significant state change in the transaction
    StateChangeValue {
        eth_value: f64,     // ETH value change (negative for outflows)
        address: String,    // Address experiencing the change
    },
    /// Detection of potential liquidity removal
    LiquidityRemoval {
        pool_address: String,
        eth_value: f64,
    },
    /// ERC20 token transfer detected
    TokenTransfer {
        token_address: String,
        from: String,
        to: String,
        amount: String,     // Formatted amount with decimals
        raw_amount: U256,   // Raw amount without decimals
    },
    /// ERC20 token approval detected
    TokenApproval {
        token_address: String,
        owner: String,
        spender: String,
        amount: String,     // Formatted amount with decimals
        raw_amount: U256,   // Raw amount without decimals
    },
    /// Token swap detected (DEX interaction)
    TokenSwap {
        token_address: String,
        pool_address: String,
        swap_type: String,  // "buy" or "sell"
        token_amount: String,
        eth_amount: String,
    },
    Custom(String),
}

/// Complete alert with transaction details and reason
#[derive(Clone, Debug)]
pub struct Alert {
    pub transaction: TransactionView,
    pub reason: AlertReason,
    pub timestamp: u64,
} 