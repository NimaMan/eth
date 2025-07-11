//! Database connection pool manager

use qarqa_core_types::{QarqaError, QarqaResult};
use sqlx::PgPool;
use tracing::{info, error};

/// Database connection pool manager
pub struct DatabaseManager {
    pool: PgPool,
}

impl DatabaseManager {
    /// Create a new database manager
    pub async fn new(database_url: &str) -> QarqaResult<Self> {
        info!("Connecting to database: {}", database_url);
        
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| {
                error!("Failed to connect to database: {}", e);
                QarqaError::Database(e.to_string())
            })?;
        
        // Test the connection
        sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                error!("Database connection test failed: {}", e);
                QarqaError::Database(e.to_string())
            })?;
        
        info!("Database connection established successfully");
        
        Ok(Self { pool })
    }
    
    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    
    /// Get a cloned connection pool
    pub fn pool_cloned(&self) -> PgPool {
        self.pool.clone()
    }
    
    /// Close the database connection pool
    pub async fn close(self) {
        self.pool.close().await;
        info!("Database connection pool closed");
    }
    
    /// Check database health
    pub async fn health_check(&self) -> QarqaResult<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_database_manager() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
        
        let db = DatabaseManager::new(&database_url).await.unwrap();
        
        // Test health check
        db.health_check().await.unwrap();
        
        // Test pool access
        let _pool = db.pool();
        let _pool_cloned = db.pool_cloned();
        
        // Close connection
        db.close().await;
    }
}