//! Block-focused data access example for the fetch_from_reth module
//!
//! This example demonstrates ALL possible ways to fetch block-related data
//! from the Reth database, including:
//! - Complete block data (headers, transactions, receipts)
//! - Block ranges and batch operations
//! - Block metadata and chain information
//! - Block-based transaction and receipt analysis
//! - Historical block data access
//!
//! To run this example:
//!   cargo run --bin fetch_from_reth_block_data_access
//!
//! Prerequisites:
//! - Local Reth node with synced database
//! - Set RETH_DATADIR environment variable

use std::env;
use std::path::PathBuf;

use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use revm_tx_simulator_lib::fetch_from_reth::{
    RethDatabaseProvider, RethDataProvider, RethDataConfig,
    provider::{BlockId, BlockData}
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("📦 Block Data Access - Comprehensive Example");
    println!("===========================================\n");
    
    // Setup database connection
    let datadir = get_reth_datadir()?;
    let config = RethDataConfig::new(&datadir);
    let provider = RethDatabaseProvider::with_config(config)?;
    
    println!("✅ Connected to Reth database");
    
    // Example 1: Chain and Block Overview
    println!("🌐 Example 1: Chain and Block Overview");
    println!("=====================================");
    analyze_chain_overview(&provider).await?;
    
    // Example 2: Detailed Block Analysis
    println!("\n📋 Example 2: Detailed Block Analysis");
    println!("====================================");
    analyze_block_details(&provider).await?;
    
    // Example 3: Block Range Operations
    println!("\n📊 Example 3: Block Range Operations");
    println!("===================================");
    analyze_block_ranges(&provider).await?;
    
    // Example 4: Transaction Analysis within Blocks
    println!("\n🔄 Example 4: Transaction Analysis within Blocks");
    println!("===============================================");
    analyze_block_transactions(&provider).await?;
    
    // Example 5: Receipt Analysis for Blocks
    println!("\n📃 Example 5: Receipt Analysis for Blocks");
    println!("========================================");
    analyze_block_receipts(&provider).await?;
    
    // Example 6: Block Batch Operations
    println!("\n⚡ Example 6: Block Batch Operations");
    println!("==================================");
    demonstrate_block_batch_operations(&provider).await?;
    
    // Example 7: Block-based Performance Analysis
    println!("\n⚡ Example 7: Block Performance Analysis");
    println!("======================================");
    analyze_block_performance(&provider).await?;
    
    // Example 8: Block Data Validation and Integrity
    println!("\n🔍 Example 8: Block Data Validation");
    println!("===================================");
    validate_block_integrity(&provider).await?;
    
    println!("\n🎉 Block data access examples completed!");
    println!("💡 This example demonstrated:");
    println!("   • Complete block data extraction");
    println!("   • Block range and batch operations");
    println!("   • Transaction and receipt analysis within blocks");
    println!("   • Chain state and metadata access");
    println!("   • Performance optimization techniques");
    println!("   • Data validation and integrity checks");
    
    Ok(())
}

/// Analyze chain overview and basic block information
async fn analyze_chain_overview(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Analyzing chain overview...");
    
    // Get comprehensive chain information
    let chain_info = provider.chain_info()?;
    
    println!("📊 Chain Information:");
    println!("   Latest Block: {}", chain_info.latest_block);
    println!("   Latest Hash: 0x{:x}", chain_info.latest_hash);
    
    if let Some(safe) = chain_info.safe_block {
        println!("   Safe Block: {}", safe);
    }
    
    if let Some(finalized) = chain_info.finalized_block {
        println!("   Finalized Block: {}", finalized);
    }
    
    if let Some(total_txs) = chain_info.total_transactions {
        println!("   Total Transactions: {}", total_txs);
    }
    
    // Test different block identifiers
    println!("\n🔍 Testing Block Identifiers:");
    
    let test_identifiers = [
        (BlockId::Latest, "Latest"),
        (BlockId::Safe, "Safe"),
        (BlockId::Finalized, "Finalized"),
        (BlockId::Number(chain_info.latest_block.saturating_sub(1)), "Latest-1"),
        (BlockId::Hash(chain_info.latest_hash), "By Hash"),
    ];
    
    for (block_id, description) in test_identifiers {
        match provider.block_exists(block_id.clone()) {
            Ok(exists) => {
                println!("   {} Block: {}", description, if exists { "✅ Exists" } else { "❌ Not found" });
            }
            Err(e) => {
                println!("   {} Block: ❌ Error - {}", description, e);
            }
        }
    }
    
    // Show recent block progression
    println!("\n📈 Recent Block Progression:");
    let latest = chain_info.latest_block;
    for i in (0..5).rev() {
        let block_num = latest.saturating_sub(i);
        match provider.block_number_to_hash(block_num) {
            Ok(Some(hash)) => {
                println!("   Block {}: {}", block_num, format_hash_short(hash));
            }
            Ok(None) => {
                println!("   Block {}: Hash not found", block_num);
            }
            Err(e) => {
                println!("   Block {}: Error - {}", block_num, e);
            }
        }
    }
    
    Ok(())
}

/// Perform detailed analysis of a specific block
async fn analyze_block_details(provider: &RethDatabaseProvider) -> Result<()> {
    let latest_block_num = provider.latest_block_number()?;
    
    println!("🔍 Analyzing latest block details (block {})...", latest_block_num);
    
    // Fetch complete block data
    match provider.fetch_block(BlockId::Latest) {
        Ok(block) => {
            println!("📦 Block Details:");
            print_block_summary(&block);
            
            // Analyze block timing
            if latest_block_num > 0 {
                let prev_block_num = latest_block_num - 1;
                match provider.fetch_block(BlockId::Number(prev_block_num)) {
                    Ok(prev_block) => {
                        let time_diff = block.timestamp.saturating_sub(prev_block.timestamp);
                        println!("   ⏱️  Block Time: {} seconds (from previous block)", time_diff);
                        
                        let gas_utilization = (block.gas_used as f64 / block.gas_limit as f64) * 100.0;
                        println!("   ⛽ Gas Utilization: {:.1}%", gas_utilization);
                        
                        // Compare with previous block
                        let prev_gas_utilization = (prev_block.gas_used as f64 / prev_block.gas_limit as f64) * 100.0;
                        let utilization_change = gas_utilization - prev_gas_utilization;
                        if utilization_change > 0.1 {
                            println!("   📈 Gas utilization increased by {:.1}%", utilization_change);
                        } else if utilization_change < -0.1 {
                            println!("   📉 Gas utilization decreased by {:.1}%", utilization_change.abs());
                        }
                    }
                    Err(_) => {
                        println!("   Could not fetch previous block for comparison");
                    }
                }
            }
            
            // Analyze block efficiency
            if block.transaction_count > 0 {
                let avg_gas_per_tx = block.gas_used / block.transaction_count as u64;
                println!("   📊 Average Gas per Transaction: {}", avg_gas_per_tx);
                
                let tx_density = block.transaction_count as f64 / (block.gas_limit as f64 / 1_000_000.0);
                println!("   📊 Transaction Density: {:.2} tx/MGas", tx_density);
            }
            
        }
        Err(e) => {
            println!("❌ Failed to fetch block details: {}", e);
        }
    }
    
    Ok(())
}

/// Demonstrate block range operations
async fn analyze_block_ranges(provider: &RethDatabaseProvider) -> Result<()> {
    let latest_block = provider.latest_block_number()?;
    
    if latest_block < 10 {
        println!("⚠️  Not enough blocks for range analysis");
        return Ok(());
    }
    
    let start_block = latest_block.saturating_sub(5);
    let end_block = latest_block;
    
    println!("🔍 Analyzing block range: {} to {}", start_block, end_block);
    
    // Fetch block range
    match provider.fetch_blocks_range(start_block, end_block) {
        Ok(blocks) => {
            println!("📊 Block Range Analysis ({} blocks):", blocks.len());
            
            let mut total_gas_used = 0u64;
            let mut total_gas_limit = 0u64;
            let mut total_transactions = 0;
            let mut total_time_span = 0u64;
            
            for (i, block) in blocks.iter().enumerate() {
                total_gas_used += block.gas_used;
                total_gas_limit += block.gas_limit;
                total_transactions += block.transaction_count;
                
                if i == 0 {
                    println!("   First Block ({}): {} tx, {} gas used", 
                             block.number, block.transaction_count, block.gas_used);
                } else if i == blocks.len() - 1 {
                    println!("   Last Block ({}): {} tx, {} gas used", 
                             block.number, block.transaction_count, block.gas_used);
                    total_time_span = block.timestamp - blocks[0].timestamp;
                }
            }
            
            if blocks.len() > 1 {
                println!("\n📈 Range Statistics:");
                println!("   Total Transactions: {}", total_transactions);
                println!("   Average Transactions per Block: {:.1}", 
                         total_transactions as f64 / blocks.len() as f64);
                println!("   Total Gas Used: {}", total_gas_used);
                println!("   Average Gas Utilization: {:.1}%", 
                         (total_gas_used as f64 / total_gas_limit as f64) * 100.0);
                
                if total_time_span > 0 {
                    println!("   Time Span: {} seconds", total_time_span);
                    println!("   Average Block Time: {:.1} seconds", 
                             total_time_span as f64 / (blocks.len() - 1) as f64);
                    println!("   Transaction Rate: {:.1} tx/second", 
                             total_transactions as f64 / total_time_span as f64);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch block range: {}", e);
        }
    }
    
    Ok(())
}

/// Analyze transactions within blocks
async fn analyze_block_transactions(provider: &RethDatabaseProvider) -> Result<()> {
    let latest_block = provider.latest_block_number()?;
    
    println!("🔍 Analyzing transactions in latest block ({})", latest_block);
    
    // Fetch all transactions in the latest block
    match provider.fetch_transactions_by_block(BlockId::Latest) {
        Ok(transactions) => {
            if transactions.is_empty() {
                println!("   📭 No transactions in this block");
                return Ok(());
            }
            
            println!("📊 Transaction Analysis ({} transactions):", transactions.len());
            
            // Transaction type analysis
            let mut eoa_to_eoa = 0;
            let mut eoa_to_contract = 0;
            let mut contract_creation = 0;
            let mut total_value = U256::ZERO;
            let mut total_gas_used = 0u64;
            let mut successful_txs = 0;
            
            for tx in &transactions {
                match tx.to {
                    Some(_) => {
                        // Check if recipient is a contract (simplified)
                        if tx.input.len() > 0 {
                            eoa_to_contract += 1;
                        } else {
                            eoa_to_eoa += 1;
                        }
                    }
                    None => {
                        contract_creation += 1;
                    }
                }
                
                total_value += tx.value;
                total_gas_used += tx.gas_used;
                
                if tx.receipt_status {
                    successful_txs += 1;
                }
            }
            
            println!("   📈 Transaction Types:");
            println!("     EOA → EOA: {}", eoa_to_eoa);
            println!("     EOA → Contract: {}", eoa_to_contract);
            println!("     Contract Creation: {}", contract_creation);
            
            println!("   💰 Value Transfer:");
            println!("     Total ETH Transferred: {:.6} ETH", wei_to_eth(total_value));
            println!("     Average per Transaction: {:.6} ETH", 
                     wei_to_eth(total_value) / transactions.len() as f64);
            
            println!("   ⛽ Gas Usage:");
            println!("     Total Gas Used: {}", total_gas_used);
            println!("     Average per Transaction: {}", total_gas_used / transactions.len() as u64);
            
            println!("   ✅ Success Rate: {:.1}% ({}/{})", 
                     (successful_txs as f64 / transactions.len() as f64) * 100.0,
                     successful_txs, transactions.len());
            
            // Show top transactions by value
            let mut sorted_txs = transactions.clone();
            sorted_txs.sort_by(|a, b| b.value.cmp(&a.value));
            
            println!("\n💎 Top Transactions by Value:");
            for (i, tx) in sorted_txs.iter().take(3).enumerate() {
                if tx.value > U256::ZERO {
                    println!("   {}. {} - {:.6} ETH", 
                             i + 1, 
                             format_hash_short(tx.hash),
                             wei_to_eth(tx.value));
                }
            }
            
            // Show gas outliers
            let mut gas_sorted = transactions.clone();
            gas_sorted.sort_by(|a, b| b.gas_used.cmp(&a.gas_used));
            
            println!("\n⛽ Highest Gas Consumers:");
            for (i, tx) in gas_sorted.iter().take(3).enumerate() {
                println!("   {}. {} - {} gas", 
                         i + 1, 
                         format_hash_short(tx.hash),
                         tx.gas_used);
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch block transactions: {}", e);
        }
    }
    
    Ok(())
}

/// Analyze receipts for blocks
async fn analyze_block_receipts(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Analyzing receipts in latest block");
    
    // Fetch all receipts for the latest block
    match provider.fetch_receipts_by_block(BlockId::Latest) {
        Ok(receipts) => {
            if receipts.is_empty() {
                println!("   📭 No receipts in this block");
                return Ok(());
            }
            
            println!("📃 Receipt Analysis ({} receipts):", receipts.len());
            
            let mut total_gas_used = 0u64;
            let mut successful_receipts = 0;
            let mut total_logs = 0;
            let mut contracts_created = 0;
            
            for receipt in &receipts {
                total_gas_used += receipt.gas_used;
                total_logs += receipt.logs.len();
                
                if receipt.status {
                    successful_receipts += 1;
                }
                
                if receipt.contractaddress.is_some() {
                    contracts_created += 1;
                }
            }
            
            println!("   📊 Receipt Statistics:");
            println!("     Successful: {} ({:.1}%)", successful_receipts,
                     (successful_receipts as f64 / receipts.len() as f64) * 100.0);
            println!("     Total Gas Used: {}", total_gas_used);
            println!("     Total Event Logs: {}", total_logs);
            println!("     Contracts Created: {}", contracts_created);
            
            if total_logs > 0 {
                println!("     Average Logs per Receipt: {:.1}", 
                         total_logs as f64 / receipts.len() as f64);
            }
            
            // Show most active receipts (by log count)
            let mut log_sorted = receipts.clone();
            log_sorted.sort_by(|a, b| b.logs.len().cmp(&a.logs.len()));
            
            println!("\n📋 Most Active Transactions (by event logs):");
            for (i, receipt) in log_sorted.iter().take(3).enumerate() {
                if receipt.logs.len() > 0 {
                    println!("   {}. {} - {} events", 
                             i + 1, 
                             format_hash_short(receipt.transaction_hash),
                             receipt.logs.len());
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch block receipts: {}", e);
        }
    }
    
    Ok(())
}

/// Demonstrate batch operations for blocks
async fn demonstrate_block_batch_operations(provider: &RethDatabaseProvider) -> Result<()> {
    let latest_block = provider.latest_block_number()?;
    
    if latest_block < 5 {
        println!("⚠️  Not enough blocks for batch operations demo");
        return Ok(());
    }
    
    println!("🔍 Demonstrating block batch operations");
    
    // Create batch of block identifiers
    let block_ids = vec![
        BlockId::Number(latest_block),
        BlockId::Number(latest_block.saturating_sub(1)),
        BlockId::Number(latest_block.saturating_sub(2)),
        BlockId::Latest,
        BlockId::Safe,
    ];
    
    // Batch fetch blocks
    match provider.fetch_blocks_batch(&block_ids) {
        Ok(blocks) => {
            println!("📦 Batch Block Fetch Results ({} blocks):", blocks.len());
            
            for block in &blocks {
                println!("   Block {}: {} tx, {} gas used", 
                         block.number, block.transaction_count, block.gas_used);
            }
            
            // Calculate batch statistics
            if blocks.len() > 1 {
                let total_txs: usize = blocks.iter().map(|b| b.transaction_count).sum();
                let total_gas: u64 = blocks.iter().map(|b| b.gas_used).sum();
                
                println!("\n📊 Batch Statistics:");
                println!("   Total Transactions: {}", total_txs);
                println!("   Total Gas Used: {}", total_gas);
                println!("   Average Transactions per Block: {:.1}", 
                         total_txs as f64 / blocks.len() as f64);
            }
        }
        Err(e) => {
            println!("❌ Batch block fetch failed: {}", e);
        }
    }
    
    Ok(())
}

/// Analyze block performance characteristics
async fn analyze_block_performance(provider: &RethDatabaseProvider) -> Result<()> {
    let latest_block = provider.latest_block_number()?;
    
    if latest_block < 20 {
        println!("⚠️  Not enough blocks for performance analysis");
        return Ok(());
    }
    
    println!("🔍 Analyzing block performance (last 20 blocks)");
    
    let start_block = latest_block.saturating_sub(19);
    let end_block = latest_block;
    
    // Measure fetch performance
    let start_time = std::time::Instant::now();
    match provider.fetch_blocks_range(start_block, end_block) {
        Ok(blocks) => {
            let fetch_duration = start_time.elapsed();
            
            println!("⚡ Performance Metrics:");
            println!("   Blocks Fetched: {}", blocks.len());
            println!("   Fetch Time: {:?}", fetch_duration);
            println!("   Average per Block: {:.2}ms", 
                     fetch_duration.as_millis() as f64 / blocks.len() as f64);
            
            // Analyze block intervals
            let mut block_times = Vec::new();
            for i in 1..blocks.len() {
                let time_diff = blocks[i].timestamp - blocks[i-1].timestamp;
                block_times.push(time_diff);
            }
            
            if !block_times.is_empty() {
                let avg_block_time = block_times.iter().sum::<u64>() as f64 / block_times.len() as f64;
                let min_time = *block_times.iter().min().unwrap_or(&0);
                let max_time = *block_times.iter().max().unwrap_or(&0);
                
                println!("\n⏱️  Block Timing Analysis:");
                println!("   Average Block Time: {:.1} seconds", avg_block_time);
                println!("   Fastest Block: {} seconds", min_time);
                println!("   Slowest Block: {} seconds", max_time);
                
                if max_time > avg_block_time as u64 * 2 {
                    println!("   ⚠️  Detected slow block ({}s vs {:.1}s avg)", max_time, avg_block_time);
                }
            }
            
            // Analyze throughput
            let total_transactions: usize = blocks.iter().map(|b| b.transaction_count).sum();
            let time_span = blocks.last().unwrap().timestamp - blocks.first().unwrap().timestamp;
            
            if time_span > 0 {
                let tps = total_transactions as f64 / time_span as f64;
                println!("\n📊 Throughput Analysis:");
                println!("   Total Transactions: {}", total_transactions);
                println!("   Time Span: {} seconds", time_span);
                println!("   Network TPS: {:.2}", tps);
            }
        }
        Err(e) => {
            println!("❌ Performance analysis failed: {}", e);
        }
    }
    
    Ok(())
}

/// Validate block data integrity
async fn validate_block_integrity(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Validating block data integrity");
    
    let _latest_block = provider.latest_block_number()?;
    
    // Test block hash/number consistency
    match provider.fetch_block(BlockId::Latest) {
        Ok(block) => {
            println!("📋 Block Integrity Checks:");
            
            // Check hash-to-number consistency
            match provider.block_hash_to_number(block.hash) {
                Ok(Some(number)) => {
                    if number == block.number {
                        println!("   ✅ Hash-to-number mapping consistent");
                    } else {
                        println!("   ❌ Hash-to-number mapping inconsistent: {} vs {}", number, block.number);
                    }
                }
                Ok(None) => {
                    println!("   ❌ Block hash not found in mapping");
                }
                Err(e) => {
                    println!("   ❌ Error checking hash-to-number mapping: {}", e);
                }
            }
            
            // Check number-to-hash consistency  
            match provider.block_number_to_hash(block.number) {
                Ok(Some(hash)) => {
                    if hash == block.hash {
                        println!("   ✅ Number-to-hash mapping consistent");
                    } else {
                        println!("   ❌ Number-to-hash mapping inconsistent");
                    }
                }
                Ok(None) => {
                    println!("   ❌ Block number not found in mapping");
                }
                Err(e) => {
                    println!("   ❌ Error checking number-to-hash mapping: {}", e);
                }
            }
            
            // Validate parent relationship
            if block.number > 0 {
                match provider.fetch_block(BlockId::Number(block.number - 1)) {
                    Ok(parent_block) => {
                        if parent_block.hash == block.parent_hash {
                            println!("   ✅ Parent-child relationship valid");
                        } else {
                            println!("   ❌ Parent-child relationship invalid");
                        }
                    }
                    Err(_) => {
                        println!("   ⚠️  Could not fetch parent block for validation");
                    }
                }
            }
            
            // Validate transaction count
            match provider.fetch_transactions_by_block(BlockId::Number(block.number)) {
                Ok(transactions) => {
                    if transactions.len() == block.transaction_count {
                        println!("   ✅ Transaction count matches block header");
                    } else {
                        println!("   ❌ Transaction count mismatch: {} vs {}", 
                                 transactions.len(), block.transaction_count);
                    }
                }
                Err(_) => {
                    println!("   ⚠️  Could not validate transaction count");
                }
            }
        }
        Err(e) => {
            println!("❌ Could not fetch block for integrity validation: {}", e);
        }
    }
    
    Ok(())
}

/// Print a summary of block data
fn print_block_summary(block: &BlockData) {
    println!("   📦 Block {}: {}", block.number, format_hash_short(block.hash));
    println!("   ⏰ Timestamp: {} ({})", block.timestamp, format_timestamp(block.timestamp));
    println!("   👨‍💻 Miner: {}", format_address_short(block.miner));
    println!("   🏠 Parent: {}", format_hash_short(block.parent_hash));
    println!("   🔄 Transactions: {}", block.transaction_count);
    println!("   ⛽ Gas Used: {} / {} ({:.1}%)", 
             block.gas_used, 
             block.gas_limit,
             (block.gas_used as f64 / block.gas_limit as f64) * 100.0);
    println!("   ⚖️  Difficulty: {}", block.difficulty);
    
    if let Some(total_difficulty) = block.total_difficulty {
        println!("   📊 Total Difficulty: {}", total_difficulty);
    }
    
    if !block.extra_data.is_empty() {
        let extra_preview = if block.extra_data.len() > 20 {
            format!("0x{}... ({} bytes)", hex::encode(&block.extra_data[..20]), block.extra_data.len())
        } else {
            format!("0x{}", hex::encode(&block.extra_data))
        };
        println!("   📝 Extra Data: {}", extra_preview);
    }
}

/// Format timestamp as human-readable string
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};
    
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(now) => {
            let block_time = Duration::from_secs(timestamp);
            if now > block_time {
                let ago = now - block_time;
                if ago.as_secs() < 60 {
                    format!("{} seconds ago", ago.as_secs())
                } else if ago.as_secs() < 3600 {
                    format!("{} minutes ago", ago.as_secs() / 60)
                } else {
                    format!("{} hours ago", ago.as_secs() / 3600)
                }
            } else {
                "Future timestamp".to_string()
            }
        }
        Err(_) => "Unknown time".to_string()
    }
}

/// Get Reth data directory from environment or use default
fn get_reth_datadir() -> Result<PathBuf> {
    if let Ok(datadir) = env::var("RETH_DATADIR") {
        Ok(PathBuf::from(datadir))
    } else {
        let default_dir = dirs::data_dir()
            .ok_or_else(|| eyre::eyre!("Could not determine data directory"))?
            .join("reth")
            .join("mainnet");
        
        println!("💡 RETH_DATADIR not set, using default: {}", default_dir.display());
        Ok(default_dir)
    }
}

/// Convert wei to ETH
fn wei_to_eth(wei: U256) -> f64 {
    let eth_divisor = U256::from(10u64.pow(18));
    if wei == U256::ZERO {
        return 0.0;
    }
    
    let eth_part = wei / eth_divisor;
    let remainder = wei % eth_divisor;
    
    let eth_value = eth_part.to_string().parse::<f64>().unwrap_or(0.0);
    let fractional_part = remainder.to_string().parse::<f64>().unwrap_or(0.0) / (10u64.pow(18) as f64);
    
    eth_value + fractional_part
}

/// Format hash for display (0x + first 4 hex chars)
fn format_hash_short(hash: B256) -> String {
    format!("0x{}", hex::encode(&hash.as_slice()[..4]))
}

/// Format address for display (0x + first 6 hex chars)
fn format_address_short(address: Address) -> String {
    format!("0x{}", hex::encode(&address.as_slice()[..6]))
}