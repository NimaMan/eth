/// Example demonstrating function detector usage
/// 
/// This example shows how to:
/// 1. Create a FunctionDetector instance
/// 2. Process mempool transactions to detect function calls
/// 3. Access the detected functions from the transaction

use mempool_processor::mempool_fetcher::{NonBlockingIpcClient, SimpleFunctionDetector};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting function detector example");

    // Initialize IPC client to receive mempool transactions
    let ipc_client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start().await?;
    info!("✅ Connected to IPC");

    // Create function detector
    let function_detector = SimpleFunctionDetector::new();
    info!("✅ Function detector initialized");

    // Main processing loop - process 100 transactions
    let mut total_processed = 0u64;
    let mut functions_detected = 0u64;
    let target_count = 100u64;
    let start_time = std::time::Instant::now();

    info!("📊 Processing {} transactions to test pipeline...", target_count);

    while total_processed < target_count {
        // Get batch of transactions from IPC
        let new_txs = ipc_client.get_transactions(100).await?;
        
        if new_txs.is_empty() {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            continue;
        }

        // Process batch through function detector
        // This modifies the functions field in each transaction
        let transactions_with_functions = function_detector.detect_batch(new_txs);
        
        // Process results
        for tx in &transactions_with_functions {
            total_processed += 1;
            
            // Check if any functions were detected
            if !tx.functions.is_empty() {
                functions_detected += 1;
                
                // Log first few detected functions for visibility
                if functions_detected <= 5 {
                    info!("🎯 Transaction {} detected functions: {:?}", 
                          tx.hash, tx.functions);
                }
                
                // Example: Check for specific functions
                if tx.functions.iter().any(|f| f.contains("liquidity")) {
                    info!("⚠️  LIQUIDITY OPERATION DETECTED in tx: {}", tx.hash);
                }
                
                if tx.functions.iter().any(|f| f.contains("enableTrading") || f.contains("openTrading")) {
                    info!("🚀 TRADING ENABLED DETECTED in tx: {}", tx.hash);
                }
            }
            
            // Stop if we've processed enough
            if total_processed >= target_count {
                break;
            }
        }
        
        // Progress update every 25 transactions
        if total_processed % 25 == 0 && total_processed > 0 {
            info!("Progress: {}/{} transactions processed...", total_processed, target_count);
        }
    }
    
    // Final statistics
    let elapsed = start_time.elapsed();
    info!("\n📊 === Final Statistics ===");
    info!("✅ Total transactions processed: {}", total_processed);
    info!("🎯 Transactions with functions: {} ({:.1}%)", 
          functions_detected,
          (functions_detected as f64 / total_processed as f64) * 100.0);
    info!("⏱️  Time elapsed: {:.2}s", elapsed.as_secs_f64());
    info!("⚡ Processing rate: {:.0} tx/sec", total_processed as f64 / elapsed.as_secs_f64());
    
    Ok(())
}