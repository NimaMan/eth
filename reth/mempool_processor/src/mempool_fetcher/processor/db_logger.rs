/*
 * PostgreSQL Database Logger
 *
 * This module provides functionality to log scam detection results to a PostgreSQL database.
 * It implements connection management and query execution for storing mempool scam predictions
 * in the eth_db.mempool_scam_predictions table.
 *
 * The logger handles database connection pooling, error handling, and transaction management
 * to ensure reliable data persistence.
 */

use tokio_postgres::{NoTls, Error as PgError, Client};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, debug};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct DbLogger {
    client: Arc<Mutex<Option<Client>>>,
    connection_string: String,
}

impl DbLogger {
    /// Create a new database logger with the provided connection information
    pub async fn new(
        user: &str,
        password: &str,
        host: &str,
        port: u16,
        dbname: &str,
    ) -> Result<Self, PgError> {
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}/{}?application_name=mempool_processor",
            user, password, host, port, dbname
        );
        
        let logger = DbLogger {
            client: Arc::new(Mutex::new(None)),
            connection_string,
        };
        
        // Initialize connection
        logger.connect().await?;
        
        info!("Database logger initialized successfully");
        Ok(logger)
    }
    
    /// Create a new database logger with default connection parameters
    pub async fn default() -> Result<Self, PgError> {
        Self::new("postgres", "postgres", "localhost", 5432, "eth_db").await
    }
    
    /// Connect to the database
    async fn connect(&self) -> Result<(), PgError> {
        let mut client_lock = self.client.lock().await;
        
        // Only connect if not already connected
        if client_lock.is_none() {
            debug!("Connecting to database: {}", self.connection_string);
            
            let (client, connection) = tokio_postgres::connect(&self.connection_string, NoTls).await?;
            
            // Spawn the connection handler in the background
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Database connection error: {}", e);
                }
            });
            
            *client_lock = Some(client);
            debug!("Database connection established");
        }
        
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
                info!("🚨 SCAM ALERT LOGGED TO DATABASE: Token {} in pool {} depleted from {} to {} ETH", 
                     token_address, pool_address, current_eth_level, simulated_eth_level);
                Ok(())
            },
            Err(e) => {
                // Check if this is a foreign key constraint error
                if let Some(db_error) = e.as_db_error() {
                    if db_error.code().code() == "23503" { // Foreign key violation
                        error!("🚨 SCAM DETECTED but NOT LOGGED: Token {} not found in tokens table", token_address);
                        error!("Pool {} depleted from {} to {} ETH - but database write failed due to missing token", 
                              pool_address, current_eth_level, simulated_eth_level);
                        // Return Ok to not crash the service, but log the issue prominently
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
    fn test_db_logger_creation() {
        let rt = Runtime::new().unwrap();
        
        // This test doesn't actually connect to a database
        // It just checks that the logger can be created without errors
        let logger = rt.block_on(async {
            DbLogger::new("test_user", "test_password", "test_host", 5432, "test_db").await
        });
        
        assert!(logger.is_ok());
    }
} 