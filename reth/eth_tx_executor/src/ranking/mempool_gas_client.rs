/// Mempool Gas Client
/// 
/// Client for querying gas metrics from the mempool processor service.
/// Replaces the WebSocket-based mempool tracker with a simpler HTTP API approach.

use super::{GasPercentiles as RankingGasPercentiles, MempoolStats as RankingMempoolStats, CongestionLevel as RankingCongestionLevel};
use ethers::types::U256;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use eyre::Result;

/// Gas price percentiles for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPercentiles {
    pub p50: U256,
    pub p75: U256,
    pub p90: U256,
    pub p95: U256,
    pub p99: U256,
}

/// Congestion level for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// HTTP client for mempool gas metrics
pub struct MempoolGasClient {
    client: Client,
    base_url: String,
}

/// Position result from gas metrics API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionResult {
    pub estimated_position: u64,
    pub confidence: f64,
    pub transactions_ahead: u64,
    pub total_pending: u64,
    pub percentile_rank: f64,
}

/// Gas metrics response
#[derive(Debug, Clone, Deserialize)]
pub struct GasMetricsResponse {
    pub metrics: MempoolGasMetrics,
    pub congestion: MempoolCongestion,
    pub mev_activity: MevActivity,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MempoolGasMetrics {
    pub gas_price_percentiles: GasPercentiles,
    pub pending_tx_count: u64,
    pub avg_gas_price: U256,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MempoolCongestion {
    pub congestion_level: CongestionLevel,
    pub arrival_rate_per_second: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MevActivity {
    pub mev_percentage: f64,
    pub high_priority_tx_count: u64,
}

impl MempoolGasClient {
    /// Create a new mempool gas client
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(1000)) // 1 second timeout
            .build()
            .expect("Failed to create HTTP client");
            
        Self {
            client,
            base_url: base_url.to_string(),
        }
    }
    
    /// Get current gas metrics
    pub async fn get_gas_metrics(&self) -> Result<GasMetricsResponse> {
        let url = format!("{}/gas-metrics", self.base_url);
        let response = self.client
            .get(&url)
            .send()
            .await?
            .json::<GasMetricsResponse>()
            .await?;
            
        Ok(response)
    }
    
    /// Get position for a given gas price
    pub async fn get_position_for_gas_price(&self, gas_price: U256) -> Result<PositionResult> {
        let url = format!("{}/gas-position?gas_price={}", self.base_url, gas_price);
        let response = self.client
            .get(&url)
            .send()
            .await?
            .json::<PositionResult>()
            .await?;
            
        Ok(response)
    }
    
    /// Get gas price needed for target position
    pub async fn get_gas_price_for_position(&self, position: u64) -> Result<U256> {
        let url = format!("{}/gas-for-position?position={}", self.base_url, position);
        let response = self.client
            .get(&url)
            .send()
            .await?
            .json::<U256>()
            .await?;
            
        Ok(response)
    }
    
    /// Convert to legacy MempoolStats format for compatibility
    pub async fn get_mempool_stats(&self) -> Result<RankingMempoolStats> {
        let metrics = self.get_gas_metrics().await?;
        
        // Convert local types to ranking types
        let gas_percentiles = RankingGasPercentiles {
            p50: metrics.metrics.gas_price_percentiles.p50,
            p75: metrics.metrics.gas_price_percentiles.p75,
            p90: metrics.metrics.gas_price_percentiles.p90,
            p95: metrics.metrics.gas_price_percentiles.p95,
            p99: metrics.metrics.gas_price_percentiles.p99,
        };
        
        let congestion_level = match metrics.congestion.congestion_level {
            CongestionLevel::Low => RankingCongestionLevel::Low,
            CongestionLevel::Medium => RankingCongestionLevel::Medium,
            CongestionLevel::High => RankingCongestionLevel::High,
            CongestionLevel::Critical => RankingCongestionLevel::Extreme,
        };
        
        Ok(RankingMempoolStats {
            pending_tx_count: metrics.metrics.pending_tx_count,
            gas_price_percentiles: gas_percentiles,
            avg_arrival_rate: metrics.congestion.arrival_rate_per_second,
            congestion_level,
            mev_tx_count: metrics.mev_activity.high_priority_tx_count,
        })
    }
}

/// Configuration for the gas client
pub struct GasClientConfig {
    /// Base URL of the mempool processor API
    pub api_url: String,
    
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Whether to use caching
    pub enable_cache: bool,
    
    /// Cache TTL in milliseconds
    pub cache_ttl_ms: u64,
}

impl Default for GasClientConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:8088".to_string(),
            timeout_ms: 1000,
            enable_cache: true,
            cache_ttl_ms: 100, // 100ms cache
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_gas_client_mock() {
        // This would normally connect to a real service
        // For testing, we just verify the client constructs properly
        let client = MempoolGasClient::new("http://localhost:8088");
        
        // In a real test, you'd mock the HTTP responses
        // or run against a test instance
        assert_eq!(client.base_url, "http://localhost:8088");
    }
}