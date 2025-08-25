/// Token Scam Analysis Example
/// 
/// Demonstrates querying token metadata, identifying scam tokens,
/// and analyzing pool information across DEX protocols.

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
    
    // Example token addresses
    let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    let example_token = "0x6B175474E89094C44Da98b954EedeAC495271d0F";  // DAI
    
    // Get token information
    println!("=== Token Information for {} ===", usdc);
    match queries::tokens::get_token_info(pg_query.db(), usdc).await? {
        Some(token) => {
            println!("Contract Address: {}", token.contract_address);
            println!("Is Scam: {}", token.is_scam.unwrap_or(false));
            if let Some(label) = token.scam_label {
                println!("Scam Label: {}", label);
            }
            if let Some(creation_tx) = token.creation_txn {
                println!("Creation TX: {}", creation_tx);
            }
            if let Some(trading_tx) = token.trading_enabled_txn {
                println!("Trading Enabled TX: {}", trading_tx);
            }
            println!();
        }
        None => {
            println!("Token not found in database\n");
        }
    }
    
    // Get all pools for a token
    println!("=== Pools for Token {} ===", example_token);
    let pools = queries::tokens::get_token_pools(pg_query.db(), example_token).await?;
    
    if pools.is_empty() {
        println!("No pools found for this token\n");
    } else {
        println!("Found {} pools:\n", pools.len());
        
        for pool in pools {
            println!("Pool Type: {}", pool.pool_type);
            if let Some(addr) = pool.pool_address {
                println!("  Address: {}", addr);
            }
            if let Some(id) = pool.pool_id {
                println!("  Pool ID: {}", id);
            }
            println!("  Pair Token: {}", pool.pair_token_address);
            if let Some(fee) = pool.fee_tier {
                println!("  Fee Tier: {:.2}%", fee as f64 / 10000.0);
            }
            println!("  Trading Enabled: {}", pool.trading_enabled.unwrap_or(false));
            if let Some(block) = pool.trading_enabled_block {
                println!("  Enabled at Block: #{}", block);
            }
            println!();
        }
    }
    
    // Get scam tokens
    println!("=== Identified Scam Tokens (Top 10) ===");
    let scam_tokens = queries::tokens::get_scam_tokens(pg_query.db(), Some(10)).await?;
    
    if scam_tokens.is_empty() {
        println!("No scam tokens found\n");
    } else {
        for (i, token) in scam_tokens.iter().enumerate() {
            println!("{}. Token: {}", i + 1, token.contract_address);
            if let Some(label) = &token.scam_label {
                println!("   Scam Type: {}", label);
            }
            if let Some(creation_tx) = &token.creation_txn {
                println!("   Creation TX: {}", creation_tx);
            }
            println!();
        }
    }
    
    // Get tokens created by a specific address
    let creator_address = "0x0000000000000000000000000000000000000000";  // Example
    println!("=== Tokens Created by {} ===", creator_address);
    let created_tokens = queries::tokens::get_tokens_by_creator(
        pg_query.db(), 
        creator_address
    ).await?;
    
    if created_tokens.is_empty() {
        println!("No tokens created by this address\n");
    } else {
        for token in created_tokens {
            println!("Token: {}", token.contract_address);
            println!("  Is Scam: {}", token.is_scam.unwrap_or(false));
            if let Some(label) = token.scam_label {
                println!("  Scam Label: {}", label);
            }
            println!();
        }
    }
    
    // Get pools by type
    println!("=== Recent Uniswap V3 Pools ===");
    let v3_pools = queries::tokens::get_pools_by_type(
        pg_query.db(),
        "V3",
        Some(5),
    ).await?;
    
    for pool in v3_pools {
        println!("Token: {}", pool.token_address);
        if let Some(addr) = pool.pool_address {
            println!("  Pool: {}", addr);
        }
        println!("  Pair: {}", pool.pair_token_address);
        if let Some(fee) = pool.fee_tier {
            println!("  Fee: {:.2}%", fee as f64 / 10000.0);
        }
        println!();
    }
    
    // Get recently enabled trading pools
    println!("=== Recently Enabled Trading (Last 10k Blocks) ===");
    let recent_pools = queries::tokens::get_recently_enabled_pools(
        pg_query.db(),
        10_000,
        Some(5),
    ).await?;
    
    for pool in recent_pools {
        println!("Token: {}", pool.token_address);
        println!("  Pool Type: {}", pool.pool_type);
        if let Some(block) = pool.trading_enabled_block {
            println!("  Enabled at Block: #{}", block);
        }
        if let Some(tx) = pool.trading_enabled_txn {
            println!("  TX: {}", tx);
        }
        println!();
    }
    
    Ok(())
}