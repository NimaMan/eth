/// Get Token Information Example
///
/// Demonstrates how to retrieve comprehensive token information
/// that can be passed to the simulation manager

use clap::Parser;
use eyre::Result;
use tracing::info;
use std::time::Duration;

use mempool_processor::token_tracking::TokenTrackingSubscriber;

#[derive(Parser, Debug)]
struct Args {
    /// Token address to get information for
    #[arg(long)]
    token: Option<String>,
    
    /// Creator address to get tokens for
    #[arg(long)]
    creator: Option<String>,
    
    /// Pool address to get token info from
    #[arg(long)]
    pool: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("info")
        .init();
    
    // Validate arguments
    if args.token.is_none() && args.creator.is_none() && args.pool.is_none() {
        eprintln!("Error: Must provide at least one of --token, --creator, or --pool");
        std::process::exit(1);
    }
    
    // Initialize token tracking subscriber
    let mut token_subscriber = TokenTrackingSubscriber::new(0.1);
    let token_cache = token_subscriber.get_cache();
    
    // Start subscriber in background
    let _subscriber_handle = tokio::spawn(async move {
        if let Err(e) = token_subscriber.start_listening().await {
            eprintln!("Token subscriber error: {}", e);
        }
    });
    
    // Wait for cache population
    info!("⏳ Waiting for token cache population...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get token information based on input
    if let Some(token_address) = args.token {
        info!("\n🔍 Getting info for token: {}", token_address);
        
        if let Some(token_info) = token_cache.get_token(&token_address).await {
            display_token_info(&token_info);
            
            // Show how this would be used in routing
            info!("\n📡 Information for Router/Simulation:");
            info!("   Primary Pool: {}", 
                if let Some(primary) = token_cache.get_primary_pool(&token_address).await {
                    format!("{:.6} {} liquidity", primary.denom_reserve, primary.denom_currency)
                } else {
                    "No pools found".to_string()
                }
            );
            info!("   Trading Status: {}", if token_info.trading_enabled { "Enabled" } else { "Disabled" });
            info!("   Tax Status: Buy {}%, Sell {}%", 
                token_info.buy_tax_python.unwrap_or(0.0),
                token_info.sell_tax_python.unwrap_or(0.0)
            );
            info!("   Owner: {}", token_info.current_owner);
            info!("   Total Pools: {}", token_info.pools.len());
        } else {
            info!("❌ Token not found in cache");
        }
    }
    
    if let Some(creator_address) = args.creator {
        info!("\n🔍 Getting tokens for creator: {}", creator_address);
        
        let all_creators = token_cache.get_all_creators().await;
        if all_creators.contains(&creator_address) {
            let tokens = token_cache.get_tokens_by_creator(&creator_address).await;
            info!("✅ Found {} tokens created by this address", tokens.len());
            
            for (idx, token_addr) in tokens.iter().enumerate() {
                info!("\n📋 Token #{}: {}", idx + 1, token_addr);
                
                if let Some(token_info) = token_cache.get_token(token_addr).await {
                    info!("   Symbol: {}", token_info.symbol.as_ref().unwrap_or(&"Unknown".to_string()));
                    info!("   Pools: {}", token_info.pools.len());
                    info!("   Trading: {}", if token_info.trading_enabled { "Enabled" } else { "Disabled" });
                    
                    // Calculate total liquidity
                    let total_eth: f64 = token_info.pools.values()
                        .filter(|p| p.denom_currency == "ETH")
                        .map(|p| p.denom_reserve)
                        .sum();
                    info!("   Total ETH Liquidity: {:.6}", total_eth);
                }
            }
        } else {
            info!("❌ Creator not found in cache");
        }
    }
    
    if let Some(pool_address) = args.pool {
        info!("\n🔍 Getting token info from pool: {}", pool_address);
        
        if let Some(pool_state) = token_cache.pools.get_pool(&pool_address).await {
            info!("✅ Found pool!");
            info!("   Token Address: {}", pool_state.token_address);
            info!("   ETH Reserve: {:.6}", pool_state.eth_reserve);
            info!("   Token Reserve: {:.2}", pool_state.token_reserve);
            
            // Get full token info
            if let Some(token_info) = token_cache.get_token(&pool_state.token_address).await {
                info!("\n📋 Token Information:");
                display_token_info(&token_info);
            } else {
                info!("⚠️  Token info not found for this pool's token");
            }
        } else {
            info!("❌ Pool not found in cache");
        }
    }
    
    info!("\n✅ Done!");
    Ok(())
}

fn display_token_info(token_info: &mempool_processor::token_tracking::types::TokenInfo) {
    info!("   Address: {}", token_info.token_address);
    info!("   Symbol: {}", token_info.symbol.as_ref().unwrap_or(&"Unknown".to_string()));
    info!("   Name: {}", token_info.name.as_ref().unwrap_or(&"Unknown".to_string()));
    info!("   Creator: {}", token_info.creator_address);
    info!("   Creation Block: {}", token_info.creation_block);
    info!("   Current Owner: {}", token_info.current_owner);
    info!("   Trading Enabled: {}", token_info.trading_enabled);
    
    if let Some(buy_tax) = token_info.buy_tax_python {
        info!("   Buy Tax: {:.1}%", buy_tax);
    }
    if let Some(sell_tax) = token_info.sell_tax_python {
        info!("   Sell Tax: {:.1}%", sell_tax);
    }
    
    info!("   Pools: {}", token_info.pools.len());
    if !token_info.pools.is_empty() {
        // Show top 3 pools by liquidity
        let mut pools: Vec<_> = token_info.pools.iter().collect();
        pools.sort_by(|a, b| b.1.denom_reserve.partial_cmp(&a.1.denom_reserve).unwrap());
        
        for (i, (pool_addr, pool)) in pools.iter().take(3).enumerate() {
            info!("     Pool {}: {} ({:.4} {} reserve)", 
                i+1, pool_addr, pool.denom_reserve, pool.denom_currency);
        }
    }
    
    if token_info.is_scam {
        info!("   ⚠️  SCAM: {}", token_info.scam_label.as_ref().unwrap_or(&"Unknown".to_string()));
    }
}