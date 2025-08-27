/// Stablecoin Analysis Example
/// 
/// Demonstrates how to use the new entities module for comprehensive stablecoin analysis.
/// This example showcases:
/// - Market share calculations using StablecoinMarketAnalyzer
/// - Supply tracking and velocity analysis
/// - Top stablecoins by market cap
/// - Supply changes between blocks

use reth_chain_query::{ChainQuery, Result};
use reth_chain_query::entities::stablecoins::{
    StablecoinMarketAnalyzer, StablecoinSupplyTracker,
    STABLECOINS, get_stablecoin_by_symbol,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🪙 Stablecoin Analysis using Entities Module");
    println!("{}", "=".repeat(70));
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = Arc::new(ChainQuery::new(reth_datadir)?);
    
    // Create analyzers
    let market_analyzer = StablecoinMarketAnalyzer::new(chain_query.clone());
    let supply_tracker = StablecoinSupplyTracker::new(chain_query.clone());
    
    // 1. Market Share Analysis
    println!("\n📊 STABLECOIN MARKET SHARE ANALYSIS");
    println!("{}", "-".repeat(70));
    
    let market_analysis = market_analyzer.analyze_market(None).await?;
    
    println!("Block: {}", market_analysis.block_number);
    println!("Total Market Supply: ${:.2}B", market_analysis.total_market_supply / 1_000_000_000.0);
    println!("\nTop 10 Stablecoins by Market Share:");
    
    for (i, stablecoin) in market_analysis.stablecoins.iter().take(10).enumerate() {
        println!("{}. {} - {:.2}% (${:.2}B)", 
            i + 1,
            stablecoin.info.symbol,
            stablecoin.market_share_percent,
            stablecoin.total_supply_formatted / 1_000_000_000.0
        );
    }
    
    // Market concentration metrics
    println!("\n📈 Market Concentration:");
    println!("Top 3 Control: {:.1}%", market_analysis.top_3_concentration);
    println!("Top 5 Control: {:.1}%", market_analysis.top_5_concentration);
    println!("HHI Index: {:.0}", market_analysis.herfindahl_index);
    
    let concentration_status = if market_analysis.top_3_concentration > 80.0 {
        "🔴 Highly Concentrated"
    } else if market_analysis.top_3_concentration > 60.0 {
        "🟡 Moderately Concentrated"
    } else {
        "🟢 Well Distributed"
    };
    println!("Status: {}", concentration_status);
    
    // 2. Individual Stablecoin Details
    println!("\n💵 INDIVIDUAL STABLECOIN DETAILS");
    println!("{}", "-".repeat(70));
    
    for symbol in ["USDC", "USDT", "DAI"] {
        if let Some(info) = get_stablecoin_by_symbol(symbol) {
            println!("\n{} ({}):", info.symbol, info.unit);
            println!("  Address: {:?}", info.address);
            println!("  Decimals: {}", info.decimals);
            
            if let Some((supply, formatted)) = supply_tracker
                .get_stablecoin_supply(info.address, None).await? 
            {
                println!("  Total Supply: ${:.2}B", formatted / 1_000_000_000.0);
                println!("  Raw Supply: {}", supply);
            }
        }
    }
    
    // 3. Supply Changes Analysis
    println!("\n📉 SUPPLY CHANGES (Last 100 blocks)");
    println!("{}", "-".repeat(70));
    
    let latest_block = chain_query.get_latest_block()?;
    let from_block = latest_block - 100;
    
    let supply_changes = supply_tracker
        .calculate_supply_changes(from_block, latest_block)
        .await?;
    
    if supply_changes.is_empty() {
        println!("No significant supply changes in the last 100 blocks");
    } else {
        println!("Detected {} supply changes:", supply_changes.len());
        for change in supply_changes.iter().take(5) {
            let action = if change.is_mint {
                "🟢 MINT"
            } else if change.is_burn {
                "🔴 BURN"
            } else {
                "🔄 CHANGE"
            };
            
            println!("  {} {}: {:+.2}% ({:+} tokens)",
                action,
                change.stablecoin.symbol,
                change.percent_change,
                change.change_amount
            );
        }
    }
    
    // 4. Quick Stats
    println!("\n📊 QUICK STATS");
    println!("{}", "-".repeat(70));
    
    println!("Total Stablecoins Tracked: {}", STABLECOINS.len());
    
    let top_5 = market_analyzer.get_top_stablecoins(5, None).await?;
    let top_5_symbols: Vec<&str> = top_5.iter().map(|s| s.info.symbol).collect();
    println!("Top 5 by Market Cap: {}", top_5_symbols.join(", "));
    
    let is_concentrated = market_analyzer.is_market_concentrated(None).await?;
    println!("Market Concentrated: {}", if is_concentrated { "Yes" } else { "No" });
    
    // 5. Supply Velocity Example (for USDC)
    if let Some(usdc) = get_stablecoin_by_symbol("USDC") {
        println!("\n⚡ USDC SUPPLY VELOCITY (Last 10 samples)");
        println!("{}", "-".repeat(70));
        
        let velocities = supply_tracker
            .calculate_supply_velocity(
                usdc.address,
                latest_block - 1000,
                latest_block,
                100, // Sample every 100 blocks
            )
            .await?;
        
        if !velocities.is_empty() {
            let avg_velocity: f64 = velocities.iter().sum::<f64>() / velocities.len() as f64;
            println!("Average velocity: {:.2} tokens/block", avg_velocity);
            println!("Max velocity: {:.2} tokens/block", 
                velocities.iter().fold(f64::MIN, |a, &b| a.max(b)));
            println!("Min velocity: {:.2} tokens/block",
                velocities.iter().fold(f64::MAX, |a, &b| a.min(b)));
        }
    }
    
    println!("\n✅ Stablecoin analysis complete!");
    
    Ok(())
}