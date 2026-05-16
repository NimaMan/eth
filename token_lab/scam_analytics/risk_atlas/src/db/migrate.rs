use eyre::Result;
use sqlx::PgPool;

pub const RISK_ATLAS_SCHEMA_SQL: &str = include_str!("../../migrations/001_create_risk_atlas.sql");

pub async fn apply(pool: &PgPool) -> Result<()> {
    for statement in RISK_ATLAS_SCHEMA_SQL.split(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
