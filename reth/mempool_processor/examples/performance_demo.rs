//! Performance Metrics Demo
//! 
//! This example demonstrates the performance tracking capabilities

use mempool_processor::performance_metrics::PerformanceTracker;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("🚀 Performance Metrics Demo");
    println!("===========================\n");
    
    // Create a tracker that logs every 5 transactions
    let tracker = PerformanceTracker::new(100, 5);
    
    println!("📊 Simulating transaction processing...\n");
    
    // Simulate processing 10 transactions with varying times
    for i in 0..10 {
        let tx_hash = format!("0x{:064x}", i);
        
        // Simulate different arrival times (some are delayed)
        let arrival_time = if i % 3 == 0 {
            // Simulate transaction that was in mempool for a while
            Instant::now() - Duration::from_millis(50)
        } else {
            Instant::now()
        };
        
        // Start tracking
        let timing = tracker.start_transaction_with_arrival(tx_hash.clone(), arrival_time).await;
        
        // Simulate queue time (varies)
        let queue_time = 2 + (i % 4);
        sleep(Duration::from_millis(queue_time)).await;
        timing.write().await.mark_processing_start();
        
        // Simulate processing time (varies)  
        let processing_time = 3 + (i % 3) * 2;
        sleep(Duration::from_millis(processing_time)).await;
        
        // Complete tracking
        tracker.complete_transaction(timing).await;
        
        println!("✅ Processed transaction {} (queue: {}ms, process: {}ms)", 
                 i + 1, queue_time, processing_time);
    }
    
    println!("\n📈 Final Performance Statistics:");
    println!("================================");
    
    let stats = tracker.get_statistics().await;
    
    println!("Stats: {:?}", stats);
    
    println!("Total transactions processed: {}", stats.total_processed);
    println!("\nQueue Time (Reth arrival → Processing start):");
    println!("  Mean: {:.2}ms", stats.queue_time_mean_ms);
    println!("  Max:  {:.2}ms", stats.queue_time_max_ms);
    
    println!("\nProcessing Time (Start → End):");
    println!("  Mean: {:.2}ms", stats.processing_time_mean_ms);
    println!("  Max:  {:.2}ms", stats.processing_time_max_ms);
    
    println!("\nTotal Time (Reth arrival → Processing end):");
    println!("  Mean: {:.2}ms", stats.total_time_mean_ms);
    println!("  Max:  {:.2}ms", stats.total_time_max_ms);
    
    println!("\n✨ Demo complete!");
}