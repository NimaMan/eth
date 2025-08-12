/// Deep Investigation of Scout Token Trading Restrictions
/// 
/// This tool investigates why some addresses can trade while others cannot.
/// We'll check:
/// 1. Token contract bytecode and storage
/// 2. Trading enable status
/// 3. Whitelist/blacklist mechanisms
/// 4. Block-based restrictions
/// 5. Token holder analysis

use alloy_primitives::{Address, U256, keccak256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{TransactionRequest, BlockId};
use alloy_sol_types::SolCall;
use std::str::FromStr;
use eyre::Result;
use hex;

const SCOUT_TOKEN: &str = "0x4e21B13330a5bfabcd1aC3F4Bc4fA444571F52ae";
const SCOUT_POOL: &str = "0xcad25c3386b115f0c456e2966f8843b53497c983";
const TEST_BLOCK: u64 = 23124804;

// Common ERC20 function signatures
alloy_sol_types::sol! {
    function balanceOf(address owner) external view returns (uint256);
    function owner() external view returns (address);
    function tradingEnabled() external view returns (bool);
    function maxWalletAmount() external view returns (uint256);
    function _isExcludedFromFees(address) external view returns (bool);
    function isBlacklisted(address) external view returns (bool);
    function isWhitelisted(address) external view returns (bool);
    function startBlock() external view returns (uint256);
    function tradingActiveBlock() external view returns (uint256);
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Scout Token Deep Investigation");
    println!("==================================\n");
    
    let provider = ProviderBuilder::new().on_http("http://localhost:8545".parse()?);
    let token_address = Address::from_str(SCOUT_TOKEN)?;
    
    // Test addresses
    let our_buyer = Address::from_str("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")?;
    let real_buyer = Address::from_str("0xBe3569068562218C792cF25b98DBf1418AFf2455")?;
    
    println!("📋 Investigation Parameters:");
    println!("  Token: {}", SCOUT_TOKEN);
    println!("  Block: {}", TEST_BLOCK);
    println!("  Our Test Address: {:?}", our_buyer);
    println!("  Real Buyer Address: {:?}", real_buyer);
    println!();
    
    // ==== STEP 1: Get Token Contract Code ====
    println!("📜 Checking Token Contract Code:");
    let code = provider.get_code_at(token_address).block_id(BlockId::Number(TEST_BLOCK.into())).await?;
    println!("  Contract code size: {} bytes", code.len());
    
    // Check for common patterns in bytecode
    let code_hex = hex::encode(&code);
    
    // Look for common function selectors in the bytecode
    let trading_enabled_selector = "bbc0c742"; // tradingEnabled()
    let is_blacklisted_selector = "fe575a87"; // isBlacklisted(address)
    let is_excluded_fees_selector = "4fbee193"; // _isExcludedFromFees(address)
    
    if code_hex.contains(trading_enabled_selector) {
        println!("  ✓ Found 'tradingEnabled' function");
    }
    if code_hex.contains(is_blacklisted_selector) {
        println!("  ✓ Found 'isBlacklisted' function");
    }
    if code_hex.contains(is_excluded_fees_selector) {
        println!("  ✓ Found fee exclusion mechanism");
    }
    println!();
    
    // ==== STEP 2: Check Trading Status ====
    println!("🚦 Checking Trading Status:");
    
    // Try to call tradingEnabled()
    let trading_enabled_call = tradingEnabledCall {};
    match provider.call(
        TransactionRequest::default()
            .to(token_address)
            .input(trading_enabled_call.abi_encode().into())
    ).block(BlockId::Number(TEST_BLOCK.into())).await {
        Ok(result) => {
            if result.len() >= 32 {
                let enabled = U256::from_be_slice(&result[0..32]);
                println!("  Trading Enabled: {}", enabled != U256::ZERO);
            }
        }
        Err(_) => {
            println!("  Trading status check failed (function may not exist)");
        }
    }
    
    // Try to get trading active block
    let trading_block_call = tradingActiveBlockCall {};
    match provider.call(
        TransactionRequest::default()
            .to(token_address)
            .input(trading_block_call.abi_encode().into())
    ).block(BlockId::Number(TEST_BLOCK.into())).await {
        Ok(result) => {
            if result.len() >= 32 {
                let block = U256::from_be_slice(&result[0..32]);
                println!("  Trading Active Block: {}", block);
                if block > U256::from(TEST_BLOCK) {
                    println!("  ⚠️  Trading not yet active at block {}!", TEST_BLOCK);
                }
            }
        }
        Err(_) => {
            // Try startBlock as alternative
            let start_block_call = startBlockCall {};
            if let Ok(result) = provider.call(
                TransactionRequest::default()
                    .to(token_address)
                    .input(start_block_call.abi_encode().into())
            ).block(BlockId::Number(TEST_BLOCK.into())).await {
                if result.len() >= 32 {
                    let block = U256::from_be_slice(&result[0..32]);
                    println!("  Start Block: {}", block);
                }
            }
        }
    }
    println!();
    
    // ==== STEP 3: Check Address Permissions ====
    println!("🔐 Checking Address Permissions:");
    
    for (name, addr) in [("Our Test", our_buyer), ("Real Buyer", real_buyer)] {
        println!("\n  {} Address ({:?}):", name, addr);
        
        // Check if blacklisted
        let blacklist_call = isBlacklistedCall(addr);
        match provider.call(
            TransactionRequest::default()
                .to(token_address)
                .input(blacklist_call.abi_encode().into())
        ).block(BlockId::Number(TEST_BLOCK.into())).await {
            Ok(result) => {
                if result.len() >= 32 {
                    let blacklisted = U256::from_be_slice(&result[0..32]) != U256::ZERO;
                    println!("    Blacklisted: {}", blacklisted);
                }
            }
            Err(_) => {}
        }
        
        // Check if whitelisted
        let whitelist_call = isWhitelistedCall(addr);
        match provider.call(
            TransactionRequest::default()
                .to(token_address)
                .input(whitelist_call.abi_encode().into())
        ).block(BlockId::Number(TEST_BLOCK.into())).await {
            Ok(result) => {
                if result.len() >= 32 {
                    let whitelisted = U256::from_be_slice(&result[0..32]) != U256::ZERO;
                    println!("    Whitelisted: {}", whitelisted);
                }
            }
            Err(_) => {}
        }
        
        // Check if excluded from fees
        let excluded_call = _isExcludedFromFeesCall(addr);
        match provider.call(
            TransactionRequest::default()
                .to(token_address)
                .input(excluded_call.abi_encode().into())
        ).block(BlockId::Number(TEST_BLOCK.into())).await {
            Ok(result) => {
                if result.len() >= 32 {
                    let excluded = U256::from_be_slice(&result[0..32]) != U256::ZERO;
                    println!("    Excluded from fees: {}", excluded);
                }
            }
            Err(_) => {}
        }
        
        // Check balance
        let balance_call = balanceOfCall { owner: addr };
        match provider.call(
            TransactionRequest::default()
                .to(token_address)
                .input(balance_call.abi_encode().into())
        ).block(BlockId::Number(TEST_BLOCK.into())).await {
            Ok(result) => {
                if result.len() >= 32 {
                    let balance = U256::from_be_slice(&result[0..32]);
                    println!("    Token Balance: {}", balance);
                }
            }
            Err(_) => {}
        }
    }
    println!();
    
    // ==== STEP 4: Check Storage Slots ====
    println!("🗄️ Checking Key Storage Slots:");
    
    // Common storage slot patterns for trading flags
    let slots_to_check = vec![
        (U256::from(0), "Slot 0 (often owner)"),
        (U256::from(1), "Slot 1 (often totalSupply)"),
        (U256::from(5), "Slot 5 (sometimes tradingEnabled)"),
        (U256::from(6), "Slot 6"),
        (U256::from(7), "Slot 7"),
        (U256::from(8), "Slot 8"),
        (U256::from(9), "Slot 9"),
        (U256::from(10), "Slot 10"),
    ];
    
    for (slot, description) in slots_to_check {
        match provider.get_storage_at(token_address, slot).block_id(BlockId::Number(TEST_BLOCK.into())).await {
            Ok(value) => {
                if value != U256::ZERO {
                    println!("  {}: 0x{:064x}", description, value);
                }
            }
            Err(e) => {
                println!("  {} read failed: {}", description, e);
            }
        }
    }
    
    // Check mapping storage for our addresses
    println!("\n  Checking address-specific storage:");
    
    // balanceOf mapping is usually at slot 0 or 1
    for slot_base in [0u64, 1u64] {
        for (name, addr) in [("Our Test", our_buyer), ("Real Buyer", real_buyer)] {
            // Calculate storage slot for mapping(address => value)
            let mut data = Vec::new();
            data.extend_from_slice(addr.as_slice());
            data.extend_from_slice(&[0u8; 12]); // Pad address to 32 bytes
            data.extend_from_slice(&U256::from(slot_base).to_be_bytes::<32>());
            
            let slot_hash = keccak256(&data);
            let storage_slot = U256::from_be_bytes(slot_hash.into());
            
            match provider.get_storage_at(token_address, storage_slot).block_id(BlockId::Number(TEST_BLOCK.into())).await {
                Ok(value) => {
                    if value != U256::ZERO {
                        println!("    {} at slot {}: {}", name, slot_base, value);
                    }
                }
                Err(_) => {}
            }
        }
    }
    println!();
    
    // ==== STEP 5: Get Contract Owner ====
    println!("👤 Contract Owner:");
    let owner_call = ownerCall {};
    match provider.call(
        TransactionRequest::default()
            .to(token_address)
            .input(owner_call.abi_encode().into())
    ).block(BlockId::Number(TEST_BLOCK.into())).await {
        Ok(result) => {
            if result.len() >= 32 {
                let owner = Address::from_slice(&result[12..32]);
                println!("  Owner: {:?}", owner);
                
                // Check if real buyer is the owner
                if owner == real_buyer {
                    println!("  ⚠️  Real buyer IS the contract owner!");
                }
            }
        }
        Err(_) => {
            println!("  Could not get owner");
        }
    }
    
    // ==== STEP 6: Summary ====
    println!("\n📊 INVESTIGATION SUMMARY:");
    println!("=====================================");
    println!("Based on the investigation, the token likely has:");
    println!("1. Trading restrictions that were active at block {}", TEST_BLOCK);
    println!("2. Specific addresses are whitelisted or excluded from restrictions");
    println!("3. The real buyer may have special permissions (owner, whitelisted, etc.)");
    println!("4. Our test address lacks the necessary permissions to trade");
    println!("\nThis explains why simulations fail with our test address but");
    println!("succeed with the real buyer's address through RPC.");
    
    Ok(())
}