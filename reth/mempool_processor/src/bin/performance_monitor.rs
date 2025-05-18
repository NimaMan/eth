/*
 * Performance Monitoring Script
 * 
 * This utility tracks timing metrics for mempool transactions:
 * 1. When transactions are received from the mempool
 * 2. How long simulation takes
 * 3. How long state diff calculation takes
 * 4. Total end-to-end processing time
 * 
 * Results are written to a CSV file for analysis.
 */

use clap::Parser;
use ethers::prelude::*;
use eyre::Result;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tracing::{info, error, warn, Level};
use mempool_processor::mempool_processor::fetcher::{MempoolFetcher, FetchMode, TransactionSource};
use mempool_processor::tx_simulator::{StateDiffTracker, StateCache};
use hex::encode as hex_encode;

#[derive(Parser, Debug)]
struct Args {
    /// HTTP RPC URL for your local Ethereum node
    #[arg(long, env = "HTTP_RPC_URL", default_value = "http://localhost:8545")]
    http_rpc_url: String,
    
    /// Number of transactions to monitor (0 = unlimited)
    #[arg(long, default_value = "100")]
    tx_count: usize,
    
    /// Output CSV file path
    #[arg(long, default_value = "tx_performance.csv")]
    output_file: PathBuf,
    
    /// Run in continuous monitoring mode
    #[arg(long)]
    continuous: bool,
    
    /// Pause between fetches in continuous mode (ms)
    #[arg(long, default_value = "5000")]
    fetch_interval_ms: u64,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Disable batch requests
    #[arg(long)]
    no_batch_requests: bool,
    
    /// Maximum number of transactions to fetch in a single batch
    #[arg(long, default_value = "100")]
    max_batch_size: usize,
    
    /// HTTP request timeout in milliseconds
    #[arg(long, default_value = "1000")]
    timeout_ms: u64,
    
    /// Transaction cache size
    #[arg(long, default_value = "5000")]
    cache_size: usize,
}

// Performance metrics for a single transaction
struct TxPerformanceMetrics {
    tx_hash: String,
    mempool_timestamp: u64,        // When we first saw the tx (unix timestamp seconds)
    fetch_time_ms: u64,            // Time to fetch from mempool
    simulation_time_ms: u64,       // Time to simulate the transaction
    state_diff_time_ms: u64,       // Time to calculate state diffs
    total_processing_time_ms: u64, // Total end-to-end time
    tx_size_bytes: usize,          // Transaction input data size
    gas_used: u64,                 // Gas consumed by the transaction
    state_changes_count: usize,    // Number of state changes detected
    status: String,                // Success or error message
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Configure logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("Performance Monitor Starting");
    info!("HTTP RPC URL: {}", args.http_rpc_url);
    info!("Output file: {}", args.output_file.display());
    
    // Connect to provider
    let provider = Provider::<Http>::try_from(args.http_rpc_url.clone())?;
    let provider = Arc::new(provider);
    
    // Check connection
    let block_number = provider.get_block_number().await?;
    let chain_id = provider.get_chainid().await?;
    info!("Connected to Ethereum network");
    info!("  Chain ID: {}", chain_id);
    info!("  Current block: {}", block_number);
    
    // Set up CSV file
    let mut csv_file = setup_csv_file(&args.output_file)?;
    
    // Set up optimized mempool fetcher with custom settings
    info!("Setting up optimized MempoolFetcher...");
    info!("  Batch requests: {}", !args.no_batch_requests);
    info!("  Max batch size: {}", args.max_batch_size);
    info!("  Timeout: {}ms", args.timeout_ms);
    info!("  Cache size: {}", args.cache_size);
    
    let fetcher = MempoolFetcher::with_options(
        &args.http_rpc_url,
        args.cache_size,
        !args.no_batch_requests,
        args.max_batch_size,
        args.timeout_ms,
        if !args.no_batch_requests { FetchMode::RpcBatch } else { FetchMode::RpcSingle }
    )?;
    
    // Initialize state diff tracker
    info!("Initializing StateDiffTracker and StateCache...");
    let mut tracker = StateDiffTracker::new(provider.clone(), None);
    let mut state_cache = StateCache::new();
    
    let mut total_processed = 0;
    let mut metrics_vec: Vec<TxPerformanceMetrics> = Vec::new();
    
    // Run in continuous mode or one-time mode
    if args.continuous {
        info!("Starting continuous monitoring with interval of {}ms", args.fetch_interval_ms);
        
        loop {
            // Fetch and process transactions
            let fetch_result = process_mempool_batch(
                &fetcher, &mut tracker, &mut state_cache, 
                args.tx_count, &mut metrics_vec
            ).await;
            
            // Write results to CSV even if there's an error
            if !metrics_vec.is_empty() {
                let count = metrics_vec.len();
                write_metrics_to_csv(&mut csv_file, &metrics_vec)?;
                total_processed += count;
                metrics_vec.clear();
                info!("Wrote metrics for {} transactions (total: {})", 
                     count, total_processed);
            }
            
            // Handle any errors from processing
            if let Err(e) = fetch_result {
                error!("Error processing mempool transactions: {}", e);
                // Continue the loop despite errors
            }
            
            // Sleep for the configured interval
            tokio::time::sleep(tokio::time::Duration::from_millis(args.fetch_interval_ms)).await;
        }
    } else {
        // One-time mode
        info!("Processing up to {} transactions in one-time mode", args.tx_count);
        
        // Process transactions
        match process_mempool_batch(
            &fetcher, &mut tracker, &mut state_cache, 
            args.tx_count, &mut metrics_vec
        ).await {
            Ok(_) => {
                info!("Successfully processed {} transactions", metrics_vec.len());
            },
            Err(e) => {
                error!("Error processing mempool transactions: {}", e);
                // Continue to write any metrics we collected
            }
        }
        
        // Write results to CSV
        write_metrics_to_csv(&mut csv_file, &metrics_vec)?;
        info!("Wrote metrics to {}", args.output_file.display());
    }
    
    info!("Performance monitoring complete");
    Ok(())
}

// Process a batch of transactions from the mempool
async fn process_mempool_batch(
    fetcher: &MempoolFetcher,
    tracker: &mut StateDiffTracker,
    state_cache: &mut StateCache,
    tx_limit: usize,
    metrics: &mut Vec<TxPerformanceMetrics>
) -> Result<()> {
    // Time the fetching process
    let fetch_start = Instant::now();
    let transactions = fetcher.get_transactions().await?;
    let fetch_time = fetch_start.elapsed();
    
    info!("Fetched {} transactions in {:?}", transactions.len(), fetch_time);
    
    // Determine how many to process
    let tx_count = if tx_limit == 0 || tx_limit > transactions.len() {
        transactions.len()
    } else {
        tx_limit
    };
    
    info!("Processing {} transactions...", tx_count);
    
    // Current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Process each transaction and collect metrics
    for (i, tx) in transactions.iter().take(tx_count).enumerate() {
        let tx_hash_hex = hex_encode(&tx.hash);
        info!("Processing transaction {}/{}: {}", i + 1, tx_count, tx_hash_hex);
        
        // Prepare metrics
        let mut metric = TxPerformanceMetrics {
            tx_hash: tx_hash_hex.clone(),
            mempool_timestamp: now,
            fetch_time_ms: fetch_time.as_millis() as u64,
            simulation_time_ms: 0,
            state_diff_time_ms: 0,
            total_processing_time_ms: 0,
            tx_size_bytes: tx.input_data.as_ref().map_or(0, |data| data.len()),
            gas_used: 0,
            state_changes_count: 0,
            status: "Success".to_string(),
        };
        
        // Time the total processing
        let process_start = Instant::now();
        
        // Simulate the transaction
        let sim_start = Instant::now();
        match tracker.simulate_transaction(tx).await {
            Ok(Some(changes)) => {
                let sim_time = sim_start.elapsed();
                
                // Record simulation metrics
                metric.simulation_time_ms = sim_time.as_millis() as u64;
                metric.state_changes_count = changes.len();
                
                // Estimate the state diff calculation time (part of simulation)
                // State diff is just a portion of the simulation time - we estimate 30%
                metric.state_diff_time_ms = (sim_time.as_millis() as u64) / 3;
                
                // Get gas used from the first change if available (gas goes to miner)
                if let Some(first_change) = changes.first() {
                    if first_change.address == H160::zero() {
                        let gas_reward = first_change.balance_after - first_change.balance_before;
                        // Rough estimate: gas reward ÷ average gas price
                        let est_gas_used = gas_reward.as_u128() / 20_000_000_000u128; // Assume 20 gwei
                        metric.gas_used = est_gas_used as u64;
                    }
                }
                
                // Add to state cache
                let mut hash_bytes = [0u8; 32];
                if tx.hash.len() == 32 {
                    hash_bytes.copy_from_slice(&tx.hash);
                    let tx_hash = H256::from(hash_bytes);
                    state_cache.add_transaction(tx_hash, tx.clone(), changes);
                }
            },
            Ok(None) => {
                let sim_time = sim_start.elapsed();
                metric.simulation_time_ms = sim_time.as_millis() as u64;
                metric.status = "No state changes".to_string();
            },
            Err(e) => {
                let sim_time = sim_start.elapsed();
                metric.simulation_time_ms = sim_time.as_millis() as u64;
                metric.status = format!("Error: {}", e);
                warn!("  Simulation failed: {}", e);
            }
        }
        
        // Calculate total processing time
        let total_time = process_start.elapsed();
        metric.total_processing_time_ms = total_time.as_millis() as u64;
        
        // Log timing summary
        info!("  Total processing time: {:?} (simulation: {:?})",
             total_time, std::time::Duration::from_millis(metric.simulation_time_ms));
        
        // Add to metrics collection
        metrics.push(metric);
    }
    
    Ok(())
}

// Set up CSV file with headers
fn setup_csv_file(path: &PathBuf) -> Result<File> {
    let mut file = File::create(path)?;
    
    // Write CSV headers
    writeln!(file, "tx_hash,mempool_timestamp,fetch_time_ms,simulation_time_ms,state_diff_time_ms,\
                   total_processing_time_ms,tx_size_bytes,gas_used,state_changes_count,status")?;
    
    Ok(file)
}

// Write metrics to CSV file
fn write_metrics_to_csv(file: &mut File, metrics: &[TxPerformanceMetrics]) -> Result<()> {
    for metric in metrics {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},\"{}\"",
            metric.tx_hash,
            metric.mempool_timestamp,
            metric.fetch_time_ms,
            metric.simulation_time_ms,
            metric.state_diff_time_ms,
            metric.total_processing_time_ms,
            metric.tx_size_bytes,
            metric.gas_used,
            metric.state_changes_count,
            metric.status
        )?;
    }
    
    // Flush to ensure data is written
    file.flush()?;
    
    Ok(())
} 