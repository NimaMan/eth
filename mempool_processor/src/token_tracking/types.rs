// types_v2.rs - Unified, efficient types for token tracking
//
// Core principles:
// 1. Single source of truth for each concept
// 2. Arc-wrapped for zero-copy sharing
// 3. Proper type aliases for clarity
// 4. Serde compatibility with token-server HTTP views

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type alias for Ethereum addresses (checksummed hex strings)
pub type Address = String;

/// Type alias for transaction hashes
pub type TxHash = String;

/// Type alias for block numbers
pub type BlockNumber = u64;

/// Pool types in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PoolType {
    #[serde(rename = "UNISWAP-V2")]
    UniswapV2,
    #[serde(rename = "UNISWAP-V3")]
    UniswapV3,
    #[serde(rename = "UNISWAP-V4")]
    UniswapV4,
    #[serde(rename = "SUSHISWAP-V2")]
    SushiSwapV2,
    #[serde(rename = "SUSHISWAP-V3")]
    SushiSwapV3,
    #[serde(rename = "PANCAKESWAP-V2")]
    PancakeSwapV2,
    #[serde(rename = "PANCAKESWAP-V3")]
    PancakeSwapV3,
    #[serde(rename = "SHIBASWAP-V2")]
    ShibaSwapV2,
    #[serde(rename = "FRAXSWAP-V2")]
    FraxswapV2,
    #[serde(rename = "CURVE")]
    Curve,
    #[serde(rename = "BALANCER")]
    Balancer,
    #[serde(other)]
    Unknown,
}

/// Complete token information - single source of truth
#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    // Identity
    #[serde(alias = "token_address")]
    pub address: Address,
    #[serde(alias = "token_symbol")]
    pub symbol: String,
    #[serde(alias = "token_name")]
    pub name: String,
    #[serde(alias = "token_decimals")]
    pub decimals: u8,
    #[serde(deserialize_with = "deserialize_supply")]
    pub total_supply: Option<String>,

    // Ownership
    pub creator_address: Address,
    pub current_owner: Address,
    #[serde(default)]
    pub tax_setter_addresses: Vec<Address>,
    pub ownership_renounced: bool,
    #[serde(default)]
    pub renouncement_block: Option<BlockNumber>,

    // Tax state in percentage points (0-100 range)
    #[serde(alias = "current_buy_tax")]
    pub buy_tax: Option<f64>,
    #[serde(alias = "current_sell_tax")]
    pub sell_tax: Option<f64>,
    #[serde(default)]
    pub last_tax_change_block: Option<BlockNumber>,

    // Tax history
    #[serde(default)]
    pub tax_history: Vec<TaxChange>,
    // Metadata
    pub creation_block: BlockNumber,
    #[serde(alias = "creation_tx")]
    pub creation_tx: TxHash,
    #[serde(default)]
    pub creation_timestamp: Option<f64>,
    pub latest_activity_block: BlockNumber,

    // Scam detection
    pub is_scam: bool,
    pub scam_label: Option<String>,

    #[serde(skip)]
    pub total_liquidity: f64,
}

/// Lifecycle states shared with token-server views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PoolLifecycle {
    Discovered,
    #[serde(alias = "SEEDED")]
    LiquidityDeposited,
    Active,
    Scam,
    Evicted,
    #[serde(other)]
    Unknown,
}

impl Default for PoolLifecycle {
    fn default() -> Self {
        PoolLifecycle::Unknown
    }
}

/// Complete pool information - single source of truth
#[derive(Debug, Clone, Deserialize)]
pub struct Pool {
    // Identity
    #[serde(alias = "pool_address")]
    pub address: Address,
    #[serde(default)] // Not provided when nested in token
    pub token_address: Address,
    pub pool_type: PoolType,

    // Reserves
    pub token_reserve: f64,
    #[serde(alias = "denom_reserve")]
    pub eth_reserve: f64,
    pub denom_currency: String,
    pub denom_address: Address,

    // Trading state (per-pool, not per-token)
    #[serde(default)]
    pub trading_enabled: bool,
    pub trading_enabled_block: Option<BlockNumber>,
    pub trading_enabled_tx: Option<TxHash>,

    // Metadata
    pub fee_tier: Option<u32>,
    pub pool_id: Option<String>,
    /// ERC20-style liquidity ownership token for V2 LP/BPT/Curve LP approvals.
    ///
    /// For V2-style pools this is normally the pool address. For Balancer this
    /// is the BPT address when it differs from the vault#pool_id identifier.
    #[serde(default)]
    pub lp_token_address: Option<Address>,
    #[serde(default)]
    pub position_manager_address: Option<Address>,
    #[serde(default)]
    pub lp_total_supply: Option<f64>,
    #[serde(default)]
    pub liquidity_positions: Vec<ConcentratedLiquidityPosition>,
    #[serde(alias = "latest_block_number")]
    pub last_updated_block: BlockNumber,
    #[serde(alias = "last_update_time")]
    pub last_updated_time: f64,

    // Scam detection
    pub is_scam: bool,
    pub scam_label: Option<String>,

    // LP token approval tracking (V2 pools only)
    #[serde(default)]
    pub lp_tokens_approved_percentage: Option<f64>,

    // Lifecycle tracking
    #[serde(default)]
    pub lifecycle: PoolLifecycle,

    #[serde(default)]
    pub control_addresses: Vec<Address>,

    #[serde(default)]
    pub can_buy: bool,

    #[serde(default)]
    pub can_sell: bool,

    // Local cache metadata
    #[serde(skip, default = "std::time::Instant::now")]
    pub received_at: std::time::Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConcentratedLiquidityPosition {
    pub position_id: String,
    #[serde(default)]
    pub token_id: Option<String>,
    pub owner: Address,
    #[serde(default)]
    pub position_manager_address: Option<Address>,
    pub liquidity: String,
    #[serde(default)]
    pub position_share_pct: Option<f64>,
    #[serde(default)]
    pub tick_lower: Option<i32>,
    #[serde(default)]
    pub tick_upper: Option<i32>,
    #[serde(default)]
    pub last_update_block: Option<BlockNumber>,
    #[serde(default)]
    pub last_update_tx: Option<TxHash>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConcentratedPositionApprovalContext {
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub denom_address: Address,
    pub denom_currency: String,
    pub position_manager_address: Address,
    pub position_id: String,
    pub token_id: Option<String>,
    pub owner: Address,
    pub position_liquidity: String,
    pub pool_liquidity: Option<String>,
    pub position_share_pct: Option<f64>,
}

/// Tax change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxChange {
    pub block_number: BlockNumber,
    pub timestamp: f64,
    pub transaction_hash: TxHash,
    pub changer_address: Address,
    pub old_buy_tax: f64,
    pub new_buy_tax: f64,
    pub old_sell_tax: f64,
    pub new_sell_tax: f64,
}

/// Pending tax change from mempool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTaxChange {
    pub transaction_hash: TxHash,
    pub from_address: Address,
    pub predicted_buy_tax: f64,
    pub predicted_sell_tax: f64,
    pub confidence: f64,
    #[serde(default)]
    pub detection_timestamp: Option<f64>,
    #[serde(default)]
    pub function_selector: Option<String>,
}

/// Batch token/pool cache update.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenUpdate {
    pub message_type: String,
    pub token_count: usize,
    pub block_number: BlockNumber,
    pub timestamp: f64,
    pub data: HashMap<Address, TokenWithPools>,
}

/// Token with embedded pool context from token-server views.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenWithPools {
    // All Token fields
    #[serde(flatten)]
    pub token: Token,

    // Pools mapped by address
    pub pools: HashMap<Address, Pool>,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_tokens: usize,
    pub max_pools: usize,
    pub eth_threshold: f64,
    pub evict_scam_first: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_tokens: 10_000,
            max_pools: 100_000,
            eth_threshold: 0.1,
            evict_scam_first: true,
        }
    }
}

/// Custom deserializer for total_supply that handles both strings and numbers
fn deserialize_supply<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct SupplyVisitor;

    impl<'de> Visitor<'de> for SupplyVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, number, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(InnerSupplyVisitor)
        }
    }

    struct InnerSupplyVisitor;

    impl<'de> Visitor<'de> for InnerSupplyVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v.to_string()))
        }
    }

    deserializer.deserialize_option(SupplyVisitor)
}
