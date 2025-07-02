/*
 * Signal Service
 *
 * This module implements a service that connects the SignalEngine with the ScamPredictionWriter
 * and ZMQ publisher to create a complete market signal detection and distribution pipeline.
 * It processes simulated transactions, analyzes them for various market signals, logs alerts
 * to the database, and publishes signals for automated trading systems.
 */

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{error, info, debug};
use serde_json;

use crate::database::ScamPredictionWriter;
use crate::pool_subscriber::cache::PoolStateCache;
use super::engine::{SignalEngine, SignalConfig};
use super::types::{MarketEvent, SimulationResult, EventType, Severity};

/// Service that integrates market signal detection with database logging and signal publishing
pub struct SignalService {
    /// The signal engine
    engine: SignalEngine,
    
    /// Database writer for persisting scam predictions
    db_writer: Arc<ScamPredictionWriter>,
    
    /// ZMQ publisher for real-time signals (optional)
    zmq_publisher: Option<Arc<zmq::Socket>>,
    
    /// Statistics for monitoring
    stats: Arc<Mutex<ServiceStats>>,
}

/// Statistics for monitoring the service
#[derive(Debug, Default, Clone)]
pub struct ServiceStats {
    /// Total number of transactions analyzed
    pub transactions_analyzed: u64,
    
    /// Number of events by type
    pub events_by_type: HashMap<EventType, u64>,
    
    /// Number of events successfully logged to database
    pub events_logged: u64,
    
    /// Number of signals published via ZMQ
    pub signals_published: u64,
    
    /// Number of database errors encountered
    pub db_errors: u64,
    
    /// Number of ZMQ publish errors
    pub zmq_errors: u64,
}

impl SignalService {
    /// Create a new SignalService with the specified components
    pub fn new(
        pool_cache: Arc<PoolStateCache>,
        db_writer: Arc<ScamPredictionWriter>,
        config: SignalConfig,
    ) -> Self {
        let engine = SignalEngine::new(pool_cache, config);
        
        Self {
            engine,
            db_writer,
            zmq_publisher: None,
            stats: Arc::new(Mutex::new(ServiceStats::default())),
        }
    }
    
    /// Create a new SignalService with ZMQ publisher
    pub fn with_zmq_publisher(
        pool_cache: Arc<PoolStateCache>,
        db_writer: Arc<ScamPredictionWriter>,
        config: SignalConfig,
        zmq_endpoint: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = SignalEngine::new(pool_cache, config);
        
        // Create ZMQ publisher socket
        let context = zmq::Context::new();
        let publisher = context.socket(zmq::PUB)?;
        publisher.bind(zmq_endpoint)?;
        
        info!("ZMQ publisher bound to {}", zmq_endpoint);
        
        Ok(Self {
            engine,
            db_writer,
            zmq_publisher: Some(Arc::new(publisher)),
            stats: Arc::new(Mutex::new(ServiceStats::default())),
        })
    }
    
    /// Create a new SignalService with default configuration
    pub async fn with_defaults(
        pool_cache: Arc<PoolStateCache>,
        db_host: &str,
        db_port: u16,
        db_name: &str,
        db_user: &str,
        db_password: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create scam prediction writer with provided connection parameters
        let db_writer = ScamPredictionWriter::new(
            db_user,
            db_password,
            db_host,
            db_port,
            db_name,
        ).await?;
        
        Ok(Self::new(
            pool_cache,
            Arc::new(db_writer),
            DecisionConfig::default(),
        ))
    }
    
    /// Get current service statistics
    pub async fn get_stats(&self) -> ServiceStats {
        self.stats.lock().await.clone()
    }
    
    /// Process a simulated transaction, detect market events, and distribute them
    pub async fn process_transaction(&self, simulation: SimulationResult) -> Result<Vec<MarketEvent>, Box<dyn std::error::Error>> {
        // Update transaction count
        {
            let mut stats = self.stats.lock().await;
            stats.transactions_analyzed += 1;
        }
        
        // Run the market event analysis
        let events = self.engine.analyze_transaction(simulation);
        
        // Update event counts by type
        {
            let mut stats = self.stats.lock().await;
            for event in &events {
                *stats.events_by_type.entry(event.event_type).or_insert(0) += 1;
            }
        }
        
        // Process each event
        let mut processed_events = Vec::new();
        
        for event in events {
            // Log to database (for all events)
            match self.log_event_to_db(&event).await {
                Ok(_) => {
                    let mut stats = self.stats.lock().await;
                    stats.events_logged += 1;
                },
                Err(e) => {
                    error!("Failed to log market event to database: {}", e);
                    let mut stats = self.stats.lock().await;
                    stats.db_errors += 1;
                }
            }
            
            // Publish via ZMQ (for high-severity events)
            if event.severity >= Severity::High {
                if let Some(ref publisher) = self.zmq_publisher {
                    match self.publish_event(&event, publisher).await {
                        Ok(_) => {
                            let mut stats = self.stats.lock().await;
                            stats.signals_published += 1;
                        },
                        Err(e) => {
                            error!("Failed to publish market event via ZMQ: {}", e);
                            let mut stats = self.stats.lock().await;
                            stats.zmq_errors += 1;
                        }
                    }
                }
            }
            
            processed_events.push(event);
        }
        
        Ok(processed_events)
    }
    
    /// Log a market event to the database
    async fn log_event_to_db(&self, event: &MarketEvent) -> Result<(), Box<dyn std::error::Error>> {
        // For backward compatibility, convert critical scam events to legacy format
        if event.event_type == EventType::ScamAlert {
            let prediction_block_number = event.block_number as i64;
            let current_eth = event.metrics.new_eth_reserve - event.metrics.eth_change;
            
            // Log the transaction hash along with the scam alert
            info!("Processing scam alert for tx: {}", event.tx_hash);
            
            self.db_writer.write_mempool_scam_prediction_with_tx(
                &event.token_address,
                &event.pool_address,
                prediction_block_number,
                current_eth,
                event.metrics.new_eth_reserve,
                0.1, // Default threshold
                Some(&event.tx_hash),
            ).await?;
        }
        
        // TODO: Add new table for all market events
        // self.db_logger.write_market_event(event).await?;
        
        Ok(())
    }
    
    /// Publish a market event via ZMQ
    async fn publish_event(&self, event: &MarketEvent, publisher: &zmq::Socket) -> Result<(), Box<dyn std::error::Error>> {
        let message = serde_json::to_string(event)?;
        publisher.send(&message, 0)?;
        
        debug!("Published {} event for pool {}", 
               match event.event_type {
                   EventType::ScamAlert => "SCAM",
                   EventType::LiquidityWarning => "LIQUIDITY",
                   EventType::TokenSupplyAlert => "SUPPLY",
                   EventType::VolumeSpike => "VOLUME",
                   EventType::PriceImpact => "PRICE",
                   EventType::LargeTrade => "TRADE",
               },
               event.pool_address);
        
        Ok(())
    }
}

// Legacy compatibility layer
pub type ScamDetectionService = SignalService;
pub type ScamDetectionConfig = SignalConfig;
pub type DecisionService = SignalService;
pub type DecisionConfig = SignalConfig;

impl SignalService {
    /// Legacy method for backward compatibility
    pub async fn process_transaction_legacy(&self, simulation: SimulationResult) -> Result<Vec<super::types::ScamAlert>, Box<dyn std::error::Error>> {
        let events = self.process_transaction(simulation).await?;
        
        // Convert only ScamAlert events to legacy format
        Ok(events.into_iter()
            .filter(|e| e.event_type == EventType::ScamAlert)
            .map(|event| super::types::ScamAlert {
                tx_hash: event.tx_hash,
                pool_address: event.pool_address,
                current_eth_reserve: event.metrics.new_eth_reserve - event.metrics.eth_change,
                simulated_eth_reserve: event.metrics.new_eth_reserve,
                percentage_drain: event.metrics.eth_percent.abs(),
                detection_time: event.detection_time,
                detection_block: event.block_number,
                reason: super::types::ScamAlertReason::LargeEthWithdrawal,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use ethers::types::H256;
    use crate::signal_engine::types::PoolEffect;
    use crate::pool_subscriber::types::PoolUpdate;
    
    // Helper to create a test pool cache with sample data
    fn create_test_pool_cache() -> Arc<PoolStateCache> {
        let cache = PoolStateCache::new(0.05);
        
        // Add some test pools
        let mut updates = HashMap::new();
        
        // Pool with low reserves
        updates.insert(
            "0x2234567890abcdef1234567890abcdef12345678".to_string(),
            PoolUpdate {
                eth_reserve: 0.1,
                token_reserve: 200.0,
                token_address: "0xtoken2".to_string(),
                block_number: 12345,
                update_time: 1626000000.0,
            }
        );
        
        cache.update_pools(updates.iter());
        Arc::new(cache)
    }
    
    // Helper to create a test simulation result
    fn create_test_simulation() -> SimulationResult {
        let tx_hash = H256::random();
        let mut affected_pools = HashMap::new();
        
        affected_pools.insert(
            "0x2234567890abcdef1234567890abcdef12345678".to_string(),
            PoolEffect {
                pool_address: "0x2234567890abcdef1234567890abcdef12345678".to_string(),
                current_eth_reserve: 0.1,
                simulated_eth_reserve: 0.01,
                current_token_reserve: 200.0,
                simulated_token_reserve: 200.0,
                eth_delta: -0.09,
                token_delta: 0.0,
                percentage_change: -90.0,
            }
        );
        
        SimulationResult {
            tx_hash: format!("{:?}", tx_hash),
            affected_pools,
            simulation_successful: true,
            error_message: None,
        }
    }
}