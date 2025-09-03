/// Direct storage slot access
/// 
/// This example shows how to:
/// 1. Read raw storage slots
/// 2. Calculate storage positions for mappings
/// 3. Detect proxy contracts
/// 4. Read complex storage layouts
/// 
/// Run with: cargo run --example read_storage

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{Address, B256, U256, keccak256};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Direct Storage Access ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // === Simple Storage Slot ===
    println!("1. Simple Storage Slots");
    println!("-" .repeat(60));
    
    // USDC contract
    let usdc = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    
    // Common storage slots in ERC20 tokens
    // Slot 0: Often used for first state variable
    // Slot 1: Often totalSupply
    // Slot 2: Often first mapping (balances)
    
    let total_supply_slot = B256::from(U256::from(1));
    let total_supply = provider.get_storage(usdc, total_supply_slot, None).await?;
    
    println!("USDC Total Supply:");
    println!("  Raw value: {}", total_supply);
    println!("  Formatted: {} USDC", total_supply / U256::from(10u64.pow(6)));
    
    println!();
    
    // === Mapping Storage Calculation ===
    println!("2. Storage Mappings (balanceOf)");
    println!("-" .repeat(60));
    
    // To read from a mapping, we need to calculate the storage slot
    // For mapping(address => uint256) at slot N:
    // storage_slot = keccak256(address . N)
    
    let holder = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?; // Vitalik
    
    // USDC balances mapping is at slot 9
    let balances_slot = 9u64;
    
    // Calculate storage position
    let mut key = Vec::new();
    key.extend_from_slice(&holder.to_fixed_bytes());
    key.extend_from_slice(&U256::from(balances_slot).to_be_bytes::<32>());
    let storage_key = B256::from(keccak256(&key));
    
    let balance = provider.get_storage(usdc, storage_key, None).await?;
    
    println!("Vitalik's USDC balance:");
    println!("  Storage slot: 0x{}", storage_key);
    println!("  Raw value: {}", balance);
    if balance > U256::ZERO {
        println!("  Formatted: {} USDC", balance / U256::from(10u64.pow(6)));
    }
    
    println!();
    
    // === Proxy Contract Detection ===
    println!("3. Proxy Contract Detection");
    println!("-" .repeat(60));
    
    // EIP-1967 standard proxy implementation slot
    // bytes32(uint256(keccak256('eip1967.proxy.implementation')) - 1)
    let implementation_slot = B256::from_str(
        "360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc"
    )?;
    
    // Check some known proxy contracts
    let proxies = vec![
        ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        ("DAI", "6B175474E89094C44Da98b954EedeAC495271d0F"),
    ];
    
    for (name, addr_str) in proxies {
        let address = Address::from_str(addr_str)?;
        let impl_address = provider.get_storage(address, implementation_slot, None).await?;
        
        if impl_address != U256::ZERO {
            // Convert U256 to Address (last 20 bytes)
            let addr_bytes = impl_address.to_be_bytes::<32>();
            let impl = Address::from_slice(&addr_bytes[12..]);
            println!("{} is a proxy:", name);
            println!("  Proxy: 0x{}", address);
            println!("  Implementation: 0x{}", impl);
        } else {
            println!("{} is NOT a proxy (0x{})", name, address);
        }
    }
    
    println!();
    
    // === Finding Storage Layout ===
    println!("4. Exploring Storage Layout");
    println!("-" .repeat(60));
    
    // Sometimes you need to explore storage to find the layout
    let contract = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    
    println!("First 10 storage slots of USDC:");
    for i in 0..10 {
        let slot = B256::from(U256::from(i));
        let value = provider.get_storage(contract, slot, None).await?;
        
        if value != U256::ZERO {
            println!("  Slot {}: 0x{:064x}", i, value);
            
            // Try to interpret the value
            if i == 1 {
                println!("    → Likely totalSupply: {}", value / U256::from(10u64.pow(6)));
            }
        }
    }
    
    println!("\n✅ Storage reading complete!");
    println!("\nTip: Use forge inspect or contract source to understand storage layout");
    
    Ok(())
}