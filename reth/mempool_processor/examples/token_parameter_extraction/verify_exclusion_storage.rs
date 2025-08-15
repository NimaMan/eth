/// Verify Exclusion Storage
///
/// This actually checks the storage slots to prove whether an address
/// is excluded from fees or not.

use alloy_provider::{Provider, ProviderBuilder};
use alloy_primitives::{Address, U256, B256, keccak256, FixedBytes};
use alloy_rpc_types::BlockId;
use std::str::FromStr;
use eyre::Result;

/// Calculate the storage slot for a mapping(address => bool)
/// In Solidity: mapping storage is at keccak256(address . slot_number)
fn calculate_mapping_slot(address: Address, base_slot: u64) -> U256 {
    let mut data = [0u8; 64];
    // Address is left-padded to 32 bytes
    data[12..32].copy_from_slice(address.as_slice());
    // Slot number is right-aligned in the second 32 bytes
    data[56..64].copy_from_slice(&base_slot.to_be_bytes());
    let hash = keccak256(&data);
    U256::from_be_bytes(hash.0)
}

async fn check_all_possible_slots(
    provider: &impl Provider,
    token: Address,
    test_address: Address,
    block: u64,
) -> Result<Vec<(u64, U256, U256)>> {
    let mut found_slots = Vec::new();
    
    // Check common slot numbers for exclusion mappings
    // These are typical slots where fee exclusion mappings are stored
    let possible_base_slots = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
        11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
    ];
    
    for base_slot in possible_base_slots {
        let storage_slot = calculate_mapping_slot(test_address, base_slot);
        
        let value = provider
            .get_storage_at(token, storage_slot)
            .block_id(BlockId::Number(block.into()))
            .await?;
        
        if !value.is_zero() {
            found_slots.push((base_slot, storage_slot, value));
            println!("   ✅ Found non-zero at base slot {}: {:?} = {:?}", 
                base_slot, storage_slot, value);
        }
    }
    
    Ok(found_slots)
}

async fn check_raw_storage_slots(
    provider: &impl Provider,
    token: Address,
    block: u64,
) -> Result<()> {
    println!("\n📊 Checking raw storage slots (0-30):");
    
    for i in 0..31 {
        let slot = U256::from(i);
        
        let value = provider
            .get_storage_at(token, slot)
            .block_id(BlockId::Number(block.into()))
            .await?;
        
        if !value.is_zero() {
            println!("   Slot {}: {:?}", i, value);
            
            // Try to interpret as address if it looks like one
            let bytes = value.to_be_bytes::<32>();
            if bytes[0..12].iter().all(|&b| b == 0) && !bytes[12..32].iter().all(|&b| b == 0) {
                let addr = Address::from_slice(&bytes[12..32]);
                println!("      → Possible address: {:?}", addr);
            }
        }
    }
    
    Ok(())
}

async fn verify_specific_exclusion(
    provider: &impl Provider,
    token: Address,
    address: Address,
    name: &str,
    block: u64,
) -> Result<()> {
    println!("\n🔍 Checking {}: {:?}", name, address);
    
    let slots = check_all_possible_slots(provider, token, address, block).await?;
    
    if slots.is_empty() {
        println!("   ❌ No non-zero storage found for this address");
        println!("      This address is likely NOT excluded from fees");
    } else {
        println!("   ✅ Found {} non-zero storage slots", slots.len());
        println!("      This address IS likely excluded or has special status");
        
        for (base, _, value) in &slots {
            // Check if it's a boolean true (1) or has other data
            if *value == U256::from(1) {
                println!("      Base slot {}: Boolean TRUE (excluded)", base);
            } else {
                println!("      Base slot {}: Value = {:?}", base, value);
            }
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Verifying DOGEINA Fee Exclusion Storage");
    println!("{}", "=".repeat(60));
    
    let token = Address::from_str("0xF2e24D564a08A3Acc31985eBD3C6Ee7FA9943a84")?; // DOGEINA
    let block = 22939570u64;
    
    let provider = ProviderBuilder::new()
        .connect_http("http://localhost:8545".parse()?);
    
    // First, check raw storage to understand the contract's storage layout
    check_raw_storage_slots(&provider, token, block).await?;
    
    // Test specific addresses
    let test_cases = vec![
        ("Successful Seller", Address::from_str("0x6D356ab697B0AB2A871B6Be79073A35664340440")?),
        ("Token Owner", Address::from_str("0x62ed63ce328665488914992b247b1ef8f78199a1")?),
        ("Our Test Address", Address::from_str("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")?),
        ("Maestro Router", Address::from_str("0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e")?),
        ("Uniswap V2 Router", Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D")?),
        ("Uniswap V2 Pair", Address::from_str("0x5423621C6D3E22465876deb92aD4f40Dc25b62A3")?),
    ];
    
    for (name, addr) in test_cases {
        verify_specific_exclusion(&provider, token, addr, name, block).await?;
    }
    
    println!("\n💡 INTERPRETATION:");
    println!("   - If an address has non-zero storage in the mapping slots,");
    println!("     it has special privileges (excluded from fees/limits)");
    println!("   - If an address has no non-zero storage, it's a regular user");
    println!("   - The exact slot number tells us which mapping it is");
    
    Ok(())
}