/// Data models for the analytics database
///
/// These structures represent the data stored in various tables
/// of the analytics MDBX database.
use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};

/// Aggregated trading data for an address-token pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    /// Block number of first trade
    pub entry_block: u64,
    /// Block number of most recent trade
    pub latest_block: u64,
    /// Total amount of currency spent buying this token
    pub total_spent: U256,
    /// Total amount of currency received from selling
    pub total_received: U256,
    /// Realized profit/loss in currency terms
    pub realized_profit: i128,
    /// Unrealized profit/loss based on current holdings
    pub unrealized_profit: i128,
    /// Number of buy transactions
    pub num_buys: u32,
    /// Number of sell transactions
    pub num_sells: u32,
    /// Current token balance
    pub token_balance: U256,
    /// Total gas spent on trades
    pub gas_spent: U256,
}

/// Aggregated metrics for an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressMetrics {
    /// First block where address was seen
    pub first_seen_block: u64,
    /// Most recent block with activity
    pub last_seen_block: u64,
    /// Total number of transactions
    pub total_transactions: u64,
    /// Total volume in USD equivalent
    pub total_volume_usd: f64,
    /// Total gas spent across all transactions
    pub total_gas_spent: U256,
    /// Total profit across all trades
    pub total_profit: f64,
    /// Number of interactions with known scams
    pub scam_interactions: u32,
    /// Whether this is a contract address
    pub is_contract: bool,
    /// Entity classification (DEX, CEX, MEV, etc.)
    pub entity_type: Option<String>,
}

/// Token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    /// Token name
    pub name: String,
    /// Token symbol
    pub symbol: String,
    /// Number of decimals
    pub decimals: u8,
    /// Total supply
    pub total_supply: U256,
    /// Address that created the token
    pub creator_address: Address,
    /// Block where token was created
    pub creation_block: u64,
    /// Transaction that created the token
    pub creation_tx: B256,
    /// Whether token is identified as scam
    pub is_scam: bool,
    /// Reason for scam classification
    pub scam_reason: Option<String>,
}

/// DEX pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolData {
    /// First token in the pair
    pub token0: Address,
    /// Second token in the pair
    pub token1: Address,
    /// Type of pool (V2, V3, V4)
    pub pool_type: PoolType,
    /// Fee tier in basis points (100 = 0.01%)
    pub fee_tier: u32,
    /// Block where pool was created
    pub creation_block: u64,
    /// Transaction that created the pool
    pub creation_tx: B256,
    /// Whether pool is currently active
    pub is_active: bool,
    /// Total volume traded through pool
    pub total_volume: U256,
    /// Most recent activity block
    pub last_activity_block: u64,
}

/// Pool type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PoolType {
    UniswapV2,
    UniswapV3,
    UniswapV4,
    SushiswapV2,
    BalancerV2,
    CurveV1,
    Other,
}

/// Currency type for trades
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Currency {
    ETH,
    WETH,
    USDC,
    USDT,
    DAI,
    Other(u8), // For other currencies, store an ID
}

impl Currency {
    /// Convert to single byte for storage
    pub fn to_byte(&self) -> u8 {
        match self {
            Currency::ETH => 0,
            Currency::WETH => 1,
            Currency::USDC => 2,
            Currency::USDT => 3,
            Currency::DAI => 4,
            Currency::Other(id) => 100 + id,
        }
    }

    /// Parse from byte
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Currency::ETH,
            1 => Currency::WETH,
            2 => Currency::USDC,
            3 => Currency::USDT,
            4 => Currency::DAI,
            b if b >= 100 => Currency::Other(b - 100),
            _ => Currency::Other(byte),
        }
    }
}
