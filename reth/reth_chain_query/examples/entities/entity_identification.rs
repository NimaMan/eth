/// Entity Identification Example
/// 
/// Demonstrates how to identify and classify addresses across all entity types.
/// This example showcases:
/// - Entity type identification (Stablecoin, CEX, ETF, Unknown)
/// - Cross-entity statistics
/// - Flow analysis between different entity types
/// - Comprehensive entity lookup

use reth_chain_query::{ChainQuery, Result, Address};
use reth_chain_query::entities::{
    common::{identify_entity_type, EntityType},
    stablecoins::{is_stablecoin, get_stablecoin_by_address, STABLECOINS},
    cex::{is_cex_address, get_cex_by_address, CEX_ADDRESS_COUNT},
    etfs::{is_etf_address, get_etf_by_address, ETF_ADDRESS_COUNT},
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Entity Identification using Entities Module");
    println!("{}", "=".repeat(70));
    
    // Initialize ChainQuery (needed for block queries)
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let _chain_query = ChainQuery::new(reth_datadir)?;
    
    // 1. Overall Statistics
    println!("\n📊 ENTITY STATISTICS");
    println!("{}", "-".repeat(70));
    println!("Stablecoins: {} tokens", STABLECOINS.len());
    println!("CEX Addresses: {} addresses", CEX_ADDRESS_COUNT);
    println!("ETF Addresses: {} addresses", ETF_ADDRESS_COUNT);
    println!("Total Tracked: {} entities", STABLECOINS.len() + CEX_ADDRESS_COUNT + ETF_ADDRESS_COUNT);
    
    // 2. Entity Identification Examples
    println!("\n🎯 ENTITY IDENTIFICATION");
    println!("{}", "-".repeat(70));
    
    let test_addresses = vec![
        ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC"),
        ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT"),
        ("0x28C6c06298d514Db089934071355E5743bf21d60", "Binance"),
        ("0x71660c4005BA85c37ccec55d0C4493E66Fe775d3", "Coinbase"),
        ("0x0171F896002665C3ea3Ed0c55f21026cA0A734A0", "BlackRock ETF"),
        ("0x210350289675a16b60Aa6ed0796F12f2Fd6CA45c", "Grayscale ETF"),
        ("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb", "Random Address"),
        ("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "WETH (not tracked)"),
    ];
    
    for (addr_str, expected) in test_addresses {
        let addr = Address::from_str(addr_str)?;
        let entity_type = identify_entity_type(addr);
        
        let type_emoji = match entity_type {
            EntityType::Stablecoin => "🪙",
            EntityType::CEX => "🏦",
            EntityType::ETF => "📈",
            EntityType::Unknown => "❓",
        };
        
        println!("\n{} Address: {}...", type_emoji, &addr_str[..10]);
        println!("  Expected: {}", expected);
        println!("  Type: {:?}", entity_type);
        
        // Get detailed information based on type
        match entity_type {
            EntityType::Stablecoin => {
                if let Some(info) = get_stablecoin_by_address(addr) {
                    println!("  Details: {} ({}) - {} decimals", 
                        info.symbol, info.unit, info.decimals);
                }
            }
            EntityType::CEX => {
                if let Some(info) = get_cex_by_address(addr) {
                    println!("  Details: {} ({})", info.name, info.exchange);
                }
            }
            EntityType::ETF => {
                if let Some(info) = get_etf_by_address(addr) {
                    println!("  Details: {} ({})", info.name, info.provider);
                }
            }
            EntityType::Unknown => {
                println!("  Details: Not a tracked entity");
            }
        }
    }
    
    // 3. Cross-Entity Analysis
    println!("\n🔄 CROSS-ENTITY FLOW PATTERNS");
    println!("{}", "-".repeat(70));
    
    let flow_scenarios = vec![
        ("CEX → Stablecoin", "0x28C6c06298d514Db089934071355E5743bf21d60", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        ("Stablecoin → CEX", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "0x71660c4005BA85c37ccec55d0C4493E66Fe775d3"),
        ("CEX → ETF", "0x28C6c06298d514Db089934071355E5743bf21d60", "0x0171F896002665C3ea3Ed0c55f21026cA0A734A0"),
        ("ETF → CEX", "0x0171F896002665C3ea3Ed0c55f21026cA0A734A0", "0x28C6c06298d514Db089934071355E5743bf21d60"),
        ("Unknown → CEX", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb", "0x28C6c06298d514Db089934071355E5743bf21d60"),
    ];
    
    for (scenario, from_str, to_str) in flow_scenarios {
        let from = Address::from_str(from_str)?;
        let to = Address::from_str(to_str)?;
        
        let from_type = identify_entity_type(from);
        let to_type = identify_entity_type(to);
        
        println!("\n{}: {:?} → {:?}", scenario, from_type, to_type);
        
        // Analyze the flow pattern
        match (from_type, to_type) {
            (EntityType::CEX, EntityType::Stablecoin) => {
                println!("  💡 Pattern: Exchange buying stablecoin reserves");
            }
            (EntityType::Stablecoin, EntityType::CEX) => {
                println!("  💡 Pattern: Stablecoin deposited to exchange");
            }
            (EntityType::CEX, EntityType::ETF) => {
                println!("  💡 Pattern: Exchange facilitating ETF creation");
            }
            (EntityType::ETF, EntityType::CEX) => {
                println!("  💡 Pattern: ETF redemption through exchange");
            }
            (EntityType::Unknown, EntityType::CEX) => {
                println!("  💡 Pattern: User deposit to exchange");
            }
            (EntityType::CEX, EntityType::Unknown) => {
                println!("  💡 Pattern: User withdrawal from exchange");
            }
            _ => {
                println!("  💡 Pattern: Other flow type");
            }
        }
    }
    
    // 4. Batch Entity Checking
    println!("\n📋 BATCH ENTITY CHECKING");
    println!("{}", "-".repeat(70));
    
    let batch_addresses = vec![
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
        "0x28C6c06298d514Db089934071355E5743bf21d60", // Binance
        "0x0171F896002665C3ea3Ed0c55f21026cA0A734A0", // BlackRock
        "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb", // Random
        "0xdAC17F958D2ee523a2206206994597C13D831ec7", // USDT
    ];
    
    let mut stablecoin_count = 0;
    let mut cex_count = 0;
    let mut etf_count = 0;
    let mut unknown_count = 0;
    
    for addr_str in &batch_addresses {
        let addr = Address::from_str(addr_str)?;
        match identify_entity_type(addr) {
            EntityType::Stablecoin => stablecoin_count += 1,
            EntityType::CEX => cex_count += 1,
            EntityType::ETF => etf_count += 1,
            EntityType::Unknown => unknown_count += 1,
        }
    }
    
    println!("Analyzed {} addresses:", batch_addresses.len());
    println!("  🪙 Stablecoins: {}", stablecoin_count);
    println!("  🏦 CEX: {}", cex_count);
    println!("  📈 ETF: {}", etf_count);
    println!("  ❓ Unknown: {}", unknown_count);
    
    // 5. Quick Lookup Functions
    println!("\n⚡ QUICK LOOKUP FUNCTIONS");
    println!("{}", "-".repeat(70));
    
    let test_addr = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    
    println!("Testing address: {:?}", test_addr);
    println!("  Is Stablecoin? {}", is_stablecoin(test_addr));
    println!("  Is CEX? {}", is_cex_address(test_addr));
    println!("  Is ETF? {}", is_etf_address(test_addr));
    
    // 6. Entity Name Resolution
    println!("\n📝 ENTITY NAME RESOLUTION");
    println!("{}", "-".repeat(70));
    
    let resolve_addresses = vec![
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "0x28C6c06298d514Db089934071355E5743bf21d60",
        "0x0171F896002665C3ea3Ed0c55f21026cA0A734A0",
    ];
    
    for addr_str in resolve_addresses {
        let addr = Address::from_str(addr_str)?;
        let entity_type = identify_entity_type(addr);
        
        let name = match entity_type {
            EntityType::Stablecoin => {
                get_stablecoin_by_address(addr)
                    .map(|info| info.symbol.to_string())
                    .unwrap_or_else(|| "Unknown Stablecoin".to_string())
            }
            EntityType::CEX => {
                get_cex_by_address(addr)
                    .map(|info| info.name.to_string())
                    .unwrap_or_else(|| "Unknown CEX".to_string())
            }
            EntityType::ETF => {
                get_etf_by_address(addr)
                    .map(|info| info.name.to_string())
                    .unwrap_or_else(|| "Unknown ETF".to_string())
            }
            EntityType::Unknown => "Not Tracked".to_string(),
        };
        
        println!("{}: {}", &addr_str[..10], name);
    }
    
    println!("\n✅ Entity identification complete!");
    println!("\n💡 Summary:");
    println!("The entities module provides fast, type-safe identification of:");
    println!("- {} stablecoins with metadata", STABLECOINS.len());
    println!("- {} CEX addresses across major exchanges", CEX_ADDRESS_COUNT);
    println!("- {} ETF addresses from major providers", ETF_ADDRESS_COUNT);
    println!("\nAll lookups are O(1) using HashMaps for maximum performance!");
    
    Ok(())
}