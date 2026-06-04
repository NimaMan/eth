use eyre::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

mod run_metadata;
mod scammer;

#[derive(Clone)]
pub struct RiskAtlasReader {
    pool: PgPool,
}

impl RiskAtlasReader {
    pub async fn connect(database_url: &str) -> Result<Self> {
        Ok(Self {
            pool: PgPool::connect(database_url).await?,
        })
    }

    pub fn connect_lazy(database_url: &str) -> Result<Self> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect_lazy(database_url)?,
        })
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
