//! Simulation Module
//!
//! Provides safe testing environment for strategy validation without real transactions

use ethers::types::{Address, H256};
use std::collections::HashMap;
use tracing::info;
use crate::alert_processor::Alert;

/// Simulation mode configuration
#[derive(Debug, Clone)]
pub enum SimulationMode {
    /// No simulation - execute real transactions
    Off,
    /// Log only - print what would be executed but don't submit
    LogOnly,
    /// Full simulation with virtual portfolio tracking
    Full {
        /// Starting ETH balance for simulation
        initial_eth_balance: f64,
        /// Virtual token positions
        initial_token_positions: HashMap<Address, f64>,
    },
}

/// Result of a simulated operation
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Whether the operation would have succeeded
    pub would_succeed: bool,
    /// Estimated gas cost in ETH
    pub estimated_gas_cost_eth: f64,
    /// Estimated execution time in milliseconds
    pub estimated_execution_time_ms: u64,
    /// Profit/loss if executed (negative = loss)
    pub estimated_pnl_eth: f64,
    /// Reason for failure (if would_succeed = false)
    pub failure_reason: Option<String>,
    /// Virtual transaction hash for tracking
    pub virtual_tx_hash: H256,
}

/// Virtual portfolio state for full simulation
#[derive(Debug, Clone)]
struct VirtualPortfolio {
    eth_balance: f64,
    token_positions: HashMap<Address, f64>,
    total_trades: u32,
    total_pnl: f64,
    gas_spent: f64,
}

/// Simulation engine
pub struct SimulationEngine {
    mode: SimulationMode,
    virtual_portfolio: Option<VirtualPortfolio>,
    transaction_counter: u64,
}

impl SimulationEngine {
    pub fn new(mode: SimulationMode) -> Self {
        let virtual_portfolio = match &mode {
            SimulationMode::Full { initial_eth_balance, initial_token_positions } => {
                Some(VirtualPortfolio {
                    eth_balance: *initial_eth_balance,
                    token_positions: initial_token_positions.clone(),
                    total_trades: 0,
                    total_pnl: 0.0,
                    gas_spent: 0.0,
                })
            }
            _ => None,
        };
        
        Self {
            mode,
            virtual_portfolio,
            transaction_counter: 0,
        }
    }
    
    /// Check if we're in simulation mode
    pub fn is_simulation_active(&self) -> bool {
        !matches!(self.mode, SimulationMode::Off)
    }
    
    /// Simulate an emergency sell transaction
    pub fn simulate_emergency_sell(
        &mut self,
        alert: &Alert,
        token_address: Address,
        amount_eth: f64,
    ) -> SimulationResult {
        self.transaction_counter += 1;
        
        match &self.mode {
            SimulationMode::Off => {
                // Should not be called in off mode
                SimulationResult {
                    would_succeed: false,
                    estimated_gas_cost_eth: 0.0,
                    estimated_execution_time_ms: 0,
                    estimated_pnl_eth: 0.0,
                    failure_reason: Some("Simulation is disabled".to_string()),
                    virtual_tx_hash: H256::random(),
                }
            }
            
            SimulationMode::LogOnly => {
                info!(
                    "🎭 SIMULATION LOG: Would execute emergency sell of {} ETH for token {:?} due to alert {}",
                    amount_eth, token_address, alert.id
                );
                
                // Estimate realistic values
                let gas_cost = self.estimate_gas_cost(amount_eth);
                let execution_time = self.estimate_execution_time(&alert);
                let estimated_pnl = -amount_eth; // Assume full loss in emergency
                
                SimulationResult {
                    would_succeed: true,
                    estimated_gas_cost_eth: gas_cost,
                    estimated_execution_time_ms: execution_time,
                    estimated_pnl_eth: estimated_pnl,
                    failure_reason: None,
                    virtual_tx_hash: H256::random(),
                }
            }
            
            SimulationMode::Full { .. } => {
                self.simulate_full_emergency_sell(alert, token_address, amount_eth)
            }
        }
    }
    
    /// Simulate a partial sell transaction
    pub fn simulate_partial_sell(
        &mut self,
        alert: &Alert,
        token_address: Address,
        amount_eth: f64,
        percentage: f64,
    ) -> SimulationResult {
        self.transaction_counter += 1;
        
        match &self.mode {
            SimulationMode::Off => {
                SimulationResult {
                    would_succeed: false,
                    estimated_gas_cost_eth: 0.0,
                    estimated_execution_time_ms: 0,
                    estimated_pnl_eth: 0.0,
                    failure_reason: Some("Simulation is disabled".to_string()),
                    virtual_tx_hash: H256::random(),
                }
            }
            
            SimulationMode::LogOnly => {
                info!(
                    "🎭 SIMULATION LOG: Would execute partial sell of {} ETH ({}%) for token {:?} due to alert {}",
                    amount_eth, percentage, token_address, alert.id
                );
                
                let gas_cost = self.estimate_gas_cost(amount_eth);
                let execution_time = self.estimate_execution_time(&alert);
                // Partial sell might preserve some value
                let estimated_pnl = -amount_eth * (percentage / 100.0) * 0.8; // 80% loss estimate
                
                SimulationResult {
                    would_succeed: true,
                    estimated_gas_cost_eth: gas_cost,
                    estimated_execution_time_ms: execution_time,
                    estimated_pnl_eth: estimated_pnl,
                    failure_reason: None,
                    virtual_tx_hash: H256::random(),
                }
            }
            
            SimulationMode::Full { .. } => {
                self.simulate_full_partial_sell(alert, token_address, amount_eth, percentage)
            }
        }
    }
    
    /// Get current virtual portfolio state (if in full simulation)
    pub fn get_virtual_portfolio(&self) -> Option<VirtualPortfolioStats> {
        self.virtual_portfolio.as_ref().map(|portfolio| {
            VirtualPortfolioStats {
                eth_balance: portfolio.eth_balance,
                token_positions: portfolio.token_positions.clone(),
                total_trades: portfolio.total_trades,
                total_pnl: portfolio.total_pnl,
                gas_spent: portfolio.gas_spent,
                roi_percentage: if portfolio.gas_spent > 0.0 {
                    (portfolio.total_pnl / portfolio.gas_spent) * 100.0
                } else {
                    0.0
                },
            }
        })
    }
    
    /// Reset virtual portfolio to initial state
    pub fn reset_virtual_portfolio(&mut self) {
        if let SimulationMode::Full { initial_eth_balance, initial_token_positions } = &self.mode {
            self.virtual_portfolio = Some(VirtualPortfolio {
                eth_balance: *initial_eth_balance,
                token_positions: initial_token_positions.clone(),
                total_trades: 0,
                total_pnl: 0.0,
                gas_spent: 0.0,
            });
            
            info!("🔄 Virtual portfolio reset to initial state");
        }
    }
    
    fn simulate_full_emergency_sell(
        &mut self,
        alert: &Alert,
        token_address: Address,
        amount_eth: f64,
    ) -> SimulationResult {
        // Calculate values first to avoid borrow conflicts
        let gas_cost = self.estimate_gas_cost(amount_eth);
        let execution_time = self.estimate_execution_time(&alert);
        
        let portfolio = self.virtual_portfolio.as_mut().unwrap();
        
        // Check if we have the position
        let current_position = *portfolio.token_positions.get(&token_address).unwrap_or(&0.0);
        
        if current_position < amount_eth {
            return SimulationResult {
                would_succeed: false,
                estimated_gas_cost_eth: 0.0,
                estimated_execution_time_ms: 0,
                estimated_pnl_eth: 0.0,
                failure_reason: Some(format!(
                    "Insufficient position: {} ETH available, {} ETH requested",
                    current_position, amount_eth
                )),
                virtual_tx_hash: H256::random(),
            };
        }
        
        // Check if we have enough ETH for gas
        if portfolio.eth_balance < gas_cost {
            return SimulationResult {
                would_succeed: false,
                estimated_gas_cost_eth: gas_cost,
                estimated_execution_time_ms: 0,
                estimated_pnl_eth: 0.0,
                failure_reason: Some(format!(
                    "Insufficient ETH for gas: {} ETH available, {} ETH needed",
                    portfolio.eth_balance, gas_cost
                )),
                virtual_tx_hash: H256::random(),
            };
        }
        
        // Emergency sell: assume we save minimal value (10-30% depending on timing)
        let save_percentage = if execution_time < 1000 { 0.3 } else if execution_time < 5000 { 0.2 } else { 0.1 };
        let recovered_eth = amount_eth * save_percentage;
        let loss = amount_eth - recovered_eth;
        
        // Update virtual portfolio
        portfolio.token_positions.insert(token_address, current_position - amount_eth);
        portfolio.eth_balance += recovered_eth - gas_cost;
        portfolio.total_trades += 1;
        portfolio.total_pnl -= loss;
        portfolio.gas_spent += gas_cost;
        
        info!(
            "🎭 VIRTUAL TRADE: Emergency sell {} ETH of {:?}, recovered {} ETH, loss: {} ETH",
            amount_eth, token_address, recovered_eth, loss
        );
        
        SimulationResult {
            would_succeed: true,
            estimated_gas_cost_eth: gas_cost,
            estimated_execution_time_ms: execution_time,
            estimated_pnl_eth: -loss,
            failure_reason: None,
            virtual_tx_hash: H256::random(),
        }
    }
    
    fn simulate_full_partial_sell(
        &mut self,
        alert: &Alert,
        token_address: Address,
        amount_eth: f64,
        percentage: f64,
    ) -> SimulationResult {
        // Calculate values first to avoid borrow conflicts
        let gas_cost = self.estimate_gas_cost(amount_eth);
        let execution_time = self.estimate_execution_time(&alert);
        
        let portfolio = self.virtual_portfolio.as_mut().unwrap();
        
        let current_position = *portfolio.token_positions.get(&token_address).unwrap_or(&0.0);
        
        if current_position < amount_eth {
            return SimulationResult {
                would_succeed: false,
                estimated_gas_cost_eth: 0.0,
                estimated_execution_time_ms: 0,
                estimated_pnl_eth: 0.0,
                failure_reason: Some(format!(
                    "Insufficient position: {} ETH available, {} ETH requested",
                    current_position, amount_eth
                )),
                virtual_tx_hash: H256::random(),
            };
        }
        
        // Partial sell: better recovery rate than emergency
        let save_percentage = 0.6 + (percentage / 100.0) * 0.3; // 60-90% recovery
        let recovered_eth = amount_eth * save_percentage;
        let loss = amount_eth - recovered_eth;
        
        // Update virtual portfolio
        portfolio.token_positions.insert(token_address, current_position - amount_eth);
        portfolio.eth_balance += recovered_eth - gas_cost;
        portfolio.total_trades += 1;
        portfolio.total_pnl -= loss;
        portfolio.gas_spent += gas_cost;
        
        info!(
            "🎭 VIRTUAL TRADE: Partial sell {} ETH ({}%) of {:?}, recovered {} ETH, loss: {} ETH",
            amount_eth, percentage, token_address, recovered_eth, loss
        );
        
        SimulationResult {
            would_succeed: true,
            estimated_gas_cost_eth: gas_cost,
            estimated_execution_time_ms: execution_time,
            estimated_pnl_eth: -loss,
            failure_reason: None,
            virtual_tx_hash: H256::random(),
        }
    }
    
    fn estimate_gas_cost(&self, amount_eth: f64) -> f64 {
        // Base gas for swap + approval if needed
        let base_gas = 150_000u64; // Gas units
        let gas_price_gwei = 30.0; // Estimate
        let gas_cost_eth = (base_gas as f64 * gas_price_gwei * 1e-9) / 1e9; // Convert to ETH
        
        // Scale slightly with amount (larger amounts might need more gas)
        gas_cost_eth * (1.0 + amount_eth / 100.0).min(2.0)
    }
    
    fn estimate_execution_time(&self, alert: &Alert) -> u64 {
        // Estimate based on priority
        let base_time_ms = match alert.params.priority {
            crate::alert_processor::Priority::Critical => 100,
            crate::alert_processor::Priority::High => 200,
            crate::alert_processor::Priority::Normal => 500,
        };
        
        base_time_ms
    }
}

/// Virtual portfolio statistics
#[derive(Debug, Clone)]
pub struct VirtualPortfolioStats {
    pub eth_balance: f64,
    pub token_positions: HashMap<Address, f64>,
    pub total_trades: u32,
    pub total_pnl: f64,
    pub gas_spent: f64,
    pub roi_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert_processor::{Action, ExecutionParams, Priority};
    
    fn create_test_alert() -> Alert {
        Alert {
            id: "test_sim".to_string(),
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
    fn test_log_only_simulation() {
        let mut engine = SimulationEngine::new(SimulationMode::LogOnly);
        let alert = create_test_alert();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        
        assert!(engine.is_simulation_active());
        
        let result = engine.simulate_emergency_sell(&alert, token, 1.0);
        assert!(result.would_succeed);
        assert!(result.estimated_gas_cost_eth > 0.0);
        assert!(result.estimated_execution_time_ms > 0);
    }
    
    #[test]
    fn test_full_simulation_success() {
        let mut initial_positions = HashMap::new();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        initial_positions.insert(token, 5.0); // 5 ETH position
        
        let mode = SimulationMode::Full {
            initial_eth_balance: 10.0,
            initial_token_positions: initial_positions,
        };
        
        let mut engine = SimulationEngine::new(mode);
        let alert = create_test_alert();
        
        let result = engine.simulate_emergency_sell(&alert, token, 2.0);
        assert!(result.would_succeed);
        assert!(result.estimated_pnl_eth < 0.0); // Should show loss
        
        // Check portfolio was updated
        let portfolio = engine.get_virtual_portfolio().unwrap();
        assert_eq!(portfolio.token_positions[&token], 3.0); // 5 - 2 = 3
        assert_eq!(portfolio.total_trades, 1);
    }
    
    #[test]
    fn test_full_simulation_insufficient_position() {
        let mut initial_positions = HashMap::new();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        initial_positions.insert(token, 1.0); // Only 1 ETH position
        
        let mode = SimulationMode::Full {
            initial_eth_balance: 10.0,
            initial_token_positions: initial_positions,
        };
        
        let mut engine = SimulationEngine::new(mode);
        let alert = create_test_alert();
        
        let result = engine.simulate_emergency_sell(&alert, token, 2.0); // Try to sell 2 ETH
        assert!(!result.would_succeed);
        assert!(result.failure_reason.is_some());
    }
    
    #[test]
    fn test_partial_sell_simulation() {
        let mut initial_positions = HashMap::new();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        initial_positions.insert(token, 5.0);
        
        let mode = SimulationMode::Full {
            initial_eth_balance: 10.0,
            initial_token_positions: initial_positions,
        };
        
        let mut engine = SimulationEngine::new(mode);
        let alert = create_test_alert();
        
        let result = engine.simulate_partial_sell(&alert, token, 2.0, 50.0);
        assert!(result.would_succeed);
        
        // Partial sell should have better PnL than emergency
        let emergency_result = engine.simulate_emergency_sell(&alert, token, 2.0);
        assert!(result.estimated_pnl_eth > emergency_result.estimated_pnl_eth);
    }
    
    #[test]
    fn test_portfolio_reset() {
        let mut initial_positions = HashMap::new();
        let token = "0x0000000000000000000000000000000000000001".parse().unwrap();
        initial_positions.insert(token, 5.0);
        
        let mode = SimulationMode::Full {
            initial_eth_balance: 10.0,
            initial_token_positions: initial_positions.clone(),
        };
        
        let mut engine = SimulationEngine::new(mode);
        let alert = create_test_alert();
        
        // Make a trade
        engine.simulate_emergency_sell(&alert, token, 2.0);
        
        // Reset portfolio
        engine.reset_virtual_portfolio();
        
        let portfolio = engine.get_virtual_portfolio().unwrap();
        assert_eq!(portfolio.eth_balance, 10.0);
        assert_eq!(portfolio.token_positions[&token], 5.0);
        assert_eq!(portfolio.total_trades, 0);
    }
}