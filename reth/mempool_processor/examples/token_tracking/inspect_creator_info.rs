/// Inspect Creator and Token Information
///
/// Get comprehensive token information for creator addresses and analyze pools.
/// Includes functionality to export all creator tokens and their pools to CSV.
///
/// Algorithm:
/// 1. Connect to token tracking cache via ZMQ subscriber
/// 2. For a given creator address, retrieve all created tokens
/// 3. For each token, collect all associated pools with reserves and price ratios
/// 4. Display comprehensive information in console
/// 5. Optionally export all data to CSV with complete pool information
/// 6. Calculate price ratios (denom_reserve / token_reserve) for each pool
/// 7. Track per-pool trading enabled status and other pool-specific metadata

use std::time::Duration;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, debug, error};
use chrono::Utc;
use serde_json;
use zmq;

use mempool_processor::token_tracking::{TokenTrackingSubscriber, types::{TokenQueryResponse, TokenInfo, PoolInfo}};

const ZMQ_REP_ENDPOINT: &str = "tcp://localhost:5558";

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
    
    /// Export creator's tokens and pools to CSV
    #[arg(long, default_value = "false")]
    export_csv: bool,
    
    /// Fetch all tokens from Python cache (comprehensive export)
    #[arg(long, default_value = "false")]
    export_all: bool,
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
    
    // Get tokens by creator (will be empty if not a creator)
    let tokens_by_creator = if all_creators.contains(&args.creator) {
        info!("✅ Found address in creators set!");
        let tokens = token_cache.get_tokens_by_creator(&args.creator).await;
        info!("\n📊 Total tokens created by this address: {}", tokens.len());
        tokens
    } else {
        Vec::new()
    };
    
    if !tokens_by_creator.is_empty() {
        
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
    } else if !all_creators.contains(&args.creator) {
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
    
    // Export to CSV if requested
    if args.export_csv || args.export_all {
        info!("\n📊 Exporting data to CSV...");
        
        let mut token_data_to_export = HashMap::new();
        
        if args.export_all {
            // Export all tokens from Python cache
            match request_all_tokens().await {
                Ok(response) => {
                    if response.status == "success" {
                        if let Some(data) = response.data {
                            token_data_to_export = data;
                            info!("✅ Retrieved {} tokens from Python cache", token_data_to_export.len());
                        }
                    } else {
                        error!("Failed to get all tokens: {}", response.error.unwrap_or("Unknown error".to_string()));
                    }
                }
                Err(e) => {
                    error!("Failed to request all tokens: {}", e);
                }
            }
        } else {
            // Export only creator's tokens
            for token_address in &tokens_by_creator {
                if let Some(token_info) = token_cache.get_token(&token_address).await {
                    token_data_to_export.insert(token_address.clone(), token_info);
                }
            }
        }
        
        if !token_data_to_export.is_empty() {
            let csv_filename = if args.export_all {
                format!("all_tokens_pools_{}.csv", Utc::now().format("%Y%m%d_%H%M%S"))
            } else {
                format!("creator_{}_tokens_pools_{}.csv", 
                    &args.creator[2..8], // First 6 chars after 0x
                    Utc::now().format("%Y%m%d_%H%M%S"))
            };
            
            match write_token_pool_csv(&token_data_to_export, &csv_filename) {
                Ok(_) => info!("✅ CSV file written: {}", csv_filename),
                Err(e) => error!("Failed to write CSV: {}", e),
            }
        }
    }
    
    Ok(())
}

async fn request_all_tokens() -> Result<TokenQueryResponse, Box<dyn std::error::Error>> {
    info!("Connecting to Python REP socket at {}", ZMQ_REP_ENDPOINT);
    
    let context = zmq::Context::new();
    let requester = context.socket(zmq::REQ)?;
    requester.connect(ZMQ_REP_ENDPOINT)?;
    
    // Create request for all tokens
    let request = serde_json::json!({
        "type": "get_all_tokens"
    });
    
    info!("Sending get_all_tokens request");
    requester.send(&request.to_string(), 0)?;
    
    // Receive response
    let response_str = match requester.recv_string(0) {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("ZMQ string conversion error: {:?}", e).into()),
        Err(e) => return Err(format!("ZMQ recv error: {:?}", e).into()),
    };
    
    debug!("Received response: {} bytes", response_str.len());
    
    // Parse response
    let response: TokenQueryResponse = serde_json::from_str(&response_str)?;
    
    Ok(response)
}

fn write_token_pool_csv(token_data: &HashMap<String, TokenInfo>, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(filename)?;
    
    // Write CSV header with all relevant fields
    writeln!(file, "token_address,token_symbol,token_name,token_decimals,total_supply,creator_address,current_owner,ownership_renounced,creation_block,creation_txn,latest_activity_block,is_scam_token,scam_label_token,pool_address,pool_type,denom_address,denom_currency,denom_reserve,token_reserve,pool_trading_enabled,pool_trading_enabled_block,pool_trading_enabled_txn,is_scam_pool,scam_label_pool,price_ratio,fee_tier,pool_id,latest_block_number,last_update_time")?;
    
    let mut total_records = 0;
    let mut tokens_processed = 0;
    
    // Process each token
    for (token_address, token_info) in token_data {
        tokens_processed += 1;
        
        // If token has no pools, write one record with token info only
        if token_info.pools.is_empty() {
            write_token_record(&mut file, token_address, token_info, None, None)?;
            total_records += 1;
        } else {
            // Write one record for each token-pool pair
            for (pool_address, pool_info) in &token_info.pools {
                write_token_record(&mut file, token_address, token_info, Some(pool_address), Some(pool_info))?;
                total_records += 1;
            }
        }
        
        if tokens_processed % 100 == 0 {
            debug!("Processed {} tokens, {} records", tokens_processed, total_records);
        }
    }
    
    info!("✅ CSV export complete: {} tokens, {} token-pool records", tokens_processed, total_records);
    
    Ok(())
}

fn write_token_record(
    file: &mut File, 
    token_address: &str, 
    token_info: &TokenInfo, 
    pool_address: Option<&String>, 
    pool_info: Option<&PoolInfo>
) -> Result<(), Box<dyn std::error::Error>> {
    
    // Escape CSV field values
    let escape_csv = |s: &str| -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace("\"", "\"\""))
        } else {
            s.to_string()
        }
    };
    
    // Token fields
    let token_symbol = token_info.symbol.as_deref().unwrap_or("").to_string();
    let token_name = token_info.name.as_deref().unwrap_or("").to_string();
    let token_decimals = token_info.decimals.map(|d| d.to_string()).unwrap_or_else(|| "".to_string());
    let total_supply = token_info.total_supply.as_deref().unwrap_or("").to_string();
    let scam_label_token = token_info.scam_label.as_deref().unwrap_or("").to_string();
    
    // Pool fields (empty if no pool)
    let pool_addr = pool_address.map(|s| s.as_str()).unwrap_or("");
    let pool_type = pool_info.map(|p| p.pool_type.as_str()).unwrap_or("");
    let denom_address = pool_info.map(|p| p.denom_address.as_str()).unwrap_or("");
    let denom_currency = pool_info.map(|p| p.denom_currency.as_str()).unwrap_or("");
    let denom_reserve = pool_info.map(|p| p.denom_reserve.to_string()).unwrap_or_else(|| "".to_string());
    let token_reserve = pool_info.map(|p| p.token_reserve.to_string()).unwrap_or_else(|| "".to_string());
    
    // Per-pool trading enabled fields
    let pool_trading_enabled = pool_info
        .and_then(|p| p.trading_enabled)
        .map(|b| b.to_string())
        .unwrap_or_else(|| "".to_string());
    let pool_trading_enabled_block = pool_info
        .and_then(|p| p.trading_enabled_block)
        .map(|b| b.to_string())
        .unwrap_or_else(|| "".to_string());
    let pool_trading_enabled_txn = pool_info
        .and_then(|p| p.trading_enabled_txn.as_deref())
        .unwrap_or("");
    
    // Pool scam fields
    let is_scam_pool = pool_info.map(|p| p.is_scam.to_string()).unwrap_or_else(|| "".to_string());
    let scam_label_pool = pool_info.and_then(|p| p.scam_label.as_deref()).unwrap_or("");
    
    // Pool-specific fields
    let fee_tier = pool_info
        .and_then(|p| p.fee_tier)
        .map(|f| f.to_string())
        .unwrap_or_else(|| "".to_string());
    let pool_id = pool_info.and_then(|p| p.pool_id.as_deref()).unwrap_or("");
    let latest_block_number = pool_info.map(|p| p.latest_block_number.to_string()).unwrap_or_else(|| "".to_string());
    let last_update_time = pool_info
        .and_then(|p| p.last_update_time)
        .map(|t| t.to_string())
        .unwrap_or_else(|| "".to_string());
    
    // Calculate price ratio (denom_reserve / token_reserve if both > 0)
    let price_ratio = if let Some(pool) = pool_info {
        if pool.token_reserve > 0.0 && pool.denom_reserve > 0.0 {
            (pool.denom_reserve / pool.token_reserve).to_string()
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    };
    
    // Write the CSV record
    writeln!(
        file,
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        escape_csv(token_address),
        escape_csv(&token_symbol),
        escape_csv(&token_name),
        token_decimals,
        escape_csv(&total_supply),
        escape_csv(&token_info.creator_address),
        escape_csv(&token_info.current_owner),
        token_info.ownership_renounced,
        token_info.creation_block,
        escape_csv(&token_info.creation_txn),
        token_info.latest_activity_block,
        token_info.is_scam,
        escape_csv(&scam_label_token),
        escape_csv(pool_addr),
        escape_csv(pool_type),
        escape_csv(denom_address),
        escape_csv(denom_currency),
        denom_reserve,
        token_reserve,
        pool_trading_enabled,
        pool_trading_enabled_block,
        escape_csv(pool_trading_enabled_txn),
        is_scam_pool,
        escape_csv(scam_label_pool),
        price_ratio,
        fee_tier,
        escape_csv(pool_id),
        latest_block_number,
        last_update_time
    )?;
    
    Ok(())
}