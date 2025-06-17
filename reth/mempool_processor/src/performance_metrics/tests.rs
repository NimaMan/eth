#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_performance_tracking_with_real_timing() {
        // Create a tracker that logs every 2 transactions
        let tracker = PerformanceTracker::new(100, 2);
        
        // Simulate processing 5 transactions
        for i in 0..5 {
            let tx_hash = format!("0x{:064x}", i);
            
            // Start tracking (simulating arrival time)
            let timing = tracker.start_transaction(tx_hash.clone()).await;
            
            // Simulate queue time (5ms)
            sleep(Duration::from_millis(5)).await;
            timing.write().await.mark_processing_start();
            
            // Simulate processing time (10ms)
            sleep(Duration::from_millis(10)).await;
            
            // Complete tracking
            tracker.complete_transaction(timing).await;
        }
        
        // Get statistics
        let stats = tracker.get_statistics().await;
        
        // Verify statistics
        assert_eq!(stats.total_processed, 5);
        assert!(stats.queue_time_mean_ms >= 5.0);
        assert!(stats.processing_time_mean_ms >= 10.0);
        assert!(stats.total_time_mean_ms >= 15.0);
        assert_eq!(stats.sample_size, 5);
        
        println!("Performance Test Results:");
        println!("  Total processed: {}", stats.total_processed);
        println!("  Queue time: {:.2}ms (mean), {:.2}ms (max)", 
                 stats.queue_time_mean_ms, stats.queue_time_max_ms);
        println!("  Processing time: {:.2}ms (mean), {:.2}ms (max)", 
                 stats.processing_time_mean_ms, stats.processing_time_max_ms);
        println!("  Total time: {:.2}ms (mean), {:.2}ms (max)", 
                 stats.total_time_mean_ms, stats.total_time_max_ms);
    }
    
    #[tokio::test]
    async fn test_performance_tracking_with_arrival_time() {
        let tracker = PerformanceTracker::new(50, 10);
        
        // Create a transaction with a past arrival time
        let past_arrival = Instant::now() - Duration::from_millis(20);
        let tx_hash = "0xdeadbeef".to_string();
        
        let timing = tracker.start_transaction_with_arrival(tx_hash, past_arrival).await;
        
        // Process immediately
        timing.write().await.mark_processing_start();
        sleep(Duration::from_millis(5)).await;
        tracker.complete_transaction(timing.clone()).await;
        
        // Check that queue time includes the 20ms delay
        let timing_data = timing.read().await;
        assert!(timing_data.queue_time_ms() >= 20.0);
        assert!(timing_data.processing_time_ms().unwrap() >= 5.0);
    }
}