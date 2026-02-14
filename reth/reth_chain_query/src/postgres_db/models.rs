/// PostgreSQL Schema Models
///
/// Rust structs matching the eth_db PostgreSQL schema for Ethereum PnL analysis.
/// These models represent aggregated on-chain data for efficient querying.
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, Row};

/// Address with aggregated metrics across all trades
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressMetrics {
    pub address_id: i64,
    pub address: String,
    pub is_contract: bool,
    pub total_erc20_tx: Option<i32>,
    pub total_erc20_trades: Option<i32>,

    // Core trading metrics
    pub scam_ratio: Option<f64>,
    pub total_profit: Option<f64>,
    pub total_volume: Option<f64>,
    pub first_seen: Option<i32>,
    pub last_seen: Option<i32>,
    pub total_tx_fee: Option<f64>,

    // Network metrics
    pub degree_centrality: Option<f64>,
    pub betweenness_centrality: Option<f64>,

    // PnL metrics
    pub total_denom_balance: Option<f64>,
    pub total_realized_profit: Option<f64>,
    pub mean_received_spent_ratio: Option<f64>,
    pub median_received_spent_ratio: Option<f64>,
    pub avg_bribe_amount: Option<f64>,
    pub total_bribe_amount: Option<f64>,

    // Labels
    pub name: Option<String>,
    pub entity_category: Option<String>,
    pub cluster_label: Option<String>,
}

/// Aggregated trades between address and token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: i32,
    pub address_id: i64,
    pub token_address: String,
    pub currency: Option<String>,

    // Core transaction metrics
    pub entry_block: Option<i32>,
    pub latest_block: Option<i32>,
    pub total_denom_spent: Option<f64>,
    pub total_denom_received: Option<f64>,
    pub denom_received_spent_ratio: Option<f64>,
    pub bribe_amount: Option<f64>,
    pub tx_fee: Option<f64>,

    // PnL metrics
    pub realized_profit: Option<f64>,
    pub unrealized_profit: Option<f64>,

    // Behavioral signals
    pub num_buys: Option<i32>,
    pub num_sells: Option<i32>,
    pub token_holdings_ratio: Option<f64>,
    pub token_sell_buy_ratio: Option<f64>,

    // Aggregated balances
    pub agg_denom_balance: Option<f64>,
    pub agg_token_balance: Option<f64>,
}

/// Token metadata with scam detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub contract_address: String,
    pub creator_address_id: Option<i64>,
    pub is_scam: Option<bool>,
    pub scam_label: Option<String>,
    pub creation_tx: Option<String>,
    pub trading_enabled_tx: Option<String>,
}

/// Pool information across DEX protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub id: i32,
    pub pool_address: Option<String>,
    pub pool_id: Option<String>, // For V4 pools
    pub pool_type: String,       // V2, V3, V4
    pub token_address: String,
    pub pair_token_address: String,
    pub fee_tier: Option<i32>,

    // Scam detection
    pub is_scam: Option<bool>,
    pub scam_label: Option<String>,
    pub scam_block: Option<i32>,
    pub scam_tx_hash: Option<String>,

    // Trading status
    pub trading_enabled: Option<bool>,
    pub trading_enabled_block: Option<i64>,
    pub trading_enabled_tx: Option<String>,
}

/// Transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_hash: String,
    pub block_number: Option<i32>,
    pub from_address_id: Option<i64>,
    pub to_address_id: Option<i64>,
    pub value: Option<f64>,
    pub status: Option<String>,
}

/// Transaction participant record (from tx_participants table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxParticipant {
    pub tx_hash: String,
    pub address_id: i64,
}

/// Combined transaction with participant info for graph building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressTransaction {
    pub tx_hash: String,
    pub block_number: i32,
    pub from_address: String,
    pub to_address: String,
    pub value: Option<f64>,
    pub participant_count: i32,
}

/// Transaction details with all participants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionWithParticipants {
    pub tx_hash: String,
    pub block_number: Option<i32>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub value: Option<f64>,
    pub status: Option<String>,
    pub participants: Vec<String>,
}

/// Block metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub block_number: i32,
    pub block_timestamp: Option<i32>,
}

/// Query result for top addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopAddressResult {
    pub address: String,
    pub total_profit: f64,
    pub total_volume: f64,
    pub trade_count: i32,
    pub scam_ratio: f64,
    pub rank: i32,
}

/// Query result for PnL analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnLAnalysis {
    pub address: String,
    pub token_address: String,
    pub total_spent: f64,
    pub total_received: f64,
    pub realized_profit: f64,
    pub unrealized_profit: f64,
    pub roi_percentage: f64,
    pub num_trades: i32,
}

macro_rules! impl_from_row {
    ($ty:ty, [$( $field:ident ),+ $(,)?]) => {
        impl<'r> FromRow<'r, PgRow> for $ty {
            fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
                Ok(Self {
                    $( $field: row.try_get(stringify!($field))?, )+
                })
            }
        }
    };
}

impl_from_row!(
    AddressMetrics,
    [
        address_id,
        address,
        is_contract,
        total_erc20_tx,
        total_erc20_trades,
        scam_ratio,
        total_profit,
        total_volume,
        first_seen,
        last_seen,
        total_tx_fee,
        degree_centrality,
        betweenness_centrality,
        total_denom_balance,
        total_realized_profit,
        mean_received_spent_ratio,
        median_received_spent_ratio,
        avg_bribe_amount,
        total_bribe_amount,
        name,
        entity_category,
        cluster_label
    ]
);

impl_from_row!(
    Trade,
    [
        id,
        address_id,
        token_address,
        currency,
        entry_block,
        latest_block,
        total_denom_spent,
        total_denom_received,
        denom_received_spent_ratio,
        bribe_amount,
        tx_fee,
        realized_profit,
        unrealized_profit,
        num_buys,
        num_sells,
        token_holdings_ratio,
        token_sell_buy_ratio,
        agg_denom_balance,
        agg_token_balance
    ]
);

impl_from_row!(
    Token,
    [
        contract_address,
        creator_address_id,
        is_scam,
        scam_label,
        creation_tx,
        trading_enabled_tx
    ]
);

impl_from_row!(
    Pool,
    [
        id,
        pool_address,
        pool_id,
        pool_type,
        token_address,
        pair_token_address,
        fee_tier,
        is_scam,
        scam_label,
        scam_block,
        scam_tx_hash,
        trading_enabled,
        trading_enabled_block,
        trading_enabled_tx
    ]
);

impl_from_row!(
    Transaction,
    [
        tx_hash,
        block_number,
        from_address_id,
        to_address_id,
        value,
        status
    ]
);

impl_from_row!(TxParticipant, [tx_hash, address_id]);

impl_from_row!(Block, [block_number, block_timestamp]);

impl_from_row!(
    PnLAnalysis,
    [
        address,
        token_address,
        total_spent,
        total_received,
        realized_profit,
        unrealized_profit,
        roi_percentage,
        num_trades
    ]
);
