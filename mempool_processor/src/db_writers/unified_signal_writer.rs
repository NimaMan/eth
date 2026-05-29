use chrono::{DateTime, TimeZone, Utc};
use eyre::Result;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tracing::{debug, info};

use crate::signal_detector::{
    HoneypotSignal, LiquidityRemovalSignal, LpApprovalSignal, Signal, TaxSignalRecord,
    TokenSupplyRiskSignal, TradingEnabledSignal,
};

/// Canonical writer for public mempool trading signals.
///
/// All public signals are written to `live_trading.signal_events` for shared
/// query/API fields. Signal-specific analytics go into detail tables keyed by
/// `signal_id`. Unsupported or unmapped protocol events must stay out of this
/// writer and remain diagnostic telemetry.
pub struct UnifiedSignalWriter {
    pool: PgPool,
}

impl UnifiedSignalWriter {
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Initializing canonical signal_events writer");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        ensure_schema(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn write_signal(&self, signal: Signal) -> Result<()> {
        match signal {
            Signal::TradingEnabled(signal) => self.write_trading_enabled(&signal).await?,
            Signal::TaxSignal(signal) => self.write_tax_change(&signal).await?,
            Signal::Honeypot(signal) => self.write_sell_blocked(&signal).await?,
            Signal::LiquidityRemoval(signal) => self.write_liquidity_removal(&signal).await?,
            Signal::LpApproval(signal) => self.write_lp_position_approval(&signal).await?,
            Signal::TokenSupplyRisk(signal) => self.write_token_supply_risk(&signal).await?,
        }

        Ok(())
    }

    pub fn has_writers(&self) -> bool {
        true
    }

    pub fn get_status(&self) -> String {
        "Canonical signal_events writer: ✓".to_string()
    }

    async fn write_trading_enabled(&self, signal: &TradingEnabledSignal) -> Result<()> {
        let protocol = normalize_pool_protocol(&signal.pool_type);
        let payload = json!({
            "buy_tax_at_signal": signal.buy_tax,
            "sell_tax_at_signal": signal.sell_tax,
            "creator_address": signal.creator_address,
            "denom_address": signal.denom_address,
            "denom_currency": signal.denom_currency,
            "denom_decimals": signal.denom_decimals,
        });
        let event = SignalEventInsert {
            event_kind: "trading_enabled",
            public_signal_type: "trading_enabled",
            severity: "info",
            token_address: Some(signal.token_address.clone()),
            pool_identifier: Some(signal.pool_address.clone()),
            pool_protocol: Some(protocol.clone()),
            denom_address: signal.denom_address.clone(),
            denom_currency: signal.denom_currency.clone(),
            denom_decimals: signal.denom_decimals,
            detection_timestamp: timestamp_from_unix(signal.timestamp),
            detected_at_head_block_number: signal.detected_at_head_block_number,
            detected_at_head_block_hash: signal.detected_at_head_block_hash.clone(),
            pending_tx_hash: Some(signal.tx_hash.clone()),
            actor_address: Some(signal.creator_address.clone()),
            subject_address: Some(signal.creator_address.clone()),
            headline: Some("Trading enabled".to_string()),
            value_1: Some(format_percent(signal.buy_tax)),
            value_2: Some(format_percent(signal.sell_tax)),
            flag: None,
            payload,
            dedupe_key: format!("trading_enabled:{}:{}", signal.pool_address, signal.tx_hash),
        };
        let signal_id = insert_event(&self.pool, event).await?;

        sqlx::query(
            r#"
            INSERT INTO live_trading.trading_enabled_details
                (signal_id, buy_tax_at_signal, sell_tax_at_signal)
            VALUES ($1, $2, $3)
            ON CONFLICT (signal_id) DO UPDATE SET
                buy_tax_at_signal = EXCLUDED.buy_tax_at_signal,
                sell_tax_at_signal = EXCLUDED.sell_tax_at_signal
            "#,
        )
        .bind(signal_id)
        .bind(signal.buy_tax)
        .bind(signal.sell_tax)
        .execute(&self.pool)
        .await?;

        if let Some(evidence) = signal.mempool_entry_evidence.as_ref() {
            write_signal_entry_evidence(&self.pool, signal_id, evidence).await?;
        }

        debug!("wrote trading_enabled signal_id={signal_id}");
        Ok(())
    }

    async fn write_tax_change(&self, signal: &TaxSignalRecord) -> Result<()> {
        let protocol = normalize_pool_protocol(&signal.pool_type);
        let severity = if signal.cant_sell || signal.sell_tax_exceeds_threshold {
            "critical"
        } else if signal.buy_tax_exceeds_threshold {
            "warning"
        } else {
            "info"
        };
        let flag = signal
            .combined_tax_bucket_to
            .clone()
            .or_else(|| signal.sell_tax_bucket_to.clone())
            .or_else(|| signal.buy_tax_bucket_to.clone());
        let payload = json!({
            "signal_type": signal.signal_type,
            "signal_details": signal.signal_details,
            "confidence": signal.confidence,
            "buy_tax": signal.buy_tax,
            "sell_tax": signal.sell_tax,
            "buy_tax_bucket_from": signal.buy_tax_bucket_from,
            "buy_tax_bucket_to": signal.buy_tax_bucket_to,
            "sell_tax_bucket_from": signal.sell_tax_bucket_from,
            "sell_tax_bucket_to": signal.sell_tax_bucket_to,
            "combined_tax_bucket_from": signal.combined_tax_bucket_from,
            "combined_tax_bucket_to": signal.combined_tax_bucket_to,
            "buy_tax_exceeds_threshold": signal.buy_tax_exceeds_threshold,
            "sell_tax_exceeds_threshold": signal.sell_tax_exceeds_threshold,
            "cant_sell": signal.cant_sell,
            "denom_address": signal.denom_address,
            "denom_currency": signal.denom_currency,
            "denom_decimals": signal.denom_decimals,
        });
        let event = SignalEventInsert {
            event_kind: "tax_change",
            public_signal_type: "tax_signal",
            severity,
            token_address: Some(signal.token_address.clone()),
            pool_identifier: Some(signal.pool_address.clone()),
            pool_protocol: Some(protocol.clone()),
            denom_address: signal.denom_address.clone(),
            denom_currency: signal.denom_currency.clone(),
            denom_decimals: signal.denom_decimals,
            detection_timestamp: timestamp_from_unix(signal.timestamp),
            detected_at_head_block_number: signal.detected_at_head_block_number,
            detected_at_head_block_hash: signal.detected_at_head_block_hash.clone(),
            pending_tx_hash: Some(signal.tx_hash.clone()),
            actor_address: Some(signal.creator_address.clone()),
            subject_address: Some(signal.creator_address.clone()),
            headline: Some(signal.signal_type.clone()),
            value_1: signal.buy_tax.map(format_percent),
            value_2: signal.sell_tax.map(format_percent),
            flag: flag.clone(),
            payload,
            dedupe_key: format!(
                "tax_change:{}:{}:{}:{}",
                signal.pool_address,
                signal.tx_hash,
                signal.signal_type,
                flag.as_deref().unwrap_or("none")
            ),
        };
        let signal_id = insert_event(&self.pool, event).await?;

        sqlx::query(
            r#"
            INSERT INTO live_trading.tax_change_details
                (
                    signal_id, signal_type, signal_details, confidence,
                    buy_tax_at_signal, sell_tax_at_signal,
                    buy_tax_bucket_from, buy_tax_bucket_to,
                    sell_tax_bucket_from, sell_tax_bucket_to,
                    combined_tax_bucket_from, combined_tax_bucket_to,
                    buy_tax_exceeds_threshold, sell_tax_exceeds_threshold, cant_sell
                )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (signal_id) DO UPDATE SET
                signal_type = EXCLUDED.signal_type,
                signal_details = EXCLUDED.signal_details,
                confidence = EXCLUDED.confidence,
                buy_tax_at_signal = EXCLUDED.buy_tax_at_signal,
                sell_tax_at_signal = EXCLUDED.sell_tax_at_signal,
                buy_tax_bucket_from = EXCLUDED.buy_tax_bucket_from,
                buy_tax_bucket_to = EXCLUDED.buy_tax_bucket_to,
                sell_tax_bucket_from = EXCLUDED.sell_tax_bucket_from,
                sell_tax_bucket_to = EXCLUDED.sell_tax_bucket_to,
                combined_tax_bucket_from = EXCLUDED.combined_tax_bucket_from,
                combined_tax_bucket_to = EXCLUDED.combined_tax_bucket_to,
                buy_tax_exceeds_threshold = EXCLUDED.buy_tax_exceeds_threshold,
                sell_tax_exceeds_threshold = EXCLUDED.sell_tax_exceeds_threshold,
                cant_sell = EXCLUDED.cant_sell
            "#,
        )
        .bind(signal_id)
        .bind(&signal.signal_type)
        .bind(&signal.signal_details)
        .bind(signal.confidence)
        .bind(signal.buy_tax)
        .bind(signal.sell_tax)
        .bind(&signal.buy_tax_bucket_from)
        .bind(&signal.buy_tax_bucket_to)
        .bind(&signal.sell_tax_bucket_from)
        .bind(&signal.sell_tax_bucket_to)
        .bind(&signal.combined_tax_bucket_from)
        .bind(&signal.combined_tax_bucket_to)
        .bind(signal.buy_tax_exceeds_threshold)
        .bind(signal.sell_tax_exceeds_threshold)
        .bind(signal.cant_sell)
        .execute(&self.pool)
        .await?;

        debug!("wrote tax_change signal_id={signal_id}");
        Ok(())
    }

    async fn write_sell_blocked(&self, signal: &HoneypotSignal) -> Result<()> {
        let protocol = normalize_pool_protocol(&signal.pool_type);
        let payload = json!({
            "can_buy": signal.can_buy,
            "can_sell": signal.can_sell,
            "buy_tax": signal.buy_tax,
            "sell_tax": signal.sell_tax,
            "failure_reason": signal.failure_reason,
            "confidence": signal.confidence,
            "denom_address": signal.denom_address,
            "denom_currency": signal.denom_currency,
            "denom_decimals": signal.denom_decimals,
        });
        let event = SignalEventInsert {
            event_kind: "sell_blocked",
            public_signal_type: "sell_blocked_signal",
            severity: "critical",
            token_address: Some(signal.token_address.clone()),
            pool_identifier: Some(signal.pool_address.clone()),
            pool_protocol: Some(protocol.clone()),
            denom_address: signal.denom_address.clone(),
            denom_currency: signal.denom_currency.clone(),
            denom_decimals: signal.denom_decimals,
            detection_timestamp: timestamp_from_unix(signal.timestamp),
            detected_at_head_block_number: signal.detected_at_head_block_number,
            detected_at_head_block_hash: signal.detected_at_head_block_hash.clone(),
            pending_tx_hash: Some(signal.tx_hash.clone()),
            actor_address: Some(signal.creator_address.clone()),
            subject_address: Some(signal.creator_address.clone()),
            headline: Some("Sell blocked".to_string()),
            value_1: signal.buy_tax.map(format_percent),
            value_2: signal.sell_tax.map(format_percent),
            flag: Some("true".to_string()),
            payload,
            dedupe_key: format!("sell_blocked:{}:{}", signal.pool_address, signal.tx_hash),
        };
        let signal_id = insert_event(&self.pool, event).await?;

        sqlx::query(
            r#"
            INSERT INTO live_trading.sell_blocked_details
                (signal_id, can_buy, can_sell, buy_tax_at_signal, sell_tax_at_signal, failure_reason, confidence)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (signal_id) DO UPDATE SET
                can_buy = EXCLUDED.can_buy,
                can_sell = EXCLUDED.can_sell,
                buy_tax_at_signal = EXCLUDED.buy_tax_at_signal,
                sell_tax_at_signal = EXCLUDED.sell_tax_at_signal,
                failure_reason = EXCLUDED.failure_reason,
                confidence = EXCLUDED.confidence
            "#,
        )
        .bind(signal_id)
        .bind(signal.can_buy)
        .bind(signal.can_sell)
        .bind(signal.buy_tax)
        .bind(signal.sell_tax)
        .bind(&signal.failure_reason)
        .bind(signal.confidence)
        .execute(&self.pool)
        .await?;

        debug!("wrote sell_blocked signal_id={signal_id}");
        Ok(())
    }

    async fn write_liquidity_removal(&self, signal: &LiquidityRemovalSignal) -> Result<()> {
        let protocol = normalize_pool_protocol(&signal.pool_type);
        let severity = match signal.removal_percentage {
            Some(pct) if pct >= 50.0 => "critical",
            Some(pct) if pct >= 20.0 => "warning",
            Some(_) => "info",
            None => "unknown",
        };
        let risk_label = removal_risk_label(signal.removal_percentage);
        let payload = json!({
            "function_name": signal.function_name,
            "removed_denom_amount": signal.estimated_eth_removed,
            "remaining_denom_amount": signal.remaining_eth,
            "removal_percentage": signal.removal_percentage,
            "denom_address": signal.denom_address,
            "denom_currency": signal.denom_currency,
            "denom_decimals": signal.denom_decimals,
        });
        let event = SignalEventInsert {
            event_kind: "liquidity_removal",
            public_signal_type: "liquidity_removal",
            severity,
            token_address: signal.token_address.clone(),
            pool_identifier: Some(signal.pool_address.clone()),
            pool_protocol: Some(protocol.clone()),
            denom_address: signal.denom_address.clone(),
            denom_currency: signal.denom_currency.clone(),
            denom_decimals: signal.denom_decimals,
            detection_timestamp: timestamp_from_unix(signal.timestamp),
            detected_at_head_block_number: signal.detected_at_head_block_number,
            detected_at_head_block_hash: signal.detected_at_head_block_hash.clone(),
            pending_tx_hash: Some(signal.tx_hash.clone()),
            actor_address: Some(signal.remover_address.clone()),
            subject_address: Some(signal.remover_address.clone()),
            headline: Some("Liquidity removal".to_string()),
            value_1: signal.estimated_eth_removed.map(format_amount),
            value_2: signal.remaining_eth.map(format_amount),
            flag: Some(risk_label.to_string()),
            payload,
            dedupe_key: format!(
                "liquidity_removal:{}:{}",
                signal.pool_address, signal.tx_hash
            ),
        };
        let signal_id = insert_event(&self.pool, event).await?;

        sqlx::query(
            r#"
            INSERT INTO live_trading.liquidity_removal_details
                (
                    signal_id, function_name, remover_address,
                    removed_denom_amount, remaining_denom_amount,
                    removal_percentage, pool_drain_risk_level
                )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (signal_id) DO UPDATE SET
                function_name = EXCLUDED.function_name,
                remover_address = EXCLUDED.remover_address,
                removed_denom_amount = EXCLUDED.removed_denom_amount,
                remaining_denom_amount = EXCLUDED.remaining_denom_amount,
                removal_percentage = EXCLUDED.removal_percentage,
                pool_drain_risk_level = EXCLUDED.pool_drain_risk_level
            "#,
        )
        .bind(signal_id)
        .bind(&signal.function_name)
        .bind(&signal.remover_address)
        .bind(signal.estimated_eth_removed)
        .bind(signal.remaining_eth)
        .bind(signal.removal_percentage)
        .bind(risk_label)
        .execute(&self.pool)
        .await?;

        debug!("wrote liquidity_removal signal_id={signal_id}");
        Ok(())
    }

    async fn write_lp_position_approval(&self, signal: &LpApprovalSignal) -> Result<()> {
        let approved_share_pct = signal
            .approved_share_pct
            .or(signal.approval_percentage)
            .filter(|pct| pct.is_finite());
        let protocol = normalize_pool_protocol(&signal.pool_type);
        let approval_model = signal
            .approval_model
            .clone()
            .unwrap_or_else(|| infer_approval_model(&protocol).to_string());
        let full_approval = approved_share_pct.map(|pct| pct >= 99.99).unwrap_or(false);
        let payload = json!({
            "lp_token_address": signal.lp_token_address,
            "approved_spender": signal.spender_address,
            "router_address": signal.router_address,
            "approval_amount": signal.amount.to_string(),
            "approval_percentage": signal.approval_percentage,
            "approved_share_pct": approved_share_pct,
            "approval_model": approval_model,
            "lp_total_supply": signal.lp_total_supply,
            "position_manager": signal.position_manager,
            "position_id": signal.position_id,
            "position_liquidity": signal.position_liquidity,
            "pool_liquidity": signal.pool_liquidity,
            "position_share_pct": signal.position_share_pct,
            "denom_address": signal.denom_address,
            "denom_currency": signal.denom_currency,
            "denom_decimals": signal.denom_decimals,
        });
        let event = SignalEventInsert {
            event_kind: "lp_position_approval",
            public_signal_type: "lp_position_approval",
            severity: if full_approval { "critical" } else { "warning" },
            token_address: Some(signal.token_address.clone()),
            pool_identifier: Some(signal.pool_address.clone()),
            pool_protocol: Some(protocol.clone()),
            denom_address: signal.denom_address.clone(),
            denom_currency: signal.denom_currency.clone(),
            denom_decimals: signal.denom_decimals,
            detection_timestamp: timestamp_from_unix_i64(signal.timestamp),
            detected_at_head_block_number: signal.detected_at_head_block_number,
            detected_at_head_block_hash: signal.detected_at_head_block_hash.clone(),
            pending_tx_hash: Some(signal.tx_hash.clone()),
            actor_address: Some(signal.approver_address.clone()),
            subject_address: Some(signal.spender_address.clone()),
            headline: Some("LP position approval".to_string()),
            value_1: approved_share_pct.map(format_percent),
            value_2: Some(signal.amount.to_string()),
            flag: Some(if full_approval { "true" } else { "false" }.to_string()),
            payload,
            dedupe_key: format!(
                "lp_position_approval:{}:{}:{}",
                signal.pool_address, signal.tx_hash, signal.spender_address
            ),
        };
        let signal_id = insert_event(&self.pool, event).await?;

        sqlx::query(
            r#"
            INSERT INTO live_trading.lp_position_approval_details
                (
                    signal_id, approval_model, lp_token_address, approved_spender,
                    approver_address, approval_amount_raw, lp_total_supply_raw,
                    approved_share_pct, position_manager, position_id,
                    position_liquidity_raw, pool_liquidity_raw, position_share_pct,
                    full_approval
                )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (signal_id) DO UPDATE SET
                approval_model = EXCLUDED.approval_model,
                lp_token_address = EXCLUDED.lp_token_address,
                approved_spender = EXCLUDED.approved_spender,
                approver_address = EXCLUDED.approver_address,
                approval_amount_raw = EXCLUDED.approval_amount_raw,
                lp_total_supply_raw = EXCLUDED.lp_total_supply_raw,
                approved_share_pct = EXCLUDED.approved_share_pct,
                position_manager = EXCLUDED.position_manager,
                position_id = EXCLUDED.position_id,
                position_liquidity_raw = EXCLUDED.position_liquidity_raw,
                pool_liquidity_raw = EXCLUDED.pool_liquidity_raw,
                position_share_pct = EXCLUDED.position_share_pct,
                full_approval = EXCLUDED.full_approval
            "#,
        )
        .bind(signal_id)
        .bind(&approval_model)
        .bind(&signal.lp_token_address)
        .bind(&signal.spender_address)
        .bind(&signal.approver_address)
        .bind(signal.amount.to_string())
        .bind(&signal.lp_total_supply)
        .bind(approved_share_pct)
        .bind(&signal.position_manager)
        .bind(&signal.position_id)
        .bind(&signal.position_liquidity)
        .bind(&signal.pool_liquidity)
        .bind(signal.position_share_pct)
        .bind(full_approval)
        .execute(&self.pool)
        .await?;

        debug!("wrote lp_position_approval signal_id={signal_id}");
        Ok(())
    }

    async fn write_token_supply_risk(&self, signal: &TokenSupplyRiskSignal) -> Result<()> {
        let payload = json!({
            "risk_type": signal.risk_type,
            "risk_details": signal.risk_details,
            "actor_address": signal.actor_address,
            "block_number": signal.block_number,
            "confidence": signal.confidence,
        });
        let event = SignalEventInsert {
            event_kind: "token_supply_risk",
            public_signal_type: "token_supply_risk",
            severity: "critical",
            token_address: Some(signal.token_address.clone()),
            pool_identifier: None,
            pool_protocol: None,
            denom_address: None,
            denom_currency: None,
            denom_decimals: None,
            detection_timestamp: timestamp_from_unix(signal.timestamp),
            detected_at_head_block_number: signal.detected_at_head_block_number,
            detected_at_head_block_hash: signal.detected_at_head_block_hash.clone(),
            pending_tx_hash: signal.tx_hash.clone(),
            actor_address: signal.actor_address.clone(),
            subject_address: signal.actor_address.clone(),
            headline: Some(signal.risk_type.clone()),
            value_1: signal.block_number.map(|block| block.to_string()),
            value_2: None,
            flag: Some("supply_risk".to_string()),
            payload,
            dedupe_key: format!(
                "token_supply_risk:{}:{}:{}",
                signal.token_address,
                signal.tx_hash.as_deref().unwrap_or("no_tx"),
                signal.risk_type
            ),
        };
        let signal_id = insert_event(&self.pool, event).await?;

        sqlx::query(
            r#"
            INSERT INTO live_trading.token_supply_risk_details
                (signal_id, risk_type, risk_details, actor_address, block_number, confidence)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (signal_id) DO UPDATE SET
                risk_type = EXCLUDED.risk_type,
                risk_details = EXCLUDED.risk_details,
                actor_address = EXCLUDED.actor_address,
                block_number = EXCLUDED.block_number,
                confidence = EXCLUDED.confidence
            "#,
        )
        .bind(signal_id)
        .bind(&signal.risk_type)
        .bind(&signal.risk_details)
        .bind(&signal.actor_address)
        .bind(signal.block_number.map(|value| value as i64))
        .bind(signal.confidence)
        .execute(&self.pool)
        .await?;

        debug!("wrote token_supply_risk signal_id={signal_id}");
        Ok(())
    }
}

struct SignalEventInsert {
    event_kind: &'static str,
    public_signal_type: &'static str,
    severity: &'static str,
    token_address: Option<String>,
    pool_identifier: Option<String>,
    pool_protocol: Option<String>,
    denom_address: Option<String>,
    denom_currency: Option<String>,
    denom_decimals: Option<u8>,
    detection_timestamp: DateTime<Utc>,
    detected_at_head_block_number: Option<u64>,
    detected_at_head_block_hash: Option<String>,
    pending_tx_hash: Option<String>,
    actor_address: Option<String>,
    subject_address: Option<String>,
    headline: Option<String>,
    value_1: Option<String>,
    value_2: Option<String>,
    flag: Option<String>,
    payload: Value,
    dedupe_key: String,
}

async fn insert_event(pool: &PgPool, event: SignalEventInsert) -> Result<i64> {
    let row = sqlx::query(
        r#"
        INSERT INTO live_trading.signal_events
            (
                event_kind, public_signal_type, severity,
                token_address, pool_identifier, pool_protocol,
                denom_address, denom_currency, denom_decimals,
                detection_timestamp, detected_at_head_block_number,
                detected_at_head_block_hash, pending_tx_hash,
                actor_address, subject_address, headline,
                value_1, value_2, flag, payload, dedupe_key
            )
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9,
             $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
        ON CONFLICT (dedupe_key) DO UPDATE SET
            public_signal_type = EXCLUDED.public_signal_type,
            severity = EXCLUDED.severity,
            pool_protocol = EXCLUDED.pool_protocol,
            denom_address = EXCLUDED.denom_address,
            denom_currency = EXCLUDED.denom_currency,
            denom_decimals = EXCLUDED.denom_decimals,
            detection_timestamp = EXCLUDED.detection_timestamp,
            detected_at_head_block_number = EXCLUDED.detected_at_head_block_number,
            detected_at_head_block_hash = EXCLUDED.detected_at_head_block_hash,
            pending_tx_hash = EXCLUDED.pending_tx_hash,
            actor_address = EXCLUDED.actor_address,
            subject_address = EXCLUDED.subject_address,
            headline = EXCLUDED.headline,
            value_1 = EXCLUDED.value_1,
            value_2 = EXCLUDED.value_2,
            flag = EXCLUDED.flag,
            payload = EXCLUDED.payload
        RETURNING signal_id
        "#,
    )
    .bind(event.event_kind)
    .bind(event.public_signal_type)
    .bind(event.severity)
    .bind(event.token_address)
    .bind(event.pool_identifier)
    .bind(event.pool_protocol)
    .bind(event.denom_address)
    .bind(event.denom_currency)
    .bind(event.denom_decimals.map(i32::from))
    .bind(event.detection_timestamp)
    .bind(
        event
            .detected_at_head_block_number
            .map(|value| value as i64),
    )
    .bind(event.detected_at_head_block_hash)
    .bind(event.pending_tx_hash)
    .bind(event.actor_address)
    .bind(event.subject_address)
    .bind(event.headline)
    .bind(event.value_1)
    .bind(event.value_2)
    .bind(event.flag)
    .bind(event.payload)
    .bind(event.dedupe_key)
    .fetch_one(pool)
    .await?;

    Ok(row.try_get::<i64, _>("signal_id")?)
}

async fn write_signal_entry_evidence(
    pool: &PgPool,
    signal_id: i64,
    evidence: &Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO live_trading.signal_entry_evidence
            (signal_id, evidence)
        VALUES ($1, $2)
        ON CONFLICT (signal_id) DO UPDATE SET
            evidence = EXCLUDED.evidence,
            created_at = NOW()
        "#,
    )
    .bind(signal_id)
    .bind(evidence)
    .execute(pool)
    .await?;

    Ok(())
}

async fn ensure_schema(pool: &PgPool) -> Result<()> {
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
            detected_at_head_block_number BIGINT,
            detected_at_head_block_hash TEXT,
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
        ALTER TABLE live_trading.signal_events
            ADD COLUMN IF NOT EXISTS detected_at_head_block_number BIGINT
        "#,
        r#"
        ALTER TABLE live_trading.signal_events
            ADD COLUMN IF NOT EXISTS detected_at_head_block_hash TEXT
        "#,
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS signal_events_dedupe_key_uidx
            ON live_trading.signal_events (dedupe_key)
        "#,
        r#"
        CREATE INDEX IF NOT EXISTS signal_events_kind_ts_idx
            ON live_trading.signal_events (event_kind, detection_timestamp DESC)
        "#,
        r#"
        CREATE INDEX IF NOT EXISTS signal_events_token_ts_idx
            ON live_trading.signal_events (token_address, detection_timestamp DESC)
        "#,
        r#"
        CREATE INDEX IF NOT EXISTS signal_events_pool_ts_idx
            ON live_trading.signal_events (pool_identifier, detection_timestamp DESC)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS live_trading.signal_entry_evidence (
            signal_id BIGINT PRIMARY KEY REFERENCES live_trading.signal_events(signal_id) ON DELETE CASCADE,
            evidence JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS live_trading.trading_enabled_details (
            signal_id BIGINT PRIMARY KEY REFERENCES live_trading.signal_events(signal_id) ON DELETE CASCADE,
            buy_tax_at_signal DOUBLE PRECISION,
            sell_tax_at_signal DOUBLE PRECISION
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS live_trading.sell_blocked_details (
            signal_id BIGINT PRIMARY KEY REFERENCES live_trading.signal_events(signal_id) ON DELETE CASCADE,
            can_buy BOOLEAN NOT NULL,
            can_sell BOOLEAN NOT NULL,
            buy_tax_at_signal DOUBLE PRECISION,
            sell_tax_at_signal DOUBLE PRECISION,
            failure_reason TEXT,
            confidence DOUBLE PRECISION
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS live_trading.tax_change_details (
            signal_id BIGINT PRIMARY KEY REFERENCES live_trading.signal_events(signal_id) ON DELETE CASCADE,
            signal_type TEXT NOT NULL,
            signal_details TEXT NOT NULL,
            confidence DOUBLE PRECISION,
            buy_tax_at_signal DOUBLE PRECISION,
            sell_tax_at_signal DOUBLE PRECISION,
            buy_tax_bucket_from TEXT,
            buy_tax_bucket_to TEXT,
            sell_tax_bucket_from TEXT,
            sell_tax_bucket_to TEXT,
            combined_tax_bucket_from TEXT,
            combined_tax_bucket_to TEXT,
            buy_tax_exceeds_threshold BOOLEAN NOT NULL DEFAULT FALSE,
            sell_tax_exceeds_threshold BOOLEAN NOT NULL DEFAULT FALSE,
            cant_sell BOOLEAN NOT NULL DEFAULT FALSE
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS live_trading.liquidity_removal_details (
            signal_id BIGINT PRIMARY KEY REFERENCES live_trading.signal_events(signal_id) ON DELETE CASCADE,
            function_name TEXT NOT NULL,
            remover_address TEXT NOT NULL,
            removed_denom_amount DOUBLE PRECISION,
            remaining_denom_amount DOUBLE PRECISION,
            removal_percentage DOUBLE PRECISION,
            pool_drain_risk_level TEXT
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS live_trading.lp_position_approval_details (
            signal_id BIGINT PRIMARY KEY REFERENCES live_trading.signal_events(signal_id) ON DELETE CASCADE,
            approval_model TEXT NOT NULL,
            lp_token_address TEXT,
            approved_spender TEXT,
            approver_address TEXT,
            approval_amount_raw TEXT,
            lp_total_supply_raw TEXT,
            approved_share_pct DOUBLE PRECISION,
            position_manager TEXT,
            position_id TEXT,
            position_liquidity_raw TEXT,
            pool_liquidity_raw TEXT,
            position_share_pct DOUBLE PRECISION,
            full_approval BOOLEAN NOT NULL DEFAULT FALSE
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS live_trading.token_supply_risk_details (
            signal_id BIGINT PRIMARY KEY REFERENCES live_trading.signal_events(signal_id) ON DELETE CASCADE,
            risk_type TEXT NOT NULL,
            risk_details TEXT NOT NULL,
            actor_address TEXT,
            block_number BIGINT,
            confidence DOUBLE PRECISION
        )
        "#,
    ] {
        sqlx::query(ddl).execute(pool).await?;
    }

    Ok(())
}

fn normalize_pool_protocol(pool_type: &str) -> String {
    match pool_type.trim().to_ascii_uppercase().as_str() {
        "UNISWAPV2" | "UNISWAP-V2" | "V2" => "UNISWAP-V2",
        "UNISWAPV3" | "UNISWAP-V3" | "V3" => "UNISWAP-V3",
        "UNISWAPV4" | "UNISWAP-V4" | "V4" => "UNISWAP-V4",
        "SUSHISWAP" | "SUSHISWAP-V2" | "SUSHI" | "SUSHI-V2" => "SUSHISWAP-V2",
        "SUSHISWAP-V3" | "SUSHI-V3" => "SUSHISWAP-V3",
        "PANCAKESWAP" | "PANCAKESWAP-V2" | "PANCAKE-V2" => "PANCAKESWAP-V2",
        "PANCAKESWAP-V3" | "PANCAKE-V3" => "PANCAKESWAP-V3",
        "SHIBASWAP" | "SHIBASWAP-V2" => "SHIBASWAP-V2",
        "FRAXSWAP" | "FRAXSWAP-V2" => "FRAXSWAP-V2",
        "CURVE" | "CURVE-V1" => "CURVE",
        "BALANCER" | "BALANCER-V2" => "BALANCER",
        "" => "UNKNOWN",
        other => other,
    }
    .to_string()
}

fn infer_approval_model(pool_protocol: &str) -> &'static str {
    match pool_protocol {
        "UNISWAP-V3" | "SUSHISWAP-V3" | "PANCAKESWAP-V3" | "UNISWAP-V4" => {
            "position_liquidity_share"
        }
        "BALANCER" => "bpt_share",
        "CURVE" => "curve_lp_share",
        _ => "erc20_lp_share",
    }
}

fn removal_risk_label(removal_percentage: Option<f64>) -> &'static str {
    match removal_percentage {
        Some(pct) if pct >= 90.0 => "draining",
        Some(pct) if pct >= 50.0 => "major",
        Some(pct) if pct >= 20.0 => "elevated",
        Some(_) => "observed",
        None => "unknown",
    }
}

fn timestamp_from_unix(timestamp: u64) -> DateTime<Utc> {
    timestamp_from_unix_i64(timestamp as i64)
}

fn timestamp_from_unix_i64(timestamp: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Utc::now)
}

fn format_percent(value: f64) -> String {
    format!("{value:.4}")
}

fn format_amount(value: f64) -> String {
    format!("{value:.18}")
}

#[cfg(test)]
mod tests {
    use super::{infer_approval_model, normalize_pool_protocol, removal_risk_label};

    #[test]
    fn canonicalizes_protocol_labels() {
        let cases = [
            ("v2", "UNISWAP-V2"),
            ("sushi", "SUSHISWAP-V2"),
            ("PANCAKE-V2", "PANCAKESWAP-V2"),
            ("pancakeswap-v3", "PANCAKESWAP-V3"),
            ("curve-v1", "CURVE"),
            ("balancer-v2", "BALANCER"),
        ];

        for (input, expected) in cases {
            assert_eq!(normalize_pool_protocol(input), expected);
        }
    }

    #[test]
    fn approval_model_follows_protocol_liquidity_ownership() {
        assert_eq!(infer_approval_model("UNISWAP-V2"), "erc20_lp_share");
        assert_eq!(
            infer_approval_model("SUSHISWAP-V3"),
            "position_liquidity_share"
        );
        assert_eq!(infer_approval_model("BALANCER"), "bpt_share");
        assert_eq!(infer_approval_model("CURVE"), "curve_lp_share");
    }

    #[test]
    fn liquidity_removal_risk_labels_are_explicit() {
        assert_eq!(removal_risk_label(None), "unknown");
        assert_eq!(removal_risk_label(Some(10.0)), "observed");
        assert_eq!(removal_risk_label(Some(30.0)), "elevated");
        assert_eq!(removal_risk_label(Some(60.0)), "major");
        assert_eq!(removal_risk_label(Some(95.0)), "draining");
    }
}
