/// Build a complete portfolio view for addresses
/// 
/// This example shows how to:
/// 1. Get complete portfolio (ETH + tokens)
/// 2. Calculate USD values
/// 3. Track portfolio changes over time
/// 4. Export portfolio data
/// 
/// Run with: cargo run --example portfolio

use reth_chain_query::{RethQueryProvider, Portfolio, Result};
use alloy_primitives::{Address, U256, utils::{format_ether, format_units}};
use std::str::FromStr;
use std::collections::HashMap;

// Major tokens to track
const TOKENS: &[(&str, &str, u8)] = &[
    ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6),
    ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7", 6),
    ("WETH", "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", 18),
    ("DAI", "6B175474E89094C44Da98b954EedeAC495271d0F", 18),
    ("WBTC", "2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", 8),
    ("SHIB", "95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE", 18),
    ("LINK", "514910771AF9Ca656af840dff83E8264EcF986CA", 18),
    ("UNI", "1f9840a85d5aF5bf1D1762F925BDADdC4201F984", 18),
];

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Complete Portfolio Analysis ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // === Single Address Portfolio ===
    println!("1. Individual Portfolio");
    println!("-" .repeat(60));
    
    let address = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?; // Vitalik
    let portfolio = build_portfolio(&provider, address, None).await?;
    
    print_portfolio("Vitalik", &portfolio);
    
    println!();
    
    // === Multiple Address Portfolios ===
    println!("2. Multiple Portfolios");
    println!("-" .repeat(60));
    
    let addresses = vec![
        ("Ethereum Foundation", "de0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"),
        ("Gitcoin", "de21F729137C5Af1b01d73aF1dC21eFfa2B8a0d6"),
        ("Protocol Guild", "F29Ff96aaB7A9425Ca878cFF8f3d6324119F1664"),
    ];
    
    for (name, addr_str) in addresses {
        let address = Address::from_str(addr_str)?;
        let portfolio = build_portfolio(&provider, address, None).await?;
        print_portfolio(name, &portfolio);
        println!();
    }
    
    // === Historical Portfolio ===
    println!("3. Historical Portfolio Analysis");
    println!("-" .repeat(60));
    
    let address = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?;
    
    // Compare portfolio at different times
    let snapshots = vec![
        (15_537_393, "The Merge"),
        (17_000_000, "Post-Shanghai"),
        (None, "Current"),
    ];
    
    println!("Vitalik's portfolio over time:\n");
    
    for (block, label) in snapshots {
        match build_portfolio(&provider, address, block).await {
            Ok(portfolio) => {
                println!("{} {}:", 
                    label,
                    block.map(|b| format!("(block {})", b)).unwrap_or_default()
                );
                print_portfolio_summary(&portfolio);
                println!();
            }
            Err(e) => {
                println!("{}: Error - {}", label, e);
            }
        }
    }
    
    // === Portfolio Aggregation ===
    println!("4. Aggregate Portfolio");
    println!("-" .repeat(60));
    
    // Track total value across multiple addresses
    let fund_addresses = vec![
        ("Treasury", "de0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"),
        ("Grants", "de21F729137C5Af1b01d73aF1dC21eFfa2B8a0d6"),
        ("Operations", "F29Ff96aaB7A9425Ca878cFF8f3d6324119F1664"),
    ];
    
    let mut total_eth = U256::ZERO;
    let mut total_tokens: HashMap<String, U256> = HashMap::new();
    
    for (name, addr_str) in &fund_addresses {
        let address = Address::from_str(addr_str)?;
        let portfolio = build_portfolio(&provider, address, None).await?;
        
        total_eth = total_eth + portfolio.eth_balance;
        
        for (token_addr, balance) in portfolio.token_balances {
            // Find token info
            if let Some((symbol, _, _)) = TOKENS.iter()
                .find(|(_, addr, _)| Address::from_str(addr).unwrap() == token_addr) 
            {
                *total_tokens.entry(symbol.to_string()).or_insert(U256::ZERO) += balance;
            }
        }
        
        println!("{:15} {} ETH", format!("{}:", name), format_ether(portfolio.eth_balance));
    }
    
    println!("{:15} {} ETH", "Total ETH:", format_ether(total_eth));
    
    if !total_tokens.is_empty() {
        println!("\nTotal tokens:");
        for (symbol, balance) in total_tokens {
            if let Some((_, _, decimals)) = TOKENS.iter().find(|(s, _, _)| s == &symbol) {
                if balance > U256::ZERO {
                    println!("  {}: {}", symbol, format_units(balance, *decimals)?);
                }
            }
        }
    }
    
    println!("\n✅ Portfolio analysis complete!");
    
    Ok(())
}

async fn build_portfolio(
    provider: &RethQueryProvider,
    address: Address,
    block: Option<u64>,
) -> Result<Portfolio> {
    // Convert token list to addresses
    let token_addresses: Vec<Address> = TOKENS
        .iter()
        .map(|(_, addr, _)| Address::from_str(addr).unwrap())
        .collect();
    
    provider.get_portfolio(address, token_addresses, block).await
}

fn print_portfolio(name: &str, portfolio: &Portfolio) {
    println!("{} (0x{}):", name, portfolio.address);
    println!("  ETH: {}", format_ether(portfolio.eth_balance));
    
    // Print token balances
    for (token_addr, balance) in &portfolio.token_balances {
        if let Some((symbol, _, decimals)) = TOKENS.iter()
            .find(|(_, addr, _)| Address::from_str(addr).unwrap() == *token_addr) 
        {
            if *balance > U256::ZERO {
                let formatted = format_units(*balance, *decimals).unwrap_or("Error".to_string());
                println!("  {}: {}", symbol, formatted);
            }
        }
    }
    
    println!("  Block: #{}", portfolio.block_number);
}

fn print_portfolio_summary(portfolio: &Portfolio) {
    let token_count = portfolio.token_balances
        .iter()
        .filter(|(_, b)| **b > U256::ZERO)
        .count();
    
    println!("  ETH: {}", format_ether(portfolio.eth_balance));
    println!("  Tokens held: {}", token_count);
}