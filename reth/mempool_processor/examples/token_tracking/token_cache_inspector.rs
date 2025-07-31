/// Token Cache Inspector
/// 
/// This example connects to the token tracking cache and provides comprehensive
/// information about tracked tokens, pools, and creators. It demonstrates:
/// - Connecting to the ZMQ-based token tracking system
/// - Listing all creator/owner/tax setter addresses
/// - Listing all tracked pools
/// - Displaying detailed information for sample tokens
/// - Using the new get_all_token_addresses method
///
/// Usage: cargo run --example token_cache_inspector

use mempool_processor::token_tracking::{TokenTrackingSubscriber, TokenTrackingCache};
use tracing::{info, warn, error};
use std::collections::HashSet;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with INFO level
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🔍 Token Cache Inspector");
    info!("=======================");
    
    // Create subscriber with 0.1 ETH threshold
    let mut subscriber = TokenTrackingSubscriber::new(0.1);
    let cache = subscriber.get_cache();
    
    // Start the subscriber in background
    info!("Starting token tracking subscriber...");
    tokio::spawn(async move {
        if let Err(e) = subscriber.start_listening().await {
            error!("Subscriber error: {}", e);
        }
    });
    
    // Wait for initial data to load
    info!("Waiting for initial data load...");
    sleep(Duration::from_secs(3)).await;
    
    // 1. Display all creator addresses
    info!("\n📋 All Creator/Owner/Tax Setter Addresses:");
    info!("==========================================");
    let all_creators = cache.get_all_creators().await;
    info!("Total authority addresses: {}", all_creators.len());
    
    if all_creators.len() > 0 {
        // Show first 10 creators
        for (i, creator) in all_creators.iter().take(10).enumerate() {
            info!("  {}. {}", i + 1, creator);
        }
        if all_creators.len() > 10 {
            info!("  ... and {} more", all_creators.len() - 10);
        }
    } else {
        warn!("No creator addresses found. Is the Python publisher running?");
    }
    
    // 2. Display all pool addresses
    info!("\n🏊 All Pool Addresses:");
    info!("======================");
    let all_pools = cache.get_all_pools().await;
    info!("Total pools tracked: {}", all_pools.len());
    
    if all_pools.len() > 0 {
        // Show first 10 pools
        for (i, pool) in all_pools.iter().take(10).enumerate() {
            info!("  {}. {}", i + 1, pool);
        }
        if all_pools.len() > 10 {
            info!("  ... and {} more", all_pools.len() - 10);
        }
    } else {
        warn!("No pools found. Is the Python publisher running?");
    }
    
    // 3. Display all token addresses using new method
    info!("\n🪙 All Token Addresses:");
    info!("=======================");
    let all_tokens = cache.get_all_token_addresses().await;
    info!("Total unique tokens: {}", all_tokens.len());
    
    if all_tokens.len() > 0 {
        // Show first 10 tokens
        for (i, token) in all_tokens.iter().take(10).enumerate() {
            info!("  {}. {}", i + 1, token);
        }
        if all_tokens.len() > 10 {
            info!("  ... and {} more", all_tokens.len() - 10);
        }
    } else {
        warn!("No tokens found. Is the Python publisher running?");
    }
    
    // 4. Display detailed information for sample tokens
    info!("\n📊 Sample Token Details:");
    info!("========================");
    
    let sample_tokens: Vec<_> = all_tokens.iter().take(10).cloned().collect();
    
    for (idx, token_address) in sample_tokens.iter().enumerate() {
        if let Some(token_info) = cache.get_token(token_address).await {
            info!("\n{}. Token: {}", idx + 1, token_address);
            info!("   Symbol: {}", token_info.symbol.as_deref().unwrap_or("N/A"));
            info!("   Name: {}", token_info.name.as_deref().unwrap_or("N/A"));
            info!("   Creator: {}", token_info.creator_address);
            info!("   Owner: {}", token_info.current_owner);
            info!("   Ownership Renounced: {}", token_info.ownership_renounced);
            info!("   Trading Enabled: {}", token_info.trading_enabled);
            info!("   Buy Tax: {}%", token_info.buy_tax.map(|t| t.to_string()).unwrap_or("N/A".to_string()));
            info!("   Sell Tax: {}%", token_info.sell_tax.map(|t| t.to_string()).unwrap_or("N/A".to_string()));
            info!("   Number of Pools: {}", token_info.pools.len());
            
            // Show pool details
            if let Some(primary_pool) = cache.get_primary_pool(token_address).await {
                info!("   Primary Pool:");
                info!("     Address: {}", primary_pool.pool_address);
                info!("     ETH Reserve: {:.6} ETH", primary_pool.denom_reserve);
                info!("     Token Reserve: {:.2}", primary_pool.token_reserve);
                info!("     Pool Type: {}", primary_pool.pool_type);
            }
            
            // Show simulation results if available
            if let Some(sim_data) = &token_info.simulation_data {
                info!("   Simulation Results:");
                info!("     Can Buy: {}", sim_data.can_buy);
                info!("     Can Sell: {}", sim_data.can_sell);
                info!("     Is Honeypot: {}", sim_data.is_honeypot);
                if let Some(buy_tax) = sim_data.measured_buy_tax {
                    info!("     Measured Buy Tax: {:.2}%", buy_tax);
                }
                if let Some(sell_tax) = sim_data.measured_sell_tax {
                    info!("     Measured Sell Tax: {:.2}%", sell_tax);
                }
                info!("     Last Simulated Block: {}", sim_data.last_simulated_block);
            }
            
            // Scam status
            if token_info.is_scam {
                warn!("   ⚠️  SCAM DETECTED: {}", token_info.scam_label.as_deref().unwrap_or("Unknown reason"));
            }
        }
    }
    
    // 5. Summary statistics
    info!("\n📈 Cache Summary:");
    info!("=================");
    info!("Total Unique Tokens: {}", all_tokens.len());
    info!("Total Authority Addresses: {}", all_creators.len());
    info!("Total Pools: {}", all_pools.len());
    
    // Calculate some interesting stats
    let mut multi_pool_tokens = 0;
    let mut scam_tokens = 0;
    let mut honeypot_tokens = 0;
    let mut trading_enabled_tokens = 0;
    
    for token_address in &all_tokens {
        if let Some(token_info) = cache.get_token(token_address).await {
            if token_info.pools.len() > 1 {
                multi_pool_tokens += 1;
            }
            if token_info.is_scam {
                scam_tokens += 1;
            }
            if token_info.trading_enabled {
                trading_enabled_tokens += 1;
            }
            if let Some(sim_data) = &token_info.simulation_data {
                if sim_data.is_honeypot {
                    honeypot_tokens += 1;
                }
            }
        }
    }
    
    info!("Tokens with Multiple Pools: {}", multi_pool_tokens);
    info!("Tokens with Trading Enabled: {}", trading_enabled_tokens);
    info!("Detected Scam Tokens: {}", scam_tokens);
    info!("Detected Honeypots: {}", honeypot_tokens);
    
    info!("\n✅ Inspection complete!");
    
    Ok(())
}