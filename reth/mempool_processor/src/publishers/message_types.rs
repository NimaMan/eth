//! Message Types for Unified Signal Publishing
//!
//! Defines the common base structure and all signal-specific data types

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Signal severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Source of the signal detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalSource {
    FunctionDetector,
    SimulationEngine,
    CreatorAnalyzer,
    TaxDecoder,
    PatternMatcher,
    Custom(String),
}

/// All possible signal types in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    // Tax-related signals
    TaxManipulation,
    TaxChange,
    
    // Liquidity signals
    LiquidityRemoval,
    LiquidityAddition,
    PoolDrain,
    
    // Trading signals
    TradingEnabled,
    TradingDisabled,
    
    // Creator/Owner actions
    OwnershipChange,
    CreatorAction,
    
    // Market events
    LargeTrade,
    PriceImpact,
    VolumeSpike,
    
    // Security signals
    ScamAlert,
    HoneypotDetected,
    RugPullRisk,
    
    // Token supply
    TokenMint,
    TokenBurn,
    SupplyManipulation,
}

impl SignalType {
    /// Get the ZMQ topic for this signal type
    pub fn topic(&self) -> &'static str {
        match self {
            SignalType::TaxManipulation => "tax_manipulation",
            SignalType::TaxChange => "tax_change",
            SignalType::LiquidityRemoval => "liquidity_removal",
            SignalType::LiquidityAddition => "liquidity_addition",
            SignalType::PoolDrain => "pool_drain",
            SignalType::TradingEnabled => "trading_enabled",
            SignalType::TradingDisabled => "trading_disabled",
            SignalType::OwnershipChange => "ownership_change",
            SignalType::CreatorAction => "creator_action",
            SignalType::LargeTrade => "large_trade",
            SignalType::PriceImpact => "price_impact",
            SignalType::VolumeSpike => "volume_spike",
            SignalType::ScamAlert => "scam_alert",
            SignalType::HoneypotDetected => "honeypot_detected",
            SignalType::RugPullRisk => "rugpull_risk",
            SignalType::TokenMint => "token_mint",
            SignalType::TokenBurn => "token_burn",
            SignalType::SupplyManipulation => "supply_manipulation",
        }
    }
    
    /// Check if this signal type requires simulation
    pub fn requires_simulation(&self) -> bool {
        match self {
            // These need simulation to understand impact
            SignalType::LiquidityRemoval |
            SignalType::PoolDrain |
            SignalType::LargeTrade |
            SignalType::PriceImpact => true,
            
            // These are detected from function calls or state
            SignalType::TaxManipulation |
            SignalType::TaxChange |
            SignalType::TradingEnabled |
            SignalType::TradingDisabled |
            SignalType::OwnershipChange |
            SignalType::CreatorAction |
            SignalType::TokenMint |
            SignalType::TokenBurn => false,
            
            // These might need simulation depending on context
            SignalType::LiquidityAddition |
            SignalType::VolumeSpike |
            SignalType::ScamAlert |
            SignalType::HoneypotDetected |
            SignalType::RugPullRisk |
            SignalType::SupplyManipulation => false, // Can be overridden
        }
    }
}

/// Base structure for all signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalBase {
    // Identity
    pub signal_id: String,
    pub signal_type: SignalType,
    pub timestamp: u64,
    
    // Transaction context
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: Option<String>,
    pub token_address: String,
    pub pool_address: Option<String>,
    
    // Signal metadata
    pub severity: Severity,
    pub confidence: f64,
    pub source: SignalSource,
    
    // Performance
    pub detection_latency_us: u64,
    pub block_number: Option<u64>,
    
    // Additional context
    pub gas_price_gwei: Option<f64>,
    pub value_eth: Option<f64>,
}

/// Tax manipulation specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxManipulationData {
    pub current_buy_tax: f64,
    pub current_sell_tax: f64,
    pub predicted_buy_tax: f64,
    pub predicted_sell_tax: f64,
    pub manipulator_address: String,
    pub function_selector: String,
    pub pattern: String,
}

/// Liquidity removal specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityRemovalData {
    pub function_name: String,
    pub liquidity_token_amount: Option<f64>,
    pub eth_amount: Option<f64>,
    pub token_amount: Option<f64>,
    pub percentage: Option<f64>,
    pub pool_impact: Option<PoolImpact>,
}

/// Trading enabled/disabled data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingStatusData {
    pub function_name: String,
    pub enabled: bool,
    pub pool_has_liquidity: bool,
    pub initial_liquidity_eth: Option<f64>,
    pub initial_liquidity_tokens: Option<f64>,
    pub simulation_confirmed: bool,
}

/// Pool drain data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolDrainData {
    pub current_eth_reserve: f64,
    pub new_eth_reserve: f64,
    pub eth_drained: f64,
    pub drain_percentage: f64,
    pub current_token_reserve: f64,
    pub new_token_reserve: f64,
    pub is_complete_drain: bool,
}

/// Creator action data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorActionData {
    pub function_name: String,
    pub creator_address: String,
    pub action_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Pool impact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolImpact {
    pub price_impact_percent: f64,
    pub liquidity_remaining_percent: f64,
    pub slippage_percent: f64,
}

/// Large trade data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeTradeData {
    pub trade_type: String, // "buy" or "sell"
    pub eth_amount: f64,
    pub token_amount: f64,
    pub price_impact: PoolImpact,
    pub is_dex_trade: bool,
    pub router_address: Option<String>,
}

/// Token supply change data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSupplyData {
    pub action: String, // "mint" or "burn"
    pub amount: f64,
    pub total_supply_before: Option<f64>,
    pub total_supply_after: Option<f64>,
    pub recipient_address: Option<String>,
}

/// Signal-specific data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "data_type", content = "data")]
pub enum SignalData {
    TaxManipulation(TaxManipulationData),
    LiquidityRemoval(LiquidityRemovalData),
    TradingStatus(TradingStatusData),
    PoolDrain(PoolDrainData),
    CreatorAction(CreatorActionData),
    LargeTrade(LargeTradeData),
    TokenSupply(TokenSupplyData),
    Generic(HashMap<String, serde_json::Value>),
}

/// Unified signal structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSignal {
    #[serde(flatten)]
    pub base: SignalBase,
    pub data: SignalData,
    pub details: String,
}

impl UnifiedSignal {
    /// Create a new unified signal
    pub fn new(
        signal_type: SignalType,
        tx_hash: String,
        from_address: String,
        token_address: String,
        data: SignalData,
        details: String,
    ) -> Self {
        let signal_id = format!("{}_{}_{}",
            signal_type.topic(),
            &tx_hash[..8],
            chrono::Utc::now().timestamp_millis()
        );
        
        Self {
            base: SignalBase {
                signal_id,
                signal_type,
                timestamp: chrono::Utc::now().timestamp() as u64,
                tx_hash,
                from_address,
                to_address: None,
                token_address,
                pool_address: None,
                severity: Severity::Medium,
                confidence: 0.8,
                source: SignalSource::FunctionDetector,
                detection_latency_us: 0,
                block_number: None,
                gas_price_gwei: None,
                value_eth: None,
            },
            data,
            details,
        }
    }
    
    /// Get the ZMQ topic for this signal
    pub fn topic(&self) -> &'static str {
        self.base.signal_type.topic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_signal_topics() {
        assert_eq!(SignalType::TaxManipulation.topic(), "tax_manipulation");
        assert_eq!(SignalType::TradingEnabled.topic(), "trading_enabled");
        assert_eq!(SignalType::ScamAlert.topic(), "scam_alert");
    }
    
    #[test]
    fn test_simulation_requirements() {
        assert!(SignalType::LiquidityRemoval.requires_simulation());
        assert!(SignalType::PoolDrain.requires_simulation());
        assert!(!SignalType::TaxChange.requires_simulation());
        assert!(!SignalType::TradingEnabled.requires_simulation());
    }
    
    #[test]
    fn test_unified_signal_creation() {
        let signal = UnifiedSignal::new(
            SignalType::TaxManipulation,
            "0x1234567890abcdef".to_string(),
            "0xfrom".to_string(),
            "0xtoken".to_string(),
            SignalData::TaxManipulation(TaxManipulationData {
                current_buy_tax: 5.0,
                current_sell_tax: 5.0,
                predicted_buy_tax: 50.0,
                predicted_sell_tax: 95.0,
                manipulator_address: "0xmanipulator".to_string(),
                function_selector: "0x12345678".to_string(),
                pattern: "HoneypotSetup".to_string(),
            }),
            "Honeypot pattern detected".to_string(),
        );
        
        assert_eq!(signal.topic(), "tax_manipulation");
        assert!(signal.base.signal_id.starts_with("tax_manipulation_"));
    }
}