//! Signal Routing Logic
//!
//! Determines how signals should be processed and whether they require simulation

use std::collections::HashSet;
use super::message_types::{SignalType, UnifiedSignal, Severity};

/// Signal routing decisions
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Whether this signal should be published immediately
    pub publish_immediately: bool,
    
    /// Whether this signal requires simulation
    pub requires_simulation: bool,
    
    /// Whether to wait for simulation results before publishing
    pub wait_for_simulation: bool,
    
    /// Additional signals to generate based on this one
    pub trigger_signals: Vec<SignalType>,
    
    /// Minimum severity override for this signal
    pub severity_override: Option<Severity>,
}

/// Signal router that makes routing decisions
pub struct SignalRouter {
    /// Token addresses that are high priority
    high_priority_tokens: HashSet<String>,
    
    /// Addresses that should trigger immediate alerts
    watchlist_addresses: HashSet<String>,
    
    /// Custom routing rules
    custom_rules: Vec<Box<dyn RoutingRule>>,
}

/// Trait for custom routing rules
pub trait RoutingRule: Send + Sync {
    fn evaluate(&self, signal: &UnifiedSignal) -> Option<RoutingDecision>;
}

impl SignalRouter {
    /// Create a new signal router
    pub fn new() -> Self {
        Self {
            high_priority_tokens: HashSet::new(),
            watchlist_addresses: HashSet::new(),
            custom_rules: Vec::new(),
        }
    }
    
    /// Add a high priority token (expects checksummed address)
    pub fn add_high_priority_token(&mut self, token: String) {
        self.high_priority_tokens.insert(token);
    }
    
    /// Add a watchlist address (expects checksummed address)
    pub fn add_watchlist_address(&mut self, address: String) {
        self.watchlist_addresses.insert(address);
    }
    
    /// Add a custom routing rule
    pub fn add_rule(&mut self, rule: Box<dyn RoutingRule>) {
        self.custom_rules.push(rule);
    }
    
    /// Make routing decision for a signal
    pub fn route(&self, signal: &UnifiedSignal) -> RoutingDecision {
        // Check custom rules first
        for rule in &self.custom_rules {
            if let Some(decision) = rule.evaluate(signal) {
                return decision;
            }
        }
        
        // Check if this is a high priority token (using checksummed addresses)
        let is_high_priority = self.high_priority_tokens.contains(&signal.base.token_address);
        
        // Check if addresses are on watchlist (using checksummed addresses)
        let is_watchlist = self.watchlist_addresses.contains(&signal.base.from_address) ||
                          signal.base.to_address.as_ref()
                              .map(|addr| self.watchlist_addresses.contains(addr))
                              .unwrap_or(false);
        
        // Base routing on signal type
        let mut decision = match signal.base.signal_type {
            // Tax manipulation: immediate alert, no simulation needed
            SignalType::TaxManipulation | SignalType::TaxChange => RoutingDecision {
                publish_immediately: true,
                requires_simulation: false,
                wait_for_simulation: false,
                trigger_signals: vec![],
                severity_override: None,
            },
            
            // Liquidity removal: needs simulation to understand impact
            SignalType::LiquidityRemoval => RoutingDecision {
                publish_immediately: true, // Alert immediately
                requires_simulation: true, // But also simulate
                wait_for_simulation: false, // Don't wait
                trigger_signals: vec![SignalType::PoolDrain], // May trigger pool drain
                severity_override: None,
            },
            
            // Trading enabled: trigger additional analysis
            SignalType::TradingEnabled => RoutingDecision {
                publish_immediately: true,
                requires_simulation: true, // Simulate a test swap
                wait_for_simulation: false,
                trigger_signals: vec![SignalType::LiquidityAddition],
                severity_override: None,
            },
            
            // Pool drain: critical, publish immediately
            SignalType::PoolDrain => RoutingDecision {
                publish_immediately: true,
                requires_simulation: false, // Already simulated
                wait_for_simulation: false,
                trigger_signals: vec![SignalType::ScamAlert],
                severity_override: Some(Severity::Critical),
            },
            
            // Large trades: simulate for impact
            SignalType::LargeTrade => RoutingDecision {
                publish_immediately: false, // Wait for simulation
                requires_simulation: true,
                wait_for_simulation: true,
                trigger_signals: vec![SignalType::PriceImpact],
                severity_override: None,
            },
            
            // Default for other types
            _ => RoutingDecision {
                publish_immediately: true,
                requires_simulation: signal.base.signal_type.requires_simulation(),
                wait_for_simulation: false,
                trigger_signals: vec![],
                severity_override: None,
            },
        };
        
        // Override for high priority or watchlist
        if is_high_priority || is_watchlist {
            decision.publish_immediately = true;
            if decision.severity_override.is_none() {
                decision.severity_override = Some(Severity::High);
            }
        }
        
        decision
    }
}

/// Example custom rule: High value transactions
pub struct HighValueRule {
    pub eth_threshold: f64,
}

impl RoutingRule for HighValueRule {
    fn evaluate(&self, signal: &UnifiedSignal) -> Option<RoutingDecision> {
        if let Some(value_eth) = signal.base.value_eth {
            if value_eth > self.eth_threshold {
                return Some(RoutingDecision {
                    publish_immediately: true,
                    requires_simulation: true,
                    wait_for_simulation: false,
                    trigger_signals: vec![SignalType::LargeTrade],
                    severity_override: Some(Severity::High),
                });
            }
        }
        None
    }
}

/// Example custom rule: Known scam patterns
pub struct ScamPatternRule {
    pub scam_functions: HashSet<String>,
}

impl RoutingRule for ScamPatternRule {
    fn evaluate(&self, signal: &UnifiedSignal) -> Option<RoutingDecision> {
        // Check if signal data contains known scam function
        match &signal.data {
            super::message_types::SignalData::CreatorAction(data) => {
                if self.scam_functions.contains(&data.function_name) {
                    return Some(RoutingDecision {
                        publish_immediately: true,
                        requires_simulation: false,
                        wait_for_simulation: false,
                        trigger_signals: vec![SignalType::ScamAlert, SignalType::RugPullRisk],
                        severity_override: Some(Severity::Critical),
                    });
                }
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publishers::message_types::*;
    
    #[test]
    fn test_basic_routing() {
        let router = SignalRouter::new();
        
        // Tax manipulation should publish immediately without simulation
        let tax_signal = UnifiedSignal::new(
            SignalType::TaxManipulation,
            "0x123".to_string(),
            "0xabc".to_string(),
            "0xtoken".to_string(),
            SignalData::Generic(std::collections::HashMap::new()),
            "Test".to_string(),
        );
        
        let decision = router.route(&tax_signal);
        assert!(decision.publish_immediately);
        assert!(!decision.requires_simulation);
        assert!(!decision.wait_for_simulation);
    }
    
    #[test]
    fn test_high_priority_routing() {
        let mut router = SignalRouter::new();
        router.add_high_priority_token("0xtoken".to_string());
        
        let signal = UnifiedSignal::new(
            SignalType::LargeTrade,
            "0x123".to_string(),
            "0xabc".to_string(),
            "0xtoken".to_string(),
            SignalData::Generic(std::collections::HashMap::new()),
            "Test".to_string(),
        );
        
        let decision = router.route(&signal);
        assert!(decision.publish_immediately); // Override for high priority
        assert_eq!(decision.severity_override, Some(Severity::High));
    }
    
    #[test]
    fn test_custom_rule() {
        let mut router = SignalRouter::new();
        router.add_rule(Box::new(HighValueRule { eth_threshold: 10.0 }));
        
        let mut signal = UnifiedSignal::new(
            SignalType::LargeTrade,
            "0x123".to_string(),
            "0xabc".to_string(),
            "0xtoken".to_string(),
            SignalData::Generic(std::collections::HashMap::new()),
            "Test".to_string(),
        );
        signal.base.value_eth = Some(15.0);
        
        let decision = router.route(&signal);
        assert!(decision.publish_immediately);
        assert!(decision.trigger_signals.contains(&SignalType::LargeTrade));
    }
}