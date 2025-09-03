/// Basic token metadata queries - name, symbol, decimals
/// 
/// This example demonstrates how to query basic ERC20 token information
/// including name, symbol, and decimals for popular tokens.
/// 
/// Run with: cargo run --example token_metadata_basic

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::Address;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Basic Token Metadata Queries ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Popular tokens to query
    let tokens = vec![
        ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7"),
        ("WETH", "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        ("DAI", "6B175474E89094C44Da98b954EedeAC495271d0F"),
        ("WBTC", "2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"),
        ("SHIB", "95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE"),
        ("UNI", "1f9840a85d5aF5bf1D1762F925BDADdC4201F984"),
        ("LINK", "514910771AF9Ca656af840dff83E8264EcF986CA"),
        ("MATIC", "7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0"),
        ("APE", "4d224452801ACEd8B2F0aebE155379bb5D594381"),
    ];
    
    println!("Querying metadata for popular ERC20 tokens:\n");
    println!("{:<10} {:<30} {:<10} {:<10}", "Symbol", "Name", "Decimals", "Address");
    println!("{}", "-".repeat(70));
    
    for (expected_symbol, addr_str) in &tokens {
        let address = Address::from_str(addr_str)?;
        
        // Get all metadata in one call
        match provider.get_token_metadata(address).await {
            Ok(metadata) => {
                println!("{:<10} {:<30} {:<10} 0x{}...{}", 
                    metadata.symbol,
                    if metadata.name.len() > 30 {
                        format!("{}...", &metadata.name[..27])
                    } else {
                        metadata.name.clone()
                    },
                    metadata.decimals,
                    &addr_str[..6],
                    &addr_str[addr_str.len()-4..]
                );
                
                // Verify symbol matches expected
                if metadata.symbol != *expected_symbol {
                    println!("  ⚠️  Symbol mismatch: expected {}, got {}", 
                        expected_symbol, metadata.symbol);
                }
            }
            Err(e) => {
                println!("{:<10} Error: {}", expected_symbol, e);
            }
        }
    }
    
    println!("\n=== Metadata Query Methods ===\n");
    
    // Demonstrate individual metadata queries
    let usdc = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    
    println!("Individual queries for USDC:");
    
    // Get name
    match provider.get_token_name(usdc, None).await {
        Ok(name) => println!("  Name: {}", name),
        Err(e) => println!("  Name: Error - {}", e),
    }
    
    // Get symbol
    match provider.get_token_symbol(usdc, None).await {
        Ok(symbol) => println!("  Symbol: {}", symbol),
        Err(e) => println!("  Symbol: Error - {}", e),
    }
    
    // Get decimals
    match provider.get_token_decimals(usdc, None).await {
        Ok(decimals) => println!("  Decimals: {}", decimals),
        Err(e) => println!("  Decimals: Error - {}", e),
    }
    
    println!("\n✅ Token metadata queries complete!");
    
    Ok(())
}