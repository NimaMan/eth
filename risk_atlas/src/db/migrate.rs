use eyre::Result;
use sqlx::PgPool;

pub use super::schema_sql::RISK_ATLAS_SCHEMA_SQL;

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
