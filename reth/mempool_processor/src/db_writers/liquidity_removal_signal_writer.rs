/// Liquidity Removal Signal Database Writer
/// 
/// Non-blocking writer that records liquidity removal signals to the database.
/// Uses a background task with batched inserts to avoid blocking the main processing pipeline.

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use sqlx::postgres::{PgPool, PgPoolOptions};
use chrono::{DateTime, Utc};
use tracing::{info, error};
use eyre::Result;
use rust_decimal::Decimal;

use crate::signal_detector::{LiquiditySignal};

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
    pub liquidity_removed_eth: Option<Decimal>,
    pub liquidity_removed_token: Option<Decimal>,
    pub remaining_liquidity_eth: Option<Decimal>,
    pub remaining_liquidity_token: Option<Decimal>,
    pub removal_percentage: Option<Decimal>,
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
            pool_type: "V2".to_string(),
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            liquidity_removed_eth: signal.eth_removed.map(|v| Decimal::from_f64_retain(v).unwrap_or_default()),
            liquidity_removed_token: signal.token_removed.map(|v| Decimal::from_f64_retain(v).unwrap_or_default()),
            remaining_liquidity_eth: signal.remaining_eth.map(|v| Decimal::from_f64_retain(v).unwrap_or_default()),
            remaining_liquidity_token: signal.remaining_token.map(|v| Decimal::from_f64_retain(v).unwrap_or_default()),
            removal_percentage: signal.removal_percentage.map(|v| Decimal::from_f64_retain(v).unwrap_or_default()),
            pool_drain_risk_level: match signal.removal_percentage.unwrap_or(0.0) {
                p if p > 80.0 => "CRITICAL".to_string(),
                p if p > 60.0 => "HIGH".to_string(),
                p if p > 30.0 => "MEDIUM".to_string(),
                _ => "LOW".to_string(),
            },
            creator_address: signal.creator_address.clone(),
            signal_source: "mempool".to_string(),
        }
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
        self.sender.send(record)
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
            error!("Failed to write final liquidity removal signal batch: {}", e);
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
        let query = sqlx::query!(
            r#"
            INSERT INTO live_trading.liquidity_removal_signals (
                token_address, pool_address, pool_type, denom_address, denom_currency,
                detection_timestamp, detection_tx_hash, liquidity_removed_eth, liquidity_removed_token,
                remaining_liquidity_eth, remaining_liquidity_token, removal_percentage,
                pool_drain_risk_level, creator_address, signal_source
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (pool_address, detection_tx_hash) DO NOTHING
            "#,
            record.token_address,
            record.pool_address,
            record.pool_type,
            record.denom_address,
            record.denom_currency,
            record.detection_timestamp.naive_utc(),
            record.detection_tx_hash,
            record.liquidity_removed_eth,
            record.liquidity_removed_token,
            record.remaining_liquidity_eth,
            record.remaining_liquidity_token,
            record.removal_percentage,
            record.pool_drain_risk_level,
            record.creator_address,
            record.signal_source
        );
        
        if let Err(e) = query.execute(&mut *transaction).await {
            error!("Failed to insert liquidity removal signal: {}", e);
        }
    }
    
    transaction.commit().await?;
    
    let duration = start.elapsed();
    info!("📊 Wrote {} liquidity removal signals in {:.2}ms", batch.len(), duration.as_secs_f64() * 1000.0);
    
    batch.clear();
    Ok(())
}