// signal_engine/types.rs
//
// Type definitions for market signal detection and signal engine.

use std::collections::HashMap;
use ethers::types::H256;
use serde::{Serialize, Deserialize};

/// Types of market events detected by the signal engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// Critical scam/rugpull detected
    ScamAlert,
    
    /// Significant liquidity change warning
    LiquidityWarning,
    
    /// Token supply anomaly (potential hidden mint)
    TokenSupplyAlert,
    
    /// Unusual volume spike
    VolumeSpike,
    
    /// Large price impact event
    PriceImpact,
    
    /// Large trade detected
    LargeTrade,
}

/// Severity levels for market events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Immediate action required
    Critical,
    
    /// High priority monitoring
    High,
    
    /// Medium priority alert
    Medium,
    
    /// Informational only
    Low,
}

/// A detected market event with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketEvent {
    /// Type of event detected
    pub event_type: EventType,
    
    /// Severity level
    pub severity: Severity,
    
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    
    /// Transaction hash that would cause this event
    pub tx_hash: String,
    
    /// Affected pool address
    pub pool_address: String,
    
    /// Token address
    pub token_address: String,
    
    /// Event-specific metrics
    pub metrics: EventMetrics,
    
    /// Unix timestamp of detection
    pub detection_time: f64,
    
    /// Unix timestamp in seconds (for AlertMessage compatibility)
    pub timestamp: u64,
    
    /// Block number when detected
    pub block_number: u64,
    
    /// Human-readable details
    pub details: String,
}

/// Event-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventMetrics {
    /// ETH change (negative for withdrawals)
    pub eth_change: f64,
    
    /// ETH change percentage
    pub eth_percent: f64,
    
    /// Token change
    pub token_change: f64,
    
    /// Token change percentage
    pub token_percent: f64,
    
    /// New ETH reserve after transaction
    pub new_eth_reserve: f64,
    
    /// New token reserve after transaction
    pub new_token_reserve: f64,
    
    /// Token symbol
    pub token_symbol: String,
    
    /// Additional event-specific data
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Result of simulating the effect of a pending transaction
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Transaction hash
    pub tx_hash: String,
    
    /// Simulation result per affected pool
    pub affected_pools: HashMap<String, PoolEffect>,
    
    /// Whether simulation was successful
    pub simulation_successful: bool,
    
    /// Error message if simulation failed
    pub error_message: Option<String>,
}

/// The effect a transaction would have on a specific pool
#[derive(Debug, Clone)]
pub struct PoolEffect {
    /// Pool address
    pub pool_address: String,
    
    /// Current ETH reserve
    pub current_eth_reserve: f64,
    
    /// Predicted ETH reserve after transaction
    pub simulated_eth_reserve: f64,
    
    /// Current token reserve
    pub current_token_reserve: f64,
    
    /// Predicted token reserve after transaction
    pub simulated_token_reserve: f64,
    
    /// Change in ETH (negative for withdrawals)
    pub eth_delta: f64,
    
    /// Change in tokens
    pub token_delta: f64,
    
    /// Percentage change (negative for withdrawals) - for backward compatibility
    pub percentage_change: f64,
}

// Legacy type aliases and compatibility layer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScamAlertReason {
    EthReserveDepleted,
    LargeEthWithdrawal,
    KnownScamPattern,
    Other,
}

/// Legacy ScamAlert structure - now maps to MarketEvent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScamAlert {
    pub tx_hash: String,
    pub pool_address: String,
    pub current_eth_reserve: f64,
    pub simulated_eth_reserve: f64,
    pub percentage_drain: f64,
    pub detection_time: f64,
    pub detection_block: u64,
    pub reason: ScamAlertReason,
}

impl From<ScamAlert> for MarketEvent {
    fn from(alert: ScamAlert) -> Self {
        let eth_change = alert.simulated_eth_reserve - alert.current_eth_reserve;
        let eth_percent = eth_change / alert.current_eth_reserve;
        
        MarketEvent {
            event_type: EventType::ScamAlert,
            severity: if alert.percentage_drain > 0.9 { 
                Severity::Critical 
            } else { 
                Severity::High 
            },
            confidence: 0.95, // Legacy alerts had high confidence
            tx_hash: alert.tx_hash,
            pool_address: alert.pool_address,
            token_address: String::new(), // Not tracked in legacy
            metrics: EventMetrics {
                eth_change,
                eth_percent,
                token_change: 0.0,
                token_percent: 0.0,
                new_eth_reserve: alert.simulated_eth_reserve,
                new_token_reserve: 0.0,
                token_symbol: String::new(),
                extra: HashMap::new(),
            },
            detection_time: alert.detection_time,
            timestamp: alert.detection_time as u64,
            block_number: alert.detection_block,
            details: format!("Legacy scam alert: {:?}", alert.reason),
        }
    }
}

/// Configuration thresholds for the signal engine
#[derive(Debug, Clone)]
pub struct SignalThresholds {
    /// Minimum ETH reserve threshold (in ETH)
    pub eth_threshold: f64,
    
    /// Percentage threshold for scam detection (0.0-1.0)
    pub scam_drain_percent: f64,
    
    /// Percentage threshold for liquidity warnings (0.0-1.0)
    pub warning_drain_percent: f64,
    
    /// Percentage threshold for token supply alerts (0.0-1.0)
    pub supply_increase_percent: f64,
    
    /// Multiplier for volume spike detection
    pub volume_spike_multiplier: f64,
    
    /// Percentage threshold for price impact (0.0-1.0)
    pub price_impact_percent: f64,
    
    /// Pool size categories (in ETH)
    pub small_pool_max_eth: f64,
    pub medium_pool_max_eth: f64,
}

impl Default for SignalThresholds {
    fn default() -> Self {
        Self {
            eth_threshold: 0.1,
            scam_drain_percent: 0.5,           // 50%
            warning_drain_percent: 0.2,         // 20%
            supply_increase_percent: 0.1,       // 10%
            volume_spike_multiplier: 5.0,       // 5x average
            price_impact_percent: 0.15,         // 15%
            small_pool_max_eth: 5.0,
            medium_pool_max_eth: 50.0,
        }
    }
}

/// Statistics for a pool including volume data
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    /// 24-hour volume in ETH
    pub volume_24h: f64,
    
    /// 24-hour transaction count
    pub tx_count_24h: u32,
    
    /// Average transaction size
    pub avg_tx_size: f64,
    
    /// Last update timestamp
    pub last_updated: f64,
}

// Type aliases for backward compatibility
pub type DecisionThresholds = SignalThresholds;