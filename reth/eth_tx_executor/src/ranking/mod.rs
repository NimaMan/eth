//! Transaction Ranking System
//! 
//! Provides optimal gas pricing and transaction positioning to ensure
//! protective transactions execute before detected scam transactions.

pub mod mempool_tracker;
pub mod gas_optimizer;
pub mod position_calculator;
pub mod data_sources;
pub mod mempool_gas_client;

pub use mempool_tracker::{MempoolTracker, MempoolState, PendingTransaction};
pub use gas_optimizer::{GasOptimizer, GasRecommendation, OptimizationStrategy};
pub use position_calculator::{PositionCalculator, PositionResult};
pub use data_sources::{BlockProcessorClient, LiveDataIntegrator};

use ethers::prelude::*;
use std::sync::Arc;
use crate::alert_processor::{Alert, Priority};

/// Main ranking system that coordinates all components
pub struct TransactionRankingSystem {
    mempool_tracker: Arc<MempoolTracker>,
    gas_optimizer: Arc<GasOptimizer>,
    position_calculator: Arc<PositionCalculator>,
    data_integrator: Arc<LiveDataIntegrator>,
}

/// Comprehensive ranking result with gas and execution strategy
#[derive(Debug, Clone)]
pub struct RankingResult {
    /// Optimal gas price to achieve target position
    pub optimal_gas_price: U256,
    /// Expected position in mempool queue
    pub expected_position: u64,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Recommended execution path
    pub execution_path: ExecutionPath,
    /// Total estimated cost including potential bribes
    pub total_cost: U256,
    /// Safety margin built into gas price
    pub safety_margin: f64,
}

/// Execution path options for transaction submission
#[derive(Debug, Clone)]
pub enum ExecutionPath {
    /// Submit to public mempool only
    PublicMempool,
    /// Submit via Flashbots bundle
    FlashbotsBundle { max_block_number: u64 },
    /// Multi-path: try public first, fallback to Flashbots
    MultiPath { timeout_ms: u64 },
}

impl TransactionRankingSystem {
    /// Create new ranking system with Reth WebSocket connection
    pub async fn new(reth_ws_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mempool_tracker = Arc::new(MempoolTracker::new(&reth_ws_url).await?);
        let data_integrator = Arc::new(LiveDataIntegrator::new().await?);
        
        let gas_optimizer = Arc::new(GasOptimizer::new(
            mempool_tracker.clone(),
            data_integrator.clone(),
        ));
        
        let position_calculator = Arc::new(PositionCalculator::new(
            mempool_tracker.clone(),
        ));
        
        Ok(Self {
            mempool_tracker,
            gas_optimizer,
            position_calculator,
            data_integrator,
        })
    }
    
    /// Start background services (mempool monitoring, data collection)
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Start mempool tracking
        let tracker = self.mempool_tracker.clone();
        tokio::spawn(async move {
            if let Err(e) = tracker.start_monitoring().await {
                tracing::error!("Mempool tracking failed: {}", e);
            }
        });
        
        // Start data integration
        let integrator = self.data_integrator.clone();
        tokio::spawn(async move {
            if let Err(e) = integrator.start_data_collection().await {
                tracing::error!("Data integration failed: {}", e);
            }
        });
        
        Ok(())
    }
    
    /// Calculate optimal ranking for alert execution
    pub async fn calculate_ranking(&self, alert: &Alert) -> Result<RankingResult, Box<dyn std::error::Error>> {
        tracing::debug!("Calculating ranking for alert: {}", alert.id);
        
        // Determine target position based on alert priority
        let target_position = self.determine_target_position(alert).await?;
        
        // Calculate optimal gas price
        let gas_recommendation = self.gas_optimizer
            .optimize_for_position(target_position, alert.params.priority)
            .await?;
        
        // Calculate expected position with recommended gas
        let position_result = self.position_calculator
            .calculate_position(gas_recommendation.gas_price)
            .await?;
        
        // Determine execution path
        let execution_path = self.select_execution_path(alert, &gas_recommendation).await;
        
        Ok(RankingResult {
            optimal_gas_price: gas_recommendation.gas_price,
            expected_position: position_result.estimated_position,
            confidence: gas_recommendation.confidence * position_result.confidence,
            execution_path,
            total_cost: gas_recommendation.total_cost,
            safety_margin: gas_recommendation.safety_margin,
        })
    }
    
    /// Get current mempool statistics
    pub async fn get_mempool_stats(&self) -> MempoolStats {
        self.mempool_tracker.get_stats().await
    }
    
    /// Determine target position based on alert characteristics
    async fn determine_target_position(&self, alert: &Alert) -> Result<gas_optimizer::TargetPosition, Box<dyn std::error::Error>> {
        match alert.params.priority {
            Priority::Critical => {
                // Critical alerts need top 1% position
                Ok(gas_optimizer::TargetPosition {
                    percentile: 99.0,
                    safety_margin: 20,
                    max_cost: None, // No cost limit for critical
                })
            }
            Priority::High => {
                // High priority needs top 5% position
                Ok(gas_optimizer::TargetPosition {
                    percentile: 95.0,
                    safety_margin: 10,
                    max_cost: alert.params.max_gas_price,
                })
            }
            Priority::Normal => {
                // Normal priority needs top 25% position
                Ok(gas_optimizer::TargetPosition {
                    percentile: 75.0,
                    safety_margin: 5,
                    max_cost: alert.params.max_gas_price,
                })
            }
        }
    }
    
    /// Select execution path based on alert and gas recommendation
    async fn select_execution_path(&self, alert: &Alert, _gas_rec: &GasRecommendation) -> ExecutionPath {
        match alert.params.priority {
            Priority::Critical => {
                // Critical alerts always use Flashbots for guaranteed inclusion
                ExecutionPath::FlashbotsBundle {
                    max_block_number: self.get_current_block_number().await + 3,
                }
            }
            Priority::High => {
                // High priority uses multi-path for speed + reliability
                ExecutionPath::MultiPath { timeout_ms: 500 }
            }
            Priority::Normal => {
                // Normal priority uses public mempool
                ExecutionPath::PublicMempool
            }
        }
    }
    
    /// Get current block number from tracker
    async fn get_current_block_number(&self) -> u64 {
        self.mempool_tracker.get_current_block_number().await
    }
}

/// Mempool statistics for monitoring
#[derive(Debug, Clone)]
pub struct MempoolStats {
    pub pending_tx_count: u64,
    pub gas_price_percentiles: GasPercentiles,
    pub avg_arrival_rate: f64, // txs per second
    pub congestion_level: CongestionLevel,
    pub mev_tx_count: u64,
}

/// Gas price percentiles in the mempool
#[derive(Debug, Clone)]
pub struct GasPercentiles {
    pub p50: U256,
    pub p75: U256,
    pub p90: U256,
    pub p95: U256,
    pub p99: U256,
}

/// Network congestion level
#[derive(Debug, Clone)]
pub enum CongestionLevel {
    Low,    // < 1000 pending txs
    Medium, // 1000-5000 pending txs
    High,   // 5000-15000 pending txs
    Extreme, // > 15000 pending txs
}