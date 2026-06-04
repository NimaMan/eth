use sqlx::PgPool;

use super::Result;

pub const CREATE_TOKEN_STATE_SQL: &str =
    include_str!("../../migrations/state/001_create_token_state.sql");

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::raw_sql(CREATE_TOKEN_STATE_SQL).execute(pool).await?;
    Ok(())
}
