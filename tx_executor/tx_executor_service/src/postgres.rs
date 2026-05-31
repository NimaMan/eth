use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::execution_config::ExecutionConfig;

const SCHEMA_SQL: &str = include_str!("schema.sql");

pub async fn connect_pool(config: &ExecutionConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(config.database_url())
        .await
        .context("failed to connect to ETH tx execution Postgres")?;

    sqlx::raw_sql(SCHEMA_SQL)
        .execute(&pool)
        .await
        .context("failed to initialize ETH tx execution schema")?;

    Ok(pool)
}
