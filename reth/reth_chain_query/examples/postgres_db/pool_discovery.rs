/// Pool Discovery Example
/// 
/// Demonstrates finding pools for tokens across different DEX protocols
/// and analyzing pool characteristics.

use reth_chain_query::postgres_db::{PostgresQuery, queries};
use eyre::Result;
use std::env;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Get database URL from environment or use default
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eth_db".to_string());
    
    println!("Connecting to PostgreSQL database...");
    let pg_query = PostgresQuery::new(&database_url).await?;
    
    // Test with multiple well-known tokens
    let tokens = vec![
        ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        ("WETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        ("DAI", "0x6B175474E89094C44Da98b954EedeAC495271d0F"),
        ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
    ];
    
    println!("=== Pool Discovery for Major Tokens ===\n");
    
    for (name, address) in &tokens {
        println!("Token: {} ({})", name, address);
        let pools = queries::tokens::get_token_pools(pg_query.db(), address).await?;
        
        if pools.is_empty() {
            println!("  No pools found\n");
            continue;
        }
        
        // Group pools by type
        let mut pools_by_type: HashMap<String, Vec<_>> = HashMap::new();
        for pool in pools {
            pools_by_type.entry(pool.pool_type.clone()).or_default().push(pool);
        }
        
        for (pool_type, type_pools) in pools_by_type {
            println!("  {} Pools: {}", pool_type, type_pools.len());
            
            for pool in type_pools.iter().take(2) {  // Show max 2 pools per type
                if let Some(addr) = &pool.pool_address {
                    println!("    - Address: {}", addr);
                } else if let Some(id) = &pool.pool_id {
                    println!("    - Pool ID: {}", id);
                }
                
                if pool_type == "V3" || pool_type == "V4" {
                    if let Some(fee) = pool.fee_tier {
                        println!("      Fee: {:.2}%", fee as f64 / 10000.0);
                    }
                }
                
                println!("      Pair: {}", get_token_symbol(&pool.pair_token_address));
                println!("      Trading: {}", 
                    if pool.trading_enabled.unwrap_or(false) { "Enabled" } else { "Disabled" }
                );
                
                if pool.is_scam.unwrap_or(false) {
                    println!("      ⚠️  SCAM: {}", pool.scam_label.as_ref().unwrap_or(&"Unknown".to_string()));
                }
            }
        }
        println!();
    }
    
    // Analyze pool distribution
    println!("=== Pool Type Distribution ===");
    let v2_pools = queries::tokens::get_pools_by_type(pg_query.db(), "V2", None).await?;
    let v3_pools = queries::tokens::get_pools_by_type(pg_query.db(), "V3", None).await?;
    let v4_pools = queries::tokens::get_pools_by_type(pg_query.db(), "V4", None).await?;
    
    println!("Uniswap V2 Pools: {}", v2_pools.len());
    println!("Uniswap V3 Pools: {}", v3_pools.len());
    println!("Uniswap V4 Pools: {}", v4_pools.len());
    println!("Total Pools: {}\n", v2_pools.len() + v3_pools.len() + v4_pools.len());
    
    // Analyze V3 fee tiers
    if !v3_pools.is_empty() {
        println!("=== Uniswap V3 Fee Tier Distribution ===");
        let mut fee_tiers: HashMap<i32, i32> = HashMap::new();
        
        for pool in &v3_pools {
            if let Some(fee) = pool.fee_tier {
                *fee_tiers.entry(fee).or_insert(0) += 1;
            }
        }
        
        for (fee, count) in fee_tiers {
            println!("  {:.2}% tier: {} pools", fee as f64 / 10000.0, count);
        }
        println!();
    }
    
    // Find multi-pool tokens (tokens with pools across multiple protocols)
    println!("=== Tokens with Multiple Pool Types ===");
    let mut token_pool_types: HashMap<String, Vec<String>> = HashMap::new();
    
    for pool_type in ["V2", "V3", "V4"] {
        let pools = queries::tokens::get_pools_by_type(pg_query.db(), pool_type, Some(100)).await?;
        for pool in pools {
            token_pool_types
                .entry(pool.token_address.clone())
                .or_default()
                .push(pool_type.to_string());
        }
    }
    
    let multi_pool_tokens: Vec<_> = token_pool_types
        .iter()
        .filter(|(_, types)| types.len() > 1)
        .take(5)
        .collect();
    
    for (token, pool_types) in multi_pool_tokens {
        println!("Token: {}", token);
        println!("  Available on: {}", pool_types.join(", "));
    }
    
    // Show connection pool statistics
    println!("\n=== Database Connection Pool Stats ===");
    let stats = pg_query.db().stats();
    println!("Pool Size: {}", stats.size);
    println!("Idle Connections: {}", stats.idle);
    
    Ok(())
}

// Helper function to get token symbol from address
fn get_token_symbol(address: &str) -> &str {
    match address.to_lowercase().as_str() {
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => "WETH",
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => "USDC",
        "0x6b175474e89094c44da98b954eedeac495271d0f" => "DAI",
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => "USDT",
        _ => address,
    }
}