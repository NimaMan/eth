/// Signal Types for Per-Pool Signal Detection System
/// 
/// CRITICAL ARCHITECTURE:
/// - ALL signals are PER-POOL, not per-token
/// - Each signal uniquely identified by (token_address, pool_address)
/// - A token with multiple pools generates multiple signals
/// - Each signal contains pool-specific metrics

use serde::{Serialize, Deserialize};

/// Binary signal types emitted by detectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Signal {
    TradingEnabled(TradingEnabledSignal),
    TaxSignal(TaxSignalRecord),
    LiquidityRemoval(LiquidityRemovalSignal),
    ScamDetection(ScamDetectionSignal),
}

/// Trading enabled signal
/// 
/// Generated when trading is enabled on a SPECIFIC pool.
/// Each pool of a token gets its own signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingEnabledSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub pool_address: String,  // Each signal is for a specific pool
    pub pool_type: String,      // V2, V3, V4
    pub creator_address: String,
    pub buy_tax: f64,
    pub sell_tax: f64,
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
    pub pool_address: String,    // Pool-specific tax warning
    pub pool_type: String,       // V2, V3, V4
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

/// Liquidity removal signal
/// 
/// Generated when liquidity is removed from a SPECIFIC pool.
/// Tracks the exact pool and amount removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityRemovalSignal {
    pub tx_hash: String,
    pub pool_address: String,
    pub pool_type: String,       // V2, V3, V4
    pub token_address: Option<String>,
    pub remover_address: String,
    pub function_name: String,
    pub estimated_eth_removed: Option<f64>,
    pub timestamp: u64,
}

/// Scam detection signal
/// 
/// Generated when a scam (liquidity drain) is detected on a SPECIFIC pool.
/// Tracks ETH drained from this particular pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScamDetectionSignal {
    pub tx_hash: String,
    pub pool_address: String,
    pub pool_type: String,       // V2, V3, V4
    pub token_address: String,
    pub scammer_address: String,
    pub eth_drained: f64,
    pub eth_remaining: f64,
    pub drain_percentage: f64,
    pub timestamp: u64,
}

/// Tax signal record for publishing
/// 
/// Generated when tax issues detected on a SPECIFIC pool.
/// Covers high taxes, honeypots, and suspicious patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxSignalRecord {
    pub tx_hash: String,
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub creator_address: String,
    pub signal_type: String,  // "HighTaxOrHoneypot", "TaxChange", "SuspiciousPattern"
    pub signal_details: String,
    pub confidence: f64,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub buy_tax_exceeds_threshold: bool,
    pub sell_tax_exceeds_threshold: bool,
    pub cant_sell: bool,
    pub timestamp: u64,
}