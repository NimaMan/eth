use alloy_primitives::U256;
use chrono::{DateTime, Utc};
use eyre::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::types::BigDecimal;
use std::str::FromStr;
/// LP Approval Signal Database Writer
///
/// Non-blocking writer that records LP approval signals to the database.
/// Uses a background task with batched inserts to avoid blocking the main processing pipeline.
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use tracing::{error, info};

use crate::signal_detector::LpApprovalSignal;

/// LP approval signal record for database insertion
#[derive(Debug, Clone)]
pub struct LpApprovalSignalRecord {
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub denom_address: String,
    pub denom_currency: Option<String>,
    pub detection_timestamp: DateTime<Utc>,
    pub detection_tx_hash: String,
    pub approved_spender: String,
    /// Raw LP-token approval amount as a base-unit decimal string.
    pub approval_amount: Option<String>,
    /// Percentage (0-100) of LP tokens approved for the router, if known.
    pub approval_percentage: Option<f64>,
    pub is_unlimited_approval: bool,
    pub approval_type: String,
    pub previous_allowance: Option<f64>,
    pub creator_address: String,
    pub signal_source: String,
}

impl LpApprovalSignalRecord {
    /// Create from LpApprovalSignal
    pub fn from_signal(signal: &LpApprovalSignal) -> Self {
        let is_unlimited_amount = signal.amount == U256::MAX;
        let approval_pct = signal.approval_percentage.or_else(|| {
            if is_unlimited_amount {
                Some(100.0)
            } else {
                None
            }
        });
        let unlimited = approval_pct
            .map(|pct| pct >= 99.99)
            .unwrap_or(is_unlimited_amount);
        let approval_amount = if is_unlimited_amount {
            None
        } else {
            Some(signal.amount.to_string())
        };

        Self {
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            pool_type: normalize_pool_type(&signal.pool_type),
            denom_address: signal
                .denom_address
                .clone()
                .unwrap_or_else(|| "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string()),
            denom_currency: signal
                .denom_currency
                .clone()
                .or_else(|| Some("WETH".to_string())),
            detection_timestamp: Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            approved_spender: signal.spender_address.clone(),
            approval_amount,
            approval_percentage: approval_pct,
            is_unlimited_approval: unlimited,
            approval_type: "LP_TOKEN".to_string(),
            previous_allowance: signal.previous_allowance,
            creator_address: signal.approver_address.clone(),
            signal_source: "mempool".to_string(),
        }
    }
}

/// LP approval signal database writer
pub struct LpApprovalSignalWriter {
    sender: mpsc::UnboundedSender<LpApprovalSignalRecord>,
    _handle: tokio::task::JoinHandle<()>,
}

impl LpApprovalSignalWriter {
    /// Create new writer with database connection
    pub async fn new() -> Result<Self> {
        let database_url = "postgresql://postgres:postgres@localhost:5432/eth_db";
        Self::new_with_database_url(database_url).await
    }

    /// Create new writer with an explicit database connection URL.
    pub async fn new_with_database_url(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let (sender, receiver) = mpsc::unbounded_channel();
        let handle = tokio::spawn(writer_task(pool, receiver));

        Ok(Self {
            sender,
            _handle: handle,
        })
    }

    /// Write an LP approval signal
    pub fn write_signal(&self, signal: &LpApprovalSignal) -> Result<()> {
        let record = LpApprovalSignalRecord::from_signal(signal);
        self.sender
            .send(record)
            .map_err(|e| eyre::eyre!("Failed to send LP approval signal: {}", e))?;
        Ok(())
    }
}

/// Background writer task that batches and writes signals to database
async fn writer_task(pool: PgPool, mut receiver: mpsc::UnboundedReceiver<LpApprovalSignalRecord>) {
    let mut batch = Vec::new();
    let mut interval = interval(Duration::from_secs(1)); // Batch every 1 second
    let batch_size = 50;

    info!("🟢 LP approval signal writer started");

    loop {
        tokio::select! {
            // Receive new signals
            signal = receiver.recv() => {
                match signal {
                    Some(record) => {
                        batch.push(record);
                        if batch.len() >= batch_size {
                            if let Err(e) = write_batch(&pool, &mut batch).await {
                                error!("Failed to write LP approval signal batch: {}", e);
                            }
                        }
                    }
                    None => {
                        // Channel closed
                        info!("🔴 LP approval signal writer stopping");
                        break;
                    }
                }
            }

            // Periodic batch write
            _ = interval.tick() => {
                if !batch.is_empty() {
                    if let Err(e) = write_batch(&pool, &mut batch).await {
                        error!("Failed to write LP approval signal batch: {}", e);
                    }
                }
            }
        }
    }

    // Write any remaining signals
    if !batch.is_empty() {
        if let Err(e) = write_batch(&pool, &mut batch).await {
            error!("Failed to write final LP approval signal batch: {}", e);
        }
    }
}

/// Write a batch of signals to database
async fn write_batch(pool: &PgPool, batch: &mut Vec<LpApprovalSignalRecord>) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    let start = Instant::now();
    let mut transaction = pool.begin().await?;

    for record in batch.iter() {
        let approval_amount = record
            .approval_amount
            .as_deref()
            .and_then(|v| BigDecimal::from_str(v).ok());
        let previous_allowance = record
            .previous_allowance
            .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

        let detection_timestamp = record.detection_timestamp.naive_utc();

        let query = sqlx::query(
            r#"
            INSERT INTO live_trading.lp_approval_signals (
                token_address, pool_address, pool_type, denom_address, denom_currency,
                detection_timestamp, detection_tx_hash, approved_spender, approval_amount,
                is_unlimited_approval, approval_type, previous_allowance,
                creator_address, signal_source
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (pool_address, detection_tx_hash, approved_spender) DO NOTHING
            "#,
        )
        .bind(&record.token_address)
        .bind(&record.pool_address)
        .bind(&record.pool_type)
        .bind(&record.denom_address)
        .bind(record.denom_currency.as_ref())
        .bind(&detection_timestamp)
        .bind(&record.detection_tx_hash)
        .bind(&record.approved_spender)
        .bind(approval_amount.as_ref())
        .bind(&record.is_unlimited_approval)
        .bind(&record.approval_type)
        .bind(previous_allowance.as_ref())
        .bind(&record.creator_address)
        .bind(&record.signal_source);

        if let Err(e) = query.execute(&mut *transaction).await {
            error!("Failed to insert LP approval signal: {}", e);
        }
    }

    transaction.commit().await?;

    let duration = start.elapsed();
    info!(
        "📊 Wrote {} LP approval signals in {:.2}ms",
        batch.len(),
        duration.as_secs_f64() * 1000.0
    );

    batch.clear();
    Ok(())
}

fn normalize_pool_type(pool_type: &str) -> String {
    match pool_type.trim().to_lowercase().as_str() {
        "uniswapv2" | "uniswap-v2" | "v2" => "UNISWAP-V2".to_string(),
        "uniswapv3" | "uniswap-v3" | "v3" => "UNISWAP-V3".to_string(),
        "uniswapv4" | "uniswap-v4" | "v4" => "UNISWAP-V4".to_string(),
        "sushiswap" | "sushi" | "sushi-swap" => "SUSHI-SWAP".to_string(),
        other if other.is_empty() => "UNKNOWN".to_string(),
        other => other.to_uppercase(),
    }
}
