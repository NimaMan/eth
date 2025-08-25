/// Block Methods Comparison Example
/// 
/// Tests Reth's different block number methods to understand their differences:
/// - best_block_number() vs last_block_number()
/// - In-memory state vs database state
/// - Which method gives the actual latest block in database

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
    println!("🔍 Reth Block Methods Comparison");
    println!("{}", "=".repeat(80));
    println!("📊 Understanding best_block_number() vs last_block_number()");
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    println!("📋 RETH BLOCK METHOD ANALYSIS:");
    println!();
    
    // Test our current method (best_block_number)
    println!("🔸 Method 1: get_latest_block() [uses best_block_number()]");
    let best_block = chain_query.get_latest_block()?;
    let best_block_info = chain_query.block.get_block_info(Some(best_block)).await?;
    println!("   Block Number: {}", best_block);
    println!("   Timestamp: {} ({})", best_block_info.timestamp, format_timestamp(best_block_info.timestamp));
    println!("   Gas Used: {} / {} ({:.1}%)", best_block_info.gas_used, best_block_info.gas_limit, 
             best_block_info.gas_used as f64 / best_block_info.gas_limit as f64 * 100.0);
    
    println!();
    
    // Now test accessing the provider directly to get last_block_number
    println!("🔹 Method 2: Direct Provider Access [testing different approaches]");
    
    // We need to access the provider factory to get last_block_number
    // Since we can't directly access it through our API, let's test block accessibility
    println!("   Testing block accessibility around best_block_number...");
    
    // Test blocks around the best block number
    let mut accessible_blocks = Vec::new();
    for offset in -5..=5i64 {
        let test_block = (best_block as i64 + offset) as u64;
        if test_block < best_block.saturating_sub(5) { continue; } // Skip negative numbers
        
        match chain_query.block.get_block_info(Some(test_block)).await {
            Ok(info) => {
                accessible_blocks.push((test_block, info.timestamp));
                println!("   ✅ Block {} accessible: {} ({})", 
                        test_block, info.timestamp, format_timestamp(info.timestamp));
            }
            Err(_) => {
                println!("   ❌ Block {} not accessible", test_block);
            }
        }
    }
    
    if let Some((latest_accessible, latest_timestamp)) = accessible_blocks.last() {
        println!();
        println!("📊 COMPARISON RESULTS:");
        println!("   best_block_number():     Block {}", best_block);
        println!("   Latest Accessible:      Block {}", latest_accessible);
        
        if *latest_accessible > best_block {
            println!("   🔍 FINDING: Database contains blocks newer than best_block_number()!");
            println!("   📝 This suggests best_block_number() is NOT the true database tip");
            println!("   📝 Actual database tip appears to be: Block {}", latest_accessible);
            
            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let latest_age = current_time.saturating_sub(*latest_timestamp);
            let best_age = current_time.saturating_sub(best_block_info.timestamp);
            
            println!("   ⏰ best_block_number() age: {} seconds", best_age);
            println!("   ⏰ Actual latest block age: {} seconds", latest_age);
            println!("   ⏰ Difference: {} seconds", best_age.saturating_sub(latest_age));
            
        } else if *latest_accessible == best_block {
            println!("   ✅ FINDING: best_block_number() matches actual database tip");
        } else {
            println!("   ❓ FINDING: Unexpected result - best_block_number() is higher than accessible blocks");
        }
    }
    
    println!();
    println!("📖 RETH IMPLEMENTATION UNDERSTANDING:");
    println!("   best_block_number(): Returns block from Finish stage checkpoint");
    println!("   last_block_number(): Returns highest block from CanonicalHeaders table");
    println!("   📝 Based on Reth source code:");
    println!("      - best_block_number() = get_stage_checkpoint(StageId::Finish).block_number");
    println!("      - last_block_number() = max(CanonicalHeaders.last(), static_files.highest())");
    
    println!();
    
    // Current time comparison
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let block_age = current_time.saturating_sub(best_block_info.timestamp);
    
    println!("🌐 NETWORK SYNC ANALYSIS:");
    println!("   Current timestamp: {}", current_time);
    println!("   Block timestamp: {}", best_block_info.timestamp);
    println!("   Block age: {} seconds", block_age);
    
    let expected_network_blocks = block_age / 12;
    println!("   Expected blocks ahead on network: ~{}", expected_network_blocks);
    
    if expected_network_blocks > 3 {
        println!("   🚨 Database may be significantly behind network!");
        println!("   💡 Recommendation: Check Reth node sync status");
    } else if expected_network_blocks > 1 {
        println!("   ⚠️  Database appears 1-3 blocks behind network (normal)");
        println!("   💡 Recommendation: This is typical database writing lag");
    } else {
        println!("   ✅ Database appears synced with network");
    }
    
    println!();
    println!("✅ Block methods analysis complete!");
    
    Ok(())
}