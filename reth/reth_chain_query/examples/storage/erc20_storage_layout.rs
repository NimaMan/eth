/// ERC20 Storage Layout Analysis
/// 
/// Demonstrates reading ERC20 token data directly from storage slots vs view function calls.
/// This example compares performance and shows how to calculate storage slot addresses
/// for mappings (balances and allowances).
/// 
/// Algorithm:
/// 1. Define major ERC20 tokens with known storage layouts
/// 2. For each token, read storage directly:
///    - Slot 2: totalSupply (most common location)
///    - Slot 0 + keccak256(address, 0): balance mapping
///    - Slot 1 + keccak256(spender, keccak256(owner, 1)): allowance mapping
/// 3. Compare with view function calls (totalSupply(), balanceOf(), allowance())
/// 4. Show performance differences and use cases for each approach
/// 
/// Standard ERC20 Storage Layout:
/// - Slot 0: balances mapping(address => uint256)
/// - Slot 1: allowances mapping(address => mapping(address => uint256))  
/// - Slot 2: totalSupply (uint256)
/// - Note: Some tokens may vary, especially proxy contracts

use alloy_primitives::{Address, U256, B256, keccak256, FixedBytes};
use reth_chain_query::{ChainQuery, Result};
use std::str::FromStr;
use std::time::Instant;

/// Major ERC20 tokens for analysis (address, symbol, decimals, expected_total_supply_slot)
const ERC20_TOKENS: &[(&str, &str, u8, u8)] = &[
    ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC", 6, 2),
    ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT", 6, 1), // USDT uses slot 1
    ("0x6B175474E89094C44Da98b954EedeAC495271d0F", "DAI", 18, 2),
    ("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "WBTC", 8, 2),
    ("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "WETH", 18, 2),
];

/// Notable addresses to check balances for
const WHALE_ADDRESSES: &[(&str, &str)] = &[
    ("0x28C6c06298d514Db089934071355E5743bf21d60", "Binance Hot Wallet"),
    ("0x21a31Ee1afC51d94C2eFcCAa2092aD1028285549", "Binance Cold Wallet"),
    ("0xF977814e90dA44bFA03b6295A0616a897441aceC", "Binance US Hot Wallet"),
    ("0x8EB8a3b98659Cce290402893d0123abb75E3ab28", "Avalanche Bridge"),
];

/// Calculate storage slot for balance mapping: keccak256(abi.encode(address, slot))
fn calculate_balance_slot(token_holder: Address, balance_slot: u8) -> B256 {
    let mut data = [0u8; 64];
    // First 32 bytes: address (left-padded)
    data[12..32].copy_from_slice(token_holder.as_slice());
    // Last 32 bytes: slot number (right-padded)  
    data[63] = balance_slot;
    
    keccak256(data)
}

/// Calculate storage slot for allowance mapping: keccak256(abi.encode(spender, keccak256(abi.encode(owner, slot))))
fn calculate_allowance_slot(owner: Address, spender: Address, allowance_slot: u8) -> B256 {
    // First calculate the owner's slot in the allowances mapping
    let mut owner_data = [0u8; 64];
    owner_data[12..32].copy_from_slice(owner.as_slice());
    owner_data[63] = allowance_slot;
    let owner_slot_hash = keccak256(owner_data);
    
    // Then calculate the spender's slot within the owner's mapping
    let mut spender_data = [0u8; 64];
    spender_data[12..32].copy_from_slice(spender.as_slice());
    spender_data[32..64].copy_from_slice(owner_slot_hash.as_slice());
    
    keccak256(spender_data)
}

/// Format token amount with proper decimals
fn format_token_amount(amount: U256, decimals: u8, symbol: &str) -> String {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole_part = amount / divisor;
    let fraction_part = amount % divisor;
    
    if decimals <= 6 {
        format!("{}.{:0width$} {}", whole_part, fraction_part, symbol, width = decimals as usize)
    } else {
        // For high decimal tokens, show fewer decimal places
        let reduced_fraction = fraction_part / U256::from(10).pow(U256::from(decimals - 6));
        format!("{}.{:06} {}", whole_part, reduced_fraction, symbol)
    }
}

#[tokio::main]  
async fn main() -> Result<()> {
    println!("💰 ERC20 Storage Layout Analysis");
    println!("{}", "=".repeat(70));
    println!("📊 Comparing direct storage reads vs view function calls");
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    let latest_block = chain_query.get_latest_block()?;
    
    println!("🔗 Latest Block: {}", latest_block);
    println!();
    
    let overall_start = Instant::now();
    let mut storage_reads = 0;
    let mut view_calls = 0;
    
    for (token_address, symbol, decimals, total_supply_slot) in ERC20_TOKENS {
        println!("🪙 {} Token Analysis", symbol);
        println!("   Address: {}", token_address);
        
        let token_addr = Address::from_str(token_address)?;
        let token_start = Instant::now();
        
        // === STORAGE READS ===
        let storage_start = Instant::now();
        
        // Read total supply from storage slot
        let total_supply_slot_b256 = B256::from(U256::from(*total_supply_slot));
        let total_supply_storage = chain_query.get_storage_at(token_addr, total_supply_slot_b256, Some(latest_block)).await?;
        storage_reads += 1;
        
        // Read balance for first whale address
        let whale_addr = Address::from_str(WHALE_ADDRESSES[0].0)?;
        let balance_slot = calculate_balance_slot(whale_addr, 0); // Most tokens use slot 0 for balances
        let balance_storage = chain_query.get_storage_at(token_addr, balance_slot, Some(latest_block)).await?;
        storage_reads += 1;
        
        // Read allowance (owner=whale, spender=uniswap router)
        let uniswap_router = Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D")?;
        let allowance_slot = calculate_allowance_slot(whale_addr, uniswap_router, 1); // Most tokens use slot 1 for allowances
        let allowance_storage = chain_query.get_storage_at(token_addr, allowance_slot, Some(latest_block)).await?;
        storage_reads += 1;
        
        let storage_time = storage_start.elapsed();
        
        // === VIEW FUNCTION CALLS ===
        let view_start = Instant::now();
        
        let total_supply_view = chain_query.get_token_total_supply(token_addr, Some(latest_block)).await?;
        view_calls += 1;
        
        let balance_view = chain_query.get_token_balance(token_addr, whale_addr, Some(latest_block)).await?;
        view_calls += 1;
        
        let view_time = view_start.elapsed();
        
        let token_time = token_start.elapsed();
        
        // === RESULTS COMPARISON ===
        println!("   📊 Performance Comparison:");
        println!("      Storage reads (3 slots): {:.2}ms", storage_time.as_millis());
        println!("      View function calls (2): {:.2}ms", view_time.as_millis());
        println!("      Total time: {:.2}ms", token_time.as_millis());
        
        println!("   💰 Total Supply:");
        println!("      Storage read: {}", format_token_amount(total_supply_storage, *decimals, symbol));
        println!("      View function: {}", format_token_amount(total_supply_view, *decimals, symbol));
        println!("      Match: {}", if total_supply_storage == total_supply_view { "✅" } else { "❌" });
        
        println!("   🏦 {} Balance ({}):", WHALE_ADDRESSES[0].1, WHALE_ADDRESSES[0].0);
        println!("      Storage read: {}", format_token_amount(balance_storage, *decimals, symbol));
        println!("      View function: {}", format_token_amount(balance_view, *decimals, symbol));
        println!("      Match: {}", if balance_storage == balance_view { "✅" } else { "❌" });
        
        println!("   🔐 Allowance ({} → Uniswap Router):", WHALE_ADDRESSES[0].1);
        println!("      Storage read: {}", format_token_amount(allowance_storage, *decimals, symbol));
        
        // Show storage slot calculations
        println!("   🔍 Storage Slot Details:");
        println!("      Total Supply Slot: {} → {}", total_supply_slot, total_supply_slot_b256);
        println!("      Balance Slot: keccak256(address, 0) → 0x{}", hex::encode(balance_slot));  
        println!("      Allowance Slot: keccak256(...) → 0x{}", hex::encode(allowance_slot));
        
        println!();
    }
    
    let total_time = overall_start.elapsed();
    
    // Performance summary
    println!("{}", "=".repeat(70));
    println!("⚡ PERFORMANCE SUMMARY");
    println!("{}", "=".repeat(70));
    println!("Total Storage Reads: {}", storage_reads);
    println!("Total View Function Calls: {}", view_calls);
    println!("Total Query Time: {:.2}ms", total_time.as_millis());
    println!("Average Storage Read Time: {:.3}ms", total_time.as_millis() as f64 / storage_reads as f64);
    println!("Average View Call Time: {:.3}ms", total_time.as_millis() as f64 / view_calls as f64);
    
    // Use case comparison
    println!();
    println!("📋 USE CASE COMPARISON");
    println!("{}", "-".repeat(70));
    println!("🏪 Storage Reads:");
    println!("   ✅ Fastest for bulk analysis");
    println!("   ✅ Atomic consistency (same block)");
    println!("   ✅ Access to internal mappings");
    println!("   ❌ Requires knowledge of storage layout");
    println!("   ❌ May break with proxy upgrades");
    
    println!();  
    println!("📞 View Function Calls:");
    println!("   ✅ Always correct (uses contract logic)");
    println!("   ✅ Works with proxy contracts");
    println!("   ✅ Handles custom implementations");
    println!("   ❌ Slower for bulk queries");
    println!("   ❌ Limited to exposed functions");
    
    println!();
    println!("💡 Recommendation: Use view functions for correctness, storage for performance!");
    println!("🎯 Perfect combination: Validate with view calls, then optimize with storage!");
    
    Ok(())
}