use sqlx::PgPool;

use crate::Result;

pub const CREATE_TOKEN_PNL_SQL: &str = include_str!("../migrations/001_create_token_pnl.sql");

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    for statement in CREATE_TOKEN_PNL_SQL.split(';').map(str::trim) {
        if statement.is_empty() {
            continue;
        }
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
