/// Example: Query ERC20 token information
/// 
/// Demonstrates the new ERC20 querying capabilities including
/// name and symbol methods.

use reth_chain_query::{ChainQuery, Result};
use alloy_primitives::Address;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    // Test tokens
    let tokens = [
        ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
        ("WETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        ("DAI", "0x6B175474E89094C44Da98b954EedeAC495271d0F"),
    ];
    
    println!("Testing ERC20 token queries with reth_chain_query");
    println!("{}", "=".repeat(60));
    
    for (expected_symbol, address_str) in &tokens {
        let address = Address::from_str(&address_str[2..])?; // Remove 0x prefix
        
        println!("\nQuerying {expected_symbol} ({address_str}):");
        
        // Get all token info using the new combined method
        match chain_query.token.get_erc20_info(address, None).await {
            Ok(info) => {
                println!("  ✓ Name: '{}'", info.name);
                println!("  ✓ Symbol: '{}'", info.symbol);
                println!("  ✓ Decimals: {}", info.decimals);
                
                // Format supply with decimals
                let supply_f64 = info.total_supply.to_string().parse::<f64>()
                    .unwrap_or(0.0) / 10_f64.powi(info.decimals as i32);
                    
                println!("  ✓ Total Supply: {:.2}", supply_f64);
                
                // Verify symbol matches expectation
                if info.symbol == *expected_symbol {
                    println!("  ✅ Symbol matches expected!");
                } else {
                    println!("  ⚠️  Symbol '{}' != expected '{}'", info.symbol, expected_symbol);
                }
            }
            Err(e) => {
                println!("  ❌ Error: {}", e);
            }
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("✅ ERC20 query test completed!");
    
    Ok(())
}