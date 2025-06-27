//! Risk Manager Module
//!
//! Manages portfolio risk with daily loss limits, circuit breakers, and position tracking

use ethers::types::{Address, U256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error};
use crate::alert_processor::Alert;

/// Risk decision based on current portfolio state
#[derive(Debug, Clone)]
pub enum RiskDecision {
    /// Allow the trade to proceed
    Allow,
    /// Reduce position size due to risk limits
    ReduceSize { new_amount: U256, reason: String },
    /// Block the trade entirely
    Block { reason: String },
    /// Emergency halt all trading
    EmergencyHalt { reason: String },
}

/// Risk configuration
#[derive(Debug, Clone)]
pub struct RiskConfig {
    /// Maximum daily loss in ETH
    pub max_daily_loss_eth: f64,
    /// Maximum loss per single trade in ETH
    pub max_single_trade_loss_eth: f64,
    /// Maximum portfolio exposure per token (percentage)
    pub max_token_exposure_percent: f64,
    /// Emergency circuit breaker threshold (daily loss %)
    pub emergency_halt_threshold_percent: f64,
    /// Cool-down period after emergency halt (seconds)
    pub emergency_cooldown_seconds: u64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_daily_loss_eth: 10.0,        // 10 ETH max daily loss
            max_single_trade_loss_eth: 2.0,  // 2 ETH max per trade
            max_token_exposure_percent: 50.0, // 50% max in any token
            emergency_halt_threshold_percent: 75.0, // Halt at 75% daily loss
            emergency_cooldown_seconds: 3600, // 1 hour cooldown
        }
    }
}

/// Daily trading statistics
#[derive(Debug, Clone)]
struct DailyStats {
    date: u64, // Unix timestamp of start of day
    total_loss: f64,
    total_trades: u32,
    emergency_halts: u32,
    last_halt_time: Option<u64>,
}

/// Position exposure tracking
#[derive(Debug, Clone)]
struct PositionExposure {
    token_address: Address,
    amount_eth: f64,
    percentage_of_portfolio: f64,
    last_updated: u64,
}

/// Risk manager handles all risk-related decisions
pub struct RiskManager {
    config: RiskConfig,
    daily_stats: DailyStats,
    position_exposures: HashMap<Address, PositionExposure>,
    total_portfolio_value_eth: f64,
    is_halted: bool,
}

impl RiskManager {
    pub fn new(config: RiskConfig) -> Self {
        let current_day = Self::get_current_day();
        
        Self {
            config,
            daily_stats: DailyStats {
                date: current_day,
                total_loss: 0.0,
                total_trades: 0,
                emergency_halts: 0,
                last_halt_time: None,
            },
            position_exposures: HashMap::new(),
            total_portfolio_value_eth: 0.0,
            is_halted: false,
        }
    }
    
    /// Evaluate if a trade should be allowed based on risk parameters
    pub fn evaluate_trade_risk(
        &mut self,
        _alert: &Alert,
        trade_amount_eth: f64,
        token_address: Address,
    ) -> RiskDecision {
        // Check if we're in emergency halt
        if self.is_halted {
            if let Some(halt_time) = self.daily_stats.last_halt_time {
                let now = Self::get_current_timestamp();
                if now - halt_time < self.config.emergency_cooldown_seconds {
                    return RiskDecision::Block {
                        reason: format!(
                            "Emergency halt active, cooldown: {}s remaining",
                            self.config.emergency_cooldown_seconds - (now - halt_time)
                        ),
                    };
                } else {
                    self.is_halted = false;
                    info!("Emergency halt cooldown expired, resuming trading");
                }
            }
        }
        
        // Update daily stats if new day
        self.update_daily_stats();
        
        // Check single trade limit
        if trade_amount_eth > self.config.max_single_trade_loss_eth {
            let reduced_amount = self.config.max_single_trade_loss_eth;
            warn!(
                "Trade amount {} ETH exceeds single trade limit {} ETH",
                trade_amount_eth, self.config.max_single_trade_loss_eth
            );
            
            return RiskDecision::ReduceSize {
                new_amount: U256::from((reduced_amount * 1e18) as u128),
                reason: format!(
                    "Reduced from {} to {} ETH due to single trade limit",
                    trade_amount_eth, reduced_amount
                ),
            };
        }
        
        // Check daily loss limit
        let projected_daily_loss = self.daily_stats.total_loss + trade_amount_eth;
        if projected_daily_loss > self.config.max_daily_loss_eth {
            let remaining_budget = self.config.max_daily_loss_eth - self.daily_stats.total_loss;
            
            if remaining_budget <= 0.0 {
                return RiskDecision::Block {
                    reason: format!(
                        "Daily loss limit {} ETH exceeded (current: {} ETH)",
                        self.config.max_daily_loss_eth,
                        self.daily_stats.total_loss
                    ),
                };
            }
            
            warn!(
                "Trade would exceed daily limit, reducing from {} to {} ETH",
                trade_amount_eth, remaining_budget
            );
            
            return RiskDecision::ReduceSize {
                new_amount: U256::from((remaining_budget * 1e18) as u128),
                reason: format!(
                    "Reduced to {} ETH to stay within daily limit",
                    remaining_budget
                ),
            };
        }
        
        // Check emergency halt threshold
        let loss_percentage = (projected_daily_loss / self.config.max_daily_loss_eth) * 100.0;
        if loss_percentage >= self.config.emergency_halt_threshold_percent {
            self.trigger_emergency_halt(format!(
                "Daily loss {}% exceeds emergency threshold {}%",
                loss_percentage, self.config.emergency_halt_threshold_percent
            ));
            
            return RiskDecision::EmergencyHalt {
                reason: "Emergency circuit breaker activated".to_string(),
            };
        }
        
        // Check token concentration risk
        if let Some(exposure_decision) = self.check_token_exposure(token_address, trade_amount_eth) {
            return exposure_decision;
        }
        
        info!(
            "Trade approved: {} ETH for token {:?} (daily loss: {}/{} ETH)",
            trade_amount_eth,
            token_address,
            projected_daily_loss,
            self.config.max_daily_loss_eth
        );
        
        RiskDecision::Allow
    }
    
    /// Record a completed trade for risk tracking
    pub fn record_trade(&mut self, token_address: Address, loss_amount_eth: f64) {
        self.update_daily_stats();
        
        self.daily_stats.total_loss += loss_amount_eth;
        self.daily_stats.total_trades += 1;
        
        // Update position exposure
        let exposure = self.position_exposures.entry(token_address).or_insert(PositionExposure {
            token_address,
            amount_eth: 0.0,
            percentage_of_portfolio: 0.0,
            last_updated: Self::get_current_timestamp(),
        });
        
        exposure.amount_eth = (exposure.amount_eth - loss_amount_eth).max(0.0);
        exposure.last_updated = Self::get_current_timestamp();
        
        if self.total_portfolio_value_eth > 0.0 {
            exposure.percentage_of_portfolio = (exposure.amount_eth / self.total_portfolio_value_eth) * 100.0;
        }
        
        info!(
            "Recorded trade: {} ETH loss for {:?}, daily total: {} ETH",
            loss_amount_eth, token_address, self.daily_stats.total_loss
        );
    }
    
    /// Update portfolio value for exposure calculations
    pub fn update_portfolio_value(&mut self, total_value_eth: f64) {
        self.total_portfolio_value_eth = total_value_eth;
        
        // Recalculate all position percentages
        for exposure in self.position_exposures.values_mut() {
            if total_value_eth > 0.0 {
                exposure.percentage_of_portfolio = (exposure.amount_eth / total_value_eth) * 100.0;
            }
        }
    }
    
    /// Get current risk statistics
    pub fn get_risk_stats(&self) -> RiskStats {
        RiskStats {
            daily_loss: self.daily_stats.total_loss,
            daily_trades: self.daily_stats.total_trades,
            daily_loss_percentage: if self.config.max_daily_loss_eth > 0.0 {
                (self.daily_stats.total_loss / self.config.max_daily_loss_eth) * 100.0
            } else {
                0.0
            },
            is_halted: self.is_halted,
            total_portfolio_value: self.total_portfolio_value_eth,
            position_count: self.position_exposures.len(),
        }
    }
    
    /// Force emergency halt
    pub fn trigger_emergency_halt(&mut self, reason: String) {
        self.is_halted = true;
        self.daily_stats.emergency_halts += 1;
        self.daily_stats.last_halt_time = Some(Self::get_current_timestamp());
        
        error!("🚨 EMERGENCY HALT TRIGGERED: {}", reason);
    }
    
    /// Clear emergency halt (manual override)
    pub fn clear_emergency_halt(&mut self) {
        self.is_halted = false;
        info!("Emergency halt manually cleared");
    }
    
    fn check_token_exposure(&self, token_address: Address, trade_amount_eth: f64) -> Option<RiskDecision> {
        if self.total_portfolio_value_eth <= 0.0 {
            return None; // Can't calculate exposure without portfolio value
        }
        
        let current_exposure = self.position_exposures
            .get(&token_address)
            .map(|e| e.percentage_of_portfolio)
            .unwrap_or(0.0);
        
        let projected_exposure = ((trade_amount_eth / self.total_portfolio_value_eth) * 100.0) + current_exposure;
        
        if projected_exposure > self.config.max_token_exposure_percent {
            let max_allowed_eth = (self.config.max_token_exposure_percent / 100.0) * self.total_portfolio_value_eth;
            let max_trade_eth = max_allowed_eth - (current_exposure / 100.0 * self.total_portfolio_value_eth);
            
            if max_trade_eth <= 0.0 {
                return Some(RiskDecision::Block {
                    reason: format!(
                        "Token exposure {}% exceeds max {}%",
                        current_exposure, self.config.max_token_exposure_percent
                    ),
                });
            }
            
            return Some(RiskDecision::ReduceSize {
                new_amount: U256::from((max_trade_eth * 1e18) as u128),
                reason: format!(
                    "Reduced to {} ETH to limit token exposure to {}%",
                    max_trade_eth, self.config.max_token_exposure_percent
                ),
            });
        }
        
        None
    }
    
    fn update_daily_stats(&mut self) {
        let current_day = Self::get_current_day();
        
        if current_day != self.daily_stats.date {
            info!(
                "New trading day: Previous loss {} ETH, {} trades",
                self.daily_stats.total_loss, self.daily_stats.total_trades
            );
            
            self.daily_stats = DailyStats {
                date: current_day,
                total_loss: 0.0,
                total_trades: 0,
                emergency_halts: 0,
                last_halt_time: None,
            };
            self.is_halted = false; // Reset halt on new day
        }
    }
    
    fn get_current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
    
    fn get_current_day() -> u64 {
        let now = Self::get_current_timestamp();
        now / 86400 // Seconds per day
    }
}

/// Risk statistics for monitoring
#[derive(Debug, Clone)]
pub struct RiskStats {
    pub daily_loss: f64,
    pub daily_trades: u32,
    pub daily_loss_percentage: f64,
    pub is_halted: bool,
    pub total_portfolio_value: f64,
    pub position_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert_processor::{Action, ExecutionParams, Priority};
    
    fn create_test_alert() -> Alert {
        Alert {
            id: "test_risk".to_string(),
            timestamp: 1234567890,
            token_address: Address::zero(),
            pool_address: Address::zero(),
            action: crate::alert_processor::Action::Sell,
            params: crate::alert_processor::ExecutionParams {
                amount: U256::from(1000),
                slippage: 0.05,
                max_gas_price: None,
                deadline_seconds: 300,
                priority: crate::alert_processor::Priority::Normal,
            },
        }
    }
    
    #[test]
    fn test_single_trade_limit() {
        let config = RiskConfig {
            max_single_trade_loss_eth: 1.0,
            ..Default::default()
        };
        
        let mut manager = RiskManager::new(config);
        let alert = create_test_alert();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        
        let decision = manager.evaluate_trade_risk(&alert, 2.0, token);
        
        match decision {
            RiskDecision::ReduceSize { new_amount, .. } => {
                let reduced_eth = new_amount.as_u128() as f64 / 1e18;
                assert!((reduced_eth - 1.0).abs() < 0.001);
            }
            _ => panic!("Expected ReduceSize decision"),
        }
    }
    
    #[test]
    fn test_daily_loss_limit() {
        let config = RiskConfig {
            max_daily_loss_eth: 5.0,
            ..Default::default()
        };
        
        let mut manager = RiskManager::new(config);
        let alert = create_test_alert();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        
        // Record some previous loss
        manager.record_trade(token, 3.0);
        
        let decision = manager.evaluate_trade_risk(&alert, 3.0, token);
        
        match decision {
            RiskDecision::ReduceSize { new_amount, .. } => {
                let reduced_eth = new_amount.as_u128() as f64 / 1e18;
                assert!((reduced_eth - 2.0).abs() < 0.001); // Should be 5.0 - 3.0 = 2.0
            }
            _ => panic!("Expected ReduceSize decision"),
        }
    }
    
    #[test]
    fn test_emergency_halt_trigger() {
        let config = RiskConfig {
            max_daily_loss_eth: 10.0,
            emergency_halt_threshold_percent: 50.0, // 50% threshold
            ..Default::default()
        };
        
        let mut manager = RiskManager::new(config);
        let alert = create_test_alert();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        
        // Try to make a trade that would trigger emergency halt
        let decision = manager.evaluate_trade_risk(&alert, 6.0, token); // 60% of daily limit
        
        match decision {
            RiskDecision::EmergencyHalt { .. } => {
                assert!(manager.is_halted);
            }
            _ => panic!("Expected EmergencyHalt decision"),
        }
    }
    
    #[test]
    fn test_trade_approval() {
        let config = RiskConfig::default();
        let mut manager = RiskManager::new(config);
        let alert = create_test_alert();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        
        let decision = manager.evaluate_trade_risk(&alert, 1.0, token);
        
        match decision {
            RiskDecision::Allow => {
                // Expected
            }
            _ => panic!("Expected Allow decision"),
        }
    }
}