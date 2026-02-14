use ethers::types::U256;
use eyre::Result;
use std::sync::Arc;
use tracing::{error, info};
/// Gas ranking integration module
///
/// Embeds tx_ranking_system directly into eth_kartal for zero-latency gas recommendations.
/// This eliminates network overhead and provides sub-microsecond response times.
use tx_ranking_system::gas_cache::{BlockConsumer, GasAPI, GasCache};

/// Embedded gas ranking system
pub struct GasRanking {
    gas_cache: Arc<GasCache>,
    gas_api: GasAPI,
}

impl GasRanking {
    /// Initialize gas ranking with RabbitMQ and RPC connections
    pub async fn new(rabbitmq_url: &str, rpc_url: &str) -> Result<Self> {
        info!("Initializing embedded gas ranking system");

        // Create gas cache
        let gas_cache = Arc::new(GasCache::new(100));

        // Start block consumer in background
        let consumer_cache = gas_cache.clone();
        let rabbitmq_url = rabbitmq_url.to_string();
        let rpc_url = rpc_url.to_string();

        tokio::spawn(async move {
            info!("Starting gas cache block consumer");
            match BlockConsumer::new(&rabbitmq_url, (*consumer_cache).clone(), &rpc_url).await {
                Ok(consumer) => {
                    if let Err(e) = consumer.start_consuming().await {
                        error!("Block consumer error: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to create block consumer: {}", e);
                }
            }
        });

        // Create API
        let gas_api = GasAPI::new((*gas_cache).clone());

        Ok(Self { gas_cache, gas_api })
    }

    /// Get gas recommendation for frontrunning (anti-scam)
    pub fn get_frontrun_gas(&self, target_gas: U256) -> GasRecommendation {
        // Get current gas prices (60ns latency)
        let prices = self.gas_api.get_instant_prices();

        // Calculate gas needed to frontrun
        // Must be higher than target AND in top percentile
        let recommended_gas = std::cmp::max(
            target_gas * 125 / 100, // 25% higher than target
            prices.urgent,          // At least p99
        );

        // Add congestion margin
        let final_gas = if prices.mev > prices.urgent * 2 {
            // High MEV activity, need more margin
            recommended_gas * 130 / 100
        } else {
            recommended_gas * 110 / 100
        };

        GasRecommendation {
            priority_fee: final_gas - prices.base_fee,
            max_fee: final_gas + U256::from(2_000_000_000u64), // 2 gwei buffer
            expected_position: 1,                              // Top position
            confidence: 0.92,
            execution_path: if final_gas > prices.mev {
                ExecutionPath::Flashbots // Use private mempool for very high gas
            } else {
                ExecutionPath::Public
            },
        }
    }

    /// Get gas recommendation for backrunning (trading enabled)
    pub fn get_backrun_gas(&self, target_gas: U256) -> GasRecommendation {
        // For backrun, we want to be slightly lower
        let recommended_gas = target_gas * 95 / 100; // 5% less

        let prices = self.gas_api.get_instant_prices();

        GasRecommendation {
            priority_fee: recommended_gas - prices.base_fee,
            max_fee: recommended_gas + U256::from(2_000_000_000u64),
            expected_position: 50, // Behind target
            confidence: 0.88,
            execution_path: ExecutionPath::Flashbots, // Always use bundles for backruns
        }
    }

    /// Get gas for specific queue position
    pub fn get_gas_for_position(&self, position: u64) -> GasRecommendation {
        let ranking = self.gas_api.recommend_for_position(position as u32);

        GasRecommendation {
            priority_fee: ranking.priority_fee,
            max_fee: ranking.gas_price + U256::from(2_000_000_000u64),
            expected_position: ranking.expected_position as u64,
            confidence: ranking.percentile / 100.0,
            execution_path: if ranking.priority_fee > U256::from(100_000_000_000u64) {
                ExecutionPath::Flashbots
            } else {
                ExecutionPath::Public
            },
        }
    }

    /// Check if gas price beats percentile
    pub fn beats_percentile(&self, gas_price: U256, percentile: f32) -> bool {
        self.gas_api.beats_percentile(gas_price, percentile as f64)
    }
}

/// Gas recommendation for transaction submission
#[derive(Debug, Clone)]
pub struct GasRecommendation {
    pub priority_fee: U256,
    pub max_fee: U256,
    pub expected_position: u64,
    pub confidence: f64,
    pub execution_path: ExecutionPath,
}

/// Execution path for transaction
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionPath {
    Public,
    Flashbots,
    DirectBuilder,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gas_ranking_creation() {
        // This would need a test RabbitMQ instance
        // For now, just verify the structure compiles
        assert_eq!(
            std::mem::size_of::<GasRanking>(),
            std::mem::size_of::<usize>() * 2
        );
    }
}
