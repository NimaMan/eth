//! Alert types for receiving scam detection signals
//!
//! These types match the AlertMessage structure from mempool processor

use serde::{Deserialize, Serialize};
use ethers::types::{Address, H256};

/// Alert message received from mempool processor
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlertMessage {
    // Alert metadata
    pub alert_id: String,
    pub timestamp: u64,
    pub severity: String,
    pub event_type: String,
    
    // Transaction context
    pub tx_hash: String,
    pub detected_latency_us: u64,
    
    // Pool information
    pub pool_address: String,
    pub pool_version: String,
    pub token_address: String,
    pub token_symbol: String,
    pub token_decimals: u8,
    
    // State changes
    pub current_eth_reserve: f64,
    pub simulated_eth_reserve: f64,
    pub eth_change_amount: f64,
    pub eth_change_percent: f64,
    
    // Current prices
    pub current_price: f64,
    pub simulated_price: f64,
    pub price_impact_percent: f64,
    
    // Risk metrics
    pub confidence_score: f64,
    pub gas_price_gwei: f64,
    
    // Additional context
    pub details: String,
}

/// Severity levels for alerts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl From<&str> for Severity {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" => Severity::Medium,
            _ => Severity::Low,
        }
    }
}

/// Event types we handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum EventType {
    ScamAlert,
    LiquidityWarning,
    LargeTrade,
}

impl From<&str> for EventType {
    fn from(s: &str) -> Self {
        match s {
            "ScamAlert" => EventType::ScamAlert,
            "LiquidityWarning" => EventType::LiquidityWarning,
            "LargeTrade" => EventType::LargeTrade,
            _ => EventType::LiquidityWarning, // Default
        }
    }
}

/// Internal scam alert representation with parsed fields
#[derive(Debug, Clone)]
pub struct ScamAlert {
    pub alert_id: String,
    pub timestamp: u64,
    pub severity: Severity,
    pub event_type: EventType,
    
    // Parsed addresses
    pub tx_hash: H256,
    pub pool_address: Address,
    pub token_address: Address,
    
    // Pool state
    pub current_eth_reserve: f64,
    pub simulated_eth_reserve: f64,
    pub eth_change_percent: f64,
    
    // Token info
    pub token_symbol: String,
    pub token_decimals: u8,
    
    // Risk assessment
    pub confidence_score: f64,
    pub requires_action: bool,
    
    // Original message for debugging
    pub raw_message: AlertMessage,
}

impl ScamAlert {
    /// Convert from AlertMessage to ScamAlert with validation
    pub fn from_alert_message(msg: AlertMessage) -> Result<Self, String> {
        // Parse addresses
        let tx_hash = msg.tx_hash.parse::<H256>()
            .map_err(|_| format!("Invalid tx_hash: {}", msg.tx_hash))?;
            
        let pool_address = msg.pool_address.parse::<Address>()
            .map_err(|_| format!("Invalid pool_address: {}", msg.pool_address))?;
            
        let token_address = msg.token_address.parse::<Address>()
            .map_err(|_| format!("Invalid token_address: {}", msg.token_address))?;
        
        let severity = Severity::from(msg.severity.as_str());
        let event_type = EventType::from(msg.event_type.as_str());
        
        // Determine if action is required based on severity and drain percentage
        let requires_action = match (severity, msg.eth_change_percent) {
            (Severity::Critical, drain) if drain < -80.0 => true,  // >80% drain
            (Severity::High, drain) if drain < -50.0 => true,      // >50% drain  
            _ => false,
        };
        
        Ok(ScamAlert {
            alert_id: msg.alert_id.clone(),
            timestamp: msg.timestamp,
            severity,
            event_type,
            tx_hash,
            pool_address,
            token_address,
            current_eth_reserve: msg.current_eth_reserve,
            simulated_eth_reserve: msg.simulated_eth_reserve,
            eth_change_percent: msg.eth_change_percent,
            token_symbol: msg.token_symbol.clone(),
            token_decimals: msg.token_decimals,
            confidence_score: msg.confidence_score,
            requires_action,
            raw_message: msg,
        })
    }
    
    /// Get the amount of ETH being drained
    pub fn eth_drain_amount(&self) -> f64 {
        self.current_eth_reserve - self.simulated_eth_reserve
    }
    
    /// Check if this is an emergency requiring immediate action
    pub fn is_emergency(&self) -> bool {
        self.severity == Severity::Critical && self.eth_change_percent < -80.0
    }
}