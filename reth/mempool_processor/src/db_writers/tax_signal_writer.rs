use chrono::{DateTime, Utc};
use eyre::Result;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use tracing::{error, info};

use crate::signal_detector::TaxSignal;

/// Tax signal record for database insertion
#[derive(Debug, Clone)]
pub struct TaxSignalRecord {
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub denom_address: String,
    pub denom_currency: Option<String>,
    pub detection_timestamp: DateTime<Utc>,
    pub detection_tx_hash: String,
    pub signal_type: String,
    pub signal_details: String,
    pub confidence: Option<f64>,
    pub buy_tax_at_signal: Option<f64>,
    pub sell_tax_at_signal: Option<f64>,
    pub buy_tax_exceeds_threshold: bool,
    pub sell_tax_exceeds_threshold: bool,
    pub cant_sell: bool,
    pub creator_address: String,
    pub signal_source: String,
}

impl TaxSignalRecord {
    /// Create from TaxSignal with additional context
    pub fn from_signal(
        signal: &TaxSignal,
        pool_address: &str,
        pool_type: &str,
        creator_address: &str,
        tx_hash: &str,
    ) -> Self {
        fn normalize_pool_type(pool_type: &str) -> String {
            match pool_type.trim().to_lowercase().as_str() {
                "uniswapv2" | "uniswap-v2" | "v2" => "UNISWAP-V2".to_string(),
                "uniswapv3" | "uniswap-v3" | "v3" => "UNISWAP-V3".to_string(),
                "uniswapv4" | "uniswap-v4" | "v4" => "UNISWAP-V4".to_string(),
                "sushiswap" | "sushi" | "sushi-swap" => "SUSHI-SWAP".to_string(),
                "curve" => "CURVE".to_string(),
                "balancer" => "BALANCER".to_string(),
                other => other.to_uppercase(),
            }
        }

        Self {
            token_address: signal.token_address.clone(),
            pool_address: pool_address.to_string(),
            pool_type: normalize_pool_type(pool_type),
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH default
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: Utc::now(),
            detection_tx_hash: tx_hash.to_string(),
            signal_type: format!("{:?}", signal.signal_type)
                .split("::")
                .last()
                .unwrap_or("Unknown")
                .to_string(),
            signal_details: signal.details.clone(),
            confidence: Some(signal.confidence),
            buy_tax_at_signal: signal.buy_tax,
            sell_tax_at_signal: signal.sell_tax,
            buy_tax_exceeds_threshold: match &signal.signal_type {
                crate::signal_detector::TaxSignalType::HighTaxOrHoneypot {
                    buy_tax_exceeds_threshold,
                    ..
                } => *buy_tax_exceeds_threshold,
                _ => false,
            },
            sell_tax_exceeds_threshold: match &signal.signal_type {
                crate::signal_detector::TaxSignalType::HighTaxOrHoneypot {
                    sell_tax_exceeds_threshold,
                    ..
                } => *sell_tax_exceeds_threshold,
                _ => false,
            },
            cant_sell: match &signal.signal_type {
                crate::signal_detector::TaxSignalType::HighTaxOrHoneypot { cant_sell, .. } => {
                    *cant_sell
                }
                _ => false,
            },
            creator_address: creator_address.to_string(),
            signal_source: "mempool".to_string(),
        }
    }
}

/// Non-blocking tax signal writer
pub struct TaxSignalWriter {
    sender: mpsc::UnboundedSender<TaxSignalRecord>,
    _handle: tokio::task::JoinHandle<()>,
}

impl TaxSignalWriter {
    /// Create a new tax signal writer with database connection
    pub async fn new(
        database_url: &str,
        batch_size: usize,
        flush_interval: Duration,
    ) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect(database_url)
            .await?;

        let (sender, receiver) = mpsc::unbounded_channel::<TaxSignalRecord>();

        let handle = tokio::spawn(Self::writer_task(
            pool,
            receiver,
            batch_size,
            flush_interval,
        ));

        Ok(Self {
            sender,
            _handle: handle,
        })
    }

    /// Submit a tax signal record for writing
    pub fn write_signal(&self, record: TaxSignalRecord) -> Result<()> {
        self.sender
            .send(record)
            .map_err(|_| eyre::eyre!("Tax signal writer channel closed"))?;
        Ok(())
    }

    /// Background writer task
    async fn writer_task(
        pool: Pool<Postgres>,
        mut receiver: mpsc::UnboundedReceiver<TaxSignalRecord>,
        batch_size: usize,
        flush_interval: Duration,
    ) {
        let mut batch = Vec::with_capacity(batch_size);
        let mut flush_timer = interval(flush_interval);
        flush_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!(
            "Tax signal writer started (batch_size: {}, flush_interval: {:?})",
            batch_size, flush_interval
        );

        loop {
            tokio::select! {
                // Receive new records
                record = receiver.recv() => {
                    match record {
                        Some(record) => {
                            batch.push(record);

                            // Flush if batch is full
                            if batch.len() >= batch_size {
                                let batch_to_write = std::mem::replace(&mut batch, Vec::with_capacity(batch_size));
                                Self::write_batch(&pool, batch_to_write).await;
                            }
                        }
                        None => {
                            // Channel closed, write remaining records and exit
                            if !batch.is_empty() {
                                Self::write_batch(&pool, batch).await;
                            }
                            info!("Tax signal writer shutting down");
                            break;
                        }
                    }
                }

                // Periodic flush
                _ = flush_timer.tick() => {
                    if !batch.is_empty() {
                        let batch_to_write = std::mem::replace(&mut batch, Vec::with_capacity(batch_size));
                        Self::write_batch(&pool, batch_to_write).await;
                    }
                }
            }
        }
    }

    /// Write a batch of records to the database
    async fn write_batch(pool: &Pool<Postgres>, records: Vec<TaxSignalRecord>) {
        if records.is_empty() {
            return;
        }

        let start = Instant::now();

        // Write records one by one to avoid complex batch insert issues
        let mut success_count = 0;
        for record in &records {
            let result = sqlx::query(
                r#"
                INSERT INTO live_trading.tax_signals (
                    token_address, pool_address, pool_type, denom_address, denom_currency,
                    detection_timestamp, detection_tx_hash,
                    signal_type, signal_details, confidence,
                    buy_tax_at_signal, sell_tax_at_signal,
                    buy_tax_exceeds_threshold, sell_tax_exceeds_threshold, cant_sell,
                    creator_address, signal_source
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
                ) ON CONFLICT (pool_address, detection_tx_hash) DO NOTHING
                "#,
            )
            .bind(&record.token_address)
            .bind(&record.pool_address)
            .bind(&record.pool_type)
            .bind(&record.denom_address)
            .bind(&record.denom_currency)
            .bind(&record.detection_timestamp)
            .bind(&record.detection_tx_hash)
            .bind(&record.signal_type)
            .bind(&record.signal_details)
            .bind(&record.confidence)
            .bind(&record.buy_tax_at_signal)
            .bind(&record.sell_tax_at_signal)
            .bind(&record.buy_tax_exceeds_threshold)
            .bind(&record.sell_tax_exceeds_threshold)
            .bind(&record.cant_sell)
            .bind(&record.creator_address)
            .bind(&record.signal_source)
            .execute(pool)
            .await;

            match result {
                Ok(_) => success_count += 1,
                Err(e) => error!("Failed to write tax signal: {:?}", e),
            }
        }

        let duration = start.elapsed();
        info!(
            "Wrote {}/{} tax signals to database in {:?}",
            success_count,
            records.len(),
            duration
        );
    }
}

impl Drop for TaxSignalWriter {
    fn drop(&mut self) {
        // Close the sender to signal the writer task to finish
        // The writer task will flush remaining records before shutting down
    }
}
