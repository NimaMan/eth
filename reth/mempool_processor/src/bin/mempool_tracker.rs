/*!
 * Improved Mempool Coverage Tracker
 * 
 * Tracks ALL transactions entering the mempool for coverage analysis.
 * ALSO records the block range during tracking for accurate coverage analysis.
 * 
 * Records: tx_hash, arrival_time, processing_time, gas_price, value, AND block range
 * 
 * Usage: cargo run --bin improved_mempool_tracker -- --duration 600
 */

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::info;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use csv::Writer;
use std::fs::File;
use std::path::Path;
use ethers::prelude::*;
use ethers::providers::{Provider, Http, Ws};
use clap::Parser;
use eyre::Result;

#[derive(Parser)]
#[command(name = "improved_mempool_tracker")]
#[command(about = "Track mempool transactions with block range for accurate coverage analysis")]
struct Args {
    /// Duration to track in seconds (default: 600 = 10 minutes)
    #[arg(short, long, default_value = "600")]
    duration: u64,
    
    /// HTTP RPC endpoint
    #[arg(long, default_value = "http://127.0.0.1:8545")]
    http_rpc: String,
    
    /// WebSocket RPC endpoint  
    #[arg(long, default_value = "ws://127.0.0.1:8546")]
    ws_rpc: String,
    
    /// Output directory
    #[arg(short, long, default_value = "examples/mempool_coverage_analysis")]
    output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolTransaction {
    pub tx_hash: String,
    pub mempool_arrival_time: f64,
    pub processing_time_ms: f64,
    pub block_number: Option<u64>, // Block when first seen (should be None for mempool)
    pub gas_price: Option<u64>,
    pub gas_limit: u64,
    pub value: String,
    pub from_address: String,
    pub to_address: Option<String>,
    pub nonce: u64,
    pub transaction_type: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingMetadata {
    pub start_time: f64,
    pub end_time: f64,
    pub start_block: u64,
    pub end_block: u64,
    pub total_transactions: usize,
    pub tracking_duration_seconds: u64,
}

pub struct MempoolTracker {
    http_provider: Provider<Http>,
    ws_provider: Provider<Ws>,
    mempool_transactions: Arc<RwLock<HashMap<String, MempoolTransaction>>>,
    total_tracked: Arc<RwLock<usize>>,
    processing_times: Arc<RwLock<Vec<f64>>>,
    start_time: Instant,
    start_block: Arc<RwLock<Option<u64>>>,
    end_block: Arc<RwLock<Option<u64>>>,
}

impl MempoolTracker {
    pub async fn new(http_rpc: &str, ws_rpc: &str) -> Result<Self> {
        let http_provider = Provider::<Http>::try_from(http_rpc)?;
        let ws_provider = Provider::<Ws>::connect(ws_rpc).await?;
        
        Ok(Self {
            http_provider,
            ws_provider,
            mempool_transactions: Arc::new(RwLock::new(HashMap::new())),
            total_tracked: Arc::new(RwLock::new(0)),
            processing_times: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            start_block: Arc::new(RwLock::new(None)),
            end_block: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Start tracking mempool transactions
    pub async fn start_tracking(&self, duration_seconds: u64) -> Result<()> {
        info!("🎯 Starting improved mempool tracking for {} seconds", duration_seconds);
        
        // Record starting block
        let current_block = self.http_provider.get_block_number().await?;
        {
            let mut start_block = self.start_block.write().await;
            *start_block = Some(current_block.as_u64());
            info!("📍 Starting at block: {}", current_block);
        }
        
        // Create subscription to pending transactions
        let mut stream = self.ws_provider.subscribe_pending_txs().await?;
        
        // Set up periodic statistics reporting
        let stats_interval = duration_seconds / 10; // Report every 10% of duration
        let mut stats_timer = interval(Duration::from_secs(stats_interval.max(30)));
        
        let tracking_duration = Duration::from_secs(duration_seconds);
        let end_time = Instant::now() + tracking_duration;
        
        info!("⏱️  Tracking will end at: {:?}", end_time);
        info!("📊 Statistics will be reported every {} seconds", stats_interval);
        
        // Start tracking loop
        loop {
            tokio::select! {
                // Handle new pending transaction
                Some(tx_hash) = stream.next() => {
                    let process_start = Instant::now();
                    
                    // Get full transaction details
                    if let Ok(Some(tx)) = self.http_provider.get_transaction(tx_hash).await {
                        let processing_time = process_start.elapsed().as_secs_f64() * 1000.0;
                        
                        let mempool_tx = MempoolTransaction {
                            tx_hash: format!("{:?}", tx_hash),
                            mempool_arrival_time: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs_f64(),
                            processing_time_ms: processing_time,
                            block_number: tx.block_number.map(|n| n.as_u64()),
                            gas_price: tx.gas_price.map(|p| p.as_u64()),
                            gas_limit: tx.gas.as_u64(),
                            value: tx.value.to_string(),
                            from_address: format!("{:?}", tx.from),
                            to_address: tx.to.map(|addr| format!("{:?}", addr)),
                            nonce: tx.nonce.as_u64(),
                            transaction_type: tx.transaction_type.unwrap_or(U64::zero()).as_u64() as u8,
                        };
                        
                        // Store transaction
                        {
                            let mut transactions = self.mempool_transactions.write().await;
                            transactions.insert(mempool_tx.tx_hash.clone(), mempool_tx);
                        }
                        
                        // Update statistics
                        {
                            let mut total = self.total_tracked.write().await;
                            *total += 1;
                            
                            let mut times = self.processing_times.write().await;
                            times.push(processing_time);
                        }
                    }
                }
                
                // Periodic statistics reporting
                _ = stats_timer.tick() => {
                    self.report_statistics().await;
                }
                
                // Check if tracking duration is complete
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(end_time)) => {
                    info!("⏰ Tracking duration complete");
                    break;
                }
            }
        }
        
        // Record ending block
        let current_block = self.http_provider.get_block_number().await?;
        {
            let mut end_block = self.end_block.write().await;
            *end_block = Some(current_block.as_u64());
            info!("📍 Ending at block: {}", current_block);
        }
        
        // Final statistics and data export
        self.report_final_statistics().await?;
        Ok(())
    }
    
    /// Report current tracking statistics
    async fn report_statistics(&self) {
        let total = *self.total_tracked.read().await;
        let elapsed = self.start_time.elapsed().as_secs();
        let rate = if elapsed > 0 { total as f64 / elapsed as f64 } else { 0.0 };
        
        let processing_times = self.processing_times.read().await;
        let avg_processing = if !processing_times.is_empty() {
            processing_times.iter().sum::<f64>() / processing_times.len() as f64
        } else {
            0.0
        };
        
        info!("📊 TRACKING STATISTICS");
        info!("   ⏱️  Elapsed: {}s", elapsed);
        info!("   📦 Total Tracked: {}", total);
        info!("   📈 Rate: {:.1} TPS", rate);
        info!("   ⚡ Avg Processing: {:.2}ms", avg_processing);
    }
    
    /// Report final statistics and save data
    async fn report_final_statistics(&self) -> Result<()> {
        let total = *self.total_tracked.read().await;
        let elapsed = self.start_time.elapsed().as_secs();
        let rate = if elapsed > 0 { total as f64 / elapsed as f64 } else { 0.0 };
        
        let processing_times = self.processing_times.read().await;
        let avg_processing = if !processing_times.is_empty() {
            processing_times.iter().sum::<f64>() / processing_times.len() as f64
        } else {
            0.0
        };
        
        let start_block = self.start_block.read().await.unwrap_or(0);
        let end_block = self.end_block.read().await.unwrap_or(0);
        let block_range = end_block - start_block + 1;
        
        info!("🎉 FINAL TRACKING RESULTS");
        info!("=========================");
        info!("⏱️  Total Duration: {}s", elapsed);
        info!("📦 Total Transactions: {}", total);
        info!("📈 Average Rate: {:.1} TPS", rate);
        info!("⚡ Average Processing: {:.2}ms", avg_processing);
        info!("📍 Block Range: {} → {} ({} blocks)", start_block, end_block, block_range);
        
        // Save transaction data to CSV
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let tx_filename = format!("{}/mempool_transactions_{}.csv", "examples/mempool_coverage_analysis", timestamp);
        let metadata_filename = format!("{}/tracking_metadata_{}.json", "examples/mempool_coverage_analysis", timestamp);
        
        self.save_to_csv(&tx_filename).await?;
        self.save_metadata(&metadata_filename, elapsed).await?;
        
        info!("💾 Transaction data saved to: {}", tx_filename);
        info!("📋 Metadata saved to: {}", metadata_filename);
        
        Ok(())
    }
    
    /// Save tracking metadata to JSON
    async fn save_metadata(&self, filename: &str, duration: u64) -> Result<()> {
        let metadata = TrackingMetadata {
            start_time: (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - duration) as f64,
            end_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as f64,
            start_block: self.start_block.read().await.unwrap_or(0),
            end_block: self.end_block.read().await.unwrap_or(0),
            total_transactions: *self.total_tracked.read().await,
            tracking_duration_seconds: duration,
        };
        
        let json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(filename, json)?;
        
        Ok(())
    }
    
    /// Save tracked transactions to CSV
    async fn save_to_csv(&self, filename: &str) -> Result<()> {
        // Create output directory if it doesn't exist
        if let Some(parent) = Path::new(filename).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = File::create(filename)?;
        let mut writer = Writer::from_writer(file);
        
        // Write CSV header
        writer.write_record(&[
            "tx_hash",
            "mempool_arrival_time", 
            "processing_time_ms",
            "block_number",
            "gas_price",
            "gas_limit", 
            "value",
            "from_address",
            "to_address",
            "nonce",
            "transaction_type"
        ])?;
        
        // Write transaction data
        let transactions = self.mempool_transactions.read().await;
        for tx in transactions.values() {
            writer.write_record(&[
                &tx.tx_hash,
                &tx.mempool_arrival_time.to_string(),
                &tx.processing_time_ms.to_string(),
                &tx.block_number.map(|n| n.to_string()).unwrap_or_default(),
                &tx.gas_price.map(|p| p.to_string()).unwrap_or_default(),
                &tx.gas_limit.to_string(),
                &tx.value,
                &tx.from_address,
                &tx.to_address.as_ref().unwrap_or(&String::new()),
                &tx.nonce.to_string(),
                &tx.transaction_type.to_string(),
            ])?;
        }
        
        writer.flush()?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let args = Args::parse();
    
    info!("🚀 Improved Mempool Coverage Tracker Starting");
    info!("⏱️  Duration: {} seconds", args.duration);
    info!("📂 Output: {}", args.output);
    
    // Create mempool tracker
    let tracker = MempoolTracker::new(&args.http_rpc, &args.ws_rpc).await?;
    
    // Start tracking
    tracker.start_tracking(args.duration).await?;
    
    info!("✅ Mempool tracking complete");
    Ok(())
}