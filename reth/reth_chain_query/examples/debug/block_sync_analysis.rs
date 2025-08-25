/// Block Sync Analysis Example
/// 
/// Compares different "latest" block concepts to understand database sync status
/// and investigate why we might be behind the network tip.

use reth_chain_query::{ChainQuery, Result};
use std::time::{SystemTime, UNIX_EPOCH};

fn format_timestamp(timestamp: u64) -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(current_time) => {
            let current_timestamp = current_time.as_secs();
            let seconds_ago = current_timestamp.saturating_sub(timestamp);
            if seconds_ago < 60 {
                format!("{} seconds ago", seconds_ago)
            } else if seconds_ago < 3600 {
                format!("{} minutes ago", seconds_ago / 60)
            } else {
                format!("{} hours ago", seconds_ago / 3600)
            }
        }
        Err(_) => "unknown time".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Block Sync Analysis");
    println!("{}", "=".repeat(80));
    println!("📊 Investigating database sync status vs network tip");
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    // Get different "latest" block concepts
    println!("📋 Comparing Different Block Number Methods:");
    
    // Method 1: Our current get_latest_block (uses best_block_number)
    let best_block = chain_query.get_latest_block()?;
    let best_block_info = chain_query.block.get_block_info(Some(best_block)).await?;
    println!("  🔸 Best Block Number: {}", best_block);
    println!("     Timestamp: {} ({})", best_block_info.timestamp, format_timestamp(best_block_info.timestamp));
    println!("     Gas Used: {} / {} ({:.1}%)", best_block_info.gas_used, best_block_info.gas_limit, 
             best_block_info.gas_used as f64 / best_block_info.gas_limit as f64 * 100.0);
    
    println!();
    
    // Method 2: Try to access provider methods directly to get last/finalized/safe blocks
    println!("🔄 Testing Recent Block Access:");
    
    // Try to access blocks that might be newer than our "best" block
    let mut newer_blocks_found = 0;
    let mut latest_accessible = best_block;
    
    for offset in 1..=10 {
        let test_block = best_block + offset;
        match chain_query.block.get_block_info(Some(test_block)).await {
            Ok(info) => {
                newer_blocks_found += 1;
                latest_accessible = test_block;
                println!("  ✅ Block {} exists: {} ({})", test_block, info.timestamp, format_timestamp(info.timestamp));
            }
            Err(_) => {
                println!("  ❌ Block {} not found", test_block);
                break;
            }
        }
    }
    
    if newer_blocks_found > 0 {
        println!();
        println!("⚠️  SYNC LAG DETECTED!");
        println!("   Database 'best' block: {}", best_block);
        println!("   Actual latest accessible: {}", latest_accessible);
        println!("   Blocks behind: {}", latest_accessible - best_block);
    } else {
        println!();
        println!("✅ Database appears to be at true latest block");
    }
    
    println!();
    println!("🌐 External Network Comparison:");
    println!("   Current time: {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    println!("   'Best' block timestamp: {}", best_block_info.timestamp);
    println!("   Time lag: {} seconds", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - best_block_info.timestamp);
    
    let expected_blocks_ahead = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - best_block_info.timestamp) / 12;
    println!("   Expected blocks ahead on network: ~{}", expected_blocks_ahead);
    
    if expected_blocks_ahead > 3 {
        println!("   🚨 Database may be significantly behind network!");
    } else if expected_blocks_ahead > 1 {
        println!("   ⚠️  Database appears to be 1-3 blocks behind network");
    } else {
        println!("   ✅ Database appears to be synced with network");
    }
    
    println!();
    println!("💡 RECOMMENDATIONS:");
    if newer_blocks_found > 0 {
        println!("   • Use latest accessible block ({}) instead of 'best' block ({})", latest_accessible, best_block);
        println!("   • Consider implementing true latest block detection");
    }
    
    if expected_blocks_ahead > 1 {
        println!("   • Check Reth node sync status");
        println!("   • Database writing may lag behind block reception");
        println!("   • Consider using finalized blocks for consistency");
    }
    
    println!("   • Add block age warnings for stale data detection");
    println!("   • Document which 'latest' concept is being used");
    
    println!();
    println!("✅ Block sync analysis complete!");
    
    Ok(())
}