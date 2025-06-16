/*
 * Scam Detection Service
 *
 * This module implements a service that connects the ScamDetectionEngine with the DbLogger
 * to create a complete scam detection and logging pipeline. It processes simulated 
 * transactions, analyzes them for potential scams, and logs alerts to the database.
 */

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

use crate::mempool_fetcher::processor::DbLogger;
use crate::pool_subscriber::cache::PoolStateCache;
use super::engine::{ScamDetectionEngine, ScamDetectionConfig};
use super::types::{ScamAlert, SimulationResult};

/// Service that integrates scam detection with database logging
pub struct ScamDetectionService {
    /// The scam detection engine
    engine: ScamDetectionEngine,
    
    /// Database logger for persisting scam alerts
    db_logger: Arc<DbLogger>,
    
    /// Statistics for monitoring
    stats: Arc<Mutex<ServiceStats>>,
}

/// Statistics for monitoring the service
#[derive(Debug, Default, Clone)]
pub struct ServiceStats {
    /// Total number of transactions analyzed
    pub transactions_analyzed: u64,
    
    /// Number of scam alerts generated
    pub scams_detected: u64,
    
    /// Number of alerts successfully logged to database
    pub alerts_logged: u64,
    
    /// Number of database errors encountered
    pub db_errors: u64,
}

impl ScamDetectionService {
    /// Create a new ScamDetectionService with the specified components
    pub fn new(
        pool_cache: Arc<PoolStateCache>,
        db_logger: Arc<DbLogger>,
        config: ScamDetectionConfig,
    ) -> Self {
        let engine = ScamDetectionEngine::new(pool_cache, config);
        
        Self {
            engine,
            db_logger,
            stats: Arc::new(Mutex::new(ServiceStats::default())),
        }
    }
    
    /// Create a new ScamDetectionService with default configuration
    pub async fn with_defaults(
        pool_cache: Arc<PoolStateCache>,
        db_host: &str,
        db_port: u16,
        db_name: &str,
        db_user: &str,
        db_password: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create DB logger with provided connection parameters
        let db_logger = DbLogger::new(
            db_user,
            db_password,
            db_host,
            db_port,
            db_name,
        ).await?;
        
        Ok(Self::new(
            pool_cache,
            Arc::new(db_logger),
            ScamDetectionConfig::default(),
        ))
    }
    
    /// Get current service statistics
    pub async fn get_stats(&self) -> ServiceStats {
        self.stats.lock().await.clone()
    }
    
    /// Process a simulated transaction, detect scams, and log alerts to database
    pub async fn process_transaction(&self, simulation: SimulationResult) -> Result<Vec<ScamAlert>, Box<dyn std::error::Error>> {
        // Update transaction count
        {
            let mut stats = self.stats.lock().await;
            stats.transactions_analyzed += 1;
        }
        
        // Run the scam detection analysis
        let alerts = self.engine.analyze_transaction(simulation);
        
        // Update scam detection count
        {
            let mut stats = self.stats.lock().await;
            stats.scams_detected += alerts.len() as u64;
        }
        
        // Log each alert to the database
        let mut logged_alerts = Vec::new();
        
        for alert in alerts {
            match self.log_alert_to_db(&alert).await {
                Ok(_) => {
                    // Successfully logged
                    {
                        let mut stats = self.stats.lock().await;
                        stats.alerts_logged += 1;
                    }
                    logged_alerts.push(alert);
                },
                Err(e) => {
                    error!("Failed to log scam alert to database: {}", e);
                    {
                        let mut stats = self.stats.lock().await;
                        stats.db_errors += 1;
                    }
                }
            }
        }
        
        Ok(logged_alerts)
    }
    
    /// Log a scam alert to the database
    async fn log_alert_to_db(&self, alert: &ScamAlert) -> Result<(), Box<dyn std::error::Error>> {
        // Convert detection block to i64 for PostgreSQL compatibility
        let prediction_block_number = alert.detection_block as i64;
        
        // Log to database using DbLogger
        self.db_logger.write_mempool_scam_prediction(
            &alert.token_address,
            &alert.pool_address,
            prediction_block_number,
            alert.current_eth_reserve,
            alert.simulated_eth_reserve,
            alert.eth_threshold,
        ).await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use ethers::types::H256;
    use crate::scam_detection::types::PoolEffect;
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
                eth_delta: -0.09,
                percentage_change: -0.9,
            }
        );
        
        SimulationResult {
            tx_hash: format!("{:?}", tx_hash),
            affected_pools,
            simulation_successful: true,
            error_message: None,
        }
    }
    
    // This is a mock implementation of DbLogger for testing
    #[derive(Debug, Clone)]
    struct MockDbLogger {
        last_logged: Arc<Mutex<Option<(String, String, i64, f64, f64, f64)>>>,
    }
    
    impl MockDbLogger {
        fn new() -> Self {
            Self {
                last_logged: Arc::new(Mutex::new(None)),
            }
        }
        
        async fn write_mempool_scam_prediction(
            &self,
            token_address: &str,
            pool_address: &str,
            prediction_block_number: i64,
            current_eth_level: f64,
            simulated_eth_level: f64,
            eth_threshold: f64,
        ) -> Result<(), tokio_postgres::Error> {
            // Store the values for verification
            let mut last_logged = self.last_logged.lock().await;
            *last_logged = Some((
                token_address.to_string(),
                pool_address.to_string(),
                prediction_block_number,
                current_eth_level,
                simulated_eth_level,
                eth_threshold,
            ));
            
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_service_logs_detected_scams() {
        // Create test components
        let pool_cache = create_test_pool_cache();
        let mock_logger = MockDbLogger::new();
        let last_logged = mock_logger.last_logged.clone();
        
        // Create the service with mock components
        let engine = ScamDetectionEngine::new(
            pool_cache,
            ScamDetectionConfig::default(),
        );
        
        let service = ScamDetectionService {
            engine,
            db_logger: Arc::new(mock_logger),
            stats: Arc::new(Mutex::new(ServiceStats::default())),
        };
        
        // Process a test transaction that should trigger a scam alert
        let simulation = create_test_simulation();
        let tx_hash = simulation.tx_hash.clone();
        
        let result = service.process_transaction(simulation).await;
        
        // Verify results
        assert!(result.is_ok());
        let alerts = result.unwrap();
        assert_eq!(alerts.len(), 1);
        
        // Verify the alert was logged to the database
        let logged = last_logged.lock().await;
        assert!(logged.is_some());
        
        let (token, pool, block, current, simulated, threshold) = logged.as_ref().unwrap();
        assert_eq!(token, "0xtoken2");
        assert_eq!(pool, "0x2234567890abcdef1234567890abcdef12345678");
        assert_eq!(*block, 12345);
        assert_eq!(*current, 0.1);
        assert_eq!(*simulated, 0.01);
        
        // Verify stats were updated
        let stats = service.get_stats().await;
        assert_eq!(stats.transactions_analyzed, 1);
        assert_eq!(stats.scams_detected, 1);
        assert_eq!(stats.alerts_logged, 1);
        assert_eq!(stats.db_errors, 0);
    }
} 