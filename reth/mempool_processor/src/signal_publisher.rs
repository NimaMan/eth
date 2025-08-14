/// Signal Publisher
/// 
/// Handles publishing of binary signals with fast ZMQ/logging and non-blocking database writes.
/// ZMQ and log writes are synchronous (fast), database writes use background tasks.

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use zmq::{Context, Socket};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chrono::Utc;
use eyre::Result;

use super::signal_detector::Signal;
// Database imports disabled for now
// use crate::db_writers::{TradingEventWriter, TradingEnabledEvent, CreatorActionEvent};

/// Configuration for signal publishing
#[derive(Debug, Clone)]
pub struct SignalPublisherConfig {
    /// ZMQ endpoint for publishing
    pub zmq_endpoint: String,
    /// Log directory path
    pub log_dir: String,
    /// Enable database writing
    pub enable_database: bool,
    /// Database channel buffer size
    pub db_channel_buffer_size: usize,
}

impl SignalPublisherConfig {
    /// Create config with the provided log directory
    pub fn with_log_dir(log_dir: &str) -> Self {
        Self {
            zmq_endpoint: "tcp://127.0.0.1:5556".to_string(),
            log_dir: log_dir.to_string(),
            enable_database: false,
            db_channel_buffer_size: 1000,
        }
    }
    
    /// Create config with timestamped log directory using provided base path
    pub fn with_timestamped_logs(base_dir: &str) -> Self {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let log_dir = format!("{}/signal_publisher_{}", base_dir, timestamp);
        Self::with_log_dir(&log_dir)
    }
}

impl Default for SignalPublisherConfig {
    fn default() -> Self {
        Self::with_timestamped_logs("/home/nima/code/crypto/logs/mempool")
    }
}

/// Signal publisher with fast ZMQ/logs and non-blocking DB
pub struct SignalPublisher {
    /// Configuration
    config: SignalPublisherConfig,
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
    scam_detection: std::fs::File,
}

/// Publisher statistics
#[derive(Debug, Default)]
pub struct PublisherStats {
    pub total_published: std::sync::atomic::AtomicU64,
    pub zmq_published: std::sync::atomic::AtomicU64,
    pub logs_written: std::sync::atomic::AtomicU64,
    pub db_written: std::sync::atomic::AtomicU64,
    pub errors: std::sync::atomic::AtomicU64,
}

impl SignalPublisher {
    /// Create new signal publisher
    pub async fn new(config: SignalPublisherConfig) -> Result<Self> {
        let stats = Arc::new(PublisherStats::default());
        
        // Setup ZMQ socket
        let context = Context::new();
        let zmq_socket = context.socket(zmq::PUB)?;
        zmq_socket.set_sndhwm(10000)?;
        zmq_socket.set_linger(0)?;
        zmq_socket.bind(&config.zmq_endpoint)?;
        info!("📡 ZMQ publisher bound to {}", config.zmq_endpoint);
        
        // Give ZMQ time to establish the socket (slow joiner problem)
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Create log directory and files
        std::fs::create_dir_all(&config.log_dir)?;
        let log_files = Self::create_log_files(&config.log_dir)?;
        info!("📁 Signal log directory: {}", config.log_dir);
        
        // Setup non-blocking database writer if enabled
        let db_sender = if config.enable_database {
            let (sender, receiver) = mpsc::channel(config.db_channel_buffer_size);
            // Use hardcoded database URL from database module
            let db_url = Some(crate::db_writers::get_default_database_url());
            Self::spawn_db_writer(db_url, receiver, stats.clone()).await?;
            Some(sender)
        } else {
            None
        };
        
        info!("📡 Signal publisher initialized");
        
        Ok(Self {
            config,
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
        
        let scam_detection = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("scam_detections.log"))?;
        
        Ok(LogFiles {
            trading_enabled,
            tax_signals,
            liquidity_removal,
            scam_detection,
        })
    }
    
    /// Spawn non-blocking database writer task
    async fn spawn_db_writer(
        database_url: Option<String>, 
        mut receiver: mpsc::Receiver<Signal>,
        stats: Arc<PublisherStats>
    ) -> Result<()> {
        if let Some(db_url) = database_url {
            // Spawn database writer task without blocking on connection
            let stats_clone = stats.clone();
            tokio::spawn(async move {
                info!("🗄️ Starting database writer task...");
                
                // Try to create the signal writers
                let signal_writer_config = crate::db_writers::SignalWriterConfig::default();
                let trading_writer = match crate::db_writers::TradingSignalWriter::new_with_defaults(signal_writer_config).await {
                    Ok(w) => Some(w),
                    Err(e) => {
                        error!("Failed to create trading signal writer: {}", e);
                        None
                    }
                };
                
                let tax_writer = match crate::db_writers::TaxSignalWriter::new(&db_url, 50, std::time::Duration::from_secs(5)).await {
                    Ok(w) => Some(w),
                    Err(e) => {
                        error!("Failed to create tax signal writer: {}", e);
                        None
                    }
                };
                
                if trading_writer.is_none() && tax_writer.is_none() {
                    error!("❌ Database writers failed to initialize - database writing disabled");
                    return;
                }
                
                info!("✅ Database writer task started successfully");
                
                while let Some(signal) = receiver.recv().await {
                    // Convert signal to database record and write
                    match signal {
                        Signal::TradingEnabled(ref s) => {
                            if let Some(ref writer) = trading_writer {
                                
                                
                                let record = crate::db_writers::TradingSignalRecord {
                                token_address: s.token_address.clone(),
                                pool_address: s.pool_address.clone(),
                                pool_type: s.pool_type.clone(),
                                denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
                                denom_currency: Some("WETH".to_string()),
                                detection_timestamp: chrono::Utc::now(),
                                detection_tx_hash: s.tx_hash.clone(),
                                price_ratio: None,
                                denom_reserve_at_signal: None,
                                token_reserve_at_signal: None,
                                buy_tax_at_signal: Some(rust_decimal::Decimal::from_f64_retain(s.buy_tax).unwrap_or_default()),
                                sell_tax_at_signal: Some(rust_decimal::Decimal::from_f64_retain(s.sell_tax).unwrap_or_default()),
                                total_supply: None,
                                owner_address: None,
                                creator_address: s.creator_address.clone(),
                                signal_source: "mempool".to_string(),
                            };
                            
                                writer.write_signal(record).await;
                                stats_clone.db_written.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                debug!("Written trading_enabled signal to database");
                            }
                        }
                        Signal::TaxSignal(ref s) => {
                            let record = crate::db_writers::TaxSignalRecord {
                                token_address: s.token_address.clone(),
                                pool_address: s.pool_address.clone(),
                                pool_type: s.pool_type.clone(),
                                denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
                                denom_currency: Some("WETH".to_string()),
                                detection_timestamp: chrono::Utc::now(),
                                detection_tx_hash: s.tx_hash.clone(),
                                signal_type: s.signal_type.clone(),
                                signal_details: s.signal_details.clone(),
                                confidence: Some(rust_decimal::Decimal::from_f64_retain(s.confidence).unwrap_or_default()),
                                buy_tax_at_signal: s.buy_tax.map(|tax| rust_decimal::Decimal::from_f64_retain(tax).unwrap_or_default()),
                                sell_tax_at_signal: s.sell_tax.map(|tax| rust_decimal::Decimal::from_f64_retain(tax).unwrap_or_default()),
                                buy_tax_exceeds_threshold: s.buy_tax_exceeds_threshold,
                                sell_tax_exceeds_threshold: s.sell_tax_exceeds_threshold,
                                cant_sell: s.cant_sell,
                                creator_address: s.creator_address.clone(),
                                signal_source: "mempool".to_string(),
                            };
                            
                            if let Some(ref writer) = tax_writer {
                                if let Err(e) = writer.write_signal(record) {
                                    error!("Failed to write tax signal to database: {}", e);
                                } else {
                                    stats_clone.db_written.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    debug!("Written tax_signal to database");
                                }
                            }
                        }
                        _ => {
                            // TODO: Handle other signal types
                            debug!("Signal type not yet supported for database writing: {:?}", signal);
                        }
                    }
                }
                warn!("Database writer task terminated");
            });
        }
        Ok(())
    }
    
    /*
    /// Write a signal to the database (disabled for now)
    async fn write_signal_to_database(db_writer: &mut TradingEventWriter, signal: &Signal) -> Result<()> {
        // Database writing disabled - implement when needed
        Ok(())
    }
    */
    
    /// Publish a signal (fast ZMQ/logs, non-blocking DB)
    pub async fn publish(&mut self, signal: Signal) -> Result<()> {
        self.stats.total_published.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
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
                        self.stats.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        error!("Database writer channel closed");
                        self.stats.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Publish signal to ZMQ (fast)
    fn publish_zmq(&self, signal: &Signal) -> Result<()> {
        let (topic, json_data) = match signal {
            Signal::TradingEnabled(s) => ("trading_enabled", serde_json::to_string(s)?),
            Signal::TaxSignal(s) => ("tax_signal", serde_json::to_string(s)?),
            Signal::LiquidityRemoval(s) => ("liquidity_removal", serde_json::to_string(s)?),
            Signal::ScamDetection(s) => ("scam_detection", serde_json::to_string(s)?),
        };
        
        // Log what we're about to send
        info!("Sending ZMQ message - Topic: '{}', Data length: {} bytes", topic, json_data.len());
        
        // Try without DONTWAIT first to ensure message is sent
        match self.zmq_socket.send_multipart(&[topic.as_bytes(), json_data.as_bytes()], 0) {
            Ok(_) => {
                self.stats.zmq_published.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                info!("✅ Successfully published {} signal to ZMQ", topic);
                Ok(())
            }
            Err(e) => {
                error!("ZMQ publish failed with error: {:?}", e);
                self.stats.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
                    "[{}] TRADING_ENABLED | Token: {} | Creator: {} | BuyTax: {}% | SellTax: {}% | TxHash: {}",
                    timestamp, s.token_address, s.creator_address, s.buy_tax, s.sell_tax, s.tx_hash
                )?;
                writeln!(self.log_files.trading_enabled, "")?; // Add empty line for readability
                self.log_files.trading_enabled.flush()?;
            }
            Signal::TaxSignal(s) => {
                writeln!(
                    self.log_files.tax_signals,
                    "[{}] TAX_SIGNAL | Token: {} | Pool: {} | Type: {} | BuyTax: {}% | SellTax: {}% | TxHash: {}",
                    timestamp, s.token_address, s.pool_address, s.signal_type,
                    s.buy_tax.unwrap_or(-1.0), s.sell_tax.unwrap_or(-1.0), s.tx_hash
                )?;
                writeln!(self.log_files.tax_signals, "")?; // Add empty line for readability
                self.log_files.tax_signals.flush()?;
            }
            Signal::LiquidityRemoval(s) => {
                let eth_info = s.estimated_eth_removed
                    .map(|eth| format!(" | EstETH: {:.3}", eth))
                    .unwrap_or_default();
                writeln!(
                    self.log_files.liquidity_removal,
                    "[{}] LIQUIDITY_REMOVAL | Pool: {} | Function: {} | Remover: {} | TxHash: {}{}",
                    timestamp, s.pool_address, s.function_name, s.remover_address, s.tx_hash, eth_info
                )?;
                writeln!(self.log_files.liquidity_removal, "")?; // Add empty line for readability
                self.log_files.liquidity_removal.flush()?;
            }
            Signal::ScamDetection(s) => {
                writeln!(
                    self.log_files.scam_detection,
                    "[{}] SCAM_DETECTED | Pool: {} | Token: {} | Scammer: {} | Drained: {:.2} ETH ({:.1}%) | Remaining: {:.2} ETH | TxHash: {}",
                    timestamp, s.pool_address, s.token_address, s.scammer_address, 
                    s.eth_drained, s.drain_percentage, s.eth_remaining, s.tx_hash
                )?;
                writeln!(self.log_files.scam_detection, "")?; // Add empty line for readability
                self.log_files.scam_detection.flush()?;
            }
        }
        
        self.stats.logs_written.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    /// Get publisher statistics
    pub fn get_stats(&self) -> PublisherStatsSnapshot {
        PublisherStatsSnapshot {
            total_published: self.stats.total_published.load(std::sync::atomic::Ordering::Relaxed),
            zmq_published: self.stats.zmq_published.load(std::sync::atomic::Ordering::Relaxed),
            logs_written: self.stats.logs_written.load(std::sync::atomic::Ordering::Relaxed),
            db_written: self.stats.db_written.load(std::sync::atomic::Ordering::Relaxed),
            errors: self.stats.errors.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// Snapshot of publisher statistics
#[derive(Debug, Clone)]
pub struct PublisherStatsSnapshot {
    pub total_published: u64,
    pub zmq_published: u64,
    pub logs_written: u64,
    pub db_written: u64,
    pub errors: u64,
}