/// Trade PnL Calculation Example
/// 
/// Demonstrates querying and analyzing profit/loss for specific address-token pairs
/// and finding the most profitable trades in the database.

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
    
    // Example address and token (you can change these)
    let example_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";  // Example address
    let example_token = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";    // USDC
    
    // Get all trades for an address
    println!("=== Trades for Address {} ===", example_address);
    let trades = queries::trades::get_trades_for_address(
        pg_query.db(),
        example_address,
        None,  // Get all tokens
    ).await?;
    
    if trades.is_empty() {
        println!("No trades found for this address\n");
    } else {
        println!("Found {} trades\n", trades.len());
        
        for (i, trade) in trades.iter().take(5).enumerate() {
            println!("Trade #{}:", i + 1);
            println!("  Token: {}", trade.token_address);
            println!("  Entry Block: {}", trade.entry_block.unwrap_or(0));
            println!("  Latest Block: {}", trade.latest_block.unwrap_or(0));
            println!("  Total Spent: ${:.2}", trade.total_denom_spent.unwrap_or(0.0));
            println!("  Total Received: ${:.2}", trade.total_denom_received.unwrap_or(0.0));
            println!("  Realized Profit: ${:.2}", trade.realized_profit.unwrap_or(0.0));
            println!("  Unrealized Profit: ${:.2}", trade.unrealized_profit.unwrap_or(0.0));
            println!("  Buys: {} | Sells: {}", 
                trade.num_buys.unwrap_or(0), 
                trade.num_sells.unwrap_or(0)
            );
            println!();
        }
    }
    
    // Get PnL for specific address-token pair
    println!("=== PnL Analysis for {} - {} ===", example_address, example_token);
    match queries::trades::get_address_token_pnl(
        pg_query.db(),
        example_address,
        example_token,
    ).await? {
        Some(pnl) => {
            println!("Address: {}", pnl.address);
            println!("Token: {}", pnl.token_address);
            println!("Total Spent: ${:.2}", pnl.total_spent);
            println!("Total Received: ${:.2}", pnl.total_received);
            println!("Realized Profit: ${:.2}", pnl.realized_profit);
            println!("Unrealized Profit: ${:.2}", pnl.unrealized_profit);
            println!("ROI: {:.2}%", pnl.roi_percentage);
            println!("Number of Trades: {}", pnl.num_trades);
            println!();
        }
        None => {
            println!("No trades found for this address-token pair\n");
        }
    }
    
    // Get most profitable trades globally
    println!("=== Top 10 Most Profitable Trades ===");
    let profitable_trades = queries::trades::get_most_profitable_trades(
        pg_query.db(),
        10,
        Some(1000.0),  // Min volume $1000
    ).await?;
    
    for (i, trade) in profitable_trades.iter().enumerate() {
        println!("{}. Token: {}", i + 1, trade.token_address);
        println!("   Realized Profit: ${:.2}", trade.realized_profit.unwrap_or(0.0));
        println!("   Total Volume: ${:.2}", 
            trade.total_denom_spent.unwrap_or(0.0) + trade.total_denom_received.unwrap_or(0.0)
        );
        println!("   R/S Ratio: {:.2}", trade.denom_received_spent_ratio.unwrap_or(0.0));
        println!("   Entry Block: #{}", trade.entry_block.unwrap_or(0));
        println!();
    }
    
    // Get recent trades
    let current_block = 20_000_000;  // Approximate current block
    let blocks_back = 10_000;        // Look back 10k blocks
    
    println!("=== Recent Trades (Last {} Blocks) ===", blocks_back);
    let recent_trades = queries::trades::get_recent_trades(
        pg_query.db(),
        current_block - blocks_back,
        current_block,
        Some(5),
    ).await?;
    
    for trade in recent_trades {
        println!("Token: {}", trade.token_address);
        println!("  Latest Block: #{}", trade.latest_block.unwrap_or(0));
        println!("  Volume: ${:.2}", 
            trade.total_denom_spent.unwrap_or(0.0) + trade.total_denom_received.unwrap_or(0.0)
        );
        println!("  Profit: ${:.2}", trade.realized_profit.unwrap_or(0.0));
        println!();
    }
    
    Ok(())
}