//! Database connection management

use eyre::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Database connection configuration
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://localhost:5432/eth_db".to_string()),
            max_connections: 10,
            min_connections: 2,
            connect_timeout: Duration::from_secs(30),
        }
    }
}

/// Create a new database connection pool
pub async fn create_pool(config: &DbConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.connect_timeout)
        .connect(&config.database_url)
        .await?;

    // Verify schema exists
    verify_schema(&pool).await?;

    Ok(pool)
}

/// Verify that the eth_db schema exists and has expected tables
async fn verify_schema(pool: &PgPool) -> Result<()> {
    let query = r#"
        SELECT COUNT(*) as count
        FROM information_schema.tables 
        WHERE table_schema = 'eth_db' 
        AND table_name IN ('addresses', 'transactions', 'tx_participants', 'tokens', 'trades')
    "#;

    let row: (i64,) = sqlx::query_as(query).fetch_one(pool).await?;

    if row.0 < 5 {
        return Err(eyre::eyre!(
            "eth_db schema is missing required tables. Found {} of 5 expected tables",
            row.0
        ));
    }

    Ok(())
}
