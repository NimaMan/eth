/// Calculate PnL Metrics Example
/// 
/// Demonstrates batch calculation of profit/loss metrics
/// for addresses based on their trading activity.

use reth_chain_query::postgres_db::{PostgresQuery, queries};
use eyre::Result;
use std::env;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // Get database URL from environment or use default
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eth_db".to_string());
    
    println!("Connecting to PostgreSQL database...");
    let pg_query = PostgresQuery::new(&database_url).await?;
    
    // Get batch size from environment
    let batch_size = env::var("BATCH_SIZE")
        .unwrap_or_else(|_| "1000".to_string())
        .parse::<i64>()
        .unwrap_or(1000);
    
    println!("Using batch size: {}", batch_size);
    println!();
    
    // Get total number of unique addresses with trades
    let progress = queries::aggregation::get_aggregation_progress(pg_query.db()).await?;
    let total_addresses = progress.addresses_with_trades;
    
    println!("=== PnL Calculation Task ===");
    println!("Total addresses with trades: {}", total_addresses);
    println!("Processing in batches of: {}", batch_size);
    println!();
    
    let mut offset = 0;
    let mut total_updated = 0;
    let start_time = Instant::now();
    
    // Process in batches
    while offset < total_addresses {
        let batch_start = Instant::now();
        
        println!("Processing batch {} to {}...", 
            offset + 1,
            std::cmp::min(offset + batch_size, total_addresses)
        );
        
        // Update batch of addresses
        let updated = queries::aggregation::batch_update_addresses(
            pg_query.db(),
            batch_size,
            offset,
        ).await?;
        
        total_updated += updated;
        let batch_time = batch_start.elapsed();
        
        println!("  Updated {} addresses in {:.2}s", updated, batch_time.as_secs_f64());
        
        if updated == 0 {
            println!("  No more addresses to update");
            break;
        }
        
        offset += batch_size;
        
        // Show progress
        let progress_pct = (offset as f64 / total_addresses as f64 * 100.0).min(100.0);
        println!("  Progress: {:.1}%", progress_pct);
    }
    
    let total_time = start_time.elapsed();
    
    println!("\n=== Calculation Complete ===");
    println!("Total addresses updated: {}", total_updated);
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Average: {:.2} addresses/second", 
        total_updated as f64 / total_time.as_secs_f64()
    );
    
    // Calculate profit metrics for recent trades
    if let Some(latest_block) = progress.latest_trade_block {
        let recent_block_threshold = latest_block - 10000; // Last 10k blocks
        
        println!("\n=== Updating Recent Trades (blocks {} to {}) ===", 
            recent_block_threshold, latest_block
        );
        
        let recent_updates = queries::aggregation::calculate_profit_metrics(
            pg_query.db(),
            recent_block_threshold,
        ).await?;
        
        println!("Updated {} addresses with recent trades", recent_updates);
    }
    
    // Show top profitable addresses after calculation
    println!("\n=== Top 5 Most Profitable Addresses (After Calculation) ===");
    let top_addresses = queries::address_metrics::get_top_profitable_addresses(
        pg_query.db(),
        5,
        None,
        true,
    ).await?;
    
    for (i, addr) in top_addresses.iter().enumerate() {
        println!("{}. {}", i + 1, addr.address);
        println!("   Total Profit: ${:.2}", addr.total_profit.unwrap_or(0.0));
        println!("   Realized Profit: ${:.2}", addr.total_realized_profit.unwrap_or(0.0));
        println!("   Total Volume: ${:.2}", addr.total_volume.unwrap_or(0.0));
        println!("   Total Trades: {}", addr.total_erc20_trades.unwrap_or(0));
        println!();
    }
    
    // Show scam ratio statistics
    println!("=== Scam Ratio Analysis ===");
    let scam_updates = queries::aggregation::calculate_scam_ratios(pg_query.db()).await?;
    println!("Updated scam ratios for {} addresses", scam_updates);
    
    let high_scam_addresses = queries::address_metrics::get_high_scam_ratio_addresses(
        pg_query.db(),
        0.8,  // 80% scam ratio
        5,
    ).await?;
    
    if !high_scam_addresses.is_empty() {
        println!("\nAddresses with >80% scam exposure:");
        for addr in high_scam_addresses {
            println!("  {} - {:.1}% scam ratio",
                addr.address,
                addr.scam_ratio.unwrap_or(0.0) * 100.0
            );
        }
    }
    
    println!("\n✓ PnL calculation complete!");
    
    Ok(())
}