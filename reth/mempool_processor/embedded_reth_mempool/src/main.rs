use embedded_reth_mempool::{EmbeddedRethConfig, EmbeddedRethListener};
use tokio::signal;
use tracing::info;

/// Integration demo showing embedded Reth library usage
/// 
/// This example demonstrates how to use the embedded_reth_mempool library
/// to achieve <50μs transaction detection latency for mempool_processor integration.
#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("embedded_reth_mempool=info,integration_demo=info")
        .init();

    info!("🚀 Starting embedded Reth integration demo...");
    info!("This demonstrates library usage for mempool_processor integration");

    // 1. Create listener with default configuration
    let config = EmbeddedRethConfig::default();
    let listener = EmbeddedRethListener::new(config).await?;
    
    // 2. Start the transaction processing loop
    listener.start_processing().await?;
    
    // 3. Subscribe to transaction stream
    let mut tx_stream = listener.subscribe();
    
    // 4. Set up graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c().await.ok();
        info!("Received shutdown signal");
    };
    
    info!("✅ Listening for transactions with <50μs latency...");
    info!("Press Ctrl+C to shutdown gracefully");
    
    // 5. Process transactions with ultra-low latency
    let mut tx_count = 0u64;
    let start_time = std::time::Instant::now();
    
    tokio::select! {
        _ = shutdown_signal => {
            info!("Shutting down gracefully...");
        }
        _ = async {
            while let Some(tx) = tx_stream.next().await {
                let processing_start = std::time::Instant::now();
                
                // This is where your signal detection logic would go
                process_transaction(&tx).await;
                
                let processing_time = processing_start.elapsed();
                
                tx_count += 1;
                if tx_count % 100 == 0 {
                    let _avg_time = start_time.elapsed().as_secs_f64() / tx_count as f64;
                    let metrics = listener.metrics().report();
                    
                    info!(
                        "Processed {} transactions | Latest: {}μs | Avg: {:.1}μs | TPS: {:.1}",
                        tx_count,
                        processing_time.as_micros(),
                        metrics.average_latency_us,
                        metrics.tps
                    );
                }
                
                // Warn if processing is getting slow
                if processing_time.as_micros() > 50 {
                    tracing::warn!(
                        "Slow processing detected: {}μs for tx {}",
                        processing_time.as_micros(),
                        tx.hash_short()
                    );
                }
            }
        } => {
            info!("Transaction stream ended");
        }
    }
    
    // 6. Shutdown with final metrics
    let final_metrics = listener.metrics().report();
    info!("Final performance metrics:\n{}", final_metrics);
    
    listener.shutdown().await?;
    
    Ok(())
}

/// Example transaction processing function
/// 
/// This simulates the kind of processing that mempool_processor
/// signal detection would perform.
async fn process_transaction(tx: &embedded_reth_mempool::EmbeddedTransaction) {
    let start = std::time::Instant::now();
    
    // Example processing (replace with your signal detection logic):
    // - Check if it's a contract interaction
    // - Analyze gas usage patterns  
    // - Detect potential scam transactions
    // - Trigger alerts or trading signals
    
    let is_interesting = tx.is_contract_interaction() && tx.gas_limit() > 100_000;
    
    let processing_time = start.elapsed();
    
    if is_interesting {
        info!(
            "📦 Interesting tx: {} | Gas: {} | Latency: {}μs | Processing: {}μs",
            tx.hash_short(),
            tx.gas_limit(),
            tx.latency_us(),
            processing_time.as_micros()
        );
        
        // Convert to mempool_processor format for integration
        let mempool_tx = embedded_reth_mempool::MempoolTransaction::from(tx);
        
        // This is where you'd call your existing signal detection
        // signal_detector.process(mempool_tx).await?;
        
        tracing::debug!("Mempool format: {:?}", mempool_tx);
    }
}