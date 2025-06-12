//! Basic usage example for the fetch_from_reth module
//!
//! This example demonstrates how to connect to a Reth database and fetch
//! transaction data directly from MDBX storage.
//!
//! To run this example:
//!   cargo run --bin fetch_from_reth_basic_usage
//!
//! Prerequisites:
//! - Local Reth node with synced database
//! - Set RETH_DATADIR environment variable or use default path

use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use alloy_primitives::B256;
use revm_tx_simulator_lib::fetch_from_reth::{
    RethDatabaseProvider, RethDataProvider, RethDataConfig, CacheConfig,
    provider::{BlockId, TransactionData, BlockData, AccountData}
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🔧 Fetch From Reth - Basic Usage Example");
    println!("=========================================\n");
    
    // Step 1: Configure database access
    let datadir = get_reth_datadir()?;
    println!("📂 Using Reth data directory: {}", datadir.display());
    
    // Create configuration with read-only access
    let config = RethDataConfig::new(&datadir)
        .with_read_only(true)           // Always use read-only for external access
        .with_static_files(true)        // Enable access to older blocks
        .with_consistency_check(true);  // Verify database integrity
    
    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("❌ Configuration validation failed: {}", e);
        eprintln!("💡 Make sure your Reth node is installed and has synced some blocks");
        return Ok(());
    }
    
    println!("✅ Database configuration validated");
    
    // Step 2: Create provider with caching
    let cache_config = CacheConfig::default()
        .with_max_size(1000)            // Cache up to 1000 transactions
        .with_ttl(std::time::Duration::from_secs(300)); // 5 minutes TTL
    
    let provider = RethDatabaseProvider::with_cache_config(config, cache_config)
        .map_err(|e| eyre::eyre!("Failed to create provider: {}", e))?;
    
    println!("✅ Connected to Reth database successfully");
    
    // Step 3: Get latest block number
    match provider.latest_block_number() {
        Ok(latest_block) => {
            println!("📊 Latest block number: {}", latest_block);
            
            if latest_block < 1000 {
                println!("⚠️  Database has very few blocks. Make sure Reth has synced some data.");
                return Ok(());
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to get latest block: {}", e);
            return Ok(());
        }
    }
    
    // Step 4: Fetch single transaction
    println!("\n🔍 Fetching Single Transaction");
    println!("------------------------------");
    
    // Use the transaction hash the user wants to look into
    let tx_hash = B256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")
        .expect("Valid transaction hash");
    
    println!("Transaction hash: 0x{:x}", tx_hash);
    
    match provider.fetch_transaction(tx_hash) {
        Ok(tx_data) => {
            println!("✅ Transaction found!");
            print_transaction_details(&tx_data);
        }
        Err(e) => {
            println!("❌ Transaction fetch failed: {}", e);
            println!("💡 This may be normal if your Reth node doesn't have this specific transaction");
        }
    }
    
    // Step 5: Demonstrate batch fetching
    println!("\n📦 Batch Transaction Fetching");
    println!("-----------------------------");
    
    let batch_hashes = vec![
        B256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae").unwrap(),
        B256::from_str("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060").unwrap(),
        B256::from_str("0x7b944d902f33bf5289c80d4734288b206673377a85720c3993ad84743dd333d6").unwrap(),
    ];
    
    match provider.fetch_batch(&batch_hashes) {
        Ok(transactions) => {
            println!("✅ Batch fetch successful! Found {} transactions", transactions.len());
            
            for (i, tx) in transactions.iter().enumerate() {
                println!("  {}. Block {}: 0x{:x}", i + 1, tx.block_number, tx.hash);
                println!("     From: 0x{:x}", tx.from);
                if let Some(to) = tx.to {
                    println!("     To: 0x{:x}", to);
                } else {
                    println!("     To: Contract Creation");
                }
                println!("     Value: {} wei", tx.value);
                println!("     Gas Used: {}", tx.gas_used);
                println!();
            }
        }
        Err(e) => {
            println!("❌ Batch fetch failed: {}", e);
        }
    }
    
    // Step 6: Check transaction existence
    println!("🔍 Checking Transaction Existence");
    println!("---------------------------------");
    
    let test_hash = B256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
    
    match provider.transaction_exists(test_hash) {
        Ok(exists) => {
            if exists {
                println!("✅ Transaction exists in database");
            } else {
                println!("ℹ️  Transaction not found (expected for test hash)");
            }
        }
        Err(e) => {
            println!("❌ Failed to check transaction existence: {}", e);
        }
    }
    
    // Step 7: Fetch block data
    println!("\n🏛️ Fetching Block Data");
    println!("----------------------");
    
    // Fetch latest block
    match provider.fetch_block(BlockId::Latest) {
        Ok(block) => {
            println!("✅ Latest block fetched!");
            print_block_summary(&block);
        }
        Err(e) => {
            println!("❌ Failed to fetch latest block: {}", e);
        }
    }
    
    // Fetch specific block by number
    let block_number = 100000; // Well-known early block
    match provider.fetch_block(BlockId::Number(block_number)) {
        Ok(block) => {
            println!("\n✅ Block {} fetched!", block_number);
            print_block_summary(&block);
        }
        Err(e) => {
            println!("❌ Failed to fetch block {}: {}", block_number, e);
        }
    }
    
    // Step 8: Fetch account state
    println!("\n👤 Fetching Account State");
    println!("------------------------");
    
    // Use a well-known address (e.g., WETH contract)
    let weth_address = alloy_primitives::Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
        .expect("Valid address");
    
    match provider.fetch_account(weth_address) {
        Ok(account) => {
            println!("✅ Account state fetched!");
            print_account_info(&account);
        }
        Err(e) => {
            println!("❌ Failed to fetch account: {}", e);
        }
    }
    
    // Fetch balance only (faster)
    match provider.fetch_balance(weth_address) {
        Ok(balance) => {
            println!("\n💰 Current balance: {} wei ({:.6} ETH)", balance, wei_to_eth(balance));
        }
        Err(e) => {
            println!("❌ Failed to fetch balance: {}", e);
        }
    }
    
    // Step 9: Chain information
    println!("\n🌐 Chain Information");
    println!("-------------------");
    
    match provider.chain_info() {
        Ok(info) => {
            println!("✅ Chain info fetched!");
            println!("   Latest block: {}", info.latest_block);
            println!("   Latest hash: 0x{:x}", info.latest_hash);
            if let Some(safe) = info.safe_block {
                println!("   Safe block: {}", safe);
            }
            if let Some(finalized) = info.finalized_block {
                println!("   Finalized block: {}", finalized);
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch chain info: {}", e);
        }
    }
    
    // Step 10: Show cache statistics
    println!("\n📈 Cache Performance Statistics");
    println!("------------------------------");
    
    let stats = provider.cache_stats();
    println!("Total requests: {}", stats.total_requests);
    println!("Cache hits: {}", stats.hits);
    println!("Cache misses: {}", stats.misses);
    println!("Hit rate: {:.1}%", stats.hit_rate());
    println!("Current cache size: {}", stats.current_size);
    
    if stats.is_performing_well() {
        println!("✅ Cache is performing well!");
    } else if stats.total_requests > 10 {
        println!("⚠️  Cache hit rate could be improved");
    }
    
    println!("\n🎉 Basic usage example completed successfully!");
    println!("💡 Explore other examples:");
    println!("   - address_analysis: Deep dive into address data");
    println!("   - block_exploration: Comprehensive block analysis");
    println!("   - performance_optimization: Advanced caching and batching");
    
    Ok(())
}

/// Get Reth data directory from environment or use default
fn get_reth_datadir() -> eyre::Result<PathBuf> {
    if let Ok(datadir) = env::var("RETH_DATADIR") {
        Ok(PathBuf::from(datadir))
    } else {
        // Use platform-specific default
        let default_dir = dirs::data_dir()
            .ok_or_else(|| eyre::eyre!("Could not determine data directory"))?
            .join("reth")
            .join("mainnet");
        
        println!("💡 RETH_DATADIR not set, using default: {}", default_dir.display());
        println!("   Set RETH_DATADIR environment variable to use a custom path");
        
        Ok(default_dir)
    }
}

/// Print block summary
fn print_block_summary(block: &BlockData) {
    println!("📋 Block Summary:");
    println!("   Number: {}", block.number);
    println!("   Hash: 0x{:x}", block.hash);
    println!("   Parent: 0x{:x}", block.parent_hash);
    println!("   Timestamp: {} ({})", block.timestamp, format_timestamp(block.timestamp));
    println!("   Miner: 0x{:x}", block.miner);
    println!("   Transactions: {}", block.transaction_count);
    println!("   Gas Used: {} / {} ({:.1}%)", 
        block.gas_used, 
        block.gas_limit,
        (block.gas_used as f64 / block.gas_limit as f64) * 100.0
    );
    if let Some(td) = block.total_difficulty {
        println!("   Total Difficulty: {}", td);
    }
}

/// Print account information
fn print_account_info(account: &AccountData) {
    println!("📋 Account Information:");
    println!("   Address: 0x{:x}", account.address);
    println!("   Balance: {} wei ({:.6} ETH)", account.balance, wei_to_eth(account.balance));
    println!("   Nonce: {}", account.nonce);
    println!("   Code Hash: 0x{:x}", account.code_hash);
    
    if let Some(code_size) = account.code_size {
        if code_size > 0 {
            println!("   Code Size: {} bytes (contract)", code_size);
        } else {
            println!("   Code Size: 0 bytes (EOA)");
        }
    }
}

/// Format timestamp to human-readable date
fn format_timestamp(timestamp: u64) -> String {
    use chrono::{DateTime, Utc};
    let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Print detailed transaction information
fn print_transaction_details(tx: &TransactionData) {
    println!("📋 Transaction Details:");
    println!("   Hash: 0x{:x}", tx.hash);
    println!("   Block: {} (0x{:x})", tx.block_number, tx.block_hash);
    println!("   Index: {}", tx.transaction_index);
    println!("   From: 0x{:x}", tx.from);
    
    if let Some(to) = tx.to {
        println!("   To: 0x{:x}", to);
    } else {
        println!("   To: Contract Creation");
        if let Some(contract_addr) = tx.contractaddress {
            println!("   Contract Address: 0x{:x}", contract_addr);
        }
    }
    
    println!("   Value: {} wei ({:.6} ETH)", tx.value, wei_to_eth(tx.value));
    println!("   Nonce: {}", tx.nonce);
    println!("   Gas Limit: {}", tx.gas_limit);
    println!("   Gas Used: {} ({:.1}%)", tx.gas_used, (tx.gas_used as f64 / tx.gas_limit as f64) * 100.0);
    println!("   Gas Price: {} wei ({:.2} gwei)", tx.gas_price, wei_to_gwei(tx.gas_price));
    println!("   Status: {}", if tx.receipt_status { "✅ Success" } else { "❌ Failed" });
    println!("   Input Size: {} bytes", tx.input.len());
    println!("   Logs: {} events", tx.logs.len());
}

/// Convert wei to ETH
fn wei_to_eth(wei: alloy_primitives::U256) -> f64 {
    let eth_divisor = alloy_primitives::U256::from(10u64.pow(18));
    let eth_amount = wei / eth_divisor;
    eth_amount.to_string().parse().unwrap_or(0.0)
}

/// Convert wei to gwei
fn wei_to_gwei(wei: alloy_primitives::U256) -> f64 {
    let gwei_divisor = alloy_primitives::U256::from(10u64.pow(9));
    let gwei_amount = wei / gwei_divisor;
    gwei_amount.to_string().parse().unwrap_or(0.0)
}