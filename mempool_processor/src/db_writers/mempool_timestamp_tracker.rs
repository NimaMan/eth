use chrono::{DateTime, Utc};
use eyre::Result;
use sqlx::{
    postgres::{PgPool, PgPoolOptions},
    Row,
};
/// Mempool Timestamp Tracker
///
/// Non-blocking tracker that records when transactions are first seen in the mempool.
/// Uses a background task with batched updates to avoid blocking the main processing pipeline.
/// Only updates transactions that already exist in the database (from block processing).
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use tracing::{debug, error, info};

/// Update request for mempool timestamp
#[derive(Debug, Clone)]
pub struct MempoolTimestamp {
    pub tx_hash: String,
    pub first_seen: DateTime<Utc>,
}

/// Tracker configuration
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Batch size for database updates
    pub batch_size: usize,
    /// Maximum time to wait before flushing batch
    pub flush_interval: Duration,
    /// Channel buffer size
    pub channel_size: usize,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            flush_interval: Duration::from_secs(300), // 5 minutes
            channel_size: 50000,
        }
    }
}

/// Non-blocking mempool timestamp tracker
pub struct MempoolTimestampTracker {
    sender: mpsc::Sender<MempoolTimestamp>,
}

impl MempoolTimestampTracker {
    /// Create a new tracker with default database connection
    pub async fn new_with_defaults(config: TrackerConfig) -> Result<Self> {
        let db_url = super::get_default_database_url();
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;
        Self::new(db_pool, config).await
    }

    /// Create a new tracker and spawn background worker
    pub async fn new(db_pool: PgPool, config: TrackerConfig) -> Result<Self> {
        let (sender, receiver) = mpsc::channel(config.channel_size);

        // Spawn background worker
        tokio::spawn(async move {
            if let Err(e) = run_tracker_worker(receiver, db_pool, config).await {
                error!("Mempool timestamp tracker worker failed: {}", e);
            }
        });

        Ok(Self { sender })
    }

    /// Record a transaction seen in mempool (non-blocking)
    pub async fn record_transaction(&self, tx_hash: String) {
        let timestamp = MempoolTimestamp {
            tx_hash,
            first_seen: Utc::now(),
        };

        // Non-blocking send, drop if channel is full
        if let Err(e) = self.sender.try_send(timestamp) {
            debug!("Mempool timestamp channel full, dropping: {}", e);
        }
    }
}

/// Background worker that processes timestamp updates
async fn run_tracker_worker(
    mut receiver: mpsc::Receiver<MempoolTimestamp>,
    db_pool: PgPool,
    config: TrackerConfig,
) -> Result<()> {
    info!("Starting mempool timestamp tracker worker");

    // Store all pending timestamps with expiration tracking
    let mut pending_timestamps: HashMap<String, DateTime<Utc>> = HashMap::new();
    let mut flush_timer = interval(config.flush_interval);
    let mut cleanup_timer = interval(Duration::from_secs(3600)); // Cleanup every hour
    let mut stats = WorkerStats::default();
    let mut last_stats_report = Instant::now();

    // Maximum age for pending timestamps (2 days)
    let max_age = chrono::Duration::days(2);

    loop {
        tokio::select! {
            // Receive new timestamps
            Some(timestamp) = receiver.recv() => {
                // Only keep the earliest timestamp for each tx
                pending_timestamps.entry(timestamp.tx_hash.clone())
                    .or_insert(timestamp.first_seen);

                stats.pending_count = pending_timestamps.len() as u64;
            }

            // Periodic flush attempt
            _ = flush_timer.tick() => {
                if !pending_timestamps.is_empty() {
                    flush_pending(&db_pool, &mut pending_timestamps, &mut stats).await;
                }
            }

            // Periodic cleanup of old entries
            _ = cleanup_timer.tick() => {
                let now = Utc::now();
                let before_count = pending_timestamps.len();

                // Remove entries older than max_age
                pending_timestamps.retain(|_hash, timestamp| {
                    now.signed_duration_since(*timestamp) < max_age
                });

                let removed = before_count - pending_timestamps.len();
                if removed > 0 {
                    stats.expired += removed as u64;
                    info!("Cleaned up {} expired mempool timestamps (older than 2 days)", removed);
                }
            }

            // Graceful shutdown when channel closes
            else => {
                info!("Mempool timestamp tracker shutting down");
                if !pending_timestamps.is_empty() {
                    flush_pending(&db_pool, &mut pending_timestamps, &mut stats).await;
                }
                break;
            }
        }

        // Report stats every 5 minutes
        if last_stats_report.elapsed() > Duration::from_secs(300) {
            info!(
                "Mempool timestamp tracker stats - Pending: {}, Total processed: {}, Updated: {}, Not yet mined: {}, Expired: {}, Errors: {}",
                pending_timestamps.len(),
                stats.total_processed,
                stats.successful_updates,
                stats.not_found,
                stats.expired,
                stats.errors
            );
            last_stats_report = Instant::now();
        }
    }

    Ok(())
}

/// Flush pending timestamps to the database (only those that have been mined)
async fn flush_pending(
    db_pool: &PgPool,
    pending: &mut HashMap<String, DateTime<Utc>>,
    stats: &mut WorkerStats,
) {
    if pending.is_empty() {
        return;
    }

    let total_pending = pending.len();
    debug!(
        "Attempting to flush {} pending mempool timestamps",
        total_pending
    );

    // Convert to vectors for the query
    let tx_hashes: Vec<String> = pending.keys().cloned().collect();
    let timestamps: Vec<DateTime<Utc>> = tx_hashes
        .iter()
        .map(|hash| &pending[hash])
        .cloned()
        .collect();

    // Perform batch update using unnest for efficiency
    // This will only update transactions that:
    // 1. Exist in the database (have been mined)
    // 2. Don't already have a mempool_first_seen timestamp
    let query = r#"
        WITH updated AS (
            UPDATE eth_db.transactions t
            SET mempool_first_seen = data.timestamp
            FROM (
                SELECT unnest($1::text[]) as hash, 
                       unnest($2::timestamptz[]) as timestamp
            ) data
            WHERE t.tx_hash = data.hash
              AND t.mempool_first_seen IS NULL
            RETURNING t.tx_hash
        )
        SELECT tx_hash FROM updated
    "#;

    match sqlx::query(query)
        .bind(&tx_hashes)
        .bind(&timestamps)
        .fetch_all(db_pool)
        .await
    {
        Ok(rows) => {
            let updated_count = rows.len();
            stats.total_processed += total_pending as u64;
            stats.successful_updates += updated_count as u64;

            // Remove successfully updated transactions from pending
            for row in rows {
                if let Ok(hash) = row.try_get::<String, _>("tx_hash") {
                    pending.remove(&hash);
                }
            }

            stats.not_found = pending.len() as u64;

            if updated_count > 0 {
                info!(
                    "Updated {} mempool timestamps, {} still pending",
                    updated_count,
                    pending.len()
                );
            }
        }
        Err(e) => {
            error!("Failed to update mempool timestamps: {}", e);
            stats.errors += total_pending as u64;
        }
    }
}

/// Worker statistics
#[derive(Default)]
struct WorkerStats {
    pending_count: u64,
    total_processed: u64,
    successful_updates: u64,
    not_found: u64,
    expired: u64,
    errors: u64,
}
