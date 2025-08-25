/// Test example for block-time conversion system
/// 
/// Demonstrates:
/// - Block to timestamp conversion
/// - Timestamp to block conversion
/// - Period boundary calculation
/// - Time-based aggregation

use reth_chain_query::{ChainQuery, BlockTimeConverter, PeriodType};
use chrono::{Utc, Duration};
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    println!("🕐 Testing Block-Time Conversion System\n");
    println!("=" .repeat(50));
    
    // Get latest block
    let latest_block = chain_query.get_latest_block()?;
    println!("Latest block: {}", latest_block);
    
    // Test 1: Block to timestamp conversion
    println!("\n📊 Block to Timestamp Conversion:");
    println!("-" .repeat(40));
    
    let test_blocks = vec![
        latest_block,
        latest_block - 100,
        latest_block - 1000,
        latest_block - 7200,  // ~1 day ago
    ];
    
    for block in test_blocks {
        let timestamp = chain_query.time_converter
            .block_to_timestamp(block)
            .await?;
        println!("Block {} -> {}", block, timestamp.to_rfc3339());
    }
    
    // Test 2: Timestamp to block conversion
    println!("\n📊 Timestamp to Block Conversion:");
    println!("-" .repeat(40));
    
    let now = Utc::now();
    let test_times = vec![
        now,
        now - Duration::hours(1),
        now - Duration::days(1),
        now - Duration::weeks(1),
    ];
    
    for time in test_times {
        let block = chain_query.time_converter
            .timestamp_to_block(time)
            .await?;
        println!("{} -> Block {}", time.format("%Y-%m-%d %H:%M:%S"), block);
    }
    
    // Test 3: Get blocks for time periods
    println!("\n📊 Time Period Boundaries:");
    println!("-" .repeat(40));
    
    let periods = vec![
        (PeriodType::Hour, 24),    // Last 24 hours
        (PeriodType::Day, 7),       // Last 7 days
        (PeriodType::Week, 4),      // Last 4 weeks
    ];
    
    for (period_type, count) in periods {
        println!("\nLast {} {}s:", count, period_type.as_str());
        
        let boundaries = chain_query.time_converter
            .get_blocks_for_last_n_periods(count, period_type)
            .await?;
        
        for (i, boundary) in boundaries.iter().take(3).enumerate() {
            println!("  Period {}: blocks {} - {} ({} - {})",
                i + 1,
                boundary.start_block,
                boundary.end_block,
                boundary.start_time.format("%Y-%m-%d %H:%M"),
                boundary.end_time.format("%Y-%m-%d %H:%M")
            );
        }
        
        if boundaries.len() > 3 {
            println!("  ... and {} more periods", boundaries.len() - 3);
        }
    }
    
    // Test 4: Calculate time range for specific blocks
    println!("\n📊 Block Range to Time Range:");
    println!("-" .repeat(40));
    
    let start_time = now - Duration::days(7);
    let end_time = now;
    
    let (start_block, end_block) = chain_query.time_converter
        .get_blocks_for_period(start_time, end_time)
        .await?;
    
    println!("Time range: {} to {}", 
        start_time.format("%Y-%m-%d %H:%M"),
        end_time.format("%Y-%m-%d %H:%M")
    );
    println!("Block range: {} to {} ({} blocks)",
        start_block,
        end_block,
        end_block - start_block
    );
    
    // Test 5: Performance metrics
    println!("\n⚡ Performance Metrics:");
    println!("-" .repeat(40));
    
    let start = std::time::Instant::now();
    
    // Batch conversion test
    let mut timestamps = Vec::new();
    for i in 0..100 {
        let block = latest_block - (i * 100);
        let timestamp = chain_query.time_converter
            .block_to_timestamp(block)
            .await?;
        timestamps.push(timestamp);
    }
    
    let elapsed = start.elapsed();
    println!("Converted 100 blocks to timestamps in {:?}", elapsed);
    println!("Average: {:?} per conversion", elapsed / 100);
    
    // Cache effectiveness
    println!("\n📦 Cache Statistics:");
    println!("-" .repeat(40));
    
    // Do same conversions again (should hit cache)
    let start = std::time::Instant::now();
    for i in 0..100 {
        let block = latest_block - (i * 100);
        let _ = chain_query.time_converter
            .block_to_timestamp(block)
            .await?;
    }
    let cached_elapsed = start.elapsed();
    
    println!("Cached conversions: {:?} ({}x faster)",
        cached_elapsed,
        (elapsed.as_nanos() / cached_elapsed.as_nanos())
    );
    
    println!("\n✅ Block-Time Conversion Test Complete!");
    
    Ok(())
}