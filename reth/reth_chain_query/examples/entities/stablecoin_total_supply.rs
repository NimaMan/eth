/// Stablecoin total supply analysis
/// 
/// This example queries total supply for major stablecoins and calculates
/// the combined stablecoin market cap on Ethereum.
/// 
/// Run with: cargo run --example stablecoin_total_supply

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{Address, U256, utils::format_units};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Stablecoin Total Supply Analysis ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Major stablecoins with their decimals
    let stablecoins = vec![
        ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6),
        ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7", 6),
        ("DAI", "6B175474E89094C44Da98b954EedeAC495271d0F", 18),
        ("BUSD", "4Fabb145d64652a948d72533023f6E7A623C7C53", 18),
        ("TUSD", "0000000000085d4780B73119b644AE5ecd22b376", 18),
        ("FRAX", "853d955aCEf822Db058eb8505911ED77F175b99e", 18),
        ("USDP", "8E870D67F660D95d5be530380D0eC0bd388289E1", 18),
        ("GUSD", "056Fd409E1d7A124BD7017459dFEa2F387b6d5Cd", 2),
        ("LUSD", "5f98805A4E8be255a32880FDeC7F6728C6568bA0", 18),
        ("sUSD", "57Ab1ec28D129707052df4dF418D58a2D46d5f51", 18),
    ];
    
    println!("Querying total supply for major stablecoins on Ethereum:\n");
    println!("{:<8} {:>20} {:>15} {:<10}", "Symbol", "Total Supply", "USD Value", "Contract");
    println!("{}", "-".repeat(70));
    
    let mut total_stablecoin_supply = 0f64;
    let mut successful_queries = 0;
    let mut failed_queries = 0;
    
    for (symbol, addr_str, decimals) in stablecoins {
        let address = Address::from_str(addr_str)?;
        
        match provider.get_token_total_supply(address, None).await {
            Ok(supply) => {
                let formatted = format_units(supply, decimals)?;
                let supply_float: f64 = formatted.parse().unwrap_or(0.0);
                total_stablecoin_supply += supply_float;
                successful_queries += 1;
                
                println!("{:<8} {:>20.2} ${:>14.2} 0x{}...", 
                    symbol, 
                    supply_float,
                    supply_float,
                    &addr_str[..6]
                );
            }
            Err(e) => {
                failed_queries += 1;
                println!("{:<8} {:>20} {:>15} Error: {}", 
                    symbol, 
                    "N/A",
                    "N/A",
                    e
                );
            }
        }
    }
    
    println!("{}", "-".repeat(70));
    println!("{:<8} {:>20.2} ${:>14.2}", 
        "TOTAL", 
        total_stablecoin_supply,
        total_stablecoin_supply
    );
    
    println!("\n=== Summary Statistics ===\n");
    println!("Total Stablecoin Market Cap on Ethereum: ${:.2} billion", 
        total_stablecoin_supply / 1_000_000_000.0
    );
    println!("Successful queries: {}", successful_queries);
    println!("Failed queries: {}", failed_queries);
    
    // Additional analysis
    println!("\n=== Market Share Analysis ===\n");
    
    // Get individual supplies for top 3
    let top_stables = vec![
        ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6),
        ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7", 6),
        ("DAI", "6B175474E89094C44Da98b954EedeAC495271d0F", 18),
    ];
    
    for (symbol, addr_str, decimals) in top_stables {
        let address = Address::from_str(addr_str)?;
        
        if let Ok(supply) = provider.get_token_total_supply(address, None).await {
            let formatted = format_units(supply, decimals)?;
            let supply_float: f64 = formatted.parse().unwrap_or(0.0);
            let market_share = (supply_float / total_stablecoin_supply) * 100.0;
            
            println!("{} Market Share: {:.2}% (${:.2}B)", 
                symbol,
                market_share,
                supply_float / 1_000_000_000.0
            );
        }
    }
    
    println!("\n✅ Stablecoin supply analysis complete!");
    
    Ok(())
}