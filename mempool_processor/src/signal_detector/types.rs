use super::LpApprovalSignal;
/// Signal Types for Per-Pool Signal Detection System
///
/// CRITICAL ARCHITECTURE:
/// - ALL signals are PER-POOL, not per-token
/// - Each signal uniquely identified by (token_address, pool_address)
/// - A token with multiple pools generates multiple signals
/// - Each signal contains pool-specific metrics
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Binary signal types emitted by detectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Signal {
    TradingEnabled(TradingEnabledSignal),
    TaxSignal(TaxSignalRecord),
    Honeypot(HoneypotSignal),
    LiquidityRemoval(LiquidityRemovalSignal),
    LpApproval(LpApprovalSignal),
    TokenSupplyRisk(TokenSupplyRiskSignal),
}

/// Trading enabled signal
///
/// Generated when trading is enabled on a SPECIFIC pool.
/// Each pool of a token gets its own signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingEnabledSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub pool_address: String, // Each signal is for a specific pool
    pub pool_type: String,    // V2, V3, V4
    #[serde(default)]
    pub denom_address: Option<String>,
    #[serde(default)]
    pub denom_currency: Option<String>,
    #[serde(default)]
    pub denom_decimals: Option<u8>,
    pub creator_address: String,
    pub buy_tax: f64,
    pub sell_tax: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mempool_entry_evidence: Option<Value>,
    pub timestamp: u64,
}

/// High tax warning signal
///
/// Generated when high taxes detected on a SPECIFIC pool.
/// Tax values are measured for this pool only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighTaxWarningSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub pool_address: String, // Pool-specific tax warning
    pub pool_type: String,    // V2, V3, V4
    pub creator_address: Option<String>,
    pub buy_tax: f64,
    pub sell_tax: f64,
    pub warning_type: TaxWarningType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaxWarningType {
    HighBuyTax,
    HighSellTax,
    PotentialHoneypot,
}

/// Honeypot/sell-blocked signal.
///
/// Generated when a pool simulation can buy but cannot sell. This is kept out
/// of tax signals because "cannot sell" is a trading-status failure, not a tax
/// bucket transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoneypotSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String,
    #[serde(default)]
    pub denom_address: Option<String>,
    #[serde(default)]
    pub denom_currency: Option<String>,
    #[serde(default)]
    pub denom_decimals: Option<u8>,
    pub creator_address: String,
    pub can_buy: bool,
    pub can_sell: bool,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub failure_reason: Option<String>,
    pub confidence: f64,
    pub timestamp: u64,
}

/// Liquidity removal signal
///
/// Generated when liquidity is removed from a SPECIFIC pool.
/// Tracks the exact pool and amount removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityRemovalSignal {
    pub tx_hash: String,
    pub pool_address: String,
    pub pool_type: String, // V2, V3, V4
    #[serde(default)]
    pub denom_address: Option<String>,
    #[serde(default)]
    pub denom_currency: Option<String>,
    #[serde(default)]
    pub denom_decimals: Option<u8>,
    pub token_address: Option<String>,
    pub remover_address: String,
    pub function_name: String,
    pub estimated_eth_removed: Option<f64>,
    pub remaining_eth: Option<f64>,
    pub removal_percentage: Option<f64>,
    pub timestamp: u64,
}

/// Tax signal record for publishing
///
/// Generated when tax issues detected on a SPECIFIC pool.
/// Covers tax bucket risks, tax changes, and suspicious tax patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxSignalRecord {
    pub tx_hash: String,
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String,
    #[serde(default)]
    pub denom_address: Option<String>,
    #[serde(default)]
    pub denom_currency: Option<String>,
    #[serde(default)]
    pub denom_decimals: Option<u8>,
    pub creator_address: String,
    pub signal_type: String, // "TaxBucketRisk", "TaxChange", "SuspiciousPattern"
    pub signal_details: String,
    pub confidence: f64,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub buy_tax_bucket_from: Option<String>,
    pub buy_tax_bucket_to: Option<String>,
    pub sell_tax_bucket_from: Option<String>,
    pub sell_tax_bucket_to: Option<String>,
    pub combined_tax_bucket_from: Option<String>,
    pub combined_tax_bucket_to: Option<String>,
    pub buy_tax_exceeds_threshold: bool,
    pub sell_tax_exceeds_threshold: bool,
    pub cant_sell: bool,
    pub timestamp: u64,
}

/// Token-level supply or mint-control risk surfaced from live token context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSupplyRiskSignal {
    pub tx_hash: Option<String>,
    pub token_address: String,
    pub risk_type: String,
    pub risk_details: String,
    pub actor_address: Option<String>,
    pub block_number: Option<u64>,
    pub confidence: f64,
    pub timestamp: u64,
}
