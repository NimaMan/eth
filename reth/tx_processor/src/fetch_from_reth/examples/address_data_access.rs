//! Address-focused data access example for the fetch_from_reth module
//!
//! This example demonstrates ALL possible ways to fetch data related to an Ethereum address
//! from the Reth database, including:
//! - Account state (balance, nonce, code)
//! - Storage data and changes
//! - Historical state at different blocks
//! - Transaction history
//! - Receipt data for address activity
//!
//! To run this example:
//!   cargo run --bin fetch_from_reth_address_data_access
//!
//! Prerequisites:
//! - Local Reth node with synced database
//! - Set RETH_DATADIR environment variable

use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use revm_tx_simulator_lib::fetch_from_reth::{
    RethDatabaseProvider, RethDataProvider, RethDataConfig,
    provider::BlockId
};

// Example addresses for demonstration
const USDC_CONTRACT: &str = "0xA0b86a33E6441c8E2f2FBE96C5F41F0Ef93F8B3A"; // USDC contract
const VITALIK_ADDRESS: &str = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"; // Vitalik's address
const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984"; // Uniswap V3 Factory

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🔍 Address Data Access - Comprehensive Example");
    println!("===============================================\n");
    
    // Setup database connection
    let datadir = get_reth_datadir()?;
    let config = RethDataConfig::new(&datadir);
    let provider = RethDatabaseProvider::with_config(config)?;
    
    println!("✅ Connected to Reth database");
    
    // Get chain info first
    let chain_info = provider.chain_info()?;
    println!("📊 Chain Info:");
    println!("   Latest block: {}", chain_info.latest_block);
    println!("   Latest hash: 0x{:x}", chain_info.latest_hash);
    if let Some(safe) = chain_info.safe_block {
        println!("   Safe block: {}", safe);
    }
    if let Some(finalized) = chain_info.finalized_block {
        println!("   Finalized block: {}", finalized);
    }
    println!();
    
    // Example 1: Comprehensive Account Analysis
    println!("🏦 Example 1: Comprehensive Account Analysis");
    println!("===========================================");
    analyze_account_comprehensively(&provider, VITALIK_ADDRESS).await?;
    
    // Example 2: Contract State and Storage Analysis
    println!("\n📋 Example 2: Contract State and Storage Analysis");
    println!("================================================");
    analyze_contract_storage(&provider, USDC_CONTRACT).await?;
    
    // Example 3: Historical State Tracking
    println!("\n⏰ Example 3: Historical State Tracking");
    println!("======================================");
    track_historical_state(&provider, VITALIK_ADDRESS).await?;
    
    // Example 4: Address Activity Analysis
    println!("\n🔄 Example 4: Address Activity Analysis");
    println!("======================================");
    analyze_address_activity(&provider, UNISWAP_V3_FACTORY).await?;
    
    // Example 5: Multi-Address Batch Analysis
    println!("\n📊 Example 5: Multi-Address Batch Analysis");
    println!("==========================================");
    batch_address_analysis(&provider).await?;
    
    // Example 6: Storage Change Tracking
    println!("\n🔄 Example 6: Storage Change Tracking");
    println!("====================================");
    track_storage_changes(&provider, USDC_CONTRACT).await?;
    
    println!("\n🎉 Address data access examples completed!");
    println!("💡 This example demonstrated:");
    println!("   • Complete account state queries");
    println!("   • Historical state access at specific blocks");
    println!("   • Contract storage reading and change tracking");
    println!("   • Batch operations for efficiency");
    println!("   • Transaction and receipt analysis for addresses");
    
    Ok(())
}

/// Perform comprehensive analysis of an account
async fn analyze_account_comprehensively(provider: &RethDatabaseProvider, address_str: &str) -> Result<()> {
    let address = Address::from_str(address_str)?;
    
    println!("🔍 Analyzing address: {}", address_str);
    
    // Fetch current account state
    match provider.fetch_account(address) {
        Ok(account) => {
            println!("📊 Current Account State:");
            println!("   Address: 0x{:x}", account.address);
            println!("   Balance: {} wei ({:.6} ETH)", account.balance, wei_to_eth(account.balance));
            println!("   Nonce: {}", account.nonce);
            println!("   Code Hash: 0x{:x}", account.code_hash);
            
            if let Some(code_size) = account.code_size {
                println!("   Code Size: {} bytes", code_size);
                
                if code_size > 0 {
                    // This is a contract
                    println!("   📋 Contract detected!");
                    
                    if let Some(code) = account.code {
                        println!("   First 32 bytes of code: 0x{}", hex::encode(&code[..code.len().min(32)]));
                    }
                } else {
                    println!("   👤 Externally Owned Account (EOA)");
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch account: {}", e);
            return Ok(());
        }
    }
    
    // Fetch account at different blocks for comparison
    let latest_block = provider.latest_block_number()?;
    if latest_block > 100 {
        let historical_blocks = [
            latest_block.saturating_sub(1),    // Previous block
            latest_block.saturating_sub(10),   // 10 blocks ago
            latest_block.saturating_sub(100),  // 100 blocks ago
        ];
        
        println!("\n📈 Historical Account State:");
        for &block_num in &historical_blocks {
            match provider.fetch_account_at_block(address, BlockId::Number(block_num)) {
                Ok(account) => {
                    println!("   Block {}: Balance = {} wei, Nonce = {}", 
                             block_num, account.balance, account.nonce);
                }
                Err(_) => {
                    println!("   Block {}: Data not available", block_num);
                }
            }
        }
    }
    
    // Check recent balance/nonce changes
    if latest_block > 10 {
        println!("\n🔄 Recent State Changes (last 10 blocks):");
        let start_block = latest_block.saturating_sub(10);
        match provider.fetch_account_state_changes(address, start_block, latest_block) {
            Ok(changes) => {
                if changes.is_empty() {
                    println!("   No state changes detected in last 10 blocks");
                } else {
                    for change in changes.iter().take(5) { // Show first 5 changes
                        println!("   Block {}: Balance changed", change.block_number);
                        if let Some(prev) = &change.previous_state {
                            println!("     Previous: {} wei, nonce {}", prev.balance, prev.nonce);
                        }
                        println!("     New: {} wei, nonce {}", 
                                 change.new_state.balance, change.new_state.nonce);
                        
                        if !change.storage_changes.is_empty() {
                            println!("     Storage changes: {} slots", change.storage_changes.len());
                        }
                    }
                }
            }
            Err(e) => {
                println!("   Could not fetch state changes: {}", e);
            }
        }
    }
    
    Ok(())
}

/// Analyze contract storage in detail
async fn analyze_contract_storage(provider: &RethDatabaseProvider, contract_str: &str) -> Result<()> {
    let contract = Address::from_str(contract_str)?;
    
    println!("🔍 Analyzing contract storage: {}", contract_str);
    
    // Check if this is actually a contract
    match provider.fetch_account(contract) {
        Ok(account) => {
            if account.code_size.unwrap_or(0) == 0 {
                println!("⚠️  This address does not appear to be a contract");
                return Ok(());
            }
            
            println!("📋 Contract Information:");
            println!("   Code Size: {} bytes", account.code_size.unwrap_or(0));
            println!("   Storage Root: 0x{:x}", account.storage_root);
        }
        Err(e) => {
            println!("❌ Failed to fetch contract account: {}", e);
            return Ok(());
        }
    }
    
    // Read common storage slots
    println!("\n🗄️  Common Storage Slots:");
    let common_slots = [
        B256::ZERO,                           // Slot 0 - often used for owner or implementation
        B256::from([0u8; 32]),               // Slot 1
        B256::from([1u8; 32]),               // Slot 2
        // Add more common slots based on contract patterns
    ];
    
    match provider.fetch_storage_batch(contract, &common_slots) {
        Ok(storage_data) => {
            for data in storage_data {
                if data.value != B256::ZERO {
                    println!("   Slot 0x{:x}: 0x{:x}", data.key, data.value);
                    
                    // Try to interpret the value
                    let uint_value = U256::from_be_slice(data.value.as_slice());
                    if uint_value > U256::ZERO && uint_value < U256::from(1000000000u64) {
                        println!("     Interpreted as uint: {}", uint_value);
                    }
                }
            }
        }
        Err(e) => {
            println!("   Could not read storage slots: {}", e);
        }
    }
    
    // Check for storage changes in recent blocks
    let latest_block = provider.latest_block_number()?;
    if latest_block > 100 {
        println!("\n🔄 Recent Storage Changes:");
        let start_block = latest_block.saturating_sub(100);
        match provider.fetch_storage_changes(contract, start_block, latest_block) {
            Ok(changes) => {
                if changes.is_empty() {
                    println!("   No storage changes in last 100 blocks");
                } else {
                    println!("   Found {} storage change events", changes.len());
                    for change in changes.iter().take(3) { // Show first 3 changes
                        println!("   Block {}: {} storage slots changed", 
                                 change.block_number, change.storage_changes.len());
                        for (slot, (old_val, new_val)) in change.storage_changes.iter().take(2) {
                            println!("     Slot 0x{:x}:", slot);
                            if let Some(old) = old_val {
                                println!("       Old: 0x{:x}", old);
                            }
                            println!("       New: 0x{:x}", new_val);
                        }
                    }
                }
            }
            Err(e) => {
                println!("   Could not fetch storage changes: {}", e);
            }
        }
    }
    
    Ok(())
}

/// Track historical state of an address
async fn track_historical_state(provider: &RethDatabaseProvider, address_str: &str) -> Result<()> {
    let address = Address::from_str(address_str)?;
    
    println!("🔍 Tracking historical state for: {}", address_str);
    
    let latest_block = provider.latest_block_number()?;
    if latest_block < 1000 {
        println!("⚠️  Not enough blocks for meaningful historical analysis");
        return Ok(());
    }
    
    // Sample blocks at different intervals
    let sample_blocks = [
        latest_block,                           // Latest
        latest_block.saturating_sub(100),      // ~20 minutes ago (assuming 12s blocks)
        latest_block.saturating_sub(500),      // ~1.7 hours ago  
        latest_block.saturating_sub(2000),     // ~6.7 hours ago
        latest_block.saturating_sub(10000),    // ~33 hours ago
    ];
    
    println!("\n📈 Balance History:");
    let mut previous_balance: Option<U256> = None;
    
    for &block_num in &sample_blocks {
        match provider.fetch_balance_at_block(address, BlockId::Number(block_num)) {
            Ok(balance) => {
                let eth_balance = wei_to_eth(balance);
                print!("   Block {}: {} wei ({:.6} ETH)", block_num, balance, eth_balance);
                
                if let Some(prev_balance) = previous_balance {
                    let change = if balance > prev_balance {
                        let diff = balance - prev_balance;
                        format!("+{:.6} ETH", wei_to_eth(diff))
                    } else if balance < prev_balance {
                        let diff = prev_balance - balance;
                        format!("-{:.6} ETH", wei_to_eth(diff))
                    } else {
                        "no change".to_string()
                    };
                    print!(" [{}]", change);
                }
                println!();
                
                previous_balance = Some(balance);
            }
            Err(_) => {
                println!("   Block {}: Data not available", block_num);
            }
        }
    }
    
    // Track nonce changes
    println!("\n🔢 Nonce History:");
    for &block_num in &sample_blocks {
        match provider.fetch_nonce_at_block(address, BlockId::Number(block_num)) {
            Ok(nonce) => {
                println!("   Block {}: nonce {}", block_num, nonce);
            }
            Err(_) => {
                println!("   Block {}: Nonce data not available", block_num);
            }
        }
    }
    
    Ok(())
}

/// Analyze address activity (transactions and receipts)
async fn analyze_address_activity(provider: &RethDatabaseProvider, address_str: &str) -> Result<()> {
    let address = Address::from_str(address_str)?;
    
    println!("🔍 Analyzing activity for address: {}", address_str);
    
    // Get recent blocks to analyze
    let latest_block = provider.latest_block_number()?;
    if latest_block < 10 {
        println!("⚠️  Not enough blocks for activity analysis");
        return Ok(());
    }
    
    let start_block = latest_block.saturating_sub(10);
    
    println!("\n🔄 Transaction Activity (last 10 blocks):");
    
    // Analyze transactions in recent blocks
    let mut total_transactions = 0;
    let mut total_gas_used = 0u64;
    let mut successful_txs = 0;
    
    for block_num in start_block..=latest_block {
        match provider.fetch_transactions_by_block(BlockId::Number(block_num)) {
            Ok(transactions) => {
                let relevant_txs: Vec<_> = transactions.iter()
                    .filter(|tx| tx.from == address || tx.to == Some(address))
                    .collect();
                
                if !relevant_txs.is_empty() {
                    println!("   Block {}: {} relevant transactions", block_num, relevant_txs.len());
                    
                    for tx in relevant_txs {
                        total_transactions += 1;
                        total_gas_used += tx.gas_used;
                        
                        if tx.receipt_status {
                            successful_txs += 1;
                        }
                        
                        let direction = if tx.from == address { "OUT" } else { "IN" };
                        println!("     {} {} - {} wei", 
                                 direction, 
                                 format_hash_short(tx.hash), 
                                 tx.value);
                    }
                }
            }
            Err(_) => {
                // Block might not be available
                continue;
            }
        }
    }
    
    if total_transactions > 0 {
        println!("\n📊 Activity Summary:");
        println!("   Total transactions: {}", total_transactions);
        println!("   Successful: {} ({:.1}%)", successful_txs, 
                 (successful_txs as f64 / total_transactions as f64) * 100.0);
        println!("   Average gas used: {}", total_gas_used / total_transactions as u64);
    } else {
        println!("   No transaction activity found in recent blocks");
    }
    
    Ok(())
}

/// Demonstrate batch operations for multiple addresses
async fn batch_address_analysis(provider: &RethDatabaseProvider) -> Result<()> {
    let addresses = [
        Address::from_str(VITALIK_ADDRESS)?,
        Address::from_str(USDC_CONTRACT)?,
        Address::from_str(UNISWAP_V3_FACTORY)?,
    ];
    
    println!("🔍 Batch analysis of {} addresses", addresses.len());
    
    // Batch fetch account data
    match provider.fetch_accounts_batch(&addresses) {
        Ok(accounts) => {
            println!("\n📊 Batch Account Data:");
            for account in accounts {
                let account_type = if account.code_size.unwrap_or(0) > 0 { "Contract" } else { "EOA" };
                println!("   {}: {} - {} wei ({} ETH)", 
                         format_address_short(account.address),
                         account_type,
                         account.balance,
                         wei_to_eth(account.balance));
            }
        }
        Err(e) => {
            println!("❌ Batch fetch failed: {}", e);
        }
    }
    
    // Batch balance check at a specific block
    let latest_block = provider.latest_block_number()?;
    if latest_block > 100 {
        let historical_block = latest_block.saturating_sub(100);
        match provider.fetch_accounts_at_block_batch(&addresses, BlockId::Number(historical_block)) {
            Ok(historical_accounts) => {
                println!("\n📈 Historical Batch Data (block {}):", historical_block);
                for account in historical_accounts {
                    println!("   {}: {} wei", 
                             format_address_short(account.address),
                             account.balance);
                }
            }
            Err(e) => {
                println!("Could not fetch historical batch data: {}", e);
            }
        }
    }
    
    Ok(())
}

/// Track storage changes for a contract
async fn track_storage_changes(provider: &RethDatabaseProvider, contract_str: &str) -> Result<()> {
    let contract = Address::from_str(contract_str)?;
    
    println!("🔍 Tracking storage changes for contract: {}", contract_str);
    
    let latest_block = provider.latest_block_number()?;
    if latest_block < 50 {
        println!("⚠️  Not enough blocks for storage change analysis");
        return Ok(());
    }
    
    let start_block = latest_block.saturating_sub(50);
    
    // Find all addresses that had storage changes in this period
    match provider.fetch_changed_accounts(start_block, latest_block) {
        Ok(changed_addresses) => {
            println!("\n🔄 Addresses with changes (last 50 blocks): {}", changed_addresses.len());
            
            if changed_addresses.contains(&contract) {
                println!("   ✅ Target contract had storage changes!");
                
                // Get detailed storage changes for this contract
                match provider.fetch_storage_changes(contract, start_block, latest_block) {
                    Ok(changes) => {
                        println!("   📊 Storage change details:");
                        for change in changes.iter().take(3) {
                            println!("     Block {}: {} storage slots changed", 
                                     change.block_number, change.storage_changes.len());
                        }
                    }
                    Err(e) => {
                        println!("   Could not get detailed storage changes: {}", e);
                    }
                }
            } else {
                println!("   ℹ️  Target contract had no storage changes in this period");
            }
        }
        Err(e) => {
            println!("❌ Could not fetch changed accounts: {}", e);
        }
    }
    
    Ok(())
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

/// Format address for display (0x + first 6 hex chars)
fn format_address_short(address: Address) -> String {
    format!("0x{}", hex::encode(&address.as_slice()[..6]))
}

/// Format hash for display (0x + first 4 hex chars)
fn format_hash_short(hash: B256) -> String {
    format!("0x{}", hex::encode(&hash.as_slice()[..4]))
}