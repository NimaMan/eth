/// Common signal types for the signal detection system

use serde::{Serialize, Deserialize};

/// Binary signal types emitted by detectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Signal {
    TradingEnabled(TradingEnabledSignal),
    HighTaxWarning(HighTaxWarningSignal),
    LiquidityRemoval(LiquidityRemovalSignal),
    ScamDetection(ScamDetectionSignal),
}

/// Trading enabled signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingEnabledSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub creator_address: String,
    pub buy_tax: u8,
    pub sell_tax: u8,
    pub timestamp: u64,
    pub block_number: u64,
}

/// High tax warning signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighTaxWarningSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub creator_address: Option<String>,
    pub buy_tax: u8,
    pub sell_tax: u8,
    pub warning_type: TaxWarningType,
    pub timestamp: u64,
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaxWarningType {
    HighBuyTax,
    HighSellTax,
    PotentialHoneypot,
}

/// Liquidity removal signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityRemovalSignal {
    pub tx_hash: String,
    pub pool_address: String,
    pub token_address: Option<String>,
    pub remover_address: String,
    pub function_name: String,
    pub estimated_eth_removed: Option<f64>,
    pub timestamp: u64,
    pub block_number: u64,
}

/// Scam detection signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScamDetectionSignal {
    pub tx_hash: String,
    pub pool_address: String,
    pub token_address: String,
    pub scammer_address: String,
    pub eth_drained: f64,
    pub eth_remaining: f64,
    pub drain_percentage: f64,
    pub timestamp: u64,
    pub block_number: u64,
}