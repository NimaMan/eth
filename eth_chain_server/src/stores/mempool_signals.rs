use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

const MAX_SIGNAL_LIMIT: i64 = 1_000;
const MAX_SIGNAL_LOOKBACK_DAYS: i64 = 3_650;

#[derive(Clone)]
pub struct MempoolSignalStore {
    pool: PgPool,
    default_limit: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MempoolSignalQuery {
    pub limit: Option<i64>,
    pub since_days: Option<i64>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MempoolSignalKind {
    All,
    TradingEnabled,
    SellBlocked,
    Tax,
    LiquidityRemoval,
    LpPositionApproval,
    TokenSupplyRisk,
}

#[derive(Debug, Serialize)]
pub struct MempoolSignalsResponse {
    pub signal_type: String,
    pub count: usize,
    pub signals: Vec<MempoolSignalView>,
}

#[derive(Debug, Serialize)]
pub struct MempoolSignalView {
    pub signal_id: String,
    pub signal_type: String,
    pub detection_timestamp: Option<String>,
    pub detection_tx_hash: Option<String>,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub pool_type: Option<String>,
    pub creator_address: Option<String>,
    pub subject_address: Option<String>,
    pub headline: Option<String>,
    pub value_1: Option<String>,
    pub value_2: Option<String>,
    pub flag: Option<String>,
    pub payload: Value,
}

impl MempoolSignalStore {
    pub fn new(database_url: &str, default_limit: i64) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_lazy(database_url)?;
        Ok(Self {
            pool,
            default_limit: default_limit.clamp(1, MAX_SIGNAL_LIMIT),
        })
    }

    pub async fn list(
        &self,
        kind: MempoolSignalKind,
        query: MempoolSignalQuery,
    ) -> Result<MempoolSignalsResponse> {
        ensure_signal_events_table(&self.pool).await?;

        let limit = query
            .limit
            .unwrap_or(self.default_limit)
            .clamp(1, MAX_SIGNAL_LIMIT);
        let since_days = query
            .since_days
            .filter(|days| *days > 0)
            .map(|days| days.clamp(1, MAX_SIGNAL_LOOKBACK_DAYS));
        let sql = signal_sql(kind);
        let rows = sqlx::query(&sql)
            .bind(limit)
            .bind(since_days)
            .fetch_all(&self.pool)
            .await?;
        let signals = rows.iter().map(row_to_signal).collect::<Result<Vec<_>>>()?;

        Ok(MempoolSignalsResponse {
            signal_type: kind.as_str().to_string(),
            count: signals.len(),
            signals,
        })
    }
}

impl MempoolSignalKind {
    pub fn from_path(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "trading-enabled" | "trading_enabled" | "trading" => Some(Self::TradingEnabled),
            "honeypot"
            | "honeypots"
            | "honeypot-signal"
            | "honeypot_signal"
            | "sell-blocked"
            | "sell_blocked"
            | "sell-blocked-signal"
            | "sell_blocked_signal" => Some(Self::SellBlocked),
            "tax" | "tax-signals" | "tax_signals" | "tax-change" | "tax_change" => Some(Self::Tax),
            "liquidity-removals" | "liquidity_removals" | "liquidity-removal"
            | "liquidity_removal" | "liquidity" => Some(Self::LiquidityRemoval),
            "lp-approvals"
            | "lp_approvals"
            | "lp-approval"
            | "lp_approval"
            | "lp-position-approval"
            | "lp_position_approval" => Some(Self::LpPositionApproval),
            "token-supply-risk" | "token_supply_risk" | "supply-risk" | "supply_risk" => {
                Some(Self::TokenSupplyRisk)
            }
            "all" => Some(Self::All),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::TradingEnabled => "trading_enabled",
            Self::SellBlocked => "sell_blocked_signal",
            Self::Tax => "tax_signal",
            Self::LiquidityRemoval => "liquidity_removal",
            Self::LpPositionApproval => "lp_position_approval",
            Self::TokenSupplyRisk => "token_supply_risk",
        }
    }

    fn event_filter(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::TradingEnabled => Some("event_kind = 'trading_enabled'"),
            Self::SellBlocked => Some("event_kind = 'sell_blocked'"),
            Self::Tax => Some("event_kind = 'tax_change'"),
            Self::LiquidityRemoval => Some("event_kind = 'liquidity_removal'"),
            Self::LpPositionApproval => Some("event_kind = 'lp_position_approval'"),
            Self::TokenSupplyRisk => Some("event_kind = 'token_supply_risk'"),
        }
    }
}

fn signal_sql(kind: MempoolSignalKind) -> String {
    let event_filter = kind
        .event_filter()
        .map(|filter| format!("AND {filter}"))
        .unwrap_or_default();

    format!(
        r#"
        SELECT
            signal_id::text AS signal_id,
            public_signal_type AS signal_type,
            detection_timestamp::text AS detection_timestamp,
            pending_tx_hash AS detection_tx_hash,
            token_address,
            pool_identifier AS pool_address,
            pool_protocol AS pool_type,
            actor_address AS creator_address,
            subject_address,
            headline,
            value_1,
            value_2,
            flag,
            payload::text AS payload
        FROM live_trading.signal_events
        WHERE ($2::bigint IS NULL OR detection_timestamp >= (NOW() - ($2::bigint * INTERVAL '1 day')))
        {event_filter}
        ORDER BY detection_timestamp DESC
        LIMIT $1
        "#
    )
}

async fn ensure_signal_events_table(pool: &PgPool) -> Result<()> {
    for ddl in [
        r#"CREATE SCHEMA IF NOT EXISTS live_trading"#,
        r#"
        CREATE TABLE IF NOT EXISTS live_trading.signal_events (
            signal_id BIGSERIAL PRIMARY KEY,
            event_kind TEXT NOT NULL,
            public_signal_type TEXT NOT NULL,
            severity TEXT NOT NULL,
            token_address TEXT,
            pool_identifier TEXT,
            pool_protocol TEXT,
            denom_address TEXT,
            denom_currency TEXT,
            denom_decimals INTEGER,
            detection_timestamp TIMESTAMPTZ NOT NULL,
            pending_tx_hash TEXT,
            actor_address TEXT,
            subject_address TEXT,
            headline TEXT,
            value_1 TEXT,
            value_2 TEXT,
            flag TEXT,
            payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            dedupe_key TEXT NOT NULL,
            signal_source TEXT NOT NULL DEFAULT 'mempool',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS signal_events_dedupe_key_uidx
            ON live_trading.signal_events (dedupe_key)
        "#,
    ] {
        sqlx::query(ddl).execute(pool).await?;
    }

    Ok(())
}

fn row_to_signal(row: &sqlx::postgres::PgRow) -> Result<MempoolSignalView> {
    let payload_text = optional_text(row, "payload")?;
    let payload = payload_text
        .as_deref()
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or(Value::Null);

    Ok(MempoolSignalView {
        signal_id: text(row, "signal_id")?,
        signal_type: text(row, "signal_type")?,
        detection_timestamp: optional_text(row, "detection_timestamp")?,
        detection_tx_hash: optional_text(row, "detection_tx_hash")?,
        token_address: optional_text(row, "token_address")?,
        pool_address: optional_text(row, "pool_address")?,
        pool_type: optional_text(row, "pool_type")?,
        creator_address: optional_text(row, "creator_address")?,
        subject_address: optional_text(row, "subject_address")?,
        headline: optional_text(row, "headline")?,
        value_1: optional_text(row, "value_1")?,
        value_2: optional_text(row, "value_2")?,
        flag: optional_text(row, "flag")?,
        payload,
    })
}

fn text(row: &sqlx::postgres::PgRow, column: &str) -> Result<String> {
    row.try_get::<String, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

fn optional_text(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<String>> {
    row.try_get::<Option<String>, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{signal_sql, MempoolSignalKind};

    #[test]
    fn old_signal_filters_map_to_explicit_event_kinds() {
        assert_eq!(
            MempoolSignalKind::from_path("honeypot"),
            Some(MempoolSignalKind::SellBlocked)
        );
        assert_eq!(
            MempoolSignalKind::from_path("lp-approval"),
            Some(MempoolSignalKind::LpPositionApproval)
        );
        assert_eq!(
            MempoolSignalKind::from_path("token-supply-risk"),
            Some(MempoolSignalKind::TokenSupplyRisk)
        );
    }

    #[test]
    fn signal_api_reads_canonical_event_table() {
        let sql = signal_sql(MempoolSignalKind::SellBlocked);
        assert!(sql.contains("FROM live_trading.signal_events"));
        assert!(sql.contains("pool_identifier AS pool_address"));
        assert!(sql.contains("pool_protocol AS pool_type"));
        assert!(sql.contains("event_kind = 'sell_blocked'"));
    }
}
