//! Core types for TX_FUND_FLOW analytics

use alloy_primitives::{Address, B256, U256};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transaction hash type
pub type TransactionHash = B256;

/// Block number type
pub type BlockNumber = u64;

/// Ethereum address with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EthAddress {
    pub address: Address,
    pub label: Option<String>,
    pub entity_type: Option<EntityType>,
}

impl From<Address> for EthAddress {
    fn from(address: Address) -> Self {
        Self {
            address,
            label: None,
            entity_type: None,
        }
    }
}

impl EthAddress {
    /// Create a new EthAddress from a string, returning an error if invalid
    pub fn from_str(s: &str) -> crate::error::QarqaResult<Self> {
        s.try_into()
    }
    
    /// Create a new EthAddress with validation
    pub fn new(address: Address) -> Self {
        Self {
            address,
            label: None,
            entity_type: None,
        }
    }
    
    /// Set the label for this address
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    
    /// Set the entity type for this address
    pub fn with_entity_type(mut self, entity_type: EntityType) -> Self {
        self.entity_type = Some(entity_type);
        self
    }
}

impl TryFrom<&str> for EthAddress {
    type Error = crate::error::QarqaError;
    
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let address = s.parse()
            .map_err(|e| crate::error::QarqaError::InvalidInput(
                format!("Invalid Ethereum address '{}': {}", s, e)
            ))?;
        
        Ok(Self {
            address,
            label: None,
            entity_type: None,
        })
    }
}

/// Transaction data from eth_db
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: TransactionHash,
    pub block_number: BlockNumber,
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub value: U256,
    pub gas_limit: u64,
    pub gas_price: U256,
    pub input_data: Vec<u8>,
    pub status: bool,
    pub gas_used: Option<u64>,
    pub timestamp: Option<DateTime<Utc>>,
}

/// Transaction participant from address_transactions table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionParticipant {
    pub address: Address,
    pub transaction_hash: TransactionHash,
    pub block_number: BlockNumber,
    pub direction: ParticipantDirection,
    pub value_change: Option<i64>,
    pub token_transfers: Option<serde_json::Value>,
}

/// Direction of participation in a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantDirection {
    In,    // Received value/tokens
    Out,   // Sent value/tokens
    Both,  // Both sent and received (e.g., swap)
}

/// Fund flow between two addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundFlow {
    pub from: Address,
    pub to: Address,
    pub amount_eth: f64,
    pub amount_tokens_usd: f64,
    pub transaction_count: u64,
    pub first_block: BlockNumber,
    pub last_block: BlockNumber,
    pub flow_type: FlowType,
}

/// Type of fund flow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FlowType {
    DirectTransfer,
    InternalTransfer,
    TokenTransfer(Address), // Token contract address
    GasPayment,
    ContractInteraction,
}

/// Complete fund flows extracted from transaction simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteFundFlows {
    pub tx_hash: String,
    pub block_number: BlockNumber,
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub eth_movements: Vec<EthMovement>,
    pub token_movements: Vec<TokenMovement>,
}

/// ETH movement between addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthMovement {
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub movement_type: EthMovementType,
}

/// Type of ETH movement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EthMovementType {
    Direct,      // Direct transaction transfer
    Internal,    // Internal contract transfer
    Gas,         // Gas payment
    Refund,      // Gas refund
    SelfDestruct, // Contract self-destruct
}

/// Token movement between addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMovement {
    pub token_address: Address,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<u8>,
}

/// Address state change from simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressStateChange {
    pub address: Address,
    pub eth_balance_before: U256,
    pub eth_balance_after: U256,
    pub eth_net_change: i128, // Can be negative
    pub nonce_before: u64,
    pub nonce_after: u64,
    pub token_changes: HashMap<Address, TokenBalanceChange>,
}

/// Token balance change for an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalanceChange {
    pub token_address: Address,
    pub balance_before: U256,
    pub balance_after: U256,
    pub net_change: i128, // Can be negative
}

/// Entity type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    // Exchanges
    CEX,           // Centralized Exchange
    DEX,           // Decentralized Exchange
    
    // Users
    Whale,         // Large holder/trader
    Retail,        // Individual user
    Bot,           // Automated trading bot
    Arbitrageur,   // Arbitrage trader
    
    // Infrastructure
    Bridge,        // Cross-chain bridge
    Mixer,         // Privacy mixer
    Miner,         // Mining pool
    Validator,     // PoS validator
    
    // Protocols
    DeFiProtocol,  // DeFi protocol contract
    Token,         // Token contract
    MultiSig,      // Multi-signature wallet
    DAO,           // DAO treasury
    
    // Unknown
    Unknown,       // Unable to classify
}

/// Risk level assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,        // 0.0 - 0.3
    Medium,     // 0.3 - 0.6  
    High,       // 0.6 - 0.8
    Critical,   // 0.8 - 1.0
}

/// Time range for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start_block: Option<BlockNumber>,
    pub end_block: Option<BlockNumber>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

impl TimeRange {
    /// Create a time range from block numbers
    pub fn from_blocks(start: BlockNumber, end: BlockNumber) -> Self {
        Self {
            start_block: Some(start),
            end_block: Some(end),
            start_time: None,
            end_time: None,
        }
    }
    
    /// Create a time range from timestamps
    pub fn from_timestamps(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            start_block: None,
            end_block: None,
            start_time: Some(start),
            end_time: Some(end),
        }
    }
    
    /// Create a time range for the last N blocks
    pub fn last_blocks(n: u64, latest_block: BlockNumber) -> Self {
        let start = if latest_block > n { latest_block - n } else { 0 };
        Self::from_blocks(start, latest_block)
    }
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub eth_threshold: f64,
    pub token_threshold_usd: f64,
    pub max_depth: u32,
    pub time_range: TimeRange,
    pub include_gas_flows: bool,
    pub include_token_flows: bool,
    pub min_transaction_count: u64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            eth_threshold: 0.01,           // 0.01 ETH minimum
            token_threshold_usd: 10.0,     // $10 minimum for tokens
            max_depth: 3,                  // 3 hops maximum
            time_range: TimeRange::last_blocks(10000, u64::MAX), // Last 10k blocks
            include_gas_flows: false,      // Usually not interesting
            include_token_flows: true,     // Include ERC20 flows
            min_transaction_count: 1,      // At least 1 transaction
        }
    }
}