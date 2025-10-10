use eyre::Result;
/// Database Connection Pool Management
///
/// Handles PostgreSQL connection pooling with configurable settings.
/// Uses sqlx for async operations and automatic reconnection.
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// PostgreSQL database connection pool
pub struct PostgresDB {
    pool: PgPool,
}

impl PostgresDB {
    /// Create new connection pool with default settings
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(3))
            .idle_timeout(Duration::from_secs(600))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Create connection pool with custom settings
    pub async fn with_options(
        database_url: &str,
        max_connections: u32,
        min_connections: u32,
    ) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(3))
            .idle_timeout(Duration::from_secs(600))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Get reference to the underlying pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Test database connectivity
    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size(),
            idle: self.pool.num_idle(),
        }
    }
}

/// Connection pool statistics
#[derive(Debug)]
pub struct PoolStats {
    pub size: u32,
    pub idle: usize,
}
