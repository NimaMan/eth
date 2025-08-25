/// Address Profitability Analysis Example
/// 
/// Demonstrates querying top profitable addresses from PostgreSQL database
/// with various filters and ranking calculations.

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
    
    // Get top 10 profitable addresses
    println!("=== Top 10 Most Profitable Addresses ===");
    let top_addresses = queries::address_metrics::get_top_profitable_addresses(
        pg_query.db(),
        10,
        Some(10000.0),  // Min volume: $10k
        true,            // Exclude contracts
    ).await?;
    
    for (i, addr) in top_addresses.iter().enumerate() {
        println!("{}. Address: {}", i + 1, addr.address);
        println!("   Total Profit: ${:.2}", addr.total_profit.unwrap_or(0.0));
        println!("   Total Volume: ${:.2}", addr.total_volume.unwrap_or(0.0));
        println!("   Scam Ratio: {:.2}%", addr.scam_ratio.unwrap_or(0.0) * 100.0);
        println!("   Network Centrality: {:.4}", addr.degree_centrality.unwrap_or(0.0));
        if let Some(label) = &addr.entity_category {
            println!("   Category: {}", label);
        }
        println!();
    }
    
    // Get addresses with high scam ratio
    println!("=== Addresses with High Scam Exposure ===");
    let scam_addresses = queries::address_metrics::get_high_scam_ratio_addresses(
        pg_query.db(),
        0.7,  // 70% scam ratio threshold
        5,
    ).await?;
    
    for addr in scam_addresses {
        println!("Address: {}", addr.address);
        println!("  Scam Ratio: {:.2}%", addr.scam_ratio.unwrap_or(0.0) * 100.0);
        println!("  Total Volume: ${:.2}", addr.total_volume.unwrap_or(0.0));
        println!();
    }
    
    // Get most active traders
    println!("=== Most Active Traders (Last 100k Blocks) ===");
    let active_traders = queries::address_metrics::get_most_active_addresses(
        pg_query.db(),
        5,
        Some(100_000),  // Last 100k blocks (~2 weeks)
    ).await?;
    
    for addr in active_traders {
        println!("Address: {}", addr.address);
        println!("  Total Trades: {}", addr.total_erc20_trades.unwrap_or(0));
        println!("  First Seen: Block #{}", addr.first_seen.unwrap_or(0));
        println!("  Last Seen: Block #{}", addr.last_seen.unwrap_or(0));
        println!("  Network Centrality: {:.4}", addr.degree_centrality.unwrap_or(0.0));
        println!();
    }
    
    // Get address ranking
    println!("=== Address Rankings (Bird-Themed Tiers) ===");
    let rankings = queries::analytics::calculate_address_ranking(
        pg_query.db(),
        10,
    ).await?;
    
    for ranking in rankings {
        println!("Address: {}", ranking.address);
        println!("  Bird Tier: {} 🦅", ranking.bird_tier);
        println!("  Profit Rank: #{}", ranking.profit_rank);
        println!("  Volume Rank: #{}", ranking.volume_rank);
        println!("  Activity Rank: #{}", ranking.activity_rank);
        println!("  Composite Score: {:.6}", ranking.composite_score);
        println!();
    }
    
    // Get global statistics
    println!("=== Global Statistics ===");
    let stats = queries::analytics::get_global_statistics(pg_query.db()).await?;
    println!("Total Addresses: {}", stats.total_addresses);
    println!("Total Contracts: {}", stats.total_contracts);
    println!("Total Tokens: {}", stats.total_tokens);
    println!("Total Trades: {}", stats.total_trades);
    println!("Total Volume: ${:.2}", stats.total_volume.unwrap_or(0.0));
    println!("Total Profit: ${:.2}", stats.total_profit.unwrap_or(0.0));
    println!("Average Scam Ratio: {:.2}%", stats.avg_scam_ratio.unwrap_or(0.0) * 100.0);
    println!("High Scam Addresses: {}", stats.high_scam_addresses);
    
    Ok(())
}