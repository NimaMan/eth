//! State Persistence Module
//!
//! Persists risk management state to PostgreSQL to survive restarts

use super::circuit_breaker::CircuitBreakerState;
use crate::common::errors::KartalError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedRiskState {
    pub circuit_breaker_states: HashMap<String, CircuitBreakerSnapshot>,
    pub risk_limits: RiskLimitsSnapshot,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerSnapshot {
    pub state: String,
    pub failure_count: u32,
    pub consecutive_successes: u32,
    pub last_state_change: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLimitsSnapshot {
    pub max_position_size_wei: String,
    pub max_daily_volume_wei: String,
    pub max_slippage_bps: u32,
    pub current_daily_volume_wei: String,
    pub positions_count: u32,
}

pub struct StatePersistence {
    pool: PgPool,
}

impl StatePersistence {
    pub async fn new(database_url: &str) -> Result<Self, KartalError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| KartalError::DatabaseError(e.to_string()))?;

        // Create tables if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS risk_state (
                id VARCHAR(50) PRIMARY KEY,
                state_data JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS alert_history (
                alert_id VARCHAR(100) PRIMARY KEY,
                processed_at TIMESTAMPTZ NOT NULL,
                alert_type VARCHAR(50) NOT NULL,
                token_address VARCHAR(42),
                tx_hash VARCHAR(66)
            );
            
            CREATE INDEX IF NOT EXISTS idx_alert_history_processed_at 
            ON alert_history(processed_at);
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| KartalError::DatabaseError(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn save_risk_state(&self, state: &PersistedRiskState) -> Result<(), KartalError> {
        let state_json = serde_json::to_value(state)
            .map_err(|e| KartalError::SerializationError(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO risk_state (id, state_data, updated_at)
            VALUES ('current', $1, $2)
            ON CONFLICT (id) DO UPDATE
            SET state_data = $1, updated_at = $2
            "#,
        )
        .bind(state_json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| KartalError::DatabaseError(e.to_string()))?;

        info!("Persisted risk state to database");
        Ok(())
    }

    pub async fn load_risk_state(&self) -> Result<Option<PersistedRiskState>, KartalError> {
        let row = sqlx::query(
            r#"
            SELECT state_data, updated_at
            FROM risk_state
            WHERE id = 'current'
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| KartalError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => {
                let state_data: serde_json::Value = row
                    .try_get("state_data")
                    .map_err(|e| KartalError::DatabaseError(e.to_string()))?;
                let state: PersistedRiskState = serde_json::from_value(state_data)
                    .map_err(|e| KartalError::SerializationError(e.to_string()))?;

                let updated_at: DateTime<Utc> = row
                    .try_get("updated_at")
                    .map_err(|e| KartalError::DatabaseError(e.to_string()))?;
                info!(
                    "Loaded risk state from database (last updated: {})",
                    updated_at
                );
                Ok(Some(state))
            }
            None => {
                info!("No persisted risk state found");
                Ok(None)
            }
        }
    }

    pub async fn record_alert(
        &self,
        alert_id: &str,
        alert_type: &str,
        token_address: Option<&str>,
        tx_hash: Option<&str>,
    ) -> Result<(), KartalError> {
        sqlx::query(
            r#"
            INSERT INTO alert_history (alert_id, processed_at, alert_type, token_address, tx_hash)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (alert_id) DO NOTHING
            "#,
        )
        .bind(alert_id)
        .bind(Utc::now())
        .bind(alert_type)
        .bind(token_address)
        .bind(tx_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| KartalError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn is_alert_processed(&self, alert_id: &str) -> Result<bool, KartalError> {
        let row = sqlx::query(
            r#"
            SELECT EXISTS(SELECT 1 FROM alert_history WHERE alert_id = $1) as exists
            "#,
        )
        .bind(alert_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| KartalError::DatabaseError(e.to_string()))?;

        let exists: bool = row
            .try_get("exists")
            .map_err(|e| KartalError::DatabaseError(e.to_string()))?;
        Ok(exists)
    }

    pub async fn cleanup_old_alerts(&self, days_to_keep: i32) -> Result<u64, KartalError> {
        let result = sqlx::query(
            r#"
            DELETE FROM alert_history
            WHERE processed_at < NOW() - INTERVAL $1
            "#,
        )
        .bind(format!("{} days", days_to_keep))
        .execute(&self.pool)
        .await
        .map_err(|e| KartalError::DatabaseError(e.to_string()))?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            info!("Cleaned up {} old alerts", deleted);
        }

        Ok(deleted)
    }
}

impl CircuitBreakerSnapshot {
    pub fn from_state(
        state: &CircuitBreakerState,
        failure_count: u32,
        consecutive_successes: u32,
        last_change: DateTime<Utc>,
    ) -> Self {
        let state_str = match state {
            CircuitBreakerState::Closed => "closed",
            CircuitBreakerState::HalfOpen => "half_open",
            CircuitBreakerState::Open => "open",
        };

        Self {
            state: state_str.to_string(),
            failure_count,
            consecutive_successes,
            last_state_change: last_change,
        }
    }

    pub fn to_state(&self) -> CircuitBreakerState {
        match self.state.as_str() {
            "closed" => CircuitBreakerState::Closed,
            "half_open" => CircuitBreakerState::HalfOpen,
            "open" => CircuitBreakerState::Open,
            _ => CircuitBreakerState::Closed, // Default to closed on unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_persistence() {
        // Test would require test database
        // Skipping for now as it requires external dependency
    }
}
