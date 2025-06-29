//! Data source integrations for enhanced gas optimization
//!
//! Integrates with live block processor and other data sources
//! to provide historical context and improve predictions.

use ethers::prelude::*;
use reqwest::Client;
use serde::Deserialize;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Integrates live data sources for gas optimization
pub struct LiveDataIntegrator {
    block_processor_client: BlockProcessorClient,
    historical_data: Arc<RwLock<HistoricalGasData>>,
    config: DataIntegratorConfig,
}

/// Configuration for data integration
#[derive(Debug, Clone)]
pub struct DataIntegratorConfig {
    /// Block processor HTTP API URL
    pub block_processor_url: String,
    /// How much historical data to keep
    pub history_window: Duration,
    /// Update interval for historical analysis
    pub update_interval: Duration,
    /// Maximum number of blocks to analyze
    pub max_blocks: usize,
}

impl Default for DataIntegratorConfig {
    fn default() -> Self {
        Self {
            block_processor_url: "http://127.0.0.1:18000".to_string(),
            history_window: Duration::from_secs(3600), // 1 hour
            update_interval: Duration::from_secs(60),   // 1 minute
            max_blocks: 100,
        }
    }
}

/// Historical gas price data
#[derive(Debug, Default)]
struct HistoricalGasData {
    /// Recent blocks with gas analysis
    blocks: VecDeque<BlockGasAnalysis>,
    /// Gas price trends
    trends: GasPriceTrends,
    /// MEV activity patterns
    mev_patterns: MevPatterns,
    /// Last update timestamp
    last_update: Option<Instant>,
}

/// Gas analysis for a single block
#[derive(Debug, Clone)]
struct BlockGasAnalysis {
    pub block_number: u64,
    pub timestamp: u64,
    pub base_fee: U256,
    pub gas_percentiles: GasPercentiles,
    pub total_transactions: u64,
    pub mev_transactions: u64,
    pub avg_gas_price: U256,
    pub max_gas_price: U256,
    pub min_gas_price: U256,
}

/// Gas price trends over time
#[derive(Debug, Default, Clone)]
pub struct GasPriceTrends {
    /// Average gas price change per minute
    pub price_velocity: f64,
    /// Volatility measure (standard deviation)
    pub volatility: f64,
    /// Recent price direction (positive = increasing)
    pub direction: f64,
}

/// MEV activity patterns
#[derive(Debug, Default, Clone)]
pub struct MevPatterns {
    /// Average MEV transaction gas premium
    pub avg_mev_premium: U256,
    /// MEV transaction frequency (per block)
    pub mev_frequency: f64,
    /// Common MEV gas price levels
    pub common_mev_prices: Vec<U256>,
}

/// Gas percentiles for historical analysis
#[derive(Debug, Clone, Default)]
pub struct GasPercentiles {
    pub p10: U256,
    pub p25: U256,
    pub p50: U256,
    pub p75: U256,
    pub p90: U256,
    pub p95: U256,
    pub p99: U256,
}

impl LiveDataIntegrator {
    /// Create new data integrator
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = DataIntegratorConfig::default();
        let block_processor_client = BlockProcessorClient::new(&config.block_processor_url);
        
        Ok(Self {
            block_processor_client,
            historical_data: Arc::new(RwLock::new(HistoricalGasData::default())),
            config,
        })
    }
    
    /// Start data collection background task
    pub async fn start_data_collection(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting live data collection");
        
        let mut interval = tokio::time::interval(self.config.update_interval);
        let historical_data = self.historical_data.clone();
        let client = self.block_processor_client.clone();
        let config = self.config.clone();
        
        loop {
            interval.tick().await;
            
            match Self::collect_recent_blocks(&client, &config).await {
                Ok(blocks) => {
                    let mut data = historical_data.write().await;
                    Self::update_historical_data(&mut data, blocks, &config);
                }
                Err(e) => {
                    warn!("Failed to collect block data: {}", e);
                }
            }
        }
    }
    
    /// Collect recent block data
    async fn collect_recent_blocks(
        client: &BlockProcessorClient,
        config: &DataIntegratorConfig,
    ) -> Result<Vec<BlockGasAnalysis>, Box<dyn std::error::Error + Send + Sync>> {
        let blocks = client.get_recent_blocks(config.max_blocks).await?;
        
        let mut analyses = Vec::new();
        for block in blocks {
            let analysis = Self::analyze_block_gas_data(&block);
            analyses.push(analysis);
        }
        
        Ok(analyses)
    }
    
    /// Analyze gas data from a processed block
    fn analyze_block_gas_data(block: &ProcessedBlock) -> BlockGasAnalysis {
        let mut gas_prices: Vec<U256> = block.transactions
            .iter()
            .filter_map(|tx| tx.fees.gas_price.parse().ok())
            .collect();
        
        gas_prices.sort();
        
        let percentiles = if !gas_prices.is_empty() {
            GasPercentiles {
                p10: gas_prices[gas_prices.len() * 10 / 100],
                p25: gas_prices[gas_prices.len() * 25 / 100],
                p50: gas_prices[gas_prices.len() * 50 / 100],
                p75: gas_prices[gas_prices.len() * 75 / 100],
                p90: gas_prices[gas_prices.len() * 90 / 100],
                p95: gas_prices[gas_prices.len() * 95 / 100],
                p99: gas_prices[gas_prices.len() * 99 / 100],
            }
        } else {
            GasPercentiles::default()
        };
        
        let mev_count = block.transactions
            .iter()
            .filter(|tx| tx.fees.bribe_amount.is_some())
            .count() as u64;
        
        let avg_gas_price = if !gas_prices.is_empty() {
            let total: U256 = gas_prices.iter().fold(U256::zero(), |acc, &x| acc + x);
            total / gas_prices.len()
        } else {
            U256::zero()
        };
        
        BlockGasAnalysis {
            block_number: block.block_number,
            timestamp: block.timestamp,
            base_fee: block.base_fee.unwrap_or_default(),
            gas_percentiles: percentiles,
            total_transactions: block.transactions.len() as u64,
            mev_transactions: mev_count,
            avg_gas_price,
            max_gas_price: gas_prices.last().cloned().unwrap_or_default(),
            min_gas_price: gas_prices.first().cloned().unwrap_or_default(),
        }
    }
    
    /// Update historical data with new blocks
    fn update_historical_data(
        data: &mut HistoricalGasData,
        new_blocks: Vec<BlockGasAnalysis>,
        config: &DataIntegratorConfig,
    ) {
        // Add new blocks
        for block in new_blocks {
            data.blocks.push_back(block);
        }
        
        // Remove old data beyond window
        let cutoff_time = Instant::now() - config.history_window;
        while let Some(front) = data.blocks.front() {
            let block_time = std::time::UNIX_EPOCH + Duration::from_secs(front.timestamp);
            if let Ok(block_instant) = block_time.duration_since(std::time::UNIX_EPOCH) {
                if Instant::now() - Duration::from_secs(block_instant.as_secs()) < cutoff_time {
                    break;
                }
            }
            data.blocks.pop_front();
        }
        
        // Update trends and patterns
        Self::update_gas_trends(&mut data.trends, &data.blocks);
        Self::update_mev_patterns(&mut data.mev_patterns, &data.blocks);
        
        data.last_update = Some(Instant::now());
        
        debug!("Updated historical data: {} blocks", data.blocks.len());
    }
    
    /// Update gas price trends
    fn update_gas_trends(trends: &mut GasPriceTrends, blocks: &VecDeque<BlockGasAnalysis>) {
        if blocks.len() < 2 {
            return;
        }
        
        // Calculate price velocity (change per minute)
        let recent_blocks: Vec<_> = blocks.iter().rev().take(10).collect();
        if recent_blocks.len() >= 2 {
            let latest = recent_blocks[0];
            let previous = recent_blocks[recent_blocks.len() - 1];
            
            let price_change = latest.avg_gas_price.saturating_sub(previous.avg_gas_price);
            let time_diff = latest.timestamp.saturating_sub(previous.timestamp);
            
            if time_diff > 0 {
                trends.price_velocity = price_change.as_u128() as f64 / (time_diff as f64 / 60.0);
            }
        }
        
        // Calculate volatility
        let prices: Vec<f64> = blocks.iter()
            .map(|b| b.avg_gas_price.as_u128() as f64)
            .collect();
        
        if !prices.is_empty() {
            let mean = prices.iter().sum::<f64>() / prices.len() as f64;
            let variance = prices.iter()
                .map(|p| (p - mean).powi(2))
                .sum::<f64>() / prices.len() as f64;
            trends.volatility = variance.sqrt();
        }
        
        // Determine direction
        trends.direction = if trends.price_velocity > 0.0 { 1.0 } else { -1.0 };
    }
    
    /// Update MEV patterns
    fn update_mev_patterns(patterns: &mut MevPatterns, blocks: &VecDeque<BlockGasAnalysis>) {
        if blocks.is_empty() {
            return;
        }
        
        // Calculate average MEV frequency
        let total_mev = blocks.iter().map(|b| b.mev_transactions).sum::<u64>();
        let total_blocks = blocks.len() as u64;
        patterns.mev_frequency = total_mev as f64 / total_blocks as f64;
        
        // TODO: Implement more sophisticated MEV pattern analysis
        // - Identify common MEV gas price levels
        // - Calculate average MEV premium
        // - Detect MEV competition patterns
    }
    
    /// Get historical gas trends
    pub async fn get_gas_trends(&self) -> GasPriceTrends {
        let data = self.historical_data.read().await;
        data.trends.clone()
    }
    
    /// Get MEV patterns
    pub async fn get_mev_patterns(&self) -> MevPatterns {
        let data = self.historical_data.read().await;
        data.mev_patterns.clone()
    }
    
    /// Get recommended gas premium based on historical data
    pub async fn get_recommended_premium(&self, urgency: f64) -> f64 {
        let data = self.historical_data.read().await;
        
        // Base premium based on recent trends
        let trend_premium = if data.trends.direction > 0.0 {
            0.1 // 10% extra if prices are rising
        } else {
            0.05 // 5% if prices are stable/falling
        };
        
        // Volatility premium
        let volatility_premium = (data.trends.volatility / 10_000_000_000.0).min(0.2); // Cap at 20%
        
        // Urgency multiplier
        let urgency_multiplier = 1.0 + urgency * 0.5; // Up to 50% extra for maximum urgency
        
        (trend_premium + volatility_premium) * urgency_multiplier
    }
    
    /// Predict gas price movement
    pub async fn predict_gas_movement(&self, minutes_ahead: u64) -> GasPrediction {
        let data = self.historical_data.read().await;
        
        // Simple linear prediction based on velocity
        let predicted_change = data.trends.price_velocity * minutes_ahead as f64;
        
        // Confidence decreases with time and increases with data quality
        let confidence = if data.blocks.len() >= 10 {
            (1.0 - (minutes_ahead as f64 / 60.0) * 0.1).max(0.1) // Decrease confidence over time
        } else {
            0.3 // Low confidence with insufficient data
        };
        
        GasPrediction {
            predicted_change: predicted_change as i128,
            confidence,
            direction: data.trends.direction,
            volatility: data.trends.volatility,
        }
    }
}

/// Block processor HTTP client
#[derive(Clone)]
pub struct BlockProcessorClient {
    client: Client,
    base_url: String,
}

impl BlockProcessorClient {
    /// Create new client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }
    
    /// Get recent blocks
    pub async fn get_recent_blocks(&self, count: usize) -> Result<Vec<ProcessedBlock>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/v1/blocks/recent?count={}", self.base_url, count);
        
        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }
        
        let blocks: Vec<ProcessedBlock> = response.json().await?;
        Ok(blocks)
    }
}

/// Processed block from block processor API
#[derive(Debug, Deserialize)]
pub struct ProcessedBlock {
    pub block_number: u64,
    pub timestamp: u64,
    pub base_fee: Option<U256>,
    pub transactions: Vec<ProcessedTransaction>,
}

/// Processed transaction from block processor API
#[derive(Debug, Deserialize)]
pub struct ProcessedTransaction {
    pub hash: String,
    pub fees: TransactionFees,
}

/// Transaction fees from block processor
#[derive(Debug, Deserialize)]
pub struct TransactionFees {
    pub gas_price: String, // Wei as string
    pub gas_used: u64,
    pub bribe_amount: Option<String>, // MEV bribe if detected
}

/// Gas price prediction
#[derive(Debug, Clone)]
pub struct GasPrediction {
    /// Predicted change in gas price (wei)
    pub predicted_change: i128,
    /// Confidence in prediction (0.0 to 1.0)
    pub confidence: f64,
    /// Price direction (-1.0 = down, 1.0 = up)
    pub direction: f64,
    /// Current volatility
    pub volatility: f64,
}