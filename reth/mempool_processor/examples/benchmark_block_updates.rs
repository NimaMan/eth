/// Benchmark block number updates from Direct Reth database
use mempool_processor::tx_simulator::DirectTxSimulator;
use std::time::Instant;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🚀 Benchmarking block number updates");
    
    // Initialize Direct simulator
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ Direct simulator initialized");
    
    // Warmup
    for _ in 0..10 {
        let _ = simulator.update_latest_block().await?;
    }
    
    // Benchmark 1000 block updates
    let iterations = 1000;
    let mut times = Vec::with_capacity(iterations);
    
    info!("⏱️  Running {} block number queries...", iterations);
    
    for i in 0..iterations {
        let start = Instant::now();
        let block = simulator.update_latest_block().await?;
        let elapsed = start.elapsed();
        times.push(elapsed.as_nanos());
        
        if i == 0 {
            info!("Current block: {}", block);
        }
    }
    
    // Calculate statistics
    times.sort();
    let avg = times.iter().sum::<u128>() / iterations as u128;
    let min = times[0];
    let max = times[times.len() - 1];
    let p50 = times[times.len() / 2];
    let p99 = times[times.len() * 99 / 100];
    
    info!("\n📊 RESULTS for {} block updates:", iterations);
    info!("   Average: {} ns ({:.3} µs)", avg, avg as f64 / 1000.0);
    info!("   Min:     {} ns ({:.3} µs)", min, min as f64 / 1000.0);
    info!("   Max:     {} ns ({:.3} µs)", max, max as f64 / 1000.0);
    info!("   P50:     {} ns ({:.3} µs)", p50, p50 as f64 / 1000.0);
    info!("   P99:     {} ns ({:.3} µs)", p99, p99 as f64 / 1000.0);
    
    let updates_per_second = 1_000_000_000.0 / avg as f64;
    info!("\n🚀 Throughput: {:.0} updates/second", updates_per_second);
    info!("   That's {:.1}x faster than a 1ms WebSocket call", 1000.0 / (avg as f64 / 1_000_000.0));
    
    // Compare with WebSocket estimate
    info!("\n📡 WebSocket comparison:");
    info!("   WebSocket eth_blockNumber: ~1-5ms (network latency)");
    info!("   Direct DB get_latest_block: ~{:.3}µs (no network)", avg as f64 / 1000.0);
    info!("   Speed improvement: {}x faster", (1000.0 * 1000.0 / avg as f64) as u64);
    
    Ok(())
}