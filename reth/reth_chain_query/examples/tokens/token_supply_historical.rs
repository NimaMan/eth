/// Historical token supply tracking
/// 
/// This example demonstrates how to track token supply changes over time
/// at different block heights, useful for understanding token inflation/deflation.
/// 
/// Run with: cargo run --example token_supply_historical

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{Address, U256, utils::format_units};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Historical Token Supply Tracking ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Track USDC supply over time
    let usdc = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    let usdc_decimals = 6;
    
    println!("Tracking USDC (USD Coin) supply changes:\n");
    
    // Important milestones in recent history
    // Note: Blocks older than ~90 days may be pruned
    let checkpoints = vec![
        (Some(20_000_000u64), "June 2024"),
        (Some(20_500_000u64), "July 2024"), 
        (Some(21_000_000u64), "September 2024"),
        (Some(21_500_000u64), "November 2024"),
        (Some(22_000_000u64), "January 2025"),
        (None, "Current"),
    ];
    
    println!("{:<15} {:<15} {:>20} {:>15}", "Block", "Date", "Total Supply", "Change");
    println!("{}", "-".repeat(70));
    
    let mut previous_supply: Option<f64> = None;
    let mut supplies = Vec::new();
    
    for (block, label) in checkpoints {
        match provider.get_token_total_supply(usdc, block).await {
            Ok(supply) => {
                let formatted = format_units(supply, usdc_decimals)?;
                let supply_float: f64 = formatted.parse().unwrap_or(0.0);
                
                let change_str = if let Some(prev) = previous_supply {
                    let change = supply_float - prev;
                    let change_pct = (change / prev) * 100.0;
                    if change >= 0.0 {
                        format!("+${:.2}M ({:+.2}%)", change / 1_000_000.0, change_pct)
                    } else {
                        format!("-${:.2}M ({:.2}%)", change.abs() / 1_000_000.0, change_pct)
                    }
                } else {
                    String::from("-")
                };
                
                println!("{:<15} {:<15} ${:>19.2} {:>15}", 
                    block.map(|b| b.to_string()).unwrap_or("Latest".to_string()),
                    label,
                    supply_float,
                    change_str
                );
                
                supplies.push((label, supply_float));
                previous_supply = Some(supply_float);
            }
            Err(e) => {
                println!("{:<15} {:<15} {:>20} Error: {}", 
                    block.map(|b| b.to_string()).unwrap_or("Latest".to_string()),
                    label,
                    "N/A",
                    e
                );
            }
        }
    }
    
    // Analysis
    if supplies.len() >= 2 {
        println!("\n=== Supply Change Analysis ===\n");
        
        let first = supplies.first().unwrap();
        let last = supplies.last().unwrap();
        
        let total_change = last.1 - first.1;
        let total_change_pct = (total_change / first.1) * 100.0;
        
        println!("Period: {} to {}", first.0, last.0);
        println!("Starting Supply: ${:.2}", first.1);
        println!("Current Supply: ${:.2}", last.1);
        println!("Total Change: ${:.2} ({:+.2}%)", total_change, total_change_pct);
        
        if total_change > 0.0 {
            println!("Status: 📈 Supply increased (minting/growth)");
        } else if total_change < 0.0 {
            println!("Status: 📉 Supply decreased (burning/redemption)");
        } else {
            println!("Status: ➡️ Supply unchanged");
        }
    }
    
    // Compare multiple tokens
    println!("\n=== Multi-Token Supply Comparison ===\n");
    
    let tokens = vec![
        ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6),
        ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7", 6),
        ("DAI", "6B175474E89094C44Da98b954EedeAC495271d0F", 18),
    ];
    
    // Get current supply for each
    println!("Current supplies:");
    for (symbol, addr_str, decimals) in tokens {
        let address = Address::from_str(addr_str)?;
        
        match provider.get_token_total_supply(address, None).await {
            Ok(supply) => {
                let formatted = format_units(supply, decimals)?;
                let supply_float: f64 = formatted.parse().unwrap_or(0.0);
                println!("  {}: ${:.2}B", symbol, supply_float / 1_000_000_000.0);
            }
            Err(e) => {
                println!("  {}: Error - {}", symbol, e);
            }
        }
    }
    
    println!("\n✅ Historical supply tracking complete!");
    
    Ok(())
}