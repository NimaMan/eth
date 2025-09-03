/// Token metadata and information queries
/// 
/// This example shows how to:
/// 1. Get token name, symbol, decimals
/// 2. Query total supply
/// 3. Get token metadata efficiently
/// 4. Handle different token standards
/// 
/// Run with: cargo run --example token_info

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{Address, utils::format_units};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Token Information Queries ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // === Basic Token Metadata ===
    println!("1. Token Metadata");
    println!("-" .repeat(60));
    
    let tokens = vec![
        ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7"),
        ("WETH", "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        ("DAI", "6B175474E89094C44Da98b954EedeAC495271d0F"),
        ("WBTC", "2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"),
        ("SHIB", "95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE"),
        ("UNI", "1f9840a85d5aF5bf1D1762F925BDADdC4201F984"),
        ("LINK", "514910771AF9Ca656af840dff83E8264EcF986CA"),
    ];
    
    for (expected_symbol, addr_str) in &tokens {
        let address = Address::from_str(addr_str)?;
        
        // Get all metadata in one call
        let metadata = provider.get_token_metadata(address).await?;
        
        println!("{} Token (0x{}):", expected_symbol, address);
        println!("  Name: {}", metadata.name);
        println!("  Symbol: {}", metadata.symbol);
        println!("  Decimals: {}", metadata.decimals);
        
        // Verify symbol matches
        if metadata.symbol != *expected_symbol {
            println!("  ⚠️  Symbol mismatch: expected {}, got {}", 
                expected_symbol, metadata.symbol);
        }
    }
    
    println!();
    
    // === Total Supply Queries ===
    println!("2. Total Supply");
    println!("-" .repeat(60));
    
    // Query total supply for major stablecoins
    let stablecoins = vec![
        ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6),
        ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7", 6),
        ("DAI", "6B175474E89094C44Da98b954EedeAC495271d0F", 18),
        ("BUSD", "4Fabb145d64652a948d72533023f6E7A623C7C53", 18),
        ("TUSD", "0000000000085d4780B73119b644AE5ecd22b376", 18),
    ];
    
    let mut total_stablecoin_supply = 0f64;
    
    for (symbol, addr_str, decimals) in stablecoins {
        let address = Address::from_str(addr_str)?;
        
        match provider.get_token_total_supply(address, None).await {
            Ok(supply) => {
                let formatted = format_units(supply, decimals)?;
                let supply_float: f64 = formatted.parse().unwrap_or(0.0);
                total_stablecoin_supply += supply_float;
                
                println!("{:6} Supply: ${:>15.2}", symbol, supply_float);
            }
            Err(e) => {
                println!("{:6} Supply: Error - {}", symbol, e);
            }
        }
    }
    
    println!("{:6} Total:  ${:>15.2}", "", total_stablecoin_supply);
    
    println!();
    
    // === Complete Token Info ===
    println!("3. Complete Token Information");
    println!("-" .repeat(60));
    
    // Get comprehensive info for a token
    let uni_token = Address::from_str("1f9840a85d5aF5bf1D1762F925BDADdC4201F984")?;
    
    let metadata = provider.get_token_metadata(uni_token).await?;
    let total_supply = provider.get_token_total_supply(uni_token, None).await?;
    
    println!("UNI Token Analysis:");
    println!("  Contract: 0x{}", uni_token);
    println!("  Name: {}", metadata.name);
    println!("  Symbol: {}", metadata.symbol);
    println!("  Decimals: {}", metadata.decimals);
    println!("  Total Supply: {}", format_units(total_supply, metadata.decimals)?);
    
    // Check some holder balances
    let holders = vec![
        ("Uniswap Treasury", "e3953D9d317B834592aB58AB2c7A6aD22b54075D"),
        ("Binance", "F977814e90dA44bFA03b6295A0616a897441aceC"),
    ];
    
    println!("\n  Major Holders:");
    for (name, addr_str) in holders {
        let holder = Address::from_str(addr_str)?;
        let balance = provider.get_token_balance(uni_token, holder, None).await?;
        
        if balance > U256::ZERO {
            let formatted = format_units(balance, metadata.decimals)?;
            println!("    {}: {} UNI", name, formatted);
        }
    }
    
    println!();
    
    // === Historical Supply ===
    println!("4. Historical Token Supply");
    println!("-" .repeat(60));
    
    let usdc = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    
    let checkpoints = vec![
        (15_537_393, "The Merge"),
        (17_000_000, "Post-Shanghai"),
        (None, "Current"),
    ];
    
    println!("USDC Supply Over Time:");
    for (block, label) in checkpoints {
        match provider.get_token_total_supply(usdc, block).await {
            Ok(supply) => {
                let formatted = format_units(supply, 6)?;
                println!("  {} {}: ${}", 
                    label,
                    block.map(|b| format!("(block {})", b)).unwrap_or_default(),
                    formatted
                );
            }
            Err(e) => {
                println!("  {}: Error - {}", label, e);
            }
        }
    }
    
    println!("\n✅ Token information queries complete!");
    
    Ok(())
}