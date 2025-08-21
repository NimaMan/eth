use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use sqlx::postgres::{PgPool, PgPoolOptions};
use chrono::{DateTime, Utc};
use tracing::{info, error, debug};
use eyre::Result;
use rust_decimal::Decimal;

use crate::signal_detector::TradingStatusSignal;

/// Trading signal record for database insertion
#[derive(Debug, Clone)]
pub struct TradingSignalRecord {
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub denom_address: String,
    pub denom_currency: Option<String>,
    pub detection_timestamp: DateTime<Utc>,
    pub detection_tx_hash: String,
    pub price_ratio: Option<Decimal>,
    pub denom_reserve_at_signal: Option<Decimal>,
    pub token_reserve_at_signal: Option<Decimal>,
    pub buy_tax_at_signal: Option<Decimal>,
    pub sell_tax_at_signal: Option<Decimal>,
    pub total_supply: Option<Decimal>,
    pub owner_address: Option<String>,
    pub creator_address: String,
    pub signal_source: String,
}

impl TradingSignalRecord {
    /// Create from TradingStatusSignal
    pub fn from_signal(signal: &TradingStatusSignal) -> Self {
        Self {
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            pool_type: "V2".to_string(), // Default, could be extracted from signal
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH default
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            price_ratio: None, // Could be calculated from reserves if available
            denom_reserve_at_signal: None, // Would need to be passed from pool state
            token_reserve_at_signal: None, // Would need to be passed from pool state
            buy_tax_at_signal: signal.buy_tax.map(|t| Decimal::from_f64_retain(t).unwrap_or_default()),
            sell_tax_at_signal: signal.sell_tax.map(|t| Decimal::from_f64_retain(t).unwrap_or_default()),
            total_supply: None, // Would need to be passed from token info
            owner_address: None, // Would need to be passed from token info
            creator_address: signal.executor.clone(),
            signal_source: "mempool".to_string(),
        }
    }

    /// Create with additional context from token cache
    pub fn from_signal_with_context(
        signal: &TradingStatusSignal,
        denom_reserve: Option<f64>,
        token_reserve: Option<f64>,
        total_supply: Option<String>,
        owner_address: Option<String>,
    ) -> Self {
        let mut record = Self::from_signal(signal);
        
        record.denom_reserve_at_signal = denom_reserve.map(|r| Decimal::from_f64_retain(r).unwrap_or_default());
        record.token_reserve_at_signal = token_reserve.map(|r| Decimal::from_f64_retain(r).unwrap_or_default());
        record.total_supply = total_supply.and_then(|s| s.parse::<Decimal>().ok());
        record.owner_address = owner_address;
        
        // Calculate price ratio if we have reserves
        if let (Some(denom_res), Some(token_res)) = (denom_reserve, token_reserve) {
            if token_res > 0.0 {
                record.price_ratio = Some(Decimal::from_f64_retain(denom_res / token_res).unwrap_or_default());
            }
        }
        
        record
    }
}

/// Writer configuration
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Batch size for database inserts
    pub batch_size: usize,
    /// Maximum time to wait before flushing batch
    pub flush_interval: Duration,
    /// Channel buffer size
    pub channel_size: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100, // Smaller batches for trading signals
            flush_interval: Duration::from_secs(10), // More frequent flushes
            channel_size: 1000,
        }
    }
}

/// Non-blocking trading signal database writer
pub struct TradingSignalWriter {
    sender: mpsc::Sender<TradingSignalRecord>,
}

impl TradingSignalWriter {
    /// Create a new writer with default database connection
    pub async fn new_with_defaults(config: WriterConfig) -> Result<Self> {
        let db_url = super::get_default_database_url();
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;
        Self::new(db_pool, config).await
    }
    
    /// Create a new writer and spawn background worker
    pub async fn new(db_pool: PgPool, config: WriterConfig) -> Result<Self> {
        let (sender, receiver) = mpsc::channel(config.channel_size);
        
        // Spawn background worker
        tokio::spawn(async move {
            if let Err(e) = run_writer_worker(receiver, db_pool, config).await {
                error!("Trading signal writer worker failed: {}", e);
            }
        });
        
        Ok(Self { sender })
    }
    
    /// Write a trading signal (non-blocking)
    pub async fn write_signal(&self, signal: TradingSignalRecord) {
        // Non-blocking send, drop if channel is full
        if let Err(e) = self.sender.try_send(signal) {
            debug!("Trading signal writer channel full, dropping: {}", e);
        }
    }
    
    /// Write a trading enabled signal directly
    pub async fn write_trading_enabled(&self, signal: &TradingStatusSignal) {
        let record = TradingSignalRecord::from_signal(signal);
        self.write_signal(record).await;
    }
    
    /// Write a trading enabled signal with additional context
    pub async fn write_trading_enabled_with_context(
        &self,
        signal: &TradingStatusSignal,
        denom_reserve: Option<f64>,
        token_reserve: Option<f64>,
        total_supply: Option<String>,
        owner_address: Option<String>,
    ) {
        let record = TradingSignalRecord::from_signal_with_context(
            signal, denom_reserve, token_reserve, total_supply, owner_address
        );
        self.write_signal(record).await;
    }
}

/// Background worker that processes signal writes
async fn run_writer_worker(
    mut receiver: mpsc::Receiver<TradingSignalRecord>,
    db_pool: PgPool,
    config: WriterConfig,
) -> Result<()> {
    info!("Starting trading signal writer worker");
    
    let mut pending_signals: Vec<TradingSignalRecord> = Vec::new();
    let mut flush_timer = interval(config.flush_interval);
    let mut stats = WriterStats::default();
    let mut last_stats_report = Instant::now();
    
    loop {
        tokio::select! {
            // Receive new signals
            Some(signal) = receiver.recv() => {
                pending_signals.push(signal);
                stats.received += 1;
                
                // Flush if batch is full
                if pending_signals.len() >= config.batch_size {
                    flush_signals(&db_pool, &mut pending_signals, &mut stats).await;
                }
            }
            
            // Periodic flush
            _ = flush_timer.tick() => {
                if !pending_signals.is_empty() {
                    flush_signals(&db_pool, &mut pending_signals, &mut stats).await;
                }
            }
            
            // Graceful shutdown when channel closes
            else => {
                info!("Trading signal writer shutting down");
                if !pending_signals.is_empty() {
                    flush_signals(&db_pool, &mut pending_signals, &mut stats).await;
                }
                break;
            }
        }
        
        // Report stats every 5 minutes
        if last_stats_report.elapsed() > Duration::from_secs(300) {
            info!(
                "Trading signal writer stats - Received: {}, Written: {}, Errors: {}",
                stats.received, stats.written, stats.errors
            );
            last_stats_report = Instant::now();
        }
    }
    
    Ok(())
}

/// Flush pending signals to database
async fn flush_signals(
    db_pool: &PgPool,
    pending: &mut Vec<TradingSignalRecord>,
    stats: &mut WriterStats,
) {
    if pending.is_empty() {
        return;
    }
    
    let batch_size = pending.len();
    debug!("Flushing {} trading signals to database", batch_size);
    
    // Build batch insert query
    let mut query = String::from(
        r#"INSERT INTO live_trading.trading_enabled_signals (
            token_address, pool_address, pool_type, denom_address, denom_currency,
            detection_timestamp, detection_tx_hash,
            price_ratio, denom_reserve_at_signal, token_reserve_at_signal,
            buy_tax_at_signal, sell_tax_at_signal,
            total_supply, owner_address, creator_address, signal_source
        ) VALUES "#
    );
    
    // Add value placeholders
    let mut params: Vec<String> = Vec::new();
    for (i, _) in pending.iter().enumerate() {
        let base = i * 16; // 16 fields per record (removed detection_block)
        params.push(format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            base + 1, base + 2, base + 3, base + 4, base + 5, base + 6, base + 7, base + 8,
            base + 9, base + 10, base + 11, base + 12, base + 13, base + 14, base + 15, base + 16
        ));
    }
    
    query.push_str(&params.join(", "));
    query.push_str(" ON CONFLICT (pool_address, detection_tx_hash) DO NOTHING");
    
    // Build the query with parameters
    let mut sql_query = sqlx::query(&query);
    
    for record in pending.iter() {
        sql_query = sql_query
            .bind(&record.token_address)
            .bind(&record.pool_address)
            .bind(&record.pool_type)
            .bind(&record.denom_address)
            .bind(&record.denom_currency)
            .bind(record.detection_timestamp)
            .bind(&record.detection_tx_hash)
            .bind(record.price_ratio)
            .bind(record.denom_reserve_at_signal)
            .bind(record.token_reserve_at_signal)
            .bind(record.buy_tax_at_signal)
            .bind(record.sell_tax_at_signal)
            .bind(record.total_supply)
            .bind(&record.owner_address)
            .bind(&record.creator_address)
            .bind(&record.signal_source);
    }
    
    match sql_query.execute(db_pool).await {
        Ok(result) => {
            let rows_affected = result.rows_affected();
            stats.written += rows_affected;
            debug!("Wrote {} trading enabled signals to database", rows_affected);
            pending.clear();
        }
        Err(e) => {
            error!("Failed to write trading signals to database: {}", e);
            stats.errors += batch_size as u64;
            // Clear pending to prevent infinite retries
            pending.clear();
        }
    }
}

/// Writer statistics
#[derive(Default)]
struct WriterStats {
    received: u64,
    written: u64,
    errors: u64,
}