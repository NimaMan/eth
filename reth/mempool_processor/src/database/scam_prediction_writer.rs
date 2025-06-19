/*
 * PostgreSQL Scam Prediction Writer
 *
 * This module provides functionality to write scam detection results to a PostgreSQL database.
 * It implements connection management and query execution for storing mempool scam predictions
 * in the eth_db.mempool_scam_predictions table.
 *
 * The writer handles database connection pooling, error handling, and transaction management
 * to ensure reliable data persistence.
 */

use tokio_postgres::{NoTls, Error as PgError, Client};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, debug};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ScamPredictionWriter {
    client: Arc<Mutex<Option<Client>>>,
    connection_string: String,
}

impl ScamPredictionWriter {
    /// Create a new scam prediction writer with the provided connection information
    pub async fn new(
        user: &str,
        password: &str,
        host: &str,
        port: u16,
        dbname: &str,
    ) -> Result<Self, PgError> {
        info!("ScamPredictionWriter::new() called with host={}, port={}, dbname={}", host, port, dbname);
        
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}/{}?application_name=mempool_processor",
            user, password, host, port, dbname
        );
        info!("ScamPredictionWriter::new() - connection string prepared");
        
        let logger = ScamPredictionWriter {
            client: Arc::new(Mutex::new(None)),
            connection_string,
        };
        info!("ScamPredictionWriter::new() - writer struct created");
        
        // Initialize connection
        info!("ScamPredictionWriter::new() - calling connect()...");
        logger.connect().await?;
        info!("ScamPredictionWriter::new() - connect() returned successfully");
        
        info!("Scam prediction writer initialized successfully");
        info!("ScamPredictionWriter::new() - returning writer instance");
        Ok(logger)
    }
    
    /// Create a new scam prediction writer with default connection parameters
    pub async fn default() -> Result<Self, PgError> {
        let user = std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string());
        let password = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
        let host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("DB_PORT")
            .unwrap_or_else(|_| "5432".to_string())
            .parse::<u16>()
            .unwrap_or(5432);
        let database = std::env::var("DB_NAME").unwrap_or_else(|_| "eth_db".to_string());
        
        Self::new(&user, &password, &host, port, &database).await
    }
    
    /// Connect to the database
    async fn connect(&self) -> Result<(), PgError> {
        info!("connect() called - acquiring lock...");
        let mut client_lock = self.client.lock().await;
        info!("connect() - lock acquired");
        
        // Only connect if not already connected
        if client_lock.is_none() {
            info!("connect() - attempting to connect to database...");
            debug!("Connecting to database: {}", self.connection_string);
            
            info!("connect() - calling tokio_postgres::connect...");
            let (client, connection) = tokio_postgres::connect(&self.connection_string, NoTls).await?;
            info!("connect() - tokio_postgres::connect returned successfully");
            
            // Spawn the connection handler in the background
            info!("connect() - spawning connection handler...");
            tokio::spawn(async move {
                info!("Connection handler task started");
                if let Err(e) = connection.await {
                    error!("Database connection error: {}", e);
                } else {
                    info!("Connection handler completed successfully");
                }
            });
            info!("connect() - connection handler spawned");
            
            *client_lock = Some(client);
            debug!("Database connection established");
            info!("connect() - client stored, connection established");
        } else {
            info!("connect() - already connected, skipping");
        }
        
        info!("connect() - releasing lock and returning Ok");
        Ok(())
    }
    
    /// Execute a direct SQL query on the database connection
    /// This is useful for administrative commands or testing
    pub async fn execute_query(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64, PgError> {
        let mut client_guard = self.client.lock().await;
        
        // Ensure we have a connection
        if client_guard.is_none() {
            drop(client_guard); // Release the lock before connecting
            self.connect().await?;
            client_guard = self.client.lock().await;
        }
        
        let client = client_guard.as_ref().expect("Database client not initialized");
        client.execute(query, params).await
    }
    
    /// Writes a mempool scam prediction to the database
    pub async fn write_mempool_scam_prediction(
        &self,
        token_address: &str,
        pool_address: &str,
        prediction_block_number: i64, // Changed to i64 to match PostgreSQL's BigInteger type
        current_eth_level: f64,
        simulated_eth_level: f64,
        eth_threshold: f64,
    ) -> Result<(), PgError> {
        self.write_mempool_scam_prediction_with_tx(
            token_address,
            pool_address,
            prediction_block_number,
            current_eth_level,
            simulated_eth_level,
            eth_threshold,
            None,
        ).await
    }
    
    /// Writes a mempool scam prediction to the database with optional transaction hash
    pub async fn write_mempool_scam_prediction_with_tx(
        &self,
        token_address: &str,
        pool_address: &str,
        prediction_block_number: i64,
        current_eth_level: f64,
        simulated_eth_level: f64,
        eth_threshold: f64,
        tx_hash: Option<&str>,
    ) -> Result<(), PgError> {
        let mut client_guard = self.client.lock().await;
        
        // Ensure we have a connection
        if client_guard.is_none() {
            drop(client_guard); // Release the lock before connecting
            self.connect().await?;
            client_guard = self.client.lock().await;
        }
        
        let client = client_guard.as_ref().expect("Database client not initialized");
        
        // Get current timestamp if needed
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i32; // Using i32 for timestamp as it's an Integer in PostgreSQL
        
        // SQL INSERT query for scam prediction
        const QUERY: &str = "
            INSERT INTO eth_db.mempool_scam_predictions (
                token_address, pool_address, prediction_block_number,
                prediction_timestamp, current_eth_level, simulated_eth_level, eth_threshold
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
        ";
        
        debug!(
            "Logging scam prediction for token: {}, pool: {}, block: {}",
            token_address, pool_address, prediction_block_number
        );
        
        // Execute the query with proper error handling
        match client.execute(
            QUERY,
            &[
                &token_address,
                &pool_address,
                &prediction_block_number,
                &current_timestamp,
                &current_eth_level,
                &simulated_eth_level,
                &eth_threshold,
            ],
        ).await {
            Ok(_) => {
                // Log the full scam details when successfully written to database
                info!("🚨 SCAM DETECTED AND LOGGED TO DATABASE:");
                if let Some(tx) = tx_hash {
                    info!("   Transaction Hash: {}", tx);
                }
                info!("   Token Address: {}", token_address);
                info!("   Pool Address: {}", pool_address);
                info!("   Block Number: {}", prediction_block_number);
                info!("   ETH Drained: {} -> {} ETH ({:.2}% loss)", 
                     current_eth_level, simulated_eth_level, 
                     ((current_eth_level - simulated_eth_level) / current_eth_level * 100.0));
                info!("   Amount Lost: {:.6} ETH", current_eth_level - simulated_eth_level);
                info!("   Database Status: Successfully logged");
                
                // Also log in a parseable format for external tools
                info!("SCAM_ALERT|{}|{}|{}|{}|{}|{}|LOGGED",
                     tx_hash.unwrap_or("NO_TX_HASH"),
                     token_address, pool_address, prediction_block_number,
                     current_eth_level, simulated_eth_level);
                     
                Ok(())
            },
            Err(e) => {
                // Check if this is a foreign key constraint error
                if let Some(db_error) = e.as_db_error() {
                    if db_error.code().code() == "23503" { // Foreign key violation
                        // Log the full scam details even if database write fails
                        error!("🚨 SCAM DETECTED (NOT IN DB - Token Missing):");
                        if let Some(tx) = tx_hash {
                            error!("   Transaction Hash: {}", tx);
                        }
                        error!("   Token Address: {}", token_address);
                        error!("   Pool Address: {}", pool_address);
                        error!("   Block Number: {}", prediction_block_number);
                        error!("   ETH Drained: {} -> {} ETH ({:.2}% loss)", 
                              current_eth_level, simulated_eth_level, 
                              ((current_eth_level - simulated_eth_level) / current_eth_level * 100.0));
                        error!("   Amount Lost: {:.6} ETH", current_eth_level - simulated_eth_level);
                        error!("   Database Status: Write failed - token not in database");
                        
                        // Still try to log important info that doesn't require the token
                        info!("SCAM_ALERT|{}|{}|{}|{}|{}|{}|MISSING_TOKEN",
                             tx_hash.unwrap_or("NO_TX_HASH"),
                             token_address, pool_address, prediction_block_number,
                             current_eth_level, simulated_eth_level);
                        
                        // Return Ok to not crash the service
                        return Ok(());
                    }
                }
                
                // For other errors, propagate them
                error!("Failed to log scam prediction: {}", e);
                Err(e)
            }
        }
    }
    
    /// Delete test records from the database
    pub async fn delete_test_records(
        &self,
        token_address: &str,
        pool_address: &str,
        prediction_block_number: i64,
    ) -> Result<u64, PgError> {
        let mut client_guard = self.client.lock().await;
        
        // Ensure we have a connection
        if client_guard.is_none() {
            drop(client_guard); // Release the lock before connecting
            self.connect().await?;
            client_guard = self.client.lock().await;
        }
        
        let client = client_guard.as_ref().expect("Database client not initialized");
        
        // SQL DELETE query
        const QUERY: &str = "
            DELETE FROM eth_db.mempool_scam_predictions
            WHERE token_address = $1
            AND pool_address = $2
            AND prediction_block_number = $3
        ";
        
        debug!(
            "Deleting test records for token: {}, pool: {}, block: {}",
            token_address, pool_address, prediction_block_number
        );
        
        // Execute the query
        let rows_deleted = client.execute(
            QUERY, 
            &[&token_address, &pool_address, &prediction_block_number]
        ).await?;
        
        debug!("Deleted {} test records", rows_deleted);
        Ok(rows_deleted)
    }
    
    /// Check if the database connection is active
    pub async fn is_connected(&self) -> bool {
        let client_guard = self.client.lock().await;
        client_guard.is_some()
    }
    
    /// Reconnect to the database if the connection is lost
    pub async fn ensure_connected(&self) -> Result<(), PgError> {
        if !self.is_connected().await {
            self.connect().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;
    
    #[test]
    fn test_scam_prediction_writer_creation() {
        let rt = Runtime::new().unwrap();
        
        // This test doesn't actually connect to a database
        // It just checks that the writer can be created without errors
        let writer = rt.block_on(async {
            ScamPredictionWriter::new("test_user", "test_password", "test_host", 5432, "test_db").await
        });
        
        assert!(writer.is_ok());
    }
} 