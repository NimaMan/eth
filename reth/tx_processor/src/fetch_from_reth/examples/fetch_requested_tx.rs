//! Example to fetch the specific transaction requested by the user
//!
//! This example fetches comprehensive information about transaction:
//! 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae

use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use alloy_primitives::B256;
use revm_tx_simulator_lib::fetch_from_reth::{
    RethDatabaseProvider, RethDataProvider, RethDataConfig,
    provider::{TransactionData, BlockId}
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🔍 Fetching Requested Transaction");
    println!("==================================\n");
    
    // The transaction hash to investigate
    let tx_hash = B256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")
        .expect("Valid transaction hash");
    
    println!("Transaction hash: 0x{:x}", tx_hash);
    println!("This transaction will be fetched directly from the Reth database.\n");
    
    // Get Reth data directory
    let datadir = get_reth_datadir()?;
    println!("📂 Using Reth data directory: {}", datadir.display());
    
    // Create configuration
    let _config = RethDataConfig::new(&datadir)
        .with_read_only(true)
        .with_static_files(true);
    
    // Create provider
    let provider = RethDatabaseProvider::new(&datadir)?;
    println!("✅ Connected to Reth database\n");
    
    // Fetch the transaction
    println!("📥 Fetching transaction data...");
    match provider.fetch_transaction(tx_hash) {
        Ok(tx_data) => {
            println!("✅ Transaction found!\n");
            print_detailed_transaction_info(&tx_data);
            
            // Also fetch the block data for additional context
            println!("\n📦 Fetching block data...");
            match provider.fetch_block(BlockId::Number(tx_data.block_number)) {
                Ok(block) => {
                    println!("✅ Block found!\n");
                    println!("Block Information:");
                    println!("-----------------");
                    println!("  Number: {}", block.number);
                    println!("  Hash: 0x{:x}", block.hash);
                    println!("  Timestamp: {} ({})", block.timestamp, format_timestamp(block.timestamp));
                    println!("  Miner: 0x{:x}", block.miner);
                    println!("  Total Transactions: {}", block.transaction_count);
                    println!("  Gas Used: {} / {} ({:.1}%)", 
                        block.gas_used, 
                        block.gas_limit,
                        (block.gas_used as f64 / block.gas_limit as f64) * 100.0
                    );
                }
                Err(e) => {
                    println!("❌ Failed to fetch block: {}", e);
                }
            }
            
            // If this is a contract interaction, try to fetch the contract data
            if let Some(to_addr) = tx_data.to {
                println!("\n🔍 Checking if recipient is a contract...");
                match provider.fetch_account(to_addr) {
                    Ok(account) => {
                        if let Some(code_size) = account.code_size {
                            if code_size > 0 {
                                println!("✅ Recipient is a contract!");
                                println!("  Address: 0x{:x}", to_addr);
                                println!("  Code Size: {} bytes", code_size);
                                println!("  Code Hash: 0x{:x}", account.code_hash);
                                println!("  Balance: {} wei ({:.6} ETH)", account.balance, wei_to_eth(account.balance));
                            } else {
                                println!("ℹ️  Recipient is an EOA (Externally Owned Account)");
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to fetch account data: {}", e);
                    }
                }
            }
            
            // If the sender is interesting, fetch its data too
            println!("\n👤 Fetching sender account data...");
            match provider.fetch_account(tx_data.from) {
                Ok(account) => {
                    println!("✅ Sender account found!");
                    println!("  Address: 0x{:x}", tx_data.from);
                    println!("  Balance: {} wei ({:.6} ETH)", account.balance, wei_to_eth(account.balance));
                    println!("  Nonce: {}", account.nonce);
                    if let Some(code_size) = account.code_size {
                        if code_size > 0 {
                            println!("  Type: Contract (Code Size: {} bytes)", code_size);
                        } else {
                            println!("  Type: EOA (Externally Owned Account)");
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to fetch sender account: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Transaction not found: {}", e);
            println!("\nPossible reasons:");
            println!("1. Your Reth node hasn't synced to the block containing this transaction");
            println!("2. The transaction hash is incorrect");
            println!("3. Database access issues");
            
            // Try to provide more context
            match provider.latest_block_number() {
                Ok(latest) => {
                    println!("\nYour Reth node has synced up to block: {}", latest);
                }
                Err(_) => {}
            }
        }
    }
    
    Ok(())
}

/// Print detailed transaction information
fn print_detailed_transaction_info(tx: &TransactionData) {
    println!("Transaction Details:");
    println!("===================");
    println!("  Hash: 0x{:x}", tx.hash);
    println!("  Block: {} (0x{:x})", tx.block_number, tx.block_hash);
    println!("  Position: Index {} in block", tx.transaction_index);
    println!();
    println!("  From: 0x{:x}", tx.from);
    
    if let Some(to) = tx.to {
        println!("  To: 0x{:x}", to);
    } else {
        println!("  To: Contract Creation");
        if let Some(contract_addr) = tx.contractaddress {
            println!("  Created Contract: 0x{:x}", contract_addr);
        }
    }
    
    println!();
    println!("  Value: {} wei ({:.6} ETH)", tx.value, wei_to_eth(tx.value));
    println!("  Nonce: {}", tx.nonce);
    println!();
    println!("  Gas Limit: {}", tx.gas_limit);
    println!("  Gas Used: {} ({:.1}%)", tx.gas_used, (tx.gas_used as f64 / tx.gas_limit as f64) * 100.0);
    println!("  Gas Price: {} wei ({:.2} gwei)", tx.gas_price, wei_to_gwei(tx.gas_price));
    println!("  Transaction Cost: {} wei ({:.6} ETH)", 
        tx.gas_price * alloy_primitives::U256::from(tx.gas_used),
        wei_to_eth(tx.gas_price * alloy_primitives::U256::from(tx.gas_used))
    );
    println!();
    println!("  Status: {}", if tx.receipt_status { "✅ Success" } else { "❌ Failed" });
    println!("  Input Data Size: {} bytes", tx.input.len());
    
    if tx.input.len() > 0 {
        println!("  Input Data (first 128 bytes):");
        let display_len = std::cmp::min(tx.input.len(), 128);
        println!("    0x{}", hex::encode(&tx.input[..display_len]));
        if tx.input.len() > 128 {
            println!("    ... ({} more bytes)", tx.input.len() - 128);
        }
        
        // Try to decode the function selector if it's a contract call
        if tx.input.len() >= 4 {
            let selector = &tx.input[..4];
            println!("  Function Selector: 0x{}", hex::encode(selector));
        }
    }
    
    println!();
    println!("  Event Logs: {} events", tx.logs.len());
    for (i, log) in tx.logs.iter().enumerate() {
        println!("    Log #{}: Address=0x{:x}, Topics={}", 
            i, 
            log.address,
            log.topics().len()
        );
        if let Some(topic0) = log.topics().first() {
            println!("      Topic[0]: 0x{:x}", topic0);
        }
    }
}

/// Get Reth data directory from environment or use default
fn get_reth_datadir() -> eyre::Result<PathBuf> {
    if let Ok(datadir) = env::var("RETH_DATADIR") {
        Ok(PathBuf::from(datadir))
    } else {
        // Use the known path from the user's system
        let default_dir = PathBuf::from("/home/nima/.local/share/reth/mainnet");
        
        if !default_dir.exists() {
            println!("⚠️  Default Reth directory not found: {}", default_dir.display());
            println!("   Please set RETH_DATADIR environment variable");
        }
        
        Ok(default_dir)
    }
}

/// Format timestamp to human-readable date
fn format_timestamp(timestamp: u64) -> String {
    use chrono::{DateTime, Utc};
    let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
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