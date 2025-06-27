//! Gas price optimization for optimal mempool positioning
//!
//! Calculates minimum gas price needed to achieve target position
//! with safety margins and cost optimization.

use ethers::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use crate::alert_processor::Priority;
use super::{MempoolTracker, LiveDataIntegrator};

/// Gas optimization engine
pub struct GasOptimizer {
    mempool_tracker: Arc<MempoolTracker>,
    data_integrator: Arc<LiveDataIntegrator>,
    config: OptimizerConfig,
    state: Arc<RwLock<OptimizerState>>,
}

/// Gas optimization configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Default safety margin percentage (e.g., 0.1 for 10%)
    pub default_safety_margin: f64,
    /// Maximum safety margin to prevent excessive overpaying
    pub max_safety_margin: f64,
    /// Minimum gas price to ever recommend
    pub min_gas_price: U256,
    /// Maximum gas price for critical transactions
    pub max_critical_gas_price: U256,
    /// Time window for historical analysis
    pub analysis_window: Duration,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            default_safety_margin: 0.1, // 10%
            max_safety_margin: 0.5,     // 50%
            min_gas_price: U256::from(1_000_000_000u64), // 1 gwei
            max_critical_gas_price: U256::from(500_000_000_000u64), // 500 gwei
            analysis_window: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Internal optimizer state
#[derive(Debug, Default)]
struct OptimizerState {
    /// Recent optimization results for learning
    recent_results: std::collections::VecDeque<OptimizationResult>,
    /// Success rate tracking
    success_rate: f64,
    /// Average accuracy of position predictions
    position_accuracy: f64,
}

/// Target position specification
#[derive(Debug, Clone)]
pub struct TargetPosition {
    /// Target percentile (e.g., 95.0 for top 5%)
    pub percentile: f64,
    /// Number of positions as safety buffer
    pub safety_margin: u64,
    /// Maximum cost willing to pay (None = no limit)
    pub max_cost: Option<U256>,
}

/// Gas optimization recommendation
#[derive(Debug, Clone)]
pub struct GasRecommendation {
    /// Recommended gas price
    pub gas_price: U256,
    /// Expected position in queue
    pub expected_position: u64,
    /// Confidence in recommendation (0.0 to 1.0)
    pub confidence: f64,
    /// Safety margin applied
    pub safety_margin: f64,
    /// Total estimated cost (gas_price * estimated_gas_used)
    pub total_cost: U256,
    /// Optimization strategy used
    pub strategy: OptimizationStrategy,
    /// Alternative recommendations
    pub alternatives: Vec<AlternativeRecommendation>,
}

/// Optimization strategy
#[derive(Debug, Clone)]
pub enum OptimizationStrategy {
    /// Aggressive: guarantee top position regardless of cost
    Aggressive,
    /// Targeted: beat specific position with minimal margin
    Targeted { target_position: u64 },
    /// Economic: achieve percentile at lowest cost
    Economic { target_percentile: f64 },
    /// Adaptive: use historical data to optimize
    Adaptive,
}

/// Alternative gas price recommendation
#[derive(Debug, Clone)]
pub struct AlternativeRecommendation {
    pub gas_price: U256,
    pub expected_position: u64,
    pub cost_savings: U256,
    pub success_probability: f64,
}

/// Result of optimization for learning
#[derive(Debug, Clone)]
struct OptimizationResult {
    recommended_gas: U256,
    actual_position: Option<u64>,
    target_position: u64,
    success: bool,
    timestamp: std::time::Instant,
}

impl GasOptimizer {
    /// Create new gas optimizer
    pub fn new(
        mempool_tracker: Arc<MempoolTracker>,
        data_integrator: Arc<LiveDataIntegrator>,
    ) -> Self {
        Self {
            mempool_tracker,
            data_integrator,
            config: OptimizerConfig::default(),
            state: Arc::new(RwLock::new(OptimizerState::default())),
        }
    }
    
    /// Optimize gas price for target position
    pub async fn optimize_for_position(
        &self,
        target: TargetPosition,
        priority: Priority,
    ) -> Result<GasRecommendation, Box<dyn std::error::Error>> {
        info!("Optimizing gas for target percentile: {}%", target.percentile);
        
        // Get current mempool state
        let mempool_stats = self.mempool_tracker.get_stats().await;
        
        // Determine optimization strategy based on priority
        let strategy = self.select_strategy(priority, &target);
        
        // Calculate base gas price needed
        let base_gas_price = self.calculate_base_gas_price(&target, &mempool_stats).await?;
        
        // Apply safety margin
        let safety_margin = self.calculate_safety_margin(&strategy, &mempool_stats);
        let recommended_gas = self.apply_safety_margin(base_gas_price, safety_margin);
        
        // Validate against constraints
        let final_gas_price = self.validate_constraints(recommended_gas, &target)?;
        
        // Calculate expected position
        let expected_position = self.mempool_tracker
            .get_position_for_gas_price(final_gas_price)
            .await;
        
        // Estimate total cost (assuming 200k gas for Uniswap swap)
        let estimated_gas_used = U256::from(200_000);
        let total_cost = final_gas_price * estimated_gas_used;
        
        // Calculate confidence based on mempool volatility
        let confidence = self.calculate_confidence(&mempool_stats, &strategy);
        
        // Generate alternatives
        let alternatives = self.generate_alternatives(base_gas_price, &target).await;
        
        let recommendation = GasRecommendation {
            gas_price: final_gas_price,
            expected_position,
            confidence,
            safety_margin,
            total_cost,
            strategy,
            alternatives,
        };
        
        // Store result for learning
        self.record_recommendation(&recommendation, &target).await;
        
        Ok(recommendation)
    }
    
    /// Select optimization strategy based on priority
    fn select_strategy(&self, priority: Priority, target: &TargetPosition) -> OptimizationStrategy {
        match priority {
            Priority::Critical => OptimizationStrategy::Aggressive,
            Priority::High => OptimizationStrategy::Targeted {
                target_position: (target.percentile * 100.0) as u64,
            },
            Priority::Normal => OptimizationStrategy::Economic {
                target_percentile: target.percentile,
            },
        }
    }
    
    /// Calculate base gas price for target position
    async fn calculate_base_gas_price(
        &self,
        target: &TargetPosition,
        mempool_stats: &super::MempoolStats,
    ) -> Result<U256, Box<dyn std::error::Error>> {
        // Convert percentile to actual position
        let total_pending = mempool_stats.pending_tx_count;
        let target_position = ((100.0 - target.percentile) / 100.0 * total_pending as f64) as u64;
        let target_with_margin = target_position.saturating_sub(target.safety_margin);
        
        debug!("Target position: {} (with margin: {})", target_position, target_with_margin);
        
        // Get gas price needed for this position
        if let Some(gas_price) = self.mempool_tracker
            .get_gas_price_for_position(target_with_margin)
            .await
        {
            Ok(gas_price)
        } else {
            // Fallback to percentile-based calculation
            let base_gas = match target.percentile {
                p if p >= 99.0 => mempool_stats.gas_price_percentiles.p99,
                p if p >= 95.0 => mempool_stats.gas_price_percentiles.p95,
                p if p >= 90.0 => mempool_stats.gas_price_percentiles.p90,
                p if p >= 75.0 => mempool_stats.gas_price_percentiles.p75,
                _ => mempool_stats.gas_price_percentiles.p50,
            };
            
            Ok(base_gas)
        }
    }
    
    /// Calculate safety margin based on strategy and market conditions
    fn calculate_safety_margin(
        &self,
        strategy: &OptimizationStrategy,
        mempool_stats: &super::MempoolStats,
    ) -> f64 {
        let base_margin = match strategy {
            OptimizationStrategy::Aggressive => 0.25, // 25% margin for critical
            OptimizationStrategy::Targeted { .. } => 0.15, // 15% margin for targeted
            OptimizationStrategy::Economic { .. } => 0.05, // 5% margin for economic
            OptimizationStrategy::Adaptive => self.config.default_safety_margin,
        };
        
        // Adjust based on congestion level
        let congestion_multiplier = match mempool_stats.congestion_level {
            super::CongestionLevel::Low => 1.0,
            super::CongestionLevel::Medium => 1.2,
            super::CongestionLevel::High => 1.5,
            super::CongestionLevel::Extreme => 2.0,
        };
        
        (base_margin * congestion_multiplier).min(self.config.max_safety_margin)
    }
    
    /// Apply safety margin to base gas price
    fn apply_safety_margin(&self, base_gas: U256, safety_margin: f64) -> U256 {
        let margin_factor = 1.0 + safety_margin;
        let base_f64 = base_gas.as_u128() as f64;
        let adjusted_f64 = base_f64 * margin_factor;
        U256::from(adjusted_f64 as u128)
    }
    
    /// Validate gas price against constraints
    fn validate_constraints(
        &self,
        gas_price: U256,
        target: &TargetPosition,
    ) -> Result<U256, Box<dyn std::error::Error>> {
        // Check minimum
        let validated = gas_price.max(self.config.min_gas_price);
        
        // Check maximum constraint from target
        let final_gas = if let Some(max_cost) = target.max_cost {
            validated.min(max_cost)
        } else {
            validated.min(self.config.max_critical_gas_price)
        };
        
        if final_gas < self.config.min_gas_price {
            return Err("Gas price below minimum threshold".into());
        }
        
        Ok(final_gas)
    }
    
    /// Calculate confidence in recommendation
    fn calculate_confidence(
        &self,
        mempool_stats: &super::MempoolStats,
        strategy: &OptimizationStrategy,
    ) -> f64 {
        let base_confidence = 0.8; // Start with 80%
        
        // Adjust based on mempool conditions
        let congestion_factor = match mempool_stats.congestion_level {
            super::CongestionLevel::Low => 1.0,
            super::CongestionLevel::Medium => 0.9,
            super::CongestionLevel::High => 0.8,
            super::CongestionLevel::Extreme => 0.7,
        };
        
        // Adjust based on strategy aggressiveness
        let strategy_factor = match strategy {
            OptimizationStrategy::Aggressive => 0.95,
            OptimizationStrategy::Targeted { .. } => 0.85,
            OptimizationStrategy::Economic { .. } => 0.75,
            OptimizationStrategy::Adaptive => 0.8,
        };
        
        let result: f64 = base_confidence * congestion_factor * strategy_factor;
        result.max(0.1).min(1.0)
    }
    
    /// Generate alternative recommendations
    async fn generate_alternatives(
        &self,
        base_gas: U256,
        _target: &TargetPosition,
    ) -> Vec<AlternativeRecommendation> {
        let mut alternatives = Vec::new();
        
        // Conservative option (lower cost, lower success probability)
        let conservative_gas = base_gas * 90 / 100; // 10% less
        let conservative_position = self.mempool_tracker
            .get_position_for_gas_price(conservative_gas)
            .await;
        
        alternatives.push(AlternativeRecommendation {
            gas_price: conservative_gas,
            expected_position: conservative_position,
            cost_savings: base_gas - conservative_gas,
            success_probability: 0.7,
        });
        
        // Aggressive option (higher cost, higher success probability)
        let aggressive_gas = base_gas * 130 / 100; // 30% more
        let aggressive_position = self.mempool_tracker
            .get_position_for_gas_price(aggressive_gas)
            .await;
        
        alternatives.push(AlternativeRecommendation {
            gas_price: aggressive_gas,
            expected_position: aggressive_position,
            cost_savings: U256::zero(), // More expensive
            success_probability: 0.95,
        });
        
        alternatives
    }
    
    /// Record recommendation for future learning
    async fn record_recommendation(&self, recommendation: &GasRecommendation, target: &TargetPosition) {
        let mut state = self.state.write().await;
        
        let result = OptimizationResult {
            recommended_gas: recommendation.gas_price,
            actual_position: None, // Will be filled in later
            target_position: ((100.0 - target.percentile) / 100.0 * 1000.0) as u64, // Rough estimate
            success: false, // Will be updated later
            timestamp: std::time::Instant::now(),
        };
        
        state.recent_results.push_back(result);
        
        // Keep only recent results
        while state.recent_results.len() > 100 {
            state.recent_results.pop_front();
        }
    }
    
    /// Update recommendation result with actual outcome
    pub async fn record_outcome(&self, gas_price: U256, actual_position: u64, success: bool) {
        let mut state = self.state.write().await;
        
        // Find matching recommendation and update it
        if let Some(result) = state.recent_results.iter_mut()
            .rev()
            .find(|r| r.recommended_gas == gas_price && r.actual_position.is_none())
        {
            result.actual_position = Some(actual_position);
            result.success = success;
            
            // Update success rate
            let recent_successes = state.recent_results.iter()
                .filter(|r| r.actual_position.is_some())
                .filter(|r| r.success)
                .count();
            
            let total_results = state.recent_results.iter()
                .filter(|r| r.actual_position.is_some())
                .count();
            
            if total_results > 0 {
                state.success_rate = recent_successes as f64 / total_results as f64;
            }
        }
    }
    
    /// Get current optimizer statistics
    pub async fn get_stats(&self) -> OptimizerStats {
        let state = self.state.read().await;
        
        OptimizerStats {
            success_rate: state.success_rate,
            position_accuracy: state.position_accuracy,
            total_optimizations: state.recent_results.len(),
        }
    }
}

/// Optimizer performance statistics
#[derive(Debug, Clone)]
pub struct OptimizerStats {
    pub success_rate: f64,
    pub position_accuracy: f64,
    pub total_optimizations: usize,
}