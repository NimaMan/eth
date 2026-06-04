//! Database models matching eth_db PostgreSQL schema

// Remove unused imports
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Address record from eth_db.addresses table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AddressRecord {
    pub address_id: i64,
    pub address: String,
    pub is_contract: bool,

    // Trading metrics
    pub total_profit: Option<f64>,
    pub total_realized_profit: Option<f64>,
    pub total_volume: Option<f64>,
    pub scam_ratio: Option<f64>,
    pub trade_frequency: Option<f64>,
    pub total_tx_fee: Option<f64>,

    // Activity
    pub first_seen: Option<i32>,
    pub last_seen: Option<i32>,

    // Metadata
    pub name: Option<String>,
    pub entity_category: Option<String>,
    pub cluster_label: Option<String>,
}

/// Transaction participant from eth_db.tx_participants table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TxParticipant {
    pub tx_hash: String,
    pub address_id: i64,
}

/// Transaction record from eth_db.transactions table (actual schema)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub tx_hash: String,
    pub block_number: i32,
    pub value: f64,
    pub status: Option<String>,
    pub from_address_id: i64,
    pub to_address_id: Option<i64>,
}

/// Related address from eth_db.related_addresses table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RelatedAddressRecord {
    pub id: i32,
    pub trade_id: Option<i32>,
    pub address_id: i64,
    pub related_address_id: i64,
    pub denom_flow: Option<f64>,
}

/// Token record from eth_db.tokens table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TokenRecord {
    pub contract_address: String,
    pub creator_address_id: Option<i64>,
    pub is_scam: Option<bool>,
    pub scam_label: Option<String>,
    pub creation_txn: Option<String>,
    pub trading_enabled_txn: Option<String>,
}

/// Trade record from eth_db.trades table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: i32,
    pub address_id: i64,
    pub token_address: String,
    pub currency: Option<String>,

    // Transaction metrics
    pub entry_block: Option<i32>,
    pub latest_block: Option<i32>,
    pub total_denom_spent: Option<f64>,
    pub total_denom_received: Option<f64>,
    pub tx_fee: Option<f64>,

    // PnL metrics
    pub realized_profit: Option<f64>,
    pub unrealized_profit: Option<f64>,

    // Activity metrics
    pub num_buys: Option<i32>,
    pub num_sells: Option<i32>,
}

/// Pool record from eth_db.pools table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PoolRecord {
    pub id: i32,
    pub pool_address: Option<String>,
    pub pool_id: Option<String>,
    pub pool_type: String,
    pub token_address: String,
    pub pair_token_address: String,
    pub fee_tier: Option<i32>,
}

/// Single transaction result for address (tx_hash + block)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AddressTransaction {
    pub tx_hash: String,
    pub block_number: i32,
}

/// Query result for address transactions  
#[derive(Debug, Clone)]
pub struct AddressTransactions {
    pub address: AddressRecord,
    pub tx_hashes: Vec<String>,
    pub total_count: i64,
}

/// Query result for address relationships
#[derive(Debug, Clone)]
pub struct AddressRelationships {
    pub address: AddressRecord,
    pub related_addresses: Vec<(i64, String, f64)>, // (address_id, address, total_flow)
}
