//! Decision Engine for protective trading strategies
//!
//! Makes real-time decisions on how to respond to scam alerts

use crate::alert_processor::{ScamAlert, Severity};
use crate::wallet::PositionTracker;
use ethers::types::{Address, U256};
use std::sync::Arc;
use tracing::{info, warn, error};

/// Decision made by the strategy engine
#[derive(Debug, Clone)]
pub enum TradingDecision {
    /// Execute emergency sell - dump everything ASAP
    EmergencySell {
        token: Address,
        amount: U256,
        reason: String,
    },
    /// Execute partial sell - reduce exposure
    PartialSell {
        token: Address,
        amount: U256,
        percentage: f64,
        reason: String,
    },
    /// Monitor only - no immediate action
    Monitor {
        token: Address,
        reason: String,
    },
    /// Skip - we don't hold this token
    Skip {
        token: Address,
        reason: String,
    },
}

/// Configuration for decision engine
#[derive(Debug, Clone)]
pub struct DecisionConfig {
    /// Minimum ETH value to take action (avoid dust trades)
    pub min_value_eth: f64,
    /// Emergency sell threshold (% drain)
    pub emergency_threshold: f64,
    /// Partial sell threshold (% drain)  
    pub partial_threshold: f64,
    /// Minimum confidence score to act
    pub min_confidence: f64,
    /// Enable test mode (no real trades)
    pub test_mode: bool,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            min_value_eth: 0.01,      // $30+ at 3000 ETH
            emergency_threshold: 80.0,  // >80% drain = emergency
            partial_threshold: 50.0,    // >50% drain = partial exit
            min_confidence: 0.8,        // 80% confidence minimum
            test_mode: false,
        }
    }
}

/// Decision engine for protective trading
pub struct DecisionEngine {
    config: DecisionConfig,
    position_tracker: Arc<PositionTracker>,
}

impl DecisionEngine {
    /// Create new decision engine
    pub fn new(config: DecisionConfig, position_tracker: Arc<PositionTracker>) -> Self {
        Self {
            config,
            position_tracker,
        }
    }
    
    /// Process a scam alert and decide what to do
    pub async fn process_alert(&self, alert: &ScamAlert) -> Result<TradingDecision, Box<dyn std::error::Error>> {
        info!("Processing alert for {} ({})", alert.token_symbol, alert.token_address);
        
        // Step 1: Check if we hold this token
        let position = match self.position_tracker.get_position(alert.token_address).await {
            Ok(pos) if pos.balance > U256::zero() => pos,
            Ok(_) => {
                info!("No position in {}, skipping", alert.token_symbol);
                return Ok(TradingDecision::Skip {
                    token: alert.token_address,
                    reason: "No tokens held".to_string(),
                });
            }
            Err(e) => {
                warn!("Failed to get position for {}: {}", alert.token_symbol, e);
                return Ok(TradingDecision::Skip {
                    token: alert.token_address,
                    reason: format!("Position check failed: {}", e),
                });
            }
        };
        
        // Step 2: Check confidence threshold
        if alert.confidence_score < self.config.min_confidence {
            info!("Low confidence ({:.2}), monitoring only", alert.confidence_score);
            return Ok(TradingDecision::Monitor {
                token: alert.token_address,
                reason: format!("Low confidence: {:.2}", alert.confidence_score),
            });
        }
        
        // Step 3: Estimate value (rough estimate based on current pool state)
        let token_price = alert.current_eth_reserve / (10f64.powi(18)); // Simplified
        let position_value_eth = position.balance_formatted * token_price;
        
        if position_value_eth < self.config.min_value_eth {
            info!("Position value too low ({:.4} ETH), skipping", position_value_eth);
            return Ok(TradingDecision::Skip {
                token: alert.token_address,
                reason: format!("Value below minimum: {:.4} ETH", position_value_eth),
            });
        }
        
        // Step 4: Decide based on severity and drain percentage
        let drain_percent = -alert.eth_change_percent; // Convert negative to positive
        
        let decision = match (alert.severity, drain_percent) {
            // Critical emergency - sell everything
            (Severity::Critical, drain) if drain >= self.config.emergency_threshold => {
                error!("🚨 EMERGENCY SELL: {} drained {:.1}%", alert.token_symbol, drain);
                TradingDecision::EmergencySell {
                    token: alert.token_address,
                    amount: position.balance,
                    reason: format!("Emergency: {:.1}% drain detected", drain),
                }
            }
            // High severity - partial sell
            (Severity::High, drain) if drain >= self.config.partial_threshold => {
                warn!("⚠️  PARTIAL SELL: {} drained {:.1}%", alert.token_symbol, drain);
                let sell_percentage = 0.75; // Sell 75% on high alerts
                let sell_amount = position.balance * U256::from((sell_percentage * 1000.0) as u64) / U256::from(1000);
                TradingDecision::PartialSell {
                    token: alert.token_address,
                    amount: sell_amount,
                    percentage: sell_percentage * 100.0,
                    reason: format!("High risk: {:.1}% drain detected", drain),
                }
            }
            // Medium severity - smaller partial sell
            (Severity::Medium, drain) if drain >= self.config.partial_threshold * 0.5 => {
                info!("📊 SMALL SELL: {} changed {:.1}%", alert.token_symbol, drain);
                let sell_percentage = 0.25; // Sell 25% on medium alerts
                let sell_amount = position.balance * U256::from((sell_percentage * 1000.0) as u64) / U256::from(1000);
                TradingDecision::PartialSell {
                    token: alert.token_address,
                    amount: sell_amount,
                    percentage: sell_percentage * 100.0,
                    reason: format!("Medium risk: {:.1}% change detected", drain),
                }
            }
            // Otherwise just monitor
            _ => {
                info!("👁️  MONITORING: {} ({:.1}% change)", alert.token_symbol, drain_percent);
                TradingDecision::Monitor {
                    token: alert.token_address,
                    reason: format!("Below action threshold: {:.1}% change", drain_percent),
                }
            }
        };
        
        // Log decision
        match &decision {
            TradingDecision::EmergencySell { amount, .. } => {
                error!("DECISION: Emergency sell {} {} tokens", amount, alert.token_symbol);
            }
            TradingDecision::PartialSell { amount, percentage, .. } => {
                warn!("DECISION: Sell {:.1}% ({} tokens) of {}", percentage, amount, alert.token_symbol);
            }
            TradingDecision::Monitor { .. } => {
                info!("DECISION: Monitor {} for now", alert.token_symbol);
            }
            TradingDecision::Skip { reason, .. } => {
                info!("DECISION: Skip {} - {}", alert.token_symbol, reason);
            }
        }
        
        Ok(decision)
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: DecisionConfig) {
        self.config = config;
        info!("Decision engine config updated");
    }
    
    /// Get current configuration
    pub fn config(&self) -> &DecisionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert_processor::EventType;
    
    #[test]
    fn test_decision_thresholds() {
        let config = DecisionConfig::default();
        assert_eq!(config.emergency_threshold, 80.0);
        assert_eq!(config.partial_threshold, 50.0);
    }
    
    #[test]
    fn test_drain_percentage_conversion() {
        // Alert has negative percent, we convert to positive for comparison
        let drain_percent = -95.0;
        let positive_drain = -drain_percent;
        assert_eq!(positive_drain, 95.0);
    }
}