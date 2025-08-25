/// Populate Address Metrics Example
/// 
/// Demonstrates how to aggregate trades data into the addresses table
/// to calculate profit metrics, scam ratios, and other aggregated values.

use reth_chain_query::postgres_db::{PostgresQuery, queries};
use eyre::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Get database URL from environment or use default
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eth_db".to_string());
    
    println!("Connecting to PostgreSQL database...");
    let pg_query = PostgresQuery::new(&database_url).await?;
    
    // Test connection
    pg_query.db().ping().await?;
    println!("✓ Connected successfully\n");
    
    // Get current population statistics
    println!("=== Current Database Statistics ===");
    let stats = queries::population::get_population_statistics(pg_query.db()).await?;
    println!("Total Addresses: {}", stats.total_addresses);
    println!("Addresses with Profit Data: {}", stats.addresses_with_profit);
    println!("Addresses with Scam Ratio: {}", stats.addresses_with_scam_ratio);
    println!("Addresses with Trades: {}", stats.addresses_with_trades);
    println!("Unique Trading Addresses: {}", stats.unique_trading_addresses);
    println!("Total Trades: {}", stats.total_trades);
    println!("Total Tokens: {}", stats.total_tokens);
    println!("Scam Tokens: {}", stats.scam_tokens);
    println!("Total Pools: {}", stats.total_pools);
    println!();
    
    // Check if we should do a full refresh or incremental update
    let full_refresh = env::var("FULL_REFRESH")
        .map(|v| v == "true")
        .unwrap_or(false);
    
    if full_refresh {
        println!("=== Running FULL REFRESH ===");
        println!("This will clear existing metrics and recalculate from scratch.");
    } else {
        println!("=== Running Incremental Update ===");
        println!("This will update only changed addresses.");
    }
    println!();
    
    // Ensure all addresses from trades exist
    println!("Step 1: Ensuring all trading addresses exist in addresses table...");
    let new_addresses = queries::population::ensure_addresses_exist(pg_query.db()).await?;
    println!("  Added {} new addresses", new_addresses);
    
    // Populate addresses from trades
    println!("\nStep 2: Aggregating trade metrics into addresses...");
    let result = queries::population::populate_addresses_from_trades(
        pg_query.db(),
        full_refresh,
    ).await?;
    println!("  Updated profit metrics for {} addresses", result.addresses_updated_profit);
    println!("  Updated scam ratios for {} addresses", result.addresses_updated_scam);
    
    // Update address balances
    println!("\nStep 3: Updating address balances from latest trades...");
    let balance_updates = queries::population::update_address_balances(pg_query.db()).await?;
    println!("  Updated balances for {} addresses", balance_updates);
    
    // Update bribe amounts
    println!("\nStep 4: Calculating bribe statistics...");
    let bribe_updates = queries::aggregation::update_bribe_amounts(pg_query.db()).await?;
    println!("  Updated bribe amounts for {} addresses", bribe_updates);
    
    // Get aggregation progress
    println!("\n=== Aggregation Progress ===");
    let progress = queries::aggregation::get_aggregation_progress(pg_query.db()).await?;
    println!("Addresses with Trades: {}", progress.addresses_with_trades);
    println!("Addresses Aggregated: {}", progress.addresses_aggregated);
    println!("Addresses with Scam Ratio: {}", progress.addresses_with_scam_ratio);
    println!("Total Trades Processed: {}", progress.total_trades);
    if let Some(block) = progress.latest_trade_block {
        println!("Latest Trade Block: #{}", block);
    }
    
    // Find addresses needing update
    println!("\n=== Addresses Needing Updates ===");
    let stale_addresses = queries::population::get_addresses_needing_update(
        pg_query.db(),
        10,
    ).await?;
    
    if stale_addresses.is_empty() {
        println!("All addresses are up to date!");
    } else {
        println!("Found {} addresses needing updates:", stale_addresses.len());
        for addr in stale_addresses.iter().take(5) {
            println!("  {} - {} trades, last updated at block {:?}, latest trade at {:?}",
                addr.address,
                addr.trade_count,
                addr.last_updated_block,
                addr.latest_trade_block
            );
        }
        
        // Update these specific addresses
        println!("\nUpdating stale addresses...");
        let address_ids: Vec<i64> = stale_addresses.iter().map(|a| a.address_id).collect();
        let updated = queries::population::calculate_profit_metrics_for_addresses(
            pg_query.db(),
            &address_ids,
        ).await?;
        println!("Updated {} addresses", updated);
    }
    
    // Final statistics
    println!("\n=== Final Database Statistics ===");
    let final_stats = queries::population::get_population_statistics(pg_query.db()).await?;
    println!("Total Addresses: {}", final_stats.total_addresses);
    println!("Addresses with Profit Data: {} (+{})",
        final_stats.addresses_with_profit,
        final_stats.addresses_with_profit - stats.addresses_with_profit
    );
    println!("Addresses with Scam Ratio: {} (+{})",
        final_stats.addresses_with_scam_ratio,
        final_stats.addresses_with_scam_ratio - stats.addresses_with_scam_ratio
    );
    
    println!("\n✓ Population complete!");
    
    Ok(())
}