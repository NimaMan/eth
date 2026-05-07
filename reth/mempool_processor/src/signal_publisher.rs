use alloy_primitives::U256;
use chrono::Utc;
use eyre::{eyre, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
/// Signal Publisher
///
/// Handles publishing of binary signals with fast ZMQ/logging and non-blocking database writes.
/// ZMQ and log writes are synchronous (fast), database writes use background tasks.
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use zmq::{Context, Socket};

use super::signal_detector::Signal;

/// Configuration for signal publishing
#[derive(Debug, Clone)]
pub struct SignalPublisherConfig {
    /// ZMQ endpoint for publishing
    pub zmq_endpoint: String,
    /// Log directory path
    pub log_dir: String,
    /// Enable database writing
    pub enable_database: bool,
    /// Database URL for optional signal persistence.
    pub database_url: Option<String>,
    /// Database channel buffer size
    pub db_channel_buffer_size: usize,
}

impl SignalPublisherConfig {
    /// Create config with the provided log directory
    pub fn with_log_dir(log_dir: &str) -> Self {
        Self {
            zmq_endpoint: "tcp://127.0.0.1:5556".to_string(),
            log_dir: log_dir.to_string(),
            enable_database: true,
            database_url: None,
            db_channel_buffer_size: 1000,
        }
    }

    /// Create config with timestamped log directory using provided base path
    pub fn with_timestamped_logs(base_dir: &str) -> Self {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let log_dir = PathBuf::from(base_dir).join(format!("signal_publisher_{}", timestamp));
        Self::with_log_dir(
            log_dir
                .to_str()
                .expect("log directory should be valid unicode path"),
        )
    }
}

impl Default for SignalPublisherConfig {
    fn default() -> Self {
        Self::with_timestamped_logs(&crate::config::mempool_log_dir_from_env())
    }
}

/// Signal publisher with fast ZMQ/logs and non-blocking DB
pub struct SignalPublisher {
    /// ZMQ socket
    zmq_socket: Socket,
    /// Log files
    log_files: LogFiles,
    /// Database writer sender (non-blocking)
    db_sender: Option<mpsc::Sender<Signal>>,
    /// Publisher statistics
    stats: Arc<PublisherStats>,
}

/// Log file handles
struct LogFiles {
    trading_enabled: std::fs::File,
    tax_signals: std::fs::File,
    liquidity_removal: std::fs::File,
    lp_approval: std::fs::File,
}

/// Publisher statistics
#[derive(Debug, Default)]
pub struct PublisherStats {
    pub total_published: std::sync::atomic::AtomicU64,
    pub trading_enabled: std::sync::atomic::AtomicU64,
    pub tax_signals: std::sync::atomic::AtomicU64,
    pub liquidity_removals: std::sync::atomic::AtomicU64,
    pub lp_approvals: std::sync::atomic::AtomicU64,
    pub scam_detections: std::sync::atomic::AtomicU64,
    pub zmq_published: std::sync::atomic::AtomicU64,
    pub logs_written: std::sync::atomic::AtomicU64,
    pub db_written: std::sync::atomic::AtomicU64,
    pub errors: std::sync::atomic::AtomicU64,
}

impl SignalPublisher {
    /// Create new signal publisher
    pub async fn new(config: SignalPublisherConfig) -> Result<Self> {
        info!("SignalPublisher::new() starting...");
        let stats = Arc::new(PublisherStats::default());

        // Setup ZMQ socket
        info!("Creating ZMQ context...");
        let context = Context::new();
        let zmq_socket = context.socket(zmq::PUB)?;
        zmq_socket.set_sndhwm(10000)?;
        zmq_socket.set_linger(0)?;
        info!("Binding ZMQ socket to {}...", config.zmq_endpoint);
        if let Err(err) = zmq_socket.bind(&config.zmq_endpoint) {
            if err.to_string().contains("Address already in use") {
                return Err(eyre!(
                    "ZMQ endpoint {} is already in use. Another signal publisher may be running. \
                     Stop the previous process or change `zmq.signal_endpoint` in the config. ({})",
                    config.zmq_endpoint,
                    err
                ));
            } else {
                return Err(eyre!(
                    "Failed to bind ZMQ endpoint {}: {}",
                    config.zmq_endpoint,
                    err
                ));
            }
        }
        info!("📡 ZMQ publisher bound to {}", config.zmq_endpoint);

        // Give ZMQ time to establish the socket (slow joiner problem)
        info!("Waiting 100ms for ZMQ socket...");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Create log directory and files
        info!("Creating log directory: {}", config.log_dir);
        std::fs::create_dir_all(&config.log_dir)?;
        let log_files = Self::create_log_files(&config.log_dir)?;
        info!("📁 Signal log directory: {}", config.log_dir);

        // Setup non-blocking database writer if enabled
        let db_sender = if config.enable_database {
            info!("Setting up database writer...");
            let (sender, receiver) = mpsc::channel(config.db_channel_buffer_size);
            info!("Spawning database writer task...");
            let database_url = config
                .database_url
                .clone()
                .unwrap_or_else(crate::db_writers::get_default_database_url);
            Self::spawn_db_writer(receiver, stats.clone(), database_url).await?;
            info!("Database writer spawned");
            Some(sender)
        } else {
            info!("Database writing disabled");
            None
        };

        info!("📡 Signal publisher initialized");

        Ok(Self {
            zmq_socket,
            log_files,
            db_sender,
            stats,
        })
    }

    /// Create log files
    fn create_log_files(log_dir: &str) -> Result<LogFiles> {
        let log_dir = PathBuf::from(log_dir);

        let trading_enabled = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("trading_enabled.log"))?;

        let tax_signals = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("tax_signals.log"))?;

        let liquidity_removal = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("liquidity_removals.log"))?;

        let lp_approval = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("lp_approval_signals.log"))?;

        Ok(LogFiles {
            trading_enabled,
            tax_signals,
            liquidity_removal,
            lp_approval,
        })
    }

    /// Spawn non-blocking database writer task
    async fn spawn_db_writer(
        mut receiver: mpsc::Receiver<Signal>,
        stats: Arc<PublisherStats>,
        database_url: String,
    ) -> Result<()> {
        // Spawn database writer task without blocking on connection
        let stats_clone = stats.clone();
        tokio::spawn(async move {
            info!("🗄️ Starting database writer task...");

            // Create unified signal writer
            let unified_writer =
                match crate::db_writers::UnifiedSignalWriter::new(&database_url).await {
                    Ok(w) => {
                        info!("✅ Unified signal writer initialized");
                        info!("  {}", w.get_status());
                        w
                    }
                    Err(e) => {
                        error!("❌ Failed to initialize unified signal writer: {}", e);
                        return;
                    }
                };

            info!("✅ Database writer task started successfully");

            while let Some(signal) = receiver.recv().await {
                // Use unified writer to handle all signal types
                if let Err(e) = unified_writer.write_signal(signal).await {
                    error!("Failed to write signal to database: {}", e);
                    stats_clone
                        .errors
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                } else {
                    stats_clone
                        .db_written
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            warn!("Database writer task terminated");
        });
        Ok(())
    }

    /// Publish a signal (fast ZMQ/logs, non-blocking DB)
    pub async fn publish(&mut self, signal: Signal) -> Result<()> {
        self.stats
            .total_published
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        match &signal {
            Signal::TradingEnabled(_) => {
                self.stats
                    .trading_enabled
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            Signal::TaxSignal(_) => {
                self.stats
                    .tax_signals
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            Signal::LiquidityRemoval(_) => {
                self.stats
                    .liquidity_removals
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            Signal::LpApproval(_) => {
                self.stats
                    .lp_approvals
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            Signal::ScamDetection(_) => {
                self.stats
                    .scam_detections
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }

        // Fast ZMQ publishing
        self.publish_zmq(&signal)?;

        // Fast log writing
        self.write_log(&signal)?;

        // Non-blocking database write
        if let Some(ref db_sender) = self.db_sender {
            if let Err(e) = db_sender.try_send(signal) {
                match e {
                    mpsc::error::TrySendError::Full(_) => {
                        warn!("Database writer channel full, dropping signal");
                        self.stats
                            .errors
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        error!("Database writer channel closed");
                        self.stats
                            .errors
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }

        Ok(())
    }

    /// Publish signal to ZMQ (fast)
    fn publish_zmq(&self, signal: &Signal) -> Result<()> {
        // Convert to topic/json; skip ScamDetection (deprecated)
        let topic_and_data: Option<(&str, String)> = match signal {
            Signal::TradingEnabled(s) => Some(("trading_enabled", serde_json::to_string(s)?)),
            Signal::TaxSignal(s) => Some(("tax_signal", serde_json::to_string(s)?)),
            Signal::LiquidityRemoval(s) => Some(("liquidity_removal", serde_json::to_string(s)?)),
            Signal::LpApproval(s) => Some(("lp_approval", serde_json::to_string(s)?)),
            Signal::ScamDetection(_) => None,
        };

        if topic_and_data.is_none() {
            return Ok(());
        }
        let (topic, json_data) = topic_and_data.unwrap();

        // Log what we're about to send
        debug!(
            "Sending ZMQ message - Topic: '{}', Data length: {} bytes",
            topic,
            json_data.len()
        );

        // Try without DONTWAIT first to ensure message is sent
        match self
            .zmq_socket
            .send_multipart(&[topic.as_bytes(), json_data.as_bytes()], 0)
        {
            Ok(_) => {
                self.stats
                    .zmq_published
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                debug!("Published {} signal to ZMQ", topic);
                Ok(())
            }
            Err(e) => {
                error!("ZMQ publish failed with error: {:?}", e);
                self.stats
                    .errors
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Err(eyre::eyre!("Failed to publish to ZMQ: {}", e))
            }
        }
    }

    /// Write signal to log files (fast)
    fn write_log(&mut self, signal: &Signal) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");

        match signal {
            Signal::TradingEnabled(s) => {
                writeln!(
                    self.log_files.trading_enabled,
                    "[{}] TRADING_ENABLED | Token: {} | Pool: {} | PoolType: {} | Creator: {} | BuyTax: {}% | SellTax: {}% | TxHash: {}",
                    timestamp, s.token_address, s.pool_address, s.pool_type, s.creator_address, s.buy_tax, s.sell_tax, s.tx_hash
                )?;
                self.log_files.trading_enabled.flush()?;
            }
            Signal::TaxSignal(s) => {
                writeln!(
                    self.log_files.tax_signals,
                    "[{}] TAX_SIGNAL | Token: {} | Pool: {} | Type: {} | BuyTax: {}% | SellTax: {}% | TxHash: {}",
                    timestamp, s.token_address, s.pool_address, s.signal_type,
                    s.buy_tax.unwrap_or(-1.0), s.sell_tax.unwrap_or(-1.0), s.tx_hash
                )?;
                self.log_files.tax_signals.flush()?;
            }
            Signal::LiquidityRemoval(s) => {
                let eth_info = s
                    .estimated_eth_removed
                    .map(|eth| format!(" | EstETH: {:.3}", eth))
                    .unwrap_or_default();
                writeln!(
                    self.log_files.liquidity_removal,
                    "[{}] LIQUIDITY_REMOVAL | Pool: {} | Function: {} | Remover: {} | TxHash: {}{}",
                    timestamp,
                    s.pool_address,
                    s.function_name,
                    s.remover_address,
                    s.tx_hash,
                    eth_info
                )?;
                self.log_files.liquidity_removal.flush()?;
            }
            // ScamDetection is deprecated: no log output
            Signal::ScamDetection(_) => {}
            Signal::LpApproval(s) => {
                let percent_str = s
                    .approval_percentage
                    .map(|pct| format!("{:.2}%", pct.min(100.0)))
                    .or_else(|| {
                        if s.amount == U256::MAX {
                            Some("100.00%".to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "N/A".to_string());

                let raw_display = if s.amount == U256::MAX {
                    "MAX".to_string()
                } else {
                    s.amount.to_string()
                };

                writeln!(
                    self.log_files.lp_approval,
                    "[{}] LP_APPROVAL | Creator: {} | LP Token: {} | Router: {} | Percent: {} | RawAmount: {} | TxHash: {}",
                    timestamp,
                    s.creator,
                    s.lp_token_address,
                    s.router_address,
                    percent_str,
                    raw_display,
                    s.tx_hash
                )?;
                self.log_files.lp_approval.flush()?;
            }
        }

        self.stats
            .logs_written
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Get publisher statistics
    pub fn get_stats(&self) -> PublisherStatsSnapshot {
        PublisherStatsSnapshot {
            total_published: self
                .stats
                .total_published
                .load(std::sync::atomic::Ordering::Relaxed),
            trading_enabled: self
                .stats
                .trading_enabled
                .load(std::sync::atomic::Ordering::Relaxed),
            tax_signals: self
                .stats
                .tax_signals
                .load(std::sync::atomic::Ordering::Relaxed),
            liquidity_removals: self
                .stats
                .liquidity_removals
                .load(std::sync::atomic::Ordering::Relaxed),
            lp_approvals: self
                .stats
                .lp_approvals
                .load(std::sync::atomic::Ordering::Relaxed),
            scam_detections: self
                .stats
                .scam_detections
                .load(std::sync::atomic::Ordering::Relaxed),
            zmq_published: self
                .stats
                .zmq_published
                .load(std::sync::atomic::Ordering::Relaxed),
            logs_written: self
                .stats
                .logs_written
                .load(std::sync::atomic::Ordering::Relaxed),
            db_written: self
                .stats
                .db_written
                .load(std::sync::atomic::Ordering::Relaxed),
            errors: self.stats.errors.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// Snapshot of publisher statistics
#[derive(Debug, Clone)]
pub struct PublisherStatsSnapshot {
    pub total_published: u64,
    pub trading_enabled: u64,
    pub tax_signals: u64,
    pub liquidity_removals: u64,
    pub lp_approvals: u64,
    pub scam_detections: u64,
    pub zmq_published: u64,
    pub logs_written: u64,
    pub db_written: u64,
    pub errors: u64,
}
