/// Validate Tax Calculations for All Tokens in Cache
///
/// This tool systematically validates tax calculations for all tokens and pools
/// tracked in the TokenTrackingCache using the movement-based calculation method.
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

    info!("🚀 Starting tax validation for all tracked tokens");

    // Configuration
    let eth_threshold = 0.3; // ETH threshold for scam detection
    let log_dir = "/home/nima/code/crypto/logs/mempool/dev";
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    
    // Create log directory
    create_dir_all(log_dir)?;
    
    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = format!("{}/tax_validation_{}.csv", log_dir, timestamp);
    let mut csv_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&csv_path)?;
    
    // Write CSV header
    writeln!(csv_file, "token_address,pool_address,pool_type,pool_eth_reserve,can_buy,can_sell,buy_tax,sell_tax,error_msg")?;
    info!("📝 Logging results to: {}", csv_path);
    
    // Initialize TokenTrackingCache and subscriber
    info!("📡 Initializing token tracking cache and subscribing to pool updates...");
    let mut subscriber = TokenTrackingSubscriber::new(eth_threshold);
    let cache = subscriber.get_cache();
    
    // Start subscription in background
    let cache_clone = cache.clone();
    tokio::spawn(async move {
        if let Err(e) = subscriber.start_listening().await {
            error!("Failed to start listening to pool updates: {}", e);
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
    
    // Initialize simulator with config
    let sim_config = BuySellSimulatorConfig::default();
    let simulator = SequentialBuySellSimulator::with_config(reth_datadir, sim_config.clone())?;
    
    info!("🔧 Simulator configuration:");
    info!("   Buyer address: {}", sim_config.buyer_address);
    info!("   Test buy amount: 0.1 ETH");
    info!("   Router: {}", sim_config.router_address);
    
    // Statistics tracking
    let mut processed = 0;
    let mut successful_buys = 0;
    let mut successful_sells = 0;
    let mut honeypots = 0;
    let mut errors = 0;
    
    // Process each pool
    for (pool_address_str, pool_state) in all_pools.iter() {
        processed += 1;
        
        // Debug: log pool_type for first few pools
        if processed <= 5 {
            info!("DEBUG: Pool {} has pool_type: '{}'", pool_address_str, pool_state.pool_type);
        }
        
        // Parse addresses
        let pool_address = match Address::from_str(pool_address_str) {
            Ok(addr) => addr,
            Err(e) => {
                error!("Invalid pool address {}: {}", pool_address_str, e);
                writeln!(csv_file, "{},{},{},{},false,false,,,Invalid pool address",
                    pool_state.token_address, pool_address_str, pool_state.pool_type, pool_state.eth_reserve)?;
                errors += 1;
                continue;
            }
        };
        
        let token_address = match Address::from_str(&pool_state.token_address) {
            Ok(addr) => addr,
            Err(e) => {
                error!("Invalid token address {}: {}", pool_state.token_address, e);
                writeln!(csv_file, "{},{},{},{},false,false,,,Invalid token address",
                    pool_state.token_address, pool_address_str, pool_state.pool_type, pool_state.eth_reserve)?;
                errors += 1;
                continue;
            }
        };
        
        // Skip pools with very low liquidity
        if pool_state.eth_reserve < 0.01 {
            info!("⏭️ Skipping pool {} - insufficient liquidity ({:.4} ETH)",
                pool_address_str, pool_state.eth_reserve);
            writeln!(csv_file, "{},{},{},{},false,false,,,Insufficient liquidity",
                pool_state.token_address, pool_address_str, pool_state.pool_type, pool_state.eth_reserve)?;
            continue;
        }
        
        info!("🔄 [{}/{}] Processing token {} in pool {}",
            processed, pool_count,
            pool_state.token_address,
            pool_address_str
        );
        
        // Run buy/sell simulation with the correct pool type
        match simulator.simulate_sequence_with_pool_type(
            token_address, 
            pool_address, 
            &pool_state.pool_type,
            None
        ).await {
            Ok(result) => {
                let can_buy = result.buy_result.success;
                let can_sell = result.sell_result.success;
                
                if can_buy {
                    successful_buys += 1;
                }
                if can_sell {
                    successful_sells += 1;
                } else if can_buy {
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
                
                // Determine error message
                let error_msg = if !can_buy {
                    result.buy_result.revert_reason.as_deref().unwrap_or("Buy failed")
                } else if !can_sell {
                    result.sell_result.revert_reason.as_deref().unwrap_or("Sell failed - potential honeypot")
                } else {
                    ""
                };
                
                // Format tax values for CSV
                let buy_tax_str = buy_tax.map(|t| format!("{:.2}", t)).unwrap_or_else(|| "".to_string());
                let sell_tax_str = sell_tax.map(|t| format!("{:.2}", t)).unwrap_or_else(|| "".to_string());
                
                // Write to CSV
                writeln!(csv_file, "{},{},{},{:.4},{},{},{},{},{}",
                    pool_state.token_address,
                    pool_address_str,
                    pool_state.pool_type,
                    pool_state.eth_reserve,
                    can_buy,
                    can_sell,
                    buy_tax_str,
                    sell_tax_str,
                    error_msg
                )?;
                
                // Log interesting findings
                if let Some(bt) = buy_tax {
                    if bt > 10.0 {
                        warn!("   ⚠️ High buy tax detected: {:.2}%", bt);
                    }
                }
                if let Some(st) = sell_tax {
                    if st > 10.0 {
                        warn!("   ⚠️ High sell tax detected: {:.2}%", st);
                    }
                }
                if can_buy && !can_sell {
                    warn!("   🍯 Potential honeypot detected!");
                }
            }
            Err(e) => {
                error!("Simulation error for {}: {}", pool_state.token_address, e);
                writeln!(csv_file, "{},{},{},{:.4},false,false,,,Simulation error: {}",
                    pool_state.token_address,
                    pool_address_str,
                    pool_state.pool_type,
                    pool_state.eth_reserve,
                    e
                )?;
                errors += 1;
            }
        }
        
        // Flush CSV file periodically
        if processed % 10 == 0 {
            csv_file.flush()?;
            info!("📊 Progress: {}/{} pools processed", processed, pool_count);
        }
    }
    
    // Final flush
    csv_file.flush()?;
    
    // Print summary
    info!("✅ Validation complete!");
    info!("📊 Summary:");
    info!("   Total pools processed: {}", processed);
    info!("   Successful buys: {} ({:.1}%)", successful_buys, (successful_buys as f64 / processed as f64) * 100.0);
    info!("   Successful sells: {} ({:.1}%)", successful_sells, (successful_sells as f64 / processed as f64) * 100.0);
    info!("   Potential honeypots: {} ({:.1}%)", honeypots, (honeypots as f64 / processed as f64) * 100.0);
    info!("   Errors: {} ({:.1}%)", errors, (errors as f64 / processed as f64) * 100.0);
    info!("📁 Results saved to: {}", csv_path);
    
    // Create summary file
    let summary_path = format!("{}/tax_validation_summary_{}.txt", log_dir, timestamp);
    let mut summary_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&summary_path)?;
    
    writeln!(summary_file, "Tax Validation Summary")?;
    writeln!(summary_file, "=====================")?;
    writeln!(summary_file, "Timestamp: {}", Local::now())?;
    writeln!(summary_file)?;
    writeln!(summary_file, "Statistics:")?;
    writeln!(summary_file, "  Total pools processed: {}", processed)?;
    writeln!(summary_file, "  Successful buys: {} ({:.1}%)", successful_buys, (successful_buys as f64 / processed as f64) * 100.0)?;
    writeln!(summary_file, "  Successful sells: {} ({:.1}%)", successful_sells, (successful_sells as f64 / processed as f64) * 100.0)?;
    writeln!(summary_file, "  Potential honeypots: {} ({:.1}%)", honeypots, (honeypots as f64 / processed as f64) * 100.0)?;
    writeln!(summary_file, "  Errors: {} ({:.1}%)", errors, (errors as f64 / processed as f64) * 100.0)?;
    writeln!(summary_file)?;
    writeln!(summary_file, "Configuration:")?;
    writeln!(summary_file, "  ETH threshold: {}", eth_threshold)?;
    writeln!(summary_file, "  Test buy amount: 0.1 ETH")?;
    writeln!(summary_file, "  Buyer address: {}", sim_config.buyer_address)?;
    writeln!(summary_file, "  Router address: {}", sim_config.router_address)?;
    writeln!(summary_file)?;
    writeln!(summary_file, "Output files:")?;
    writeln!(summary_file, "  CSV results: {}", csv_path)?;
    writeln!(summary_file, "  This summary: {}", summary_path)?;
    
    info!("📁 Summary saved to: {}", summary_path);
    
    Ok(())
}