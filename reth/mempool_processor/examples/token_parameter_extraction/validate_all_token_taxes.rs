/// Validate Tax Calculations for All Tokens in Cache
///
/// This tool systematically validates tax calculations for all tokens and pools
/// tracked in the TokenTrackingCache, extracting all available parameters including
/// price ratio, reserves, taxes, creator info, etc.
///
/// Usage: cargo run --example validate_all_token_taxes

use mempool_processor::token_tracking::TokenTrackingSubscriber;
use mempool_processor::simulator::{SequentialBuySellSimulator, BuySellSimulatorConfig};
use mempool_processor::token_parameter_extraction::{
    calculate_buy_tax_from_movements,
    calculate_sell_tax_from_movements,
};
use alloy_primitives::Address;
use std::str::FromStr;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use eyre::Result;
use chrono::Local;
use tracing::{info, warn, error};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting comprehensive token parameter extraction and tax validation");

    // Configuration
    let eth_threshold = 0.3; // ETH threshold for scam detection
    let log_dir = "/home/nima/code/crypto/logs/mempool/dev";
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    
    // Create log directory
    create_dir_all(log_dir)?;
    
    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = format!("{}/token_parameters_{}.csv", log_dir, timestamp);
    let mut csv_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&csv_path)?;
    
    // Write comprehensive CSV header
    writeln!(csv_file, 
        "token_address,token_symbol,token_name,pool_address,pool_type,denom_currency,\
        eth_reserve,token_reserve,price_ratio,market_cap_eth,\
        creator_address,creator_tx,block_number,creation_time,\
        can_buy,can_sell,buy_tax,sell_tax,\
        is_honeypot,total_supply,decimals,\
        liquidity_locked,owner_address,trading_enabled,\
        error_msg"
    )?;
    info!("📝 Logging results to: {}", csv_path);
    
    // Initialize TokenTrackingCache and subscriber
    info!("📡 Initializing token tracking cache and subscribing to pool updates...");
    let subscriber = TokenTrackingSubscriber::new(eth_threshold);
    let cache = subscriber.get_cache();
    
    // Start subscription in background
    let cache_clone = cache.clone();
    tokio::spawn(async move {
        let mut subscriber_mut = subscriber;
        if let Err(e) = subscriber_mut.start_listening().await {
            error!("Failed to subscribe to pool updates: {}", e);
        }
    });
    
    // Wait for initial sync (give it some time to populate)
    info!("⏳ Waiting for initial pool state synchronization...");
    sleep(Duration::from_secs(5)).await;
    
    // Get all pools from cache
    let all_pools = cache_clone.pools.get_all_pools().await;
    let pool_count = all_pools.len();
    
    if pool_count == 0 {
        warn!("⚠️ No pools found in cache. Make sure Python pool publisher is running.");
        warn!("   Run: python /home/nima/code/crypto/py/pool_state_publisher_batch.py");
        return Ok(());
    }
    
    info!("📊 Found {} pools to validate", pool_count);
    
    // Initialize simulator with updated config (0.01 ETH default)
    let sim_config = BuySellSimulatorConfig::default();
    let simulator = SequentialBuySellSimulator::with_config(reth_datadir, sim_config.clone())?;
    
    info!("🔧 Simulator configuration:");
    info!("   Buyer address: {}", sim_config.buyer_address);
    info!("   Test buy amount: 0.01 ETH (with 0.001 ETH fallback)");
    info!("   Router: {}", sim_config.router_address);
    
    // Statistics tracking
    let mut processed = 0;
    let mut successful_buys = 0;
    let mut successful_sells = 0;
    let mut honeypots = 0;
    let mut high_tax_tokens = 0;
    let mut errors = 0;
    
    // Process each pool
    for (pool_address_str, pool_state) in all_pools.iter() {
        processed += 1;
        
        // Parse addresses
        let pool_address = match Address::from_str(pool_address_str) {
            Ok(addr) => addr,
            Err(e) => {
                error!("Invalid pool address {}: {}", pool_address_str, e);
                errors += 1;
                continue;
            }
        };
        
        let token_address = match Address::from_str(&pool_state.token_address) {
            Ok(addr) => addr,
            Err(e) => {
                error!("Invalid token address {}: {}", pool_state.token_address, e);
                errors += 1;
                continue;
            }
        };
        
        // Skip pools with very low liquidity
        if pool_state.eth_reserve < 0.01 {
            info!("⏭️  [{}/{}] Skipping pool {} - insufficient liquidity ({:.6} ETH)",
                processed, pool_count, pool_address_str, pool_state.eth_reserve);
            continue;
        }
        
        info!("🔄 [{}/{}] Processing token {} in pool {}",
            processed, pool_count,
            pool_state.token_address,
            pool_address_str
        );
        
        // Get token info from cache
        let token_info = cache_clone.get_token(&pool_state.token_address).await;
        
        // Extract all available parameters from token_info
        let token_symbol = token_info.as_ref()
            .and_then(|i| i.symbol.as_deref())
            .unwrap_or("");
        let token_name = token_info.as_ref()
            .and_then(|i| i.name.as_deref())
            .unwrap_or("");
        let pool_type = &pool_state.pool_type;
        let denom_currency = "ETH"; // Always ETH for now
        let eth_reserve = pool_state.eth_reserve;
        let token_reserve = pool_state.token_reserve;
        let price_ratio = if token_reserve > 0.0 {
            Some(eth_reserve / token_reserve)
        } else {
            None
        };
        
        // Calculate market cap if we have total supply
        let market_cap_eth = if let Some(ref info) = token_info {
            if let (Some(ref total_supply_str), Some(price)) = (info.total_supply.as_ref(), price_ratio) {
                if let Ok(total_supply) = total_supply_str.parse::<f64>() {
                    if price > 0.0 {
                        Some(total_supply * price)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // Creator info from token_info
        let creator_address = token_info.as_ref()
            .map(|i| i.creator_address.as_str())
            .unwrap_or("");
        let creator_tx = token_info.as_ref()
            .map(|i| i.creation_txn.as_str())
            .unwrap_or("");
        let block_number = token_info.as_ref()
            .map(|i| i.creation_block)
            .unwrap_or(0);
        let creation_time = "".to_string(); // Not available in current structure
        
        // Token metadata
        let total_supply = token_info.as_ref()
            .and_then(|i| i.total_supply.as_ref())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let decimals = token_info.as_ref()
            .and_then(|i| i.decimals)
            .unwrap_or(18);
        let owner_address = token_info.as_ref()
            .map(|i| i.current_owner.as_str())
            .unwrap_or("");
        
        // Run buy/sell simulation
        let (can_buy, can_sell, buy_tax, sell_tax, error_msg, is_honeypot) = 
            match simulator.simulate_sequence(token_address, pool_address, None).await {
                Ok(result) => {
                    let can_buy = result.buy_result.success;
                    let can_sell = result.sell_result.success;
                    
                    if can_buy {
                        successful_buys += 1;
                    }
                    if can_sell {
                        successful_sells += 1;
                    }
                    
                    let is_honeypot = can_buy && !can_sell;
                    if is_honeypot {
                        honeypots += 1;
                    }
                    
                    // Calculate buy tax using movement method
                    let buy_tax = if can_buy {
                        calculate_buy_tax_from_movements(
                            &result.buy_result.state_changes,
                            &pool_address,
                            &sim_config.buyer_address,
                        )
                    } else {
                        None
                    };
                    
                    // Calculate sell tax using movement method
                    let sell_tax = if can_sell {
                        calculate_sell_tax_from_movements(
                            &result.sell_result.state_changes,
                            &pool_address,
                            &sim_config.buyer_address,
                        )
                    } else {
                        None
                    };
                    
                    // Track high tax tokens
                    if let Some(bt) = buy_tax {
                        if bt > 10.0 {
                            high_tax_tokens += 1;
                        }
                    }
                    if let Some(st) = sell_tax {
                        if st > 10.0 && !high_tax_tokens.to_string().contains(&processed.to_string()) {
                            high_tax_tokens += 1;
                        }
                    }
                    
                    // Determine error message
                    let error_msg = if !can_buy {
                        result.buy_result.revert_reason.as_deref().unwrap_or("Buy failed")
                    } else if !can_sell {
                        result.sell_result.revert_reason.as_deref().unwrap_or("Sell failed")
                    } else {
                        ""
                    };
                    
                    (can_buy, can_sell, buy_tax, sell_tax, error_msg.to_string(), is_honeypot)
                }
                Err(e) => {
                    error!("Simulation error for {}: {}", pool_state.token_address, e);
                    errors += 1;
                    (false, false, None, None, format!("Simulation error: {}", e), false)
                }
            };
        
        // Format values for CSV
        let buy_tax_str = buy_tax.map(|t| format!("{:.2}", t)).unwrap_or_else(|| "".to_string());
        let sell_tax_str = sell_tax.map(|t| format!("{:.2}", t)).unwrap_or_else(|| "".to_string());
        let market_cap_str = market_cap_eth.map(|mc| format!("{:.6}", mc)).unwrap_or_else(|| "".to_string());
        let price_ratio_str = price_ratio.map(|pr| format!("{:.12}", pr)).unwrap_or_else(|| "".to_string());
        
        // Placeholder values for fields we don't have yet
        let liquidity_locked = ""; // Would need to check on-chain
        let trading_enabled = if can_buy { "true" } else { "false" };
        
        // Write comprehensive data to CSV
        writeln!(csv_file, 
            "{},{},{},{},{},{},{:.6},{:.2},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            pool_state.token_address,
            token_symbol,
            token_name,
            pool_address_str,
            pool_type,
            denom_currency,
            eth_reserve,
            token_reserve,
            price_ratio_str,
            market_cap_str,
            creator_address,
            creator_tx,
            block_number,
            creation_time.as_str(),
            can_buy,
            can_sell,
            buy_tax_str,
            sell_tax_str,
            is_honeypot,
            total_supply,
            decimals,
            liquidity_locked,
            owner_address,
            trading_enabled,
            error_msg
        )?;
        
        // Log interesting findings
        if let Some(bt) = buy_tax {
            if bt > 10.0 {
                warn!("   ⚠️ High buy tax: {:.2}%", bt);
            }
        }
        if let Some(st) = sell_tax {
            if st > 10.0 {
                warn!("   ⚠️ High sell tax: {:.2}%", st);
            }
        }
        if is_honeypot {
            warn!("   🍯 Honeypot detected!");
        }
        
        // Flush CSV file periodically
        if processed % 10 == 0 {
            csv_file.flush()?;
            info!("📊 Progress: {}/{} pools processed", processed, pool_count);
        }
        
        // Add small delay to avoid overwhelming the system
        if processed % 50 == 0 {
            sleep(Duration::from_millis(100)).await;
        }
    }
    
    // Final flush
    csv_file.flush()?;
    
    // Print summary
    info!("✅ Validation complete!");
    info!("📊 Summary:");
    info!("   Total pools processed: {}", processed);
    info!("   Successful buys: {} ({:.1}%)", successful_buys, 
        (successful_buys as f64 / processed.max(1) as f64) * 100.0);
    info!("   Successful sells: {} ({:.1}%)", successful_sells, 
        (successful_sells as f64 / processed.max(1) as f64) * 100.0);
    info!("   Potential honeypots: {} ({:.1}%)", honeypots, 
        (honeypots as f64 / processed.max(1) as f64) * 100.0);
    info!("   High tax tokens: {} ({:.1}%)", high_tax_tokens,
        (high_tax_tokens as f64 / processed.max(1) as f64) * 100.0);
    info!("   Errors: {} ({:.1}%)", errors, 
        (errors as f64 / processed.max(1) as f64) * 100.0);
    info!("📁 Results saved to: {}", csv_path);
    
    // Create detailed summary file
    let summary_path = format!("{}/token_parameters_summary_{}.txt", log_dir, timestamp);
    let mut summary_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&summary_path)?;
    
    writeln!(summary_file, "Token Parameter Extraction and Tax Validation Summary")?;
    writeln!(summary_file, "====================================================")?;
    writeln!(summary_file, "Timestamp: {}", Local::now())?;
    writeln!(summary_file)?;
    writeln!(summary_file, "Statistics:")?;
    writeln!(summary_file, "  Total pools processed: {}", processed)?;
    writeln!(summary_file, "  Successful buys: {} ({:.1}%)", successful_buys, 
        (successful_buys as f64 / processed.max(1) as f64) * 100.0)?;
    writeln!(summary_file, "  Successful sells: {} ({:.1}%)", successful_sells, 
        (successful_sells as f64 / processed.max(1) as f64) * 100.0)?;
    writeln!(summary_file, "  Potential honeypots: {} ({:.1}%)", honeypots, 
        (honeypots as f64 / processed.max(1) as f64) * 100.0)?;
    writeln!(summary_file, "  High tax tokens (>10%): {} ({:.1}%)", high_tax_tokens,
        (high_tax_tokens as f64 / processed.max(1) as f64) * 100.0)?;
    writeln!(summary_file, "  Errors: {} ({:.1}%)", errors, 
        (errors as f64 / processed.max(1) as f64) * 100.0)?;
    writeln!(summary_file)?;
    writeln!(summary_file, "Configuration:")?;
    writeln!(summary_file, "  ETH threshold: {}", eth_threshold)?;
    writeln!(summary_file, "  Test buy amount: 0.01 ETH (with 0.001 ETH fallback)")?;
    writeln!(summary_file, "  Buyer address: {}", sim_config.buyer_address)?;
    writeln!(summary_file, "  Router address: {}", sim_config.router_address)?;
    writeln!(summary_file, "  RETH datadir: {}", reth_datadir)?;
    writeln!(summary_file)?;
    writeln!(summary_file, "Parameters Extracted:")?;
    writeln!(summary_file, "  - Token address, symbol, name")?;
    writeln!(summary_file, "  - Pool address, type, denomination")?;
    writeln!(summary_file, "  - ETH and token reserves")?;
    writeln!(summary_file, "  - Price ratio and market cap")?;
    writeln!(summary_file, "  - Creator address and transaction")?;
    writeln!(summary_file, "  - Block number and creation time")?;
    writeln!(summary_file, "  - Buy/sell capability and taxes")?;
    writeln!(summary_file, "  - Honeypot detection")?;
    writeln!(summary_file, "  - Total supply and decimals")?;
    writeln!(summary_file, "  - Owner address and trading status")?;
    writeln!(summary_file)?;
    writeln!(summary_file, "Output files:")?;
    writeln!(summary_file, "  CSV results: {}", csv_path)?;
    writeln!(summary_file, "  This summary: {}", summary_path)?;
    
    info!("📁 Summary saved to: {}", summary_path);
    
    Ok(())
}