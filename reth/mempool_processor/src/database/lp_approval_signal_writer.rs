/// LP Approval Signal Database Writer
/// 
/// Non-blocking writer that records LP approval signals to the database.
/// Uses a background task with batched inserts to avoid blocking the main processing pipeline.

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use sqlx::postgres::{PgPool, PgPoolOptions};
use chrono::{DateTime, Utc};
use tracing::{info, warn, error, debug};
use eyre::Result;
use rust_decimal::Decimal;

use crate::signal_detector::{LpApprovalSignal};

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
    pub approval_amount: Option<Decimal>,
    pub is_unlimited_approval: bool,
    pub approval_type: String,
    pub previous_allowance: Option<Decimal>,
    pub creator_address: String,
    pub signal_source: String,
}

impl LpApprovalSignalRecord {
    /// Create from LpApprovalSignal
    pub fn from_signal(signal: &LpApprovalSignal) -> Self {
        let unlimited = signal.amount_approved.map(|a| a >= 1e30).unwrap_or(false);
        
        Self {
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            pool_type: "V2".to_string(),
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            approved_spender: signal.spender_address.clone(),
            approval_amount: signal.amount_approved.map(|v| Decimal::from_f64_retain(v).unwrap_or_default()),
            is_unlimited_approval: unlimited,
            approval_type: "TOKEN".to_string(),
            previous_allowance: signal.previous_allowance.map(|v| Decimal::from_f64_retain(v).unwrap_or_default()),
            creator_address: signal.creator_address.clone(),
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
        self.sender.send(record)
            .map_err(|e| eyre::eyre!("Failed to send LP approval signal: {}", e))?;
        Ok(())
    }
}

/// Background writer task that batches and writes signals to database
async fn writer_task(
    pool: PgPool,
    mut receiver: mpsc::UnboundedReceiver<LpApprovalSignalRecord>,
) {
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
        let query = sqlx::query!(
            r#"
            INSERT INTO live_trading.lp_approval_signals (
                token_address, pool_address, pool_type, denom_address, denom_currency,
                detection_timestamp, detection_tx_hash, approved_spender, approval_amount,
                is_unlimited_approval, approval_type, previous_allowance,
                creator_address, signal_source
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (pool_address, detection_tx_hash, approved_spender) DO NOTHING
            "#,
            record.token_address,
            record.pool_address,
            record.pool_type,
            record.denom_address,
            record.denom_currency,
            record.detection_timestamp.naive_utc(),
            record.detection_tx_hash,
            record.approved_spender,
            record.approval_amount,
            record.is_unlimited_approval,
            record.approval_type,
            record.previous_allowance,
            record.creator_address,
            record.signal_source
        );
        
        if let Err(e) = query.execute(&mut *transaction).await {
            error!("Failed to insert LP approval signal: {}", e);
        }
    }
    
    transaction.commit().await?;
    
    let duration = start.elapsed();
    info!("📊 Wrote {} LP approval signals in {:.2}ms", batch.len(), duration.as_secs_f64() * 1000.0);
    
    batch.clear();
    Ok(())
}