/// Inspect Creator and Token Information
///
/// Get comprehensive token information for creator addresses and analyze pools

use std::time::Duration;
use clap::Parser;
use eyre::Result;
use tracing::info;

use mempool_processor::token_tracking::TokenTrackingSubscriber;

#[derive(Parser, Debug)]
struct Args {
    /// Creator address to inspect
    #[arg(long)]
    creator: String,
    
    /// Also check a specific target address (token or pool)
    #[arg(long)]
    target: Option<String>,
    
    /// Show detailed pool information
    #[arg(long, default_value = "false")]
    detailed: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("info")
        .init();
    
    info!("🔍 Inspecting creator: {}", args.creator);
    if let Some(target) = &args.target {
        info!("🎯 Also checking target: {}", target);
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
    
    // Get cache statistics
    let total_pools = token_cache.pools.get_pool_count().await;
    let total_creators = token_cache.get_creator_count().await;
    info!("\n📊 Cache Statistics:");
    info!("   Total Pools: {}", total_pools);
    info!("   Total Creators: {}", total_creators);
    
    // Check if address is a creator
    info!("\n🔍 Checking creator cache...");
    let all_creators = token_cache.get_all_creators().await;
    
    if all_creators.contains(&args.creator) {
        info!("✅ Found address in creators set!");
        
        // Get all tokens created by this address
        let tokens_by_creator = token_cache.get_tokens_by_creator(&args.creator).await;
        info!("\n📊 Total tokens created by this address: {}", tokens_by_creator.len());
        
        // Display information for each token
        for (idx, token_address) in tokens_by_creator.iter().enumerate() {
            if let Some(token_info) = token_cache.get_token(&token_address).await {
                info!("\n📋 Token #{} Information:", idx + 1);
                info!("   Token Address: {}", token_info.token_address);
                info!("   Creator: {}", token_info.creator_address);
                info!("   Creation Block: {}", token_info.creation_block);
                info!("   Creation Tx: {}", token_info.creation_txn);
                info!("   Trading Enabled: {}", token_info.trading_enabled);
                if token_info.trading_enabled {
                    if let Some(tx) = &token_info.trading_enabled_txn {
                        info!("   Trading Enabled Tx: {}", tx);
                    }
                }
                info!("   Current Owner: {}", token_info.current_owner);
                info!("   Ownership Renounced: {}", token_info.ownership_renounced);
                
                // Token metadata
                if let Some(symbol) = &token_info.symbol {
                    info!("   Symbol: {}", symbol);
                }
                if let Some(name) = &token_info.name {
                    info!("   Name: {}", name);
                }
                if let Some(decimals) = token_info.decimals {
                    info!("   Decimals: {}", decimals);
                }
                if let Some(supply) = &token_info.total_supply {
                    info!("   Total Supply: {}", supply);
                }
                
                // Tax information
                if let Some(buy_tax) = token_info.buy_tax_python {
                    info!("   Buy Tax: {:.1}%", buy_tax);
                }
                if let Some(sell_tax) = token_info.sell_tax_python {
                    info!("   Sell Tax: {:.1}%", sell_tax);
                }
                if let Some(last_tax_update) = &token_info.last_tax_update_txn {
                    info!("   Last Tax Update: {}", last_tax_update);
                }
                
                // Tax setter addresses
                if !token_info.tax_setter_addresses.is_empty() {
                    info!("   Tax Setters: {} addresses", token_info.tax_setter_addresses.len());
                    if args.detailed {
                        for setter in &token_info.tax_setter_addresses {
                            info!("     - {}", setter);
                        }
                    }
                }
                
                // Scam status
                if token_info.is_scam {
                    info!("   ⚠️  SCAM: {}", token_info.scam_label.as_ref().unwrap_or(&"Unknown reason".to_string()));
                }
                
                // Pool information
                let pool_count = token_info.pools.len();
                if pool_count == 0 {
                    info!("   Pools: None found");
                } else {
                    info!("   Pools: {} found", pool_count);
                    
                    // Sort pools by ETH reserve (highest first)
                    let mut pools: Vec<_> = token_info.pools.iter().collect();
                    pools.sort_by(|a, b| b.1.denom_reserve.partial_cmp(&a.1.denom_reserve).unwrap());
                    
                    for (i, (pool_addr, pool)) in pools.iter().take(if args.detailed { pools.len() } else { 3 }).enumerate() {
                        info!("\n   Pool {} ({})", i+1, pool_addr);
                        info!("      Type: {}", pool.pool_type);
                        info!("      {} Reserve: {:.6}", pool.denom_currency, pool.denom_reserve);
                        info!("      Token Reserve: {:.2}", pool.token_reserve);
                        info!("      Latest Block: {}", pool.latest_block_number);
                        if let Some(fee_tier) = pool.fee_tier {
                            info!("      Fee Tier: {}bps", fee_tier);
                        }
                        if pool.is_scam {
                            info!("      ⚠️  SCAM POOL: {}", pool.scam_label.as_ref().unwrap_or(&"Unknown".to_string()));
                        }
                    }
                    
                    if !args.detailed && pool_count > 3 {
                        info!("      ... and {} more pools", pool_count - 3);
                    }
                    
                    // Show primary pool
                    if let Some(primary_pool) = token_cache.get_primary_pool(&token_address).await {
                        info!("\n   Primary Pool (highest liquidity): {} {} reserve", 
                            primary_pool.denom_reserve, primary_pool.denom_currency);
                    }
                }
                
                // Simulation data if available
                if let Some(sim_data) = &token_info.simulation_data {
                    info!("\n   Simulation Results:");
                    info!("      Can Buy: {}", sim_data.can_buy);
                    info!("      Can Sell: {}", sim_data.can_sell);
                    if let Some(buy_tax) = sim_data.measured_buy_tax {
                        info!("      Measured Buy Tax: {:.1}%", buy_tax);
                    }
                    if let Some(sell_tax) = sim_data.measured_sell_tax {
                        info!("      Measured Sell Tax: {:.1}%", sell_tax);
                    }
                    info!("      Is Honeypot: {}", sim_data.is_honeypot);
                    info!("      Last Simulated Block: {}", sim_data.last_simulated_block);
                    if let Some(error) = &sim_data.simulation_error {
                        info!("      Simulation Error: {}", error);
                    }
                }
            }
        }
    } else {
        info!("❌ Creator not found in cache");
        info!("   This could mean:");
        info!("   - The address hasn't created any tokens");
        info!("   - The tokens don't meet the 0.1 ETH threshold");
        info!("   - The cache hasn't received updates for this creator yet");
    }
    
    // If target address provided, analyze it
    if let Some(target) = &args.target {
        info!("\n\n🎯 Analyzing target address: {}", target);
        
        // Check if it's a pool
        if let Some(pool) = token_cache.pools.get_pool(target).await {
            info!("✅ Target is a POOL!");
            info!("   Token: {}", pool.token_address);
            info!("   ETH Reserve: {:.6} ETH", pool.eth_reserve);
            info!("   Token Reserve: {:.2}", pool.token_reserve);
            info!("   Last Updated Block: {}", pool.last_updated_block);
            info!("   Last Update: {:.0}s ago", pool.received_at.elapsed().as_secs());
            
            // Get token info for this pool's token
            if let Some(token_info) = token_cache.get_token(&pool.token_address).await {
                info!("\n   Token Details:");
                info!("     Symbol: {}", token_info.symbol.as_ref().unwrap_or(&"Unknown".to_string()));
                info!("     Name: {}", token_info.name.as_ref().unwrap_or(&"Unknown".to_string()));
                info!("     Creator: {}", token_info.creator_address);
                info!("     Trading Enabled: {}", token_info.trading_enabled);
            }
        }
        
        // Check if it's a token
        if let Some(token_info) = token_cache.get_token(target).await {
            info!("✅ Target is a TOKEN!");
            info!("   Symbol: {}", token_info.symbol.as_ref().unwrap_or(&"Unknown".to_string()));
            info!("   Name: {}", token_info.name.as_ref().unwrap_or(&"Unknown".to_string()));
            info!("   Creator: {}", token_info.creator_address);
            info!("   Creation Block: {}", token_info.creation_block);
            info!("   Pools: {}", token_info.pools.len());
            
            // Calculate total liquidity
            let total_eth: f64 = token_info.pools.values()
                .filter(|p| p.denom_currency == "ETH")
                .map(|p| p.denom_reserve)
                .sum();
            info!("   Total ETH Liquidity: {:.6} ETH", total_eth);
        }
        
        // Check if it's a creator
        if all_creators.contains(target) {
            info!("✅ Target is also a CREATOR!");
            let tokens = token_cache.get_tokens_by_creator(target).await;
            info!("   Tokens created: {}", tokens.len());
        }
    }
    
    info!("\n✅ Inspection complete!");
    
    Ok(())
}