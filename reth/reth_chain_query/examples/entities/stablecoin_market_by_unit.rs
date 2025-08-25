/// Stablecoin Market Analysis by Currency Unit
/// 
/// Demonstrates market share analysis grouped by currency unit (USD, EUR, JPY, etc.)
/// Each unit shows totals in its own denomination without cross-currency conversion.
/// This provides accurate representation of each currency's stablecoin ecosystem.

use reth_chain_query::{ChainQuery, Result};
use reth_chain_query::entities::stablecoins::StablecoinMarketAnalyzer;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("💱 Stablecoin Market Analysis by Currency Unit");
    println!("{}", "=".repeat(70));
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = Arc::new(ChainQuery::new(reth_datadir)?);
    
    // Create market analyzer
    let analyzer = StablecoinMarketAnalyzer::new(chain_query.clone());
    
    // Get latest block
    let latest_block = chain_query.get_latest_block()?;
    println!("\n📊 Analyzing at block: {}", latest_block);
    println!("{}", "-".repeat(70));
    
    // Analyze market by currency unit
    let market_by_unit = analyzer.analyze_market_by_unit(Some(latest_block)).await?;
    
    // Display results for each currency unit
    println!("\n🌍 MARKET BY CURRENCY UNIT");
    println!("{}", "=".repeat(70));
    
    // Sort units by total supply for better display
    let mut units: Vec<_> = market_by_unit.units.iter().collect();
    units.sort_by(|a, b| {
        b.1.total_supply_in_unit
            .partial_cmp(&a.1.total_supply_in_unit)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    
    for (unit_name, unit_data) in units {
        // Skip units with no supply
        if unit_data.total_supply_in_unit == 0.0 {
            continue;
        }
        
        println!("\n💵 {} Market", unit_name);
        println!("{}", "-".repeat(50));
        
        // Format total based on currency
        let total_formatted = match unit_name.as_str() {
            "US Dollar" => format!("${:.2}", unit_data.total_supply_in_unit),
            "Euro" => format!("€{:.2}", unit_data.total_supply_in_unit),
            "Japanese Yen" => format!("¥{:.0}", unit_data.total_supply_in_unit),
            "British Pound" => format!("£{:.2}", unit_data.total_supply_in_unit),
            "Singapore Dollar" => format!("S${:.2}", unit_data.total_supply_in_unit),
            "Australian Dollar" => format!("A${:.2}", unit_data.total_supply_in_unit),
            "Canadian Dollar" => format!("C${:.2}", unit_data.total_supply_in_unit),
            "Chinese Yuan" => format!("¥{:.2}", unit_data.total_supply_in_unit),
            "Indonesian Rupiah" => format!("Rp {:.0}", unit_data.total_supply_in_unit),
            _ => format!("{:.2} {}", unit_data.total_supply_in_unit, unit_name),
        };
        
        println!("Total Supply: {}", total_formatted);
        println!("Active Tokens: {}", unit_data.tokens.len());
        
        // Show top tokens in this unit
        println!("\nTop Stablecoins:");
        for (i, token) in unit_data.tokens.iter().take(5).enumerate() {
            let rank = i + 1;
            println!("  {}. {} ({})", 
                rank,
                token.info.symbol,
                token.info.name
            );
            
            // Format amount based on currency
            let amount_formatted = match unit_name.as_str() {
                "US Dollar" => format!("${:.2}", token.total_supply_formatted),
                "Euro" => format!("€{:.2}", token.total_supply_formatted),
                "Japanese Yen" => format!("¥{:.0}", token.total_supply_formatted),
                "British Pound" => format!("£{:.2}", token.total_supply_formatted),
                "Singapore Dollar" => format!("S${:.2}", token.total_supply_formatted),
                "Australian Dollar" => format!("A${:.2}", token.total_supply_formatted),
                "Canadian Dollar" => format!("C${:.2}", token.total_supply_formatted),
                "Chinese Yuan" => format!("¥{:.2}", token.total_supply_formatted),
                "Indonesian Rupiah" => format!("Rp {:.0}", token.total_supply_formatted),
                _ => format!("{:.2}", token.total_supply_formatted),
            };
            
            println!("     Supply: {}", amount_formatted);
            println!("     Market Share (within {}): {:.2}%", unit_name, token.market_share_percent);
        }
        
        // Show concentration metrics for this unit
        if unit_data.tokens.len() >= 3 {
            let top_3_share: f64 = unit_data.tokens
                .iter()
                .take(3)
                .map(|t| t.market_share_percent)
                .sum();
            
            println!("\nConcentration Metrics:");
            println!("  Top 3 Control: {:.2}% of {} market", top_3_share, unit_name);
            
            if top_3_share > 80.0 {
                println!("  ⚠️  Highly concentrated market");
            } else if top_3_share > 60.0 {
                println!("  ⚡ Moderately concentrated market");
            } else {
                println!("  ✅ Competitive market");
            }
        }
    }
    
    // Summary statistics
    println!("\n📈 OVERALL SUMMARY");
    println!("{}", "=".repeat(70));
    
    let active_units = units.iter()
        .filter(|(_, data)| data.total_supply_in_unit > 0.0)
        .count();
    
    let total_tokens: usize = units.iter()
        .map(|(_, data)| data.tokens.len())
        .sum();
    
    println!("Active Currency Units: {}", active_units);
    println!("Total Active Stablecoins: {}", total_tokens);
    
    // Show unit diversity
    println!("\nUnit Diversity:");
    for (unit_name, unit_data) in &units {
        if unit_data.total_supply_in_unit > 0.0 {
            let token_count = unit_data.tokens.len();
            let emoji = if token_count > 5 { "🔥" } 
                else if token_count > 2 { "📊" }
                else { "🌱" };
            
            println!("  {} {}: {} token(s)", emoji, unit_name, token_count);
        }
    }
    
    // Dominant unit analysis
    if let Some((dominant_unit, dominant_data)) = units.first() {
        println!("\n🏆 Dominant Currency Unit: {}", dominant_unit);
        println!("  Has {} active stablecoins", dominant_data.tokens.len());
        
        if let Some(top_token) = dominant_data.tokens.first() {
            println!("  Led by: {} ({:.2}% of {} market)", 
                top_token.info.symbol,
                top_token.market_share_percent,
                dominant_unit
            );
        }
    }
    
    println!("\n✅ Market analysis by currency unit complete!");
    
    println!("\n💡 Key Insights:");
    println!("- Each currency unit's market is analyzed independently");
    println!("- Market shares are calculated WITHIN each unit (not across units)");
    println!("- No cross-currency conversion is performed");
    println!("- This provides accurate representation of each currency's ecosystem");
    
    Ok(())
}