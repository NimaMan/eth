//! Position calculation and prediction in mempool queue
//!
//! Calculates expected transaction position based on gas price
//! and provides confidence estimates.

use ethers::prelude::*;
use std::sync::Arc;
use tracing::debug;
use super::MempoolTracker;

/// Calculates transaction positions in mempool queue
pub struct PositionCalculator {
    mempool_tracker: Arc<MempoolTracker>,
}

/// Result of position calculation
#[derive(Debug, Clone)]
pub struct PositionResult {
    /// Estimated position in queue (1 = first)
    pub estimated_position: u64,
    /// Confidence in estimate (0.0 to 1.0)
    pub confidence: f64,
    /// Number of transactions with higher gas price
    pub transactions_ahead: u64,
    /// Total pending transactions
    pub total_pending: u64,
    /// Percentile rank (0.0 to 100.0)
    pub percentile_rank: f64,
}

impl PositionCalculator {
    /// Create new position calculator
    pub fn new(mempool_tracker: Arc<MempoolTracker>) -> Self {
        Self { mempool_tracker }
    }
    
    /// Calculate expected position for given gas price
    pub async fn calculate_position(&self, gas_price: U256) -> Result<PositionResult, Box<dyn std::error::Error>> {
        debug!("Calculating position for gas price: {}", gas_price);
        
        // Get current mempool stats
        let mempool_stats = self.mempool_tracker.get_stats().await;
        let total_pending = mempool_stats.pending_tx_count;
        
        // Calculate position based on gas price ranking
        let transactions_ahead = self.mempool_tracker
            .get_position_for_gas_price(gas_price)
            .await;
        
        let estimated_position = transactions_ahead + 1;
        
        // Calculate percentile rank
        let percentile_rank = if total_pending > 0 {
            ((total_pending - transactions_ahead) as f64 / total_pending as f64) * 100.0
        } else {
            100.0 // If no pending transactions, we'd be first
        };
        
        // Calculate confidence based on mempool stability
        let confidence = self.calculate_confidence(&mempool_stats);
        
        Ok(PositionResult {
            estimated_position,
            confidence,
            transactions_ahead,
            total_pending,
            percentile_rank,
        })
    }
    
    /// Calculate confidence in position estimate
    fn calculate_confidence(&self, mempool_stats: &super::MempoolStats) -> f64 {
        let base_confidence = 0.8;
        
        // Reduce confidence during high congestion (more volatility)
        let congestion_factor = match mempool_stats.congestion_level {
            super::CongestionLevel::Low => 1.0,
            super::CongestionLevel::Medium => 0.9,
            super::CongestionLevel::High => 0.8,
            super::CongestionLevel::Extreme => 0.6,
        };
        
        // Reduce confidence if arrival rate is very high (rapid changes)
        let arrival_factor = if mempool_stats.avg_arrival_rate > 100.0 {
            0.8 // High volatility
        } else if mempool_stats.avg_arrival_rate > 50.0 {
            0.9
        } else {
            1.0 // Stable
        };
        
        // Reduce confidence if many MEV transactions (unpredictable behavior)
        let mev_factor = if mempool_stats.mev_tx_count > mempool_stats.pending_tx_count / 10 {
            0.85 // > 10% MEV transactions
        } else {
            1.0
        };
        
        let result: f64 = base_confidence * congestion_factor * arrival_factor * mev_factor;
        result.max(0.1).min(1.0)
    }
    
    /// Find gas price needed to achieve target position
    pub async fn find_gas_for_position(&self, target_position: u64) -> Option<U256> {
        self.mempool_tracker.get_gas_price_for_position(target_position).await
    }
    
    /// Find gas price needed to achieve target percentile
    pub async fn find_gas_for_percentile(&self, target_percentile: f64) -> Option<U256> {
        let mempool_stats = self.mempool_tracker.get_stats().await;
        let total_pending = mempool_stats.pending_tx_count;
        
        if total_pending == 0 {
            return None;
        }
        
        // Convert percentile to position
        let target_position = ((100.0 - target_percentile) / 100.0 * total_pending as f64) as u64;
        
        self.find_gas_for_position(target_position).await
    }
    
    /// Estimate inclusion probability for given gas price
    pub async fn estimate_inclusion_probability(&self, gas_price: U256, blocks_ahead: u64) -> f64 {
        let position_result = self.calculate_position(gas_price).await
            .unwrap_or_else(|_| PositionResult {
                estimated_position: u64::MAX,
                confidence: 0.0,
                transactions_ahead: u64::MAX,
                total_pending: 0,
                percentile_rank: 0.0,
            });
        
        // Estimate based on position and number of blocks
        let transactions_per_block = 200.0; // Approximate for Ethereum
        let total_slots_available = transactions_per_block * blocks_ahead as f64;
        
        if position_result.estimated_position as f64 <= total_slots_available {
            // Very likely to be included
            0.95 * position_result.confidence
        } else {
            // Calculate probability based on how far beyond available slots
            let overflow_factor = position_result.estimated_position as f64 / total_slots_available;
            let base_probability = (1.0 / overflow_factor).min(0.8);
            base_probability * position_result.confidence
        }
    }
    
    /// Get detailed position analysis
    pub async fn get_position_analysis(&self, gas_price: U256) -> PositionAnalysis {
        let position_result = self.calculate_position(gas_price).await
            .unwrap_or_else(|_| PositionResult {
                estimated_position: u64::MAX,
                confidence: 0.0,
                transactions_ahead: u64::MAX,
                total_pending: 0,
                percentile_rank: 0.0,
            });
        
        let mempool_stats = self.mempool_tracker.get_stats().await;
        
        // Calculate time estimates
        let avg_block_time = 12.0; // seconds
        let transactions_per_block = 200.0;
        let estimated_blocks_to_inclusion = (position_result.estimated_position as f64 / transactions_per_block).ceil();
        let estimated_time_to_inclusion = estimated_blocks_to_inclusion * avg_block_time;
        
        // Compare with percentiles
        let gas_percentile_rank = if gas_price >= mempool_stats.gas_price_percentiles.p99 {
            99.0
        } else if gas_price >= mempool_stats.gas_price_percentiles.p95 {
            95.0
        } else if gas_price >= mempool_stats.gas_price_percentiles.p90 {
            90.0
        } else if gas_price >= mempool_stats.gas_price_percentiles.p75 {
            75.0
        } else if gas_price >= mempool_stats.gas_price_percentiles.p50 {
            50.0
        } else {
            25.0
        };
        
        PositionAnalysis {
            risk_level: self.assess_risk_level(&position_result, &mempool_stats),
            position_result,
            estimated_blocks_to_inclusion: estimated_blocks_to_inclusion as u64,
            estimated_time_to_inclusion_seconds: estimated_time_to_inclusion as u64,
            gas_percentile_rank,
            is_competitive: gas_percentile_rank >= 90.0,
        }
    }
    
    /// Assess risk level of position
    fn assess_risk_level(&self, position: &PositionResult, stats: &super::MempoolStats) -> RiskLevel {
        // High risk if position is low and mempool is congested
        if position.percentile_rank < 50.0 {
            match stats.congestion_level {
                super::CongestionLevel::Extreme => RiskLevel::High,
                super::CongestionLevel::High => RiskLevel::Medium,
                _ => RiskLevel::Low,
            }
        } else if position.percentile_rank < 80.0 {
            match stats.congestion_level {
                super::CongestionLevel::Extreme => RiskLevel::Medium,
                _ => RiskLevel::Low,
            }
        } else {
            RiskLevel::Low
        }
    }
}

/// Detailed position analysis
#[derive(Debug, Clone)]
pub struct PositionAnalysis {
    pub position_result: PositionResult,
    pub estimated_blocks_to_inclusion: u64,
    pub estimated_time_to_inclusion_seconds: u64,
    pub gas_percentile_rank: f64,
    pub is_competitive: bool,
    pub risk_level: RiskLevel,
}

/// Risk level for transaction inclusion
#[derive(Debug, Clone)]
pub enum RiskLevel {
    Low,    // Very likely to be included quickly
    Medium, // May take several blocks
    High,   // High risk of delayed inclusion or failure
}