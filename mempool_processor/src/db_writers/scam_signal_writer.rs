use chrono::{DateTime, Utc};
use eyre::Result;
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use tracing::{error, info};

/// Scam signal types
#[derive(Debug, Clone)]
pub enum ScamType {
    Honeypot,
    CantSell,
    HighTax,
    LiquidityDrain,
    LpApproval,
}

impl ScamType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScamType::Honeypot => "honeypot",
            ScamType::CantSell => "cant_sell",
            ScamType::HighTax => "high_tax",
            ScamType::LiquidityDrain => "liquidity_drain",
            ScamType::LpApproval => "lp_approval",
        }
    }
}

/// Scam signal record for database insertion
#[derive(Debug, Clone)]
pub struct ScamSignalRecord {
    pub scam_type: ScamType,
    pub token_address: String,
    pub pool_address: Option<String>,
    pub scammer_address: String,
    pub detection_timestamp: DateTime<Utc>,
    pub detection_tx_hash: String,
    pub scam_details: serde_json::Value,
    pub signal_source: String,
}

impl ScamSignalRecord {
    /// Create a cant_sell signal (honeypot)
    pub fn cant_sell(
        token_address: String,
        pool_address: String,
        scammer_address: String,
        tx_hash: String,
        buy_tax: Option<f64>,
        sell_tax: Option<f64>,
    ) -> Self {
        Self {
            scam_type: ScamType::CantSell,
            token_address,
            pool_address: Some(pool_address),
            scammer_address,
            detection_timestamp: Utc::now(),
            detection_tx_hash: tx_hash,
            scam_details: json!({
                "buy_tax": buy_tax.unwrap_or(0.0),
                "sell_tax": sell_tax,
                "can_buy": true,
                "can_sell": false
            }),
            signal_source: "mempool".to_string(),
        }
    }

    /// Create a honeypot signal for a pool that can be bought but cannot be sold.
    pub fn honeypot(
        token_address: String,
        pool_address: String,
        creator_address: String,
        tx_hash: String,
        buy_tax: Option<f64>,
        sell_tax: Option<f64>,
        failure_reason: Option<String>,
        pool_type: String,
    ) -> Self {
        Self {
            scam_type: ScamType::Honeypot,
            token_address,
            pool_address: Some(pool_address),
            scammer_address: creator_address,
            detection_timestamp: Utc::now(),
            detection_tx_hash: tx_hash,
            scam_details: json!({
                "pool_type": pool_type,
                "buy_tax": buy_tax,
                "sell_tax": sell_tax,
                "can_buy": true,
                "can_sell": false,
                "failure_reason": failure_reason
            }),
            signal_source: "mempool".to_string(),
        }
    }

    /// Create a high_tax signal
    pub fn high_tax(
        token_address: String,
        pool_address: String,
        scammer_address: String,
        tx_hash: String,
        buy_tax: f64,
        sell_tax: f64,
    ) -> Self {
        Self {
            scam_type: ScamType::HighTax,
            token_address,
            pool_address: Some(pool_address),
            scammer_address,
            detection_timestamp: Utc::now(),
            detection_tx_hash: tx_hash,
            scam_details: json!({
                "buy_tax": buy_tax,
                "sell_tax": sell_tax
            }),
            signal_source: "mempool".to_string(),
        }
    }

    /// Create a liquidity_drain signal
    pub fn liquidity_drain(
        token_address: String,
        pool_address: String,
        scammer_address: String,
        tx_hash: String,
        eth_drained: f64,
        drain_percentage: f64,
        remaining_eth: f64,
    ) -> Self {
        Self {
            scam_type: ScamType::LiquidityDrain,
            token_address,
            pool_address: Some(pool_address),
            scammer_address,
            detection_timestamp: Utc::now(),
            detection_tx_hash: tx_hash,
            scam_details: json!({
                "eth_drained": eth_drained,
                "drain_percentage": drain_percentage,
                "remaining_eth": remaining_eth
            }),
            signal_source: "mempool".to_string(),
        }
    }

    /// Create an lp_approval signal
    pub fn lp_approval(
        token_address: String,
        pool_address: String,
        owner_address: String,
        approved_spender: String,
        tx_hash: String,
        pool_liquidity_eth: f64,
    ) -> Self {
        Self {
            scam_type: ScamType::LpApproval,
            token_address,
            pool_address: Some(pool_address),
            scammer_address: owner_address, // LP owner who approved
            detection_timestamp: Utc::now(),
            detection_tx_hash: tx_hash,
            scam_details: json!({
                "approved_spender": approved_spender,
                "pool_liquidity_eth": pool_liquidity_eth
            }),
            signal_source: "mempool".to_string(),
        }
    }
}

/// Non-blocking scam signal writer
pub struct ScamSignalWriter {
    sender: mpsc::UnboundedSender<ScamSignalRecord>,
    _handle: tokio::task::JoinHandle<()>,
}

impl ScamSignalWriter {
    /// Create a new scam signal writer with database connection
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

        let (sender, receiver) = mpsc::unbounded_channel::<ScamSignalRecord>();

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

    /// Submit a scam signal record for writing
    pub fn write_signal(&self, record: ScamSignalRecord) -> Result<()> {
        self.sender
            .send(record)
            .map_err(|_| eyre::eyre!("Scam signal writer channel closed"))?;
        Ok(())
    }

    /// Background writer task
    async fn writer_task(
        pool: Pool<Postgres>,
        mut receiver: mpsc::UnboundedReceiver<ScamSignalRecord>,
        batch_size: usize,
        flush_interval: Duration,
    ) {
        let mut batch = Vec::with_capacity(batch_size);
        let mut flush_timer = interval(flush_interval);
        flush_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!(
            "Scam signal writer started (batch_size: {}, flush_interval: {:?})",
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
                            info!("Scam signal writer shutting down");
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
    async fn write_batch(pool: &Pool<Postgres>, records: Vec<ScamSignalRecord>) {
        if records.is_empty() {
            return;
        }

        let start = Instant::now();

        // Write records one by one to avoid complex batch insert issues
        let mut success_count = 0;
        for record in &records {
            let result = sqlx::query(
                r#"
                INSERT INTO live_trading.scam_signals (
                    scam_type, token_address, pool_address, scammer_address,
                    detection_timestamp, detection_tx_hash, scam_details, signal_source
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8
                ) ON CONFLICT (pool_address, detection_tx_hash, scam_type) DO NOTHING
                "#,
            )
            .bind(record.scam_type.as_str())
            .bind(&record.token_address)
            .bind(&record.pool_address)
            .bind(&record.scammer_address)
            .bind(&record.detection_timestamp)
            .bind(&record.detection_tx_hash)
            .bind(&record.scam_details)
            .bind(&record.signal_source)
            .execute(pool)
            .await;

            match result {
                Ok(_) => success_count += 1,
                Err(e) => error!("Failed to write scam signal: {:?}", e),
            }
        }

        let duration = start.elapsed();
        info!(
            "Wrote {}/{} scam signals to database in {:?}",
            success_count,
            records.len(),
            duration
        );
    }
}

impl Drop for ScamSignalWriter {
    fn drop(&mut self) {
        // Close the sender to signal the writer task to finish
        // The writer task will flush remaining records before shutting down
    }
}
