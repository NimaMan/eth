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
    Honeypot,
    Tax,
    LiquidityRemoval,
    LpApproval,
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
            "honeypot" | "honeypots" | "honeypot-signal" | "honeypot_signal" | "sell-blocked"
            | "sell_blocked" => Some(Self::Honeypot),
            "tax" | "tax-signals" | "tax_signals" => Some(Self::Tax),
            "liquidity-removals" | "liquidity_removals" | "liquidity-removal"
            | "liquidity_removal" | "liquidity" => Some(Self::LiquidityRemoval),
            "lp-approvals" | "lp_approvals" | "lp-approval" | "lp_approval" => {
                Some(Self::LpApproval)
            }
            "all" => Some(Self::All),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::TradingEnabled => "trading_enabled",
            Self::Honeypot => "honeypot_signal",
            Self::Tax => "tax_signal",
            Self::LiquidityRemoval => "liquidity_removal",
            Self::LpApproval => "lp_approval",
        }
    }
}

fn signal_sql(kind: MempoolSignalKind) -> String {
    let body = match kind {
        MempoolSignalKind::All => format!(
            "{} UNION ALL {} UNION ALL {} UNION ALL {} UNION ALL {}",
            trading_enabled_select(),
            honeypot_select(),
            tax_select(),
            liquidity_removal_select(),
            lp_approval_select()
        ),
        MempoolSignalKind::TradingEnabled => trading_enabled_select().to_string(),
        MempoolSignalKind::Honeypot => honeypot_select().to_string(),
        MempoolSignalKind::Tax => tax_select().to_string(),
        MempoolSignalKind::LiquidityRemoval => liquidity_removal_select().to_string(),
        MempoolSignalKind::LpApproval => lp_approval_select().to_string(),
    };

    format!(
        "SELECT signal_id, signal_type, detection_timestamp, detection_tx_hash, \
         token_address, pool_address, pool_type, creator_address, subject_address, \
         headline, value_1, value_2, flag, payload \
         FROM ({body}) signals \
         WHERE ($2::bigint IS NULL OR sort_timestamp >= (NOW()::timestamp - ($2::bigint * INTERVAL '1 day'))) \
         ORDER BY sort_timestamp DESC LIMIT $1"
    )
}

fn trading_enabled_select() -> &'static str {
    r#"
    SELECT
        signal_id::text AS signal_id,
        'trading_enabled' AS signal_type,
        detection_timestamp::text AS detection_timestamp,
        detection_tx_hash,
        token_address,
        pool_address,
        pool_type,
        creator_address,
        creator_address AS subject_address,
        'Trading enabled' AS headline,
        buy_tax_at_signal::text AS value_1,
        sell_tax_at_signal::text AS value_2,
        NULL::text AS flag,
        jsonb_build_object(
            'buy_tax_at_signal', buy_tax_at_signal::text,
            'sell_tax_at_signal', sell_tax_at_signal::text,
            'price_ratio', price_ratio::text,
            'denom_reserve_at_signal', denom_reserve_at_signal::text,
            'token_reserve_at_signal', token_reserve_at_signal::text,
            'owner_address', owner_address,
            'signal_source', signal_source,
            'created_at', created_at::text
        )::text AS payload,
        detection_timestamp AS sort_timestamp
    FROM live_trading.trading_enabled_signals
    "#
}

fn honeypot_select() -> &'static str {
    r#"
    SELECT
        signal_id::text AS signal_id,
        'honeypot_signal' AS signal_type,
        detection_timestamp::text AS detection_timestamp,
        detection_tx_hash,
        token_address,
        pool_address,
        COALESCE(scam_details->>'pool_type', 'UNKNOWN') AS pool_type,
        scammer_address AS creator_address,
        scammer_address AS subject_address,
        'Legacy sell-blocked signal' AS headline,
        scam_details->>'buy_tax' AS value_1,
        scam_details->>'sell_tax' AS value_2,
        'true'::text AS flag,
        jsonb_build_object(
            'signal_type', scam_type,
            'signal_details', 'Pool can be bought but cannot be sold',
            'confidence', '0.95',
            'buy_tax_at_signal', scam_details->>'buy_tax',
            'sell_tax_at_signal', scam_details->>'sell_tax',
            'can_buy', scam_details->>'can_buy',
            'can_sell', scam_details->>'can_sell',
            'failure_reason', scam_details->>'failure_reason',
            'signal_source', signal_source,
            'created_at', created_at::text
        )::text AS payload,
        detection_timestamp AS sort_timestamp
    FROM live_trading.scam_signals
    WHERE scam_type IN ('honeypot', 'cant_sell')

    UNION ALL

    SELECT
        signal_id::text AS signal_id,
        'honeypot_signal' AS signal_type,
        detection_timestamp::text AS detection_timestamp,
        detection_tx_hash,
        token_address,
        pool_address,
        pool_type,
        creator_address,
        creator_address AS subject_address,
        'Honeypot / cannot sell' AS headline,
        buy_tax_at_signal::text AS value_1,
        sell_tax_at_signal::text AS value_2,
        cant_sell::text AS flag,
        jsonb_build_object(
            'signal_type', signal_type,
            'signal_details', signal_details,
            'confidence', confidence::text,
            'buy_tax_at_signal', buy_tax_at_signal::text,
            'sell_tax_at_signal', sell_tax_at_signal::text,
            'can_buy', NULL::boolean,
            'can_sell', false,
            'legacy_source', 'tax_signals',
            'signal_source', signal_source,
            'created_at', created_at::text
        )::text AS payload,
        detection_timestamp AS sort_timestamp
    FROM live_trading.tax_signals
    WHERE cant_sell
    "#
}

fn tax_select() -> &'static str {
    r#"
    SELECT
        signal_id::text AS signal_id,
        'tax_signal' AS signal_type,
        detection_timestamp::text AS detection_timestamp,
        detection_tx_hash,
        token_address,
        pool_address,
        pool_type,
        creator_address,
        creator_address AS subject_address,
        signal_type AS headline,
        buy_tax_at_signal::text AS value_1,
        sell_tax_at_signal::text AS value_2,
        combined_tax_bucket_to AS flag,
        jsonb_build_object(
            'signal_type', signal_type,
            'signal_details', signal_details,
            'confidence', confidence::text,
            'buy_tax_at_signal', buy_tax_at_signal::text,
            'sell_tax_at_signal', sell_tax_at_signal::text,
            'buy_tax_bucket_from', buy_tax_bucket_from,
            'buy_tax_bucket_to', buy_tax_bucket_to,
            'sell_tax_bucket_from', sell_tax_bucket_from,
            'sell_tax_bucket_to', sell_tax_bucket_to,
            'combined_tax_bucket_from', combined_tax_bucket_from,
            'combined_tax_bucket_to', combined_tax_bucket_to,
            'buy_tax_exceeds_threshold', buy_tax_exceeds_threshold,
            'sell_tax_exceeds_threshold', sell_tax_exceeds_threshold,
            'signal_source', signal_source,
            'created_at', created_at::text
        )::text AS payload,
        detection_timestamp AS sort_timestamp
    FROM live_trading.tax_signals
    WHERE NOT COALESCE(cant_sell, false)
    "#
}

fn liquidity_removal_select() -> &'static str {
    r#"
    SELECT
        signal_id::text AS signal_id,
        'liquidity_removal' AS signal_type,
        detection_timestamp::text AS detection_timestamp,
        detection_tx_hash,
        token_address,
        pool_address,
        pool_type,
        creator_address,
        creator_address AS subject_address,
        concat(pool_drain_risk_level, ' liquidity removal') AS headline,
        liquidity_removed_eth::text AS value_1,
        remaining_liquidity_eth::text AS value_2,
        pool_drain_risk_level AS flag,
        jsonb_build_object(
            'liquidity_removed_eth', liquidity_removed_eth::text,
            'liquidity_removed_token', liquidity_removed_token::text,
            'remaining_liquidity_eth', remaining_liquidity_eth::text,
            'remaining_liquidity_token', remaining_liquidity_token::text,
            'removal_percentage', removal_percentage::text,
            'pool_drain_risk_level', pool_drain_risk_level,
            'signal_source', signal_source,
            'created_at', created_at::text
        )::text AS payload,
        detection_timestamp AS sort_timestamp
    FROM live_trading.liquidity_removal_signals
    "#
}

fn lp_approval_select() -> &'static str {
    r#"
    SELECT
        signal_id::text AS signal_id,
        'lp_approval' AS signal_type,
        detection_timestamp::text AS detection_timestamp,
        detection_tx_hash,
        token_address,
        pool_address,
        pool_type,
        creator_address,
        approved_spender AS subject_address,
        CASE
            WHEN approval_percentage >= 99.99 THEN 'Full LP approval'
            ELSE 'LP approval ' || to_char(approval_percentage, 'FM999990.00') || '%'
        END AS headline,
        approval_percentage::text AS value_1,
        NULL::text AS value_2,
        COALESCE(approval_percentage >= 99.99, false)::text AS flag,
        jsonb_build_object(
            'approved_spender', approved_spender,
            'approval_percentage', approval_percentage,
            'is_full_approval', COALESCE(approval_percentage >= 99.99, false),
            'approval_type', approval_type,
            'signal_source', signal_source,
            'created_at', created_at::text
        )::text AS payload,
        detection_timestamp AS sort_timestamp
    FROM live_trading.lp_approval_signals
    WHERE approval_percentage IS NOT NULL
    "#
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
