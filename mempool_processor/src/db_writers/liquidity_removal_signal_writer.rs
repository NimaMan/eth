use chrono::{DateTime, Utc};
use eyre::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::types::BigDecimal;
use std::str::FromStr;
/// Liquidity Removal Signal Database Writer
///
/// Non-blocking writer that records liquidity removal signals to the database.
/// Uses a background task with batched inserts to avoid blocking the main processing pipeline.
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use tracing::{error, info};

use crate::signal_detector::LiquiditySignal;

/// Liquidity removal signal record for database insertion
#[derive(Debug, Clone)]
pub struct LiquidityRemovalSignalRecord {
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub denom_address: String,
    pub denom_currency: Option<String>,
    pub detection_timestamp: DateTime<Utc>,
    pub detection_tx_hash: String,
    pub liquidity_removed_denom: Option<f64>,
    pub remaining_liquidity_denom: Option<f64>,
    pub removal_percentage: Option<f64>,
    pub pool_drain_risk_level: String,
    pub creator_address: String,
    pub signal_source: String,
}

impl LiquidityRemovalSignalRecord {
    /// Create from LiquiditySignal
    pub fn from_signal(signal: &LiquiditySignal) -> Self {
        Self {
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            pool_type: normalize_pool_type(&signal.pool_type),
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            liquidity_removed_denom: signal.eth_removed,
            remaining_liquidity_denom: signal.remaining_eth,
            removal_percentage: signal.removal_percentage,
            pool_drain_risk_level: drain_risk_label(signal.removal_percentage),
            creator_address: signal.creator_address.clone(),
            signal_source: "mempool".to_string(),
        }
    }
}

fn drain_risk_label(removal_percentage: Option<f64>) -> String {
    let Some(removal_percentage) = removal_percentage else {
        return "UNKNOWN".to_string();
    };

    match removal_percentage {
        p if p > 50.0 => "DRAINING".to_string(),
        p if p >= 20.0 => "SIGNIFICANT".to_string(),
        _ => "LOW".to_string(),
    }
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

/// Liquidity removal signal database writer
pub struct LiquidityRemovalSignalWriter {
    sender: mpsc::UnboundedSender<LiquidityRemovalSignalRecord>,
    _handle: tokio::task::JoinHandle<()>,
}

impl LiquidityRemovalSignalWriter {
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

    /// Write a liquidity removal signal
    pub fn write_signal(&self, signal: &LiquiditySignal) -> Result<()> {
        let record = LiquidityRemovalSignalRecord::from_signal(signal);
        self.sender
            .send(record)
            .map_err(|e| eyre::eyre!("Failed to send liquidity removal signal: {}", e))?;
        Ok(())
    }
}

/// Background writer task that batches and writes signals to database
async fn writer_task(
    pool: PgPool,
    mut receiver: mpsc::UnboundedReceiver<LiquidityRemovalSignalRecord>,
) {
    let mut batch = Vec::new();
    let mut interval = interval(Duration::from_secs(1)); // Batch every 1 second
    let batch_size = 50;

    info!("🟢 Liquidity removal signal writer started");

    loop {
        tokio::select! {
            // Receive new signals
            signal = receiver.recv() => {
                match signal {
                    Some(record) => {
                        batch.push(record);
                        if batch.len() >= batch_size {
                            if let Err(e) = write_batch(&pool, &mut batch).await {
                                error!("Failed to write liquidity removal signal batch: {}", e);
                            }
                        }
                    }
                    None => {
                        // Channel closed
                        info!("🔴 Liquidity removal signal writer stopping");
                        break;
                    }
                }
            }

            // Periodic batch write
            _ = interval.tick() => {
                if !batch.is_empty() {
                    if let Err(e) = write_batch(&pool, &mut batch).await {
                        error!("Failed to write liquidity removal signal batch: {}", e);
                    }
                }
            }
        }
    }

    // Write any remaining signals
    if !batch.is_empty() {
        if let Err(e) = write_batch(&pool, &mut batch).await {
            error!(
                "Failed to write final liquidity removal signal batch: {}",
                e
            );
        }
    }
}

/// Write a batch of signals to database
async fn write_batch(pool: &PgPool, batch: &mut Vec<LiquidityRemovalSignalRecord>) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    let start = Instant::now();
    let mut transaction = pool.begin().await?;

    for record in batch.iter() {
        let liquidity_removed = record
            .liquidity_removed_denom
            .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());
        let remaining_liquidity = record
            .remaining_liquidity_denom
            .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());
        let removal_percentage = record
            .removal_percentage
            .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

        let detection_timestamp = record.detection_timestamp.naive_utc();

        let query = sqlx::query(
            r#"
            INSERT INTO live_trading.liquidity_removal_signals (
                token_address, pool_address, pool_type, denom_address, denom_currency,
                detection_timestamp, detection_tx_hash, liquidity_removed_eth,
                remaining_liquidity_eth, removal_percentage, pool_drain_risk_level, creator_address, signal_source
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (pool_address, detection_tx_hash) DO NOTHING
            "#,
        )
        .bind(&record.token_address)
        .bind(&record.pool_address)
        .bind(&record.pool_type)
        .bind(&record.denom_address)
        .bind(record.denom_currency.as_ref())
        .bind(&detection_timestamp)
        .bind(&record.detection_tx_hash)
        .bind(liquidity_removed.as_ref())
        .bind(remaining_liquidity.as_ref())
        .bind(removal_percentage.as_ref())
        .bind(&record.pool_drain_risk_level)
        .bind(&record.creator_address)
        .bind(&record.signal_source);

        if let Err(e) = query.execute(&mut *transaction).await {
            error!("Failed to insert liquidity removal signal: {}", e);
        }
    }

    transaction.commit().await?;

    let duration = start.elapsed();
    info!(
        "📊 Wrote {} liquidity removal signals in {:.2}ms",
        batch.len(),
        duration.as_secs_f64() * 1000.0
    );

    batch.clear();
    Ok(())
}
