//! Advanced data access example for the fetch_from_reth module
//!
//! This example demonstrates advanced blockchain data analysis capabilities:
//! - Historical state change tracking
//! - Cross-block state comparison
//! - Advanced transaction pattern analysis
//! - Contract interaction mapping
//! - Performance optimization techniques
//! - Data correlation and insights
//!
//! To run this example:
//!   cargo run --bin fetch_from_reth_advanced_data_access
//!
//! Prerequisites:
//! - Local Reth node with substantial synced data (>10K blocks recommended)
//! - Set RETH_DATADIR environment variable

use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::collections::{HashMap, BTreeMap, HashSet};

use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use revm_tx_simulator_lib::fetch_from_reth::{
    RethDatabaseProvider, RethDataProvider, RethDataConfig,
    provider::{BlockId, AccountData, StateChange}
};

// Well-known addresses for analysis
const USDC_CONTRACT: &str = "0xA0b86a33E6441c8E2f2FBE96C5F41F0Ef93F8B3A";
const WETH_CONTRACT: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";

#[derive(Debug, Clone)]
struct ContractInteraction {
    from: Address,
    to: Address,
    block_number: u64,
    transaction_hash: B256,
    value: U256,
    gas_used: u64,
    success: bool,
}

#[derive(Debug, Clone)]
struct AddressActivity {
    address: Address,
    first_seen_block: u64,
    last_seen_block: u64,
    total_transactions: u64,
    total_value_sent: U256,
    total_value_received: U256,
    total_gas_used: u64,
    unique_counterparties: HashSet<Address>,
    contract_interactions: Vec<ContractInteraction>,
}

#[derive(Debug, Clone)]
struct StateEvolution {
    address: Address,
    snapshots: Vec<(u64, AccountData)>, // (block_number, account_state)
    storage_changes: BTreeMap<u64, HashMap<B256, (Option<B256>, B256)>>, // block -> slot -> (old, new)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with higher verbosity for advanced analysis
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    println!("🚀 Advanced Data Access - Comprehensive Blockchain Analysis");
    println!("===========================================================\n");
    
    // Setup database connection with high-performance configuration
    let datadir = get_reth_datadir()?;
    let config = RethDataConfig::new(&datadir);
    
    let provider = RethDatabaseProvider::with_config(config)?;
    
    println!("✅ Connected to Reth database with high-performance configuration");
    
    // Verify we have enough data for advanced analysis
    let latest_block = provider.latest_block_number()?;
    if latest_block < 1000 {
        println!("⚠️  This example requires at least 1000 blocks for meaningful analysis");
        println!("   Current block: {}", latest_block);
        println!("   Consider running when more data is available");
        return Ok(());
    }
    
    println!("📊 Database contains {} blocks - proceeding with advanced analysis\n", latest_block);
    
    // Example 1: Historical State Evolution Analysis
    println!("📈 Example 1: Historical State Evolution Analysis");
    println!("===============================================");
    analyze_state_evolution(&provider).await?;
    
    // Example 2: Contract Interaction Pattern Analysis
    println!("\n🔄 Example 2: Contract Interaction Pattern Analysis");
    println!("=================================================");
    analyze_contract_interactions(&provider).await?;
    
    // Example 3: Address Activity Profiling
    println!("\n👤 Example 3: Address Activity Profiling");
    println!("=======================================");
    profile_address_activity(&provider).await?;
    
    // Example 4: Cross-Block State Correlation
    println!("\n🔗 Example 4: Cross-Block State Correlation");
    println!("==========================================");
    analyze_state_correlations(&provider).await?;
    
    // Example 5: Transaction Flow Analysis
    println!("\n💸 Example 5: Transaction Flow Analysis");
    println!("======================================");
    analyze_transaction_flows(&provider).await?;
    
    // Example 6: Gas Usage Pattern Analysis
    println!("\n⛽ Example 6: Gas Usage Pattern Analysis");
    println!("======================================");
    analyze_gas_patterns(&provider).await?;
    
    // Example 7: Performance Optimization Demonstration
    println!("\n⚡ Example 7: Performance Optimization Techniques");
    println!("===============================================");
    demonstrate_performance_optimization(&provider).await?;
    
    // Show final cache statistics
    let final_stats = provider.cache_stats();
    println!("\n📊 Final Cache Performance:");
    println!("   Total Requests: {}", final_stats.total_requests);
    println!("   Hit Rate: {:.1}%", final_stats.hit_rate());
    println!("   Cache Size: {}", final_stats.current_size);
    
    println!("\n🎉 Advanced data access analysis completed!");
    println!("💡 This example demonstrated:");
    println!("   • Historical state evolution tracking");
    println!("   • Contract interaction pattern analysis");
    println!("   • Address activity profiling and correlation");
    println!("   • Transaction flow analysis");
    println!("   • Gas usage optimization insights");
    println!("   • High-performance data access patterns");
    
    Ok(())
}

/// Analyze how account states evolve over time
async fn analyze_state_evolution(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Analyzing state evolution for key contracts...");
    
    let contracts = [
        (USDC_CONTRACT, "USDC Contract"),
        (WETH_CONTRACT, "WETH Contract"),
        (UNISWAP_V3_FACTORY, "Uniswap V3 Factory"),
    ];
    
    for (contract_str, name) in contracts {
        let contract = Address::from_str(contract_str)?;
        println!("\n📋 Analyzing {}: {}", name, contract_str);
        
        let mut evolution = StateEvolution {
            address: contract,
            snapshots: Vec::new(),
            storage_changes: BTreeMap::new(),
        };
        
        // Sample state at different points in time
        let latest_block = provider.latest_block_number()?;
        let sample_blocks = [
            latest_block,
            latest_block.saturating_sub(100),
            latest_block.saturating_sub(500),
            latest_block.saturating_sub(1000),
        ];
        
        for &block_num in &sample_blocks {
            match provider.fetch_account_at_block(contract, BlockId::Number(block_num)) {
                Ok(account) => {
                    evolution.snapshots.push((block_num, account));
                }
                Err(_) => {
                    // Account might not exist at this block
                    continue;
                }
            }
        }
        
        if evolution.snapshots.len() > 1 {
            println!("   📈 State Evolution ({} snapshots):", evolution.snapshots.len());
            
            for (i, (block_num, account)) in evolution.snapshots.iter().enumerate() {
                println!("     Block {}: Balance = {} wei, Nonce = {}", 
                         block_num, account.balance, account.nonce);
                
                if i > 0 {
                    let prev_account = &evolution.snapshots[i-1].1;
                    if account.balance != prev_account.balance {
                        let change = if account.balance > prev_account.balance {
                            format!("+{}", account.balance - prev_account.balance)
                        } else {
                            format!("-{}", prev_account.balance - account.balance)
                        };
                        println!("       Balance change: {} wei", change);
                    }
                    
                    if account.nonce != prev_account.nonce {
                        println!("       Nonce change: {} -> {}", prev_account.nonce, account.nonce);
                    }
                }
            }
            
            // Analyze storage changes in recent blocks
            let recent_start = latest_block.saturating_sub(50);
            match provider.fetch_storage_changes(contract, recent_start, latest_block) {
                Ok(changes) => {
                    if !changes.is_empty() {
                        println!("   🗄️  Recent Storage Activity: {} change events", changes.len());
                        
                        let mut total_slots_changed = 0;
                        for change in &changes {
                            total_slots_changed += change.storage_changes.len();
                        }
                        
                        if total_slots_changed > 0 {
                            println!("     Total storage slots modified: {}", total_slots_changed);
                            println!("     Average slots per change: {:.1}", 
                                     total_slots_changed as f64 / changes.len() as f64);
                        }
                    } else {
                        println!("   📭 No storage changes detected in recent blocks");
                    }
                }
                Err(_) => {
                    println!("   ⚠️  Could not analyze storage changes");
                }
            }
        } else {
            println!("   ⚠️  Insufficient historical data for evolution analysis");
        }
    }
    
    Ok(())
}

/// Analyze contract interaction patterns
async fn analyze_contract_interactions(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Analyzing contract interaction patterns...");
    
    let latest_block = provider.latest_block_number()?;
    let analysis_start = latest_block.saturating_sub(100); // Last 100 blocks
    
    println!("   Analyzing blocks {} to {}", analysis_start, latest_block);
    
    let mut interactions: Vec<ContractInteraction> = Vec::new();
    let mut contract_activity: HashMap<Address, u32> = HashMap::new();
    let mut user_activity: HashMap<Address, u32> = HashMap::new();
    
    // Analyze transactions in the block range
    for block_num in analysis_start..=latest_block {
        match provider.fetch_transactions_by_block(BlockId::Number(block_num)) {
            Ok(transactions) => {
                for tx in transactions {
                    // Count activity
                    *user_activity.entry(tx.from).or_insert(0) += 1;
                    
                    if let Some(to_addr) = tx.to {
                        *contract_activity.entry(to_addr).or_insert(0) += 1;
                        
                        // Check if this is a contract interaction (has input data)
                        if !tx.input.is_empty() {
                            interactions.push(ContractInteraction {
                                from: tx.from,
                                to: to_addr,
                                block_number: block_num,
                                transaction_hash: tx.hash,
                                value: tx.value,
                                gas_used: tx.gas_used,
                                success: tx.receipt_status,
                            });
                        }
                    }
                }
            }
            Err(_) => {
                // Skip blocks that can't be fetched
                continue;
            }
        }
    }
    
    if !interactions.is_empty() {
        println!("\n📊 Contract Interaction Analysis:");
        println!("   Total Interactions: {}", interactions.len());
        
        // Find most active contracts
        let mut sorted_contracts: Vec<_> = contract_activity.iter().collect();
        sorted_contracts.sort_by(|a, b| b.1.cmp(a.1));
        
        println!("   🏆 Most Active Contracts:");
        for (i, (contract, count)) in sorted_contracts.iter().take(5).enumerate() {
            println!("     {}. {} - {} interactions", 
                     i + 1, format_address_short(**contract), count);
        }
        
        // Find most active users
        let mut sorted_users: Vec<_> = user_activity.iter().collect();
        sorted_users.sort_by(|a, b| b.1.cmp(a.1));
        
        println!("   👤 Most Active Users:");
        for (i, (user, count)) in sorted_users.iter().take(5).enumerate() {
            println!("     {}. {} - {} transactions", 
                     i + 1, format_address_short(**user), count);
        }
        
        // Analyze interaction success rates
        let successful_interactions = interactions.iter().filter(|i| i.success).count();
        let success_rate = (successful_interactions as f64 / interactions.len() as f64) * 100.0;
        
        println!("   ✅ Success Rate: {:.1}% ({}/{})", 
                 success_rate, successful_interactions, interactions.len());
        
        // Analyze gas usage patterns
        let total_gas: u64 = interactions.iter().map(|i| i.gas_used).sum();
        let avg_gas = total_gas / interactions.len() as u64;
        
        println!("   ⛽ Gas Usage:");
        println!("     Total: {} gas", total_gas);
        println!("     Average: {} gas per interaction", avg_gas);
        
        // Find gas outliers
        let max_gas = interactions.iter().map(|i| i.gas_used).max().unwrap_or(0);
        let min_gas = interactions.iter().map(|i| i.gas_used).min().unwrap_or(0);
        
        if max_gas > avg_gas * 3 {
            println!("     ⚠️  High gas interaction detected: {} gas ({}x average)", 
                     max_gas, max_gas / avg_gas);
        }
        
        println!("     Range: {} - {} gas", min_gas, max_gas);
        
    } else {
        println!("   📭 No contract interactions found in analyzed blocks");
    }
    
    Ok(())
}

/// Profile comprehensive address activity
async fn profile_address_activity(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Profiling address activity patterns...");
    
    let latest_block = provider.latest_block_number()?;
    let analysis_blocks = 50; // Analyze last 50 blocks
    let start_block = latest_block.saturating_sub(analysis_blocks - 1);
    
    let mut address_profiles: HashMap<Address, AddressActivity> = HashMap::new();
    
    println!("   Analyzing {} blocks for address activity", analysis_blocks);
    
    // Collect activity data
    for block_num in start_block..=latest_block {
        match provider.fetch_transactions_by_block(BlockId::Number(block_num)) {
            Ok(transactions) => {
                for tx in transactions {
                    // Profile sender activity
                    let sender_profile = address_profiles.entry(tx.from).or_insert_with(|| {
                        AddressActivity {
                            address: tx.from,
                            first_seen_block: block_num,
                            last_seen_block: block_num,
                            total_transactions: 0,
                            total_value_sent: U256::ZERO,
                            total_value_received: U256::ZERO,
                            total_gas_used: 0,
                            unique_counterparties: HashSet::new(),
                            contract_interactions: Vec::new(),
                        }
                    });
                    
                    sender_profile.last_seen_block = block_num;
                    sender_profile.total_transactions += 1;
                    sender_profile.total_value_sent += tx.value;
                    sender_profile.total_gas_used += tx.gas_used;
                    
                    if let Some(to_addr) = tx.to {
                        sender_profile.unique_counterparties.insert(to_addr);
                        
                        // Track contract interactions
                        if !tx.input.is_empty() {
                            sender_profile.contract_interactions.push(ContractInteraction {
                                from: tx.from,
                                to: to_addr,
                                block_number: block_num,
                                transaction_hash: tx.hash,
                                value: tx.value,
                                gas_used: tx.gas_used,
                                success: tx.receipt_status,
                            });
                        }
                        
                        // Profile recipient activity
                        let recipient_profile = address_profiles.entry(to_addr).or_insert_with(|| {
                            AddressActivity {
                                address: to_addr,
                                first_seen_block: block_num,
                                last_seen_block: block_num,
                                total_transactions: 0,
                                total_value_sent: U256::ZERO,
                                total_value_received: U256::ZERO,
                                total_gas_used: 0,
                                unique_counterparties: HashSet::new(),
                                contract_interactions: Vec::new(),
                            }
                        });
                        
                        recipient_profile.last_seen_block = block_num;
                        recipient_profile.total_value_received += tx.value;
                        recipient_profile.unique_counterparties.insert(tx.from);
                    }
                }
            }
            Err(_) => {
                continue;
            }
        }
    }
    
    if !address_profiles.is_empty() {
        println!("\n📊 Address Activity Profile Results:");
        println!("   Total Active Addresses: {}", address_profiles.len());
        
        // Find most active addresses by transaction count
        let mut by_tx_count: Vec<_> = address_profiles.values().collect();
        by_tx_count.sort_by(|a, b| b.total_transactions.cmp(&a.total_transactions));
        
        println!("   🏆 Most Active by Transaction Count:");
        for (i, profile) in by_tx_count.iter().take(5).enumerate() {
            println!("     {}. {} - {} transactions", 
                     i + 1, format_address_short(profile.address), profile.total_transactions);
            println!("        Active blocks: {} to {}", 
                     profile.first_seen_block, profile.last_seen_block);
            println!("        Unique counterparties: {}", profile.unique_counterparties.len());
            
            if profile.total_value_sent > U256::ZERO {
                println!("        Value sent: {:.6} ETH", wei_to_eth(profile.total_value_sent));
            }
            
            if profile.total_value_received > U256::ZERO {
                println!("        Value received: {:.6} ETH", wei_to_eth(profile.total_value_received));
            }
            
            if !profile.contract_interactions.is_empty() {
                println!("        Contract interactions: {}", profile.contract_interactions.len());
            }
        }
        
        // Find addresses with highest value transfer
        let mut by_value: Vec<_> = address_profiles.values()
            .filter(|p| p.total_value_sent > U256::ZERO)
            .collect();
        by_value.sort_by(|a, b| b.total_value_sent.cmp(&a.total_value_sent));
        
        if !by_value.is_empty() {
            println!("\n   💰 Highest Value Senders:");
            for (i, profile) in by_value.iter().take(3).enumerate() {
                println!("     {}. {} - {:.6} ETH sent", 
                         i + 1, format_address_short(profile.address), wei_to_eth(profile.total_value_sent));
            }
        }
        
        // Analyze activity patterns
        let total_tx: u64 = address_profiles.values().map(|p| p.total_transactions).sum();
        let avg_tx_per_address = total_tx as f64 / address_profiles.len() as f64;
        
        println!("\n   📈 Activity Statistics:");
        println!("     Average transactions per address: {:.1}", avg_tx_per_address);
        
        let high_activity_threshold = (avg_tx_per_address * 2.0) as u64;
        let high_activity_count = address_profiles.values()
            .filter(|p| p.total_transactions > high_activity_threshold)
            .count();
        
        if high_activity_count > 0 {
            println!("     High-activity addresses (>{}tx): {}", high_activity_threshold, high_activity_count);
        }
        
    } else {
        println!("   📭 No address activity found in analyzed blocks");
    }
    
    Ok(())
}

/// Analyze state correlations across multiple addresses
async fn analyze_state_correlations(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Analyzing cross-address state correlations...");
    
    let addresses = [
        Address::from_str(USDC_CONTRACT)?,
        Address::from_str(WETH_CONTRACT)?,
        Address::from_str(UNISWAP_V3_FACTORY)?,
    ];
    
    let latest_block = provider.latest_block_number()?;
    let sample_blocks = [
        latest_block,
        latest_block.saturating_sub(10),
        latest_block.saturating_sub(50),
        latest_block.saturating_sub(100),
    ];
    
    println!("   Analyzing {} addresses across {} time points", addresses.len(), sample_blocks.len());
    
    // Fetch state data for all addresses at all time points
    let mut state_matrix: Vec<Vec<Option<AccountData>>> = Vec::new();
    
    for &block_num in &sample_blocks {
        let mut block_states = Vec::new();
        
        for &addr in &addresses {
            match provider.fetch_account_at_block(addr, BlockId::Number(block_num)) {
                Ok(account) => block_states.push(Some(account)),
                Err(_) => block_states.push(None),
            }
        }
        
        state_matrix.push(block_states);
    }
    
    println!("\n📊 State Correlation Analysis:");
    
    // Analyze balance correlations
    for (addr_idx, &addr) in addresses.iter().enumerate() {
        println!("   Address {}: {}", addr_idx + 1, format_address_short(addr));
        
        let mut balances = Vec::new();
        let mut nonces = Vec::new();
        
        for (time_idx, &block_num) in sample_blocks.iter().enumerate() {
            if let Some(Some(account)) = state_matrix.get(time_idx).and_then(|states| states.get(addr_idx)) {
                balances.push((block_num, account.balance));
                nonces.push((block_num, account.nonce));
            }
        }
        
        if balances.len() > 1 {
            println!("     Balance evolution:");
            for (block, balance) in &balances {
                println!("       Block {}: {} wei", block, balance);
            }
            
            // Calculate balance change rate
            if let (Some((first_block, first_balance)), Some((last_block, last_balance))) = 
                (balances.first(), balances.last()) {
                
                if *last_block > *first_block && *last_balance != *first_balance {
                    let blocks_elapsed = last_block - first_block;
                    let balance_change = if *last_balance > *first_balance {
                        (*last_balance - *first_balance, "increase")
                    } else {
                        (*first_balance - *last_balance, "decrease")
                    };
                    
                    println!("     Net change: {} {} over {} blocks", 
                             balance_change.0, balance_change.1, blocks_elapsed);
                }
            }
        }
        
        if nonces.len() > 1 {
            println!("     Nonce progression:");
            for (block, nonce) in &nonces {
                println!("       Block {}: nonce {}", block, nonce);
            }
        }
    }
    
    // Look for simultaneous changes
    println!("\n🔗 Simultaneous Change Detection:");
    let mut simultaneous_changes = 0;
    
    for block_idx in 1..state_matrix.len() {
        let mut changes_in_block = Vec::new();
        
        for addr_idx in 0..addresses.len() {
            if let (Some(Some(prev_state)), Some(Some(curr_state))) = (
                state_matrix.get(block_idx - 1).and_then(|s| s.get(addr_idx)),
                state_matrix.get(block_idx).and_then(|s| s.get(addr_idx))
            ) {
                if prev_state.balance != curr_state.balance || prev_state.nonce != curr_state.nonce {
                    changes_in_block.push(addr_idx);
                }
            }
        }
        
        if changes_in_block.len() > 1 {
            simultaneous_changes += 1;
            println!("   Block {}: {} addresses changed simultaneously", 
                     sample_blocks[block_idx], changes_in_block.len());
        }
    }
    
    if simultaneous_changes == 0 {
        println!("   No simultaneous state changes detected");
    }
    
    Ok(())
}

/// Analyze transaction flows and patterns
async fn analyze_transaction_flows(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Analyzing transaction flow patterns...");
    
    let latest_block = provider.latest_block_number()?;
    let analysis_blocks = 20;
    let start_block = latest_block.saturating_sub(analysis_blocks - 1);
    
    let mut flow_data: HashMap<(Address, Address), (u32, U256)> = HashMap::new(); // (from, to) -> (count, total_value)
    let mut value_distribution: Vec<U256> = Vec::new();
    
    println!("   Analyzing transaction flows in last {} blocks", analysis_blocks);
    
    for block_num in start_block..=latest_block {
        match provider.fetch_transactions_by_block(BlockId::Number(block_num)) {
            Ok(transactions) => {
                for tx in transactions {
                    if let Some(to_addr) = tx.to {
                        let flow_key = (tx.from, to_addr);
                        let (count, total_value) = flow_data.entry(flow_key).or_insert((0, U256::ZERO));
                        *count += 1;
                        *total_value += tx.value;
                        
                        if tx.value > U256::ZERO {
                            value_distribution.push(tx.value);
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }
    
    if !flow_data.is_empty() {
        println!("\n💸 Transaction Flow Analysis:");
        println!("   Unique address pairs: {}", flow_data.len());
        
        // Find most frequent flows
        let mut by_frequency: Vec<_> = flow_data.iter().collect();
        by_frequency.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        
        println!("   🔄 Most Frequent Flows:");
        for (i, ((from, to), (count, total_value))) in by_frequency.iter().take(5).enumerate() {
            println!("     {}. {} → {}: {} transactions", 
                     i + 1, format_address_short(*from), format_address_short(*to), count);
            
            if *total_value > U256::ZERO {
                println!("        Total value: {:.6} ETH", wei_to_eth(*total_value));
                let avg_value = *total_value / U256::from(*count);
                println!("        Average value: {:.6} ETH", wei_to_eth(avg_value));
            }
        }
        
        // Find highest value flows
        let mut by_value: Vec<_> = flow_data.iter()
            .filter(|(_, (_, value))| *value > U256::ZERO)
            .collect();
        by_value.sort_by(|a, b| b.1.1.cmp(&a.1.1));
        
        if !by_value.is_empty() {
            println!("\n   💰 Highest Value Flows:");
            for (i, ((from, to), (count, total_value))) in by_value.iter().take(3).enumerate() {
                println!("     {}. {} → {}: {:.6} ETH ({} transactions)", 
                         i + 1, format_address_short(*from), format_address_short(*to), 
                         wei_to_eth(*total_value), count);
            }
        }
        
        // Analyze value distribution
        if !value_distribution.is_empty() {
            value_distribution.sort();
            
            let total_transactions_with_value = value_distribution.len();
            let median_idx = total_transactions_with_value / 2;
            let median_value = value_distribution[median_idx];
            
            let total_value: U256 = value_distribution.iter().fold(U256::ZERO, |acc, &val| acc + val);
            let avg_value = total_value / U256::from(total_transactions_with_value);
            
            println!("\n   📊 Value Distribution:");
            println!("     Transactions with value: {}", total_transactions_with_value);
            println!("     Median value: {:.6} ETH", wei_to_eth(median_value));
            println!("     Average value: {:.6} ETH", wei_to_eth(avg_value));
            println!("     Min value: {:.6} ETH", wei_to_eth(value_distribution[0]));
            println!("     Max value: {:.6} ETH", wei_to_eth(value_distribution[total_transactions_with_value - 1]));
        }
        
    } else {
        println!("   📭 No transaction flows found in analyzed blocks");
    }
    
    Ok(())
}

/// Analyze gas usage patterns
async fn analyze_gas_patterns(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Analyzing gas usage patterns...");
    
    let latest_block = provider.latest_block_number()?;
    let analysis_blocks = 30;
    let start_block = latest_block.saturating_sub(analysis_blocks - 1);
    
    let mut gas_data: Vec<(u64, u64, u64)> = Vec::new(); // (block_number, gas_used, gas_limit)
    let mut transaction_gas: Vec<u64> = Vec::new();
    
    for block_num in start_block..=latest_block {
        match provider.fetch_block(BlockId::Number(block_num)) {
            Ok(block) => {
                gas_data.push((block_num, block.gas_used, block.gas_limit));
                
                // Get transaction-level gas data
                if let Ok(transactions) = provider.fetch_transactions_by_block(BlockId::Number(block_num)) {
                    for tx in transactions {
                        transaction_gas.push(tx.gas_used);
                    }
                }
            }
            Err(_) => continue,
        }
    }
    
    if !gas_data.is_empty() {
        println!("\n⛽ Gas Usage Pattern Analysis:");
        
        // Block-level analysis
        let total_gas_used: u64 = gas_data.iter().map(|(_, used, _)| used).sum();
        let total_gas_limit: u64 = gas_data.iter().map(|(_, _, limit)| limit).sum();
        let avg_utilization = (total_gas_used as f64 / total_gas_limit as f64) * 100.0;
        
        println!("   📊 Block-level Gas Statistics:");
        println!("     Blocks analyzed: {}", gas_data.len());
        println!("     Average utilization: {:.1}%", avg_utilization);
        
        // Find utilization extremes
        let mut utilizations: Vec<_> = gas_data.iter()
            .map(|(block, used, limit)| (*block, (*used as f64 / *limit as f64) * 100.0))
            .collect();
        utilizations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        println!("     Highest utilization: Block {} ({:.1}%)", 
                 utilizations[0].0, utilizations[0].1);
        println!("     Lowest utilization: Block {} ({:.1}%)", 
                 utilizations.last().unwrap().0, utilizations.last().unwrap().1);
        
        // Transaction-level analysis
        if !transaction_gas.is_empty() {
            transaction_gas.sort();
            
            let median_gas = transaction_gas[transaction_gas.len() / 2];
            let avg_gas = transaction_gas.iter().sum::<u64>() / transaction_gas.len() as u64;
            let min_gas = transaction_gas[0];
            let max_gas = transaction_gas[transaction_gas.len() - 1];
            
            println!("\n   🔄 Transaction-level Gas Statistics:");
            println!("     Transactions analyzed: {}", transaction_gas.len());
            println!("     Average gas: {}", avg_gas);
            println!("     Median gas: {}", median_gas);
            println!("     Range: {} - {}", min_gas, max_gas);
            
            // Identify gas usage patterns
            let basic_transfer_threshold = 21000;
            let complex_contract_threshold = 100000;
            
            let basic_transfers = transaction_gas.iter().filter(|&&gas| gas <= basic_transfer_threshold).count();
            let complex_contracts = transaction_gas.iter().filter(|&&gas| gas > complex_contract_threshold).count();
            let standard_contracts = transaction_gas.len() - basic_transfers - complex_contracts;
            
            println!("\n   📋 Transaction Type Distribution:");
            println!("     Basic transfers (≤21K gas): {} ({:.1}%)", 
                     basic_transfers, 
                     (basic_transfers as f64 / transaction_gas.len() as f64) * 100.0);
            println!("     Standard contracts (21K-100K): {} ({:.1}%)", 
                     standard_contracts,
                     (standard_contracts as f64 / transaction_gas.len() as f64) * 100.0);
            println!("     Complex contracts (>100K): {} ({:.1}%)", 
                     complex_contracts,
                     (complex_contracts as f64 / transaction_gas.len() as f64) * 100.0);
        }
        
    } else {
        println!("   📭 No gas data found for analysis");
    }
    
    Ok(())
}

/// Demonstrate performance optimization techniques
async fn demonstrate_performance_optimization(provider: &RethDatabaseProvider) -> Result<()> {
    println!("🔍 Demonstrating performance optimization techniques...");
    
    let latest_block = provider.latest_block_number()?;
    let test_blocks = [
        latest_block,
        latest_block.saturating_sub(1),
        latest_block.saturating_sub(2),
        latest_block.saturating_sub(3),
        latest_block.saturating_sub(4),
    ];
    
    // Test 1: Individual vs Batch Block Fetching
    println!("\n⚡ Test 1: Individual vs Batch Block Fetching");
    
    let start_time = std::time::Instant::now();
    let mut individual_results = Vec::new();
    for &block_num in &test_blocks {
        if let Ok(block) = provider.fetch_block(BlockId::Number(block_num)) {
            individual_results.push(block);
        }
    }
    let individual_duration = start_time.elapsed();
    
    let start_time = std::time::Instant::now();
    let block_ids: Vec<_> = test_blocks.iter().map(|&num| BlockId::Number(num)).collect();
    let batch_results = provider.fetch_blocks_batch(&block_ids).unwrap_or_default();
    let batch_duration = start_time.elapsed();
    
    println!("   Individual fetching: {:?} ({} blocks)", individual_duration, individual_results.len());
    println!("   Batch fetching: {:?} ({} blocks)", batch_duration, batch_results.len());
    
    if batch_duration < individual_duration && !batch_results.is_empty() {
        let improvement = ((individual_duration.as_micros() as f64 / batch_duration.as_micros() as f64) - 1.0) * 100.0;
        println!("   ✅ Batch fetching is {:.1}% faster", improvement);
    }
    
    // Test 2: Cache Performance Analysis
    println!("\n⚡ Test 2: Cache Performance Analysis");
    
    let initial_stats = provider.cache_stats();
    
    // Warm up cache with repeated requests
    for _ in 0..3 {
        for &block_num in &test_blocks {
            let _ = provider.fetch_block(BlockId::Number(block_num));
        }
    }
    
    let final_stats = provider.cache_stats();
    
    println!("   Initial cache state: {} hits, {} misses", initial_stats.hits, initial_stats.misses);
    println!("   Final cache state: {} hits, {} misses", final_stats.hits, final_stats.misses);
    
    let new_hits = final_stats.hits - initial_stats.hits;
    let new_requests = (final_stats.hits + final_stats.misses) - (initial_stats.hits + initial_stats.misses);
    
    if new_requests > 0 {
        let hit_rate = (new_hits as f64 / new_requests as f64) * 100.0;
        println!("   Cache hit rate for test: {:.1}%", hit_rate);
        
        if hit_rate > 50.0 {
            println!("   ✅ Cache is providing good performance benefits");
        }
    }
    
    // Test 3: Memory Usage Estimation
    println!("\n⚡ Test 3: Memory Usage Analysis");
    
    let cache_size = final_stats.current_size;
    let estimated_memory_per_entry = 2048; // Rough estimate in bytes
    let estimated_cache_memory = cache_size * estimated_memory_per_entry;
    
    println!("   Cache entries: {}", cache_size);
    println!("   Estimated cache memory: {:.1} KB", estimated_cache_memory as f64 / 1024.0);
    
    if cache_size > 0 {
        let efficiency = final_stats.hit_rate();
        println!("   Cache efficiency: {:.1}%", efficiency);
        
        if efficiency > 70.0 {
            println!("   ✅ Cache is highly efficient");
        } else if efficiency > 40.0 {
            println!("   ⚠️  Cache efficiency could be improved");
        } else {
            println!("   ❌ Cache efficiency is low");
        }
    }
    
    println!("\n💡 Performance Recommendations:");
    println!("   • Use batch operations for multiple queries");
    println!("   • Enable caching for repeated data access");
    println!("   • Consider cache TTL based on data freshness needs");
    println!("   • Monitor cache hit rates and adjust cache size accordingly");
    
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