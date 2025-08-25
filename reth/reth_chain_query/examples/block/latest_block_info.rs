/// Latest Block Information Example
/// 
/// Demonstrates querying the latest block information including
/// gas metrics, timestamps, and block details. This is useful for
/// monitoring network conditions and block production.

use reth_chain_query::{ChainQuery, Result};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn format_timestamp(timestamp: u64) -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(current_time) => {
            let current_timestamp = current_time.as_secs();
            let block_time = SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp);
            
            match SystemTime::now().duration_since(block_time) {
                Ok(elapsed) => {
                    let seconds_ago = elapsed.as_secs();
                    if seconds_ago < 60 {
                        format!("{} seconds ago", seconds_ago)
                    } else if seconds_ago < 3600 {
                        format!("{} minutes ago", seconds_ago / 60)
                    } else if seconds_ago < 86400 {
                        format!("{} hours ago", seconds_ago / 3600)
                    } else {
                        format!("{} days ago", seconds_ago / 86400)
                    }
                }
                Err(_) => {
                    // Block is in the future (shouldn't happen)
                    format!("{} seconds in the future", (timestamp as i64) - (current_timestamp as i64))
                }
            }
        }
        Err(_) => "unknown time".to_string()
    }
}

fn format_gas_price(gas_price_wei: u64) -> String {
    let gwei = gas_price_wei as f64 / 1_000_000_000.0;
    format!("{:.2} Gwei", gwei)
}

fn format_gas_percentage(gas_used: u64, gas_limit: u64) -> f64 {
    (gas_used as f64 / gas_limit as f64) * 100.0
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧱 Latest Block Information");
    println!("{}", "=".repeat(60));
    println!("📊 Querying current Ethereum network state");
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    let overall_start = Instant::now();
    
    // Get latest block number
    println!("🔍 Getting latest block number...");
    let block_number_start = Instant::now();
    let latest_block = chain_query.get_latest_block()?;
    let block_number_time = block_number_start.elapsed();
    
    println!("   Latest Block: {} ({:.2}ms)", latest_block, block_number_time.as_millis());
    println!("   ℹ️  Using Reth's best_block_number (latest fully processed block)");
    
    // Get detailed block information
    println!("📋 Getting detailed block information...");
    let block_info_start = Instant::now();
    let block_info = chain_query.block.get_block_info(Some(latest_block)).await?;
    let block_info_time = block_info_start.elapsed();
    
    println!("   Block info retrieved in {:.2}ms", block_info_time.as_millis());
    println!();
    
    let total_query_time = overall_start.elapsed();
    
    // Display comprehensive block information
    println!("{}", "=".repeat(60));
    println!("📈 BLOCK #{} DETAILS", latest_block);
    println!("{}", "=".repeat(60));
    
    println!("⏰ TIMING INFORMATION:");
    println!("   Block Number: {}", block_info.number);
    println!("   Timestamp: {} ({})", block_info.timestamp, format_timestamp(block_info.timestamp));
    
    // Add database sync status information
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let block_age_seconds = current_time.saturating_sub(block_info.timestamp);
    let expected_network_blocks = block_age_seconds / 12;
    
    if block_age_seconds > 60 {
        println!("   🚨 SYNC WARNING: Block is {} seconds old (~{} blocks behind network)", block_age_seconds, expected_network_blocks);
    } else if expected_network_blocks >= 3 {
        println!("   ⚠️  DATABASE LAG: Block is {} seconds old (~{} blocks behind network)", block_age_seconds, expected_network_blocks);
    } else if expected_network_blocks >= 2 {
        println!("   ℹ️  NORMAL LAG: Block is {} seconds old (~{} blocks behind network)", block_age_seconds, expected_network_blocks);
    } else {
        println!("   ✅ SYNCED: Block is {} seconds old (up to date)", block_age_seconds);
    }
    
    // Calculate block time if we have previous blocks
    if latest_block > 0 {
        println!("   Querying previous block for timing analysis...");
        let prev_start = Instant::now();
        match chain_query.block.get_block_info(Some(latest_block - 1)).await {
            Ok(prev_block) => {
                let prev_query_time = prev_start.elapsed();
                let block_time = block_info.timestamp.saturating_sub(prev_block.timestamp);
                println!("   Block Time: {} seconds (prev block query: {:.2}ms)", block_time, prev_query_time.as_millis());
            }
            Err(_) => {
                println!("   Block Time: Unable to calculate");
            }
        }
    }
    
    println!();
    
    println!("⛽ GAS METRICS:");
    println!("   Gas Limit: {}", block_info.gas_limit);
    println!("   Gas Used: {}", block_info.gas_used);
    println!("   Gas Utilization: {:.1}%", format_gas_percentage(block_info.gas_used, block_info.gas_limit));
    
    if let Some(base_fee) = block_info.base_fee_per_gas {
        println!("   Base Fee: {} ({} wei)", format_gas_price(base_fee as u64), base_fee);
    } else {
        println!("   Base Fee: Not available (pre-EIP-1559)");
    }
    
    println!();
    
    // Query multiple recent blocks for trend analysis
    println!("📊 RECENT BLOCK ANALYSIS (last 10 blocks):");
    let blocks_start = Instant::now();
    
    let mut block_times = Vec::new();
    let mut gas_utilizations = Vec::new();
    let mut base_fees = Vec::new();
    
    for i in 0..10 {
        if latest_block >= i {
            let block_num = latest_block - i;
            match chain_query.block.get_block_info(Some(block_num)).await {
                Ok(info) => {
                    gas_utilizations.push(format_gas_percentage(info.gas_used, info.gas_limit));
                    
                    if let Some(base_fee) = info.base_fee_per_gas {
                        base_fees.push(base_fee as f64 / 1_000_000_000.0); // Convert to Gwei
                    }
                    
                    // Calculate block time if not the oldest block
                    if i < 9 && latest_block >= i + 1 {
                        match chain_query.block.get_block_info(Some(block_num - 1)).await {
                            Ok(prev_info) => {
                                let time_diff = info.timestamp.saturating_sub(prev_info.timestamp);
                                block_times.push(time_diff);
                            }
                            Err(_) => {}
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
    
    let blocks_query_time = blocks_start.elapsed();
    println!("   Recent blocks analyzed in {:.2}ms", blocks_query_time.as_millis());
    
    // Block time statistics
    if !block_times.is_empty() {
        let avg_block_time = block_times.iter().sum::<u64>() as f64 / block_times.len() as f64;
        let min_block_time = *block_times.iter().min().unwrap();
        let max_block_time = *block_times.iter().max().unwrap();
        
        println!("   Average Block Time: {:.1}s", avg_block_time);
        println!("   Min Block Time: {}s", min_block_time);
        println!("   Max Block Time: {}s", max_block_time);
        
        // Network health indicator
        if avg_block_time > 15.0 {
            println!("   ⚠️  Blocks are slower than target (12-13s)");
        } else if avg_block_time < 11.0 {
            println!("   🚀 Blocks are faster than usual");
        } else {
            println!("   ✅ Block times are healthy");
        }
    }
    
    // Gas utilization statistics
    if !gas_utilizations.is_empty() {
        let avg_gas_util = gas_utilizations.iter().sum::<f64>() / gas_utilizations.len() as f64;
        let max_gas_util = gas_utilizations.iter().fold(0.0f64, |a, &b| a.max(b));
        let min_gas_util = gas_utilizations.iter().fold(100.0f64, |a, &b| a.min(b));
        
        println!("   Average Gas Utilization: {:.1}%", avg_gas_util);
        println!("   Min Gas Utilization: {:.1}%", min_gas_util);
        println!("   Max Gas Utilization: {:.1}%", max_gas_util);
        
        // Network congestion indicator
        if avg_gas_util > 90.0 {
            println!("   🔴 Network is highly congested");
        } else if avg_gas_util > 70.0 {
            println!("   🟡 Network has moderate congestion");
        } else {
            println!("   🟢 Network is not congested");
        }
    }
    
    // Base fee trend
    if base_fees.len() >= 3 {
        let recent_fees = &base_fees[0..3];
        let avg_recent = recent_fees.iter().sum::<f64>() / recent_fees.len() as f64;
        
        let older_fees = &base_fees[3..];
        let avg_older = if !older_fees.is_empty() {
            older_fees.iter().sum::<f64>() / older_fees.len() as f64
        } else {
            avg_recent
        };
        
        println!("   Average Base Fee (recent): {:.2} Gwei", avg_recent);
        println!("   Average Base Fee (older): {:.2} Gwei", avg_older);
        
        let fee_trend = ((avg_recent - avg_older) / avg_older) * 100.0;
        if fee_trend > 10.0 {
            println!("   📈 Base fees are rising ({:+.1}%)", fee_trend);
        } else if fee_trend < -10.0 {
            println!("   📉 Base fees are falling ({:+.1}%)", fee_trend);
        } else {
            println!("   ➡️  Base fees are stable ({:+.1}%)", fee_trend);
        }
    }
    
    println!();
    println!("{}", "=".repeat(60));
    println!("⚡ PERFORMANCE SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Total Query Time: {:.2}ms", total_query_time.as_millis());
    println!("Block Number Query: {:.2}ms", block_number_time.as_millis());
    println!("Block Info Query: {:.2}ms", block_info_time.as_millis());
    println!("Recent Blocks Analysis: {:.2}ms", blocks_query_time.as_millis());
    
    // Compare with RPC estimates
    let estimated_rpc_time = 100 + 200 + (10 * 200); // block number + block info + 10 recent blocks
    let speedup = estimated_rpc_time as f64 / total_query_time.as_millis() as f64;
    println!();
    println!("🔄 RPC Comparison:");
    println!("Estimated RPC time: {}ms", estimated_rpc_time);
    println!("Actual query time: {}ms", total_query_time.as_millis());
    println!("Speedup: {:.1}x faster", speedup);
    
    println!("\n💡 This data enables real-time network monitoring and congestion analysis!");
    println!("✅ Block analysis complete!");
    
    Ok(())
}