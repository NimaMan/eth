/// Stablecoin Market Share Analysis
/// 
/// This example demonstrates a complete replacement for the existing RPC-based
/// stablecoin market share API endpoint. It combines token metadata queries,
/// total supply analysis, and whale holder tracking to provide comprehensive
/// stablecoin market analysis.
/// 
/// Algorithm:
/// 1. Query metadata (name, symbol, decimals) for all major stablecoins
/// 2. Get total supply for each stablecoin
/// 3. Calculate market caps and rankings
/// 4. Analyze whale holder concentrations for top stablecoins
/// 5. Provide detailed market share breakdown with performance metrics
/// 
/// This replaces the slow RPC-based implementation that makes 120+ API calls
/// with a single database session completing in milliseconds.

use alloy_primitives::{Address, U256};
use reth_chain_query::{ChainQuery, Result};
use std::str::FromStr;
use std::time::Instant;
use std::collections::HashMap;

/// Major stablecoins for comprehensive market analysis
const STABLECOINS: &[(&str, &str, &str)] = &[
    // Top tier stablecoins
    ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC", "USD Coin"),
    ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT", "Tether USD"),
    ("0x6B175474E89094C44Da98b954EedeAC495271d0F", "DAI", "MakerDAO DAI"),
    ("0x4Fabb145d64652a948d72533023f6E7A623C7C53", "BUSD", "Binance USD"),
    ("0x853d955aCEf822Db058eb8505911ED77F175b99e", "FRAX", "Frax"),
    
    // Second tier stablecoins
    ("0x99D8a9C45b2ecA8864373A26D1459e3Dff1e17F3", "MIM", "Magic Internet Money"),
    ("0x5f98805A4E8be255a32880FDeC7F6728C6568bA0", "LUSD", "Liquity USD"),
    ("0x03ab458634910AaD20eF5f1C8ee96F1D6ac54919", "RAI", "Rai Reflex Index"),
    ("0x8E870D67F660D95d5be530380D0eC0bd388289E1", "USDP", "Pax Dollar"),
    ("0x674C6511D85Af900e7f6B6FA6B87c2c13c5b1e01", "PYUSD", "PayPal USD"),
    
    // Algorithmic and newer stablecoins
    ("0x57Ab1ec28D129707052df4dF418D58a2D46d5f51", "sUSD", "Synthetix USD"),
    ("0x0E2EC54fC0B509F445631Bf4b91AB8168230C752", "LinkUSD", "LinkUSD"),
    ("0xf939E0A03FB07F59A73314E73794Be0E57ac1b4E", "crvUSD", "Curve USD"),
    ("0x596834457497E6bF9C72b3C6C77fE6F8eFcC0c74", "sUSDS", "sUSDS"),
    ("0xBc6DA0FE9aD5f3b0d58160288917AA56653660E9", "alUSD", "Alchemix USD"),
    ("0x1a7e4e63778B4f12a199C062f3eFdD288afCBce8", "EURA", "EURA"),
    ("0x2A3bFF78B79A009976EeA096a51A948a3dC00e34", "WUSDM", "WUSDM"),
    ("0xfB782396C9D43Ddf256e73EecA51FdE40e27E291", "GHO", "GHO"),
    ("0x35d084f5822B6ACa0e58F41FA8ad8C8E5a39DF9b", "dUSD", "dUSD"),
    ("0x7122985656e38BDC0302Db86685bb972b145bD3C", "STONE", "STONE"),
];

/// Major whale addresses for concentration analysis
const WHALE_ADDRESSES: &[(&str, &str)] = &[
    ("0x28C6c06298d514Db089934071355E5743bf21d60", "Binance Hot Wallet"),
    ("0x21a31Ee1afC51d94C2eFcCAa2092aD1028285549", "Binance Cold Wallet"),
    ("0xF977814e90dA44bFA03b6295A0616a897441aceC", "Binance US"),
    ("0x8EB8a3b98659Cce290402893d0123abb75E3ab28", "Avalanche Bridge"),
    ("0x40ec5B33f54e0E8A33A975908C5BA1c14e5BbbDf", "Polygon Bridge"),
    ("0xA0c68C638235ee32657e8f720a23ceC1bFc77C77", "Polygon Bridge 2"),
    ("0x3041CbD36888bECc7bbCBc0045E3B1f144466f5f", "Circle Multi-Sig"),
];

#[derive(Debug)]
struct StablecoinInfo {
    address: Address,
    symbol: String,
    name: String,
    decimals: u8,
    total_supply: U256,
    total_supply_formatted: f64,
    market_share_percent: f64,
    whale_concentration: WhaleConcentration,
}

#[derive(Debug)]
struct WhaleConcentration {
    total_whale_balance: U256,
    total_whale_balance_formatted: f64,
    concentration_percent: f64,
    top_holders: Vec<(String, f64)>,
}

impl Default for WhaleConcentration {
    fn default() -> Self {
        Self {
            total_whale_balance: U256::ZERO,
            total_whale_balance_formatted: 0.0,
            concentration_percent: 0.0,
            top_holders: Vec::new(),
        }
    }
}

/// Format token amount with decimals
fn format_token_amount(amount: U256, decimals: u8) -> f64 {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    // Convert to f64 for display
    let whole_f64 = whole.to_string().parse::<f64>().unwrap_or(0.0);
    let fraction_f64 = fraction.to_string().parse::<f64>().unwrap_or(0.0) / 10_f64.powi(decimals as i32);
    
    whole_f64 + fraction_f64
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("💰 Comprehensive Stablecoin Market Share Analysis");
    println!("{}", "=".repeat(70));
    println!("🔄 Direct replacement for RPC-based stablecoin market share API");
    println!("📊 Analyzing {} stablecoins with whale concentration data", STABLECOINS.len());
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    let latest_block = chain_query.get_latest_block()?;
    
    println!("🔗 Analysis Block: {}", latest_block);
    println!();
    
    let overall_start = Instant::now();
    let mut stablecoin_data = Vec::new();
    let mut total_market_supply = 0.0;
    let mut query_count = 0;
    
    println!("📈 Gathering Stablecoin Data...");
    
    for (address_str, expected_symbol, expected_name) in STABLECOINS {
        let token_start = Instant::now();
        let address = Address::from_str(address_str)?;
        
        println!("   🪙 Analyzing {} ({})...", expected_symbol, expected_name);
        
        // Get token metadata
        let symbol = chain_query.token.get_erc20_symbol(address, Some(latest_block)).await.unwrap_or_else(|_| expected_symbol.to_string());
        let name = chain_query.token.get_erc20_name(address, Some(latest_block)).await.unwrap_or_else(|_| expected_name.to_string());
        let decimals = chain_query.token.get_erc20_decimals(address, Some(latest_block)).await.unwrap_or(18);
        let total_supply = chain_query.token.get_erc20_total_supply(address, Some(latest_block)).await.unwrap_or(U256::ZERO);
        
        query_count += 4; // symbol, name, decimals, total_supply
        
        let total_supply_formatted = format_token_amount(total_supply, decimals);
        total_market_supply += total_supply_formatted;
        
        // Analyze whale concentration for top stablecoins
        let mut whale_concentration = WhaleConcentration::default();
        
        if total_supply > U256::ZERO && ["USDC", "USDT", "DAI", "BUSD", "FRAX"].contains(&expected_symbol) {
            let whale_start = Instant::now();
            let mut whale_balances = Vec::new();
            
            for (whale_address_str, whale_name) in WHALE_ADDRESSES {
                let whale_address = Address::from_str(whale_address_str)?;
                let balance = chain_query.token.get_erc20_balance(address, whale_address, Some(latest_block)).await.unwrap_or(U256::ZERO);
                let balance_formatted = format_token_amount(balance, decimals);
                
                if balance_formatted > 0.0 {
                    whale_balances.push((whale_name.to_string(), balance_formatted));
                    whale_concentration.total_whale_balance += balance;
                }
                
                query_count += 1;
            }
            
            whale_concentration.total_whale_balance_formatted = format_token_amount(whale_concentration.total_whale_balance, decimals);
            whale_concentration.concentration_percent = (whale_concentration.total_whale_balance_formatted / total_supply_formatted) * 100.0;
            
            // Sort whale balances by amount (descending)
            whale_balances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            whale_concentration.top_holders = whale_balances.into_iter().take(3).collect();
            
            let whale_time = whale_start.elapsed();
            println!("      🐋 Whale analysis: {:.1}ms", whale_time.as_millis());
        }
        
        let token_time = token_start.elapsed();
        
        stablecoin_data.push(StablecoinInfo {
            address,
            symbol: symbol.clone(),
            name: name.clone(),
            decimals,
            total_supply,
            total_supply_formatted,
            market_share_percent: 0.0, // Will calculate after we have total market
            whale_concentration,
        });
        
        println!("      ⚡ Total time: {:.1}ms | Supply: {:.0} {}", token_time.as_millis(), total_supply_formatted, symbol);
    }
    
    // Calculate market share percentages
    for stablecoin in &mut stablecoin_data {
        stablecoin.market_share_percent = (stablecoin.total_supply_formatted / total_market_supply) * 100.0;
    }
    
    // Sort by market cap (descending)
    stablecoin_data.sort_by(|a, b| b.total_supply_formatted.partial_cmp(&a.total_supply_formatted).unwrap_or(std::cmp::Ordering::Equal));
    
    let total_time = overall_start.elapsed();
    
    // Display comprehensive results
    println!();
    println!("{}", "=".repeat(70));
    println!("📊 STABLECOIN MARKET SHARE ANALYSIS");
    println!("{}", "=".repeat(70));
    println!("💰 Total Market Supply: ${:.2}B", total_market_supply / 1_000_000_000.0);
    println!("🏆 Total Stablecoins Analyzed: {}", stablecoin_data.len());
    println!();
    
    println!("🥇 TOP 10 STABLECOINS BY MARKET SHARE:");
    println!("{}", "-".repeat(70));
    
    for (rank, stablecoin) in stablecoin_data.iter().enumerate().take(10) {
        println!("{}. {} ({}) - {:.1}%", 
                rank + 1, 
                stablecoin.symbol, 
                stablecoin.name,
                stablecoin.market_share_percent);
        println!("   💰 Supply: ${:.0} ({} tokens)", 
                stablecoin.total_supply_formatted,
                stablecoin.symbol);
        
        if stablecoin.whale_concentration.concentration_percent > 0.0 {
            println!("   🐋 Whale Concentration: {:.1}% (${:.0})", 
                    stablecoin.whale_concentration.concentration_percent,
                    stablecoin.whale_concentration.total_whale_balance_formatted);
            
            if !stablecoin.whale_concentration.top_holders.is_empty() {
                println!("   🏦 Top Holders:");
                for (holder_name, balance) in &stablecoin.whale_concentration.top_holders {
                    println!("      • {}: ${:.0}", holder_name, balance);
                }
            }
        }
        
        println!();
    }
    
    // Market concentration analysis
    let top_3_share: f64 = stablecoin_data.iter().take(3).map(|s| s.market_share_percent).sum();
    let top_5_share: f64 = stablecoin_data.iter().take(5).map(|s| s.market_share_percent).sum();
    
    println!("{}", "=".repeat(70));
    println!("📈 MARKET CONCENTRATION METRICS");
    println!("{}", "=".repeat(70));
    println!("🥇 Top 3 Market Share: {:.1}%", top_3_share);
    println!("🏆 Top 5 Market Share: {:.1}%", top_5_share);
    
    let market_health = if top_3_share > 90.0 {
        "🔴 Highly Concentrated"
    } else if top_3_share > 75.0 {
        "🟡 Moderately Concentrated"
    } else {
        "🟢 Well Distributed"
    };
    
    println!("🎯 Market Health: {}", market_health);
    
    // Performance summary
    println!();
    println!("{}", "=".repeat(70));
    println!("⚡ PERFORMANCE SUMMARY");
    println!("{}", "=".repeat(70));
    println!("Total Queries Executed: {}", query_count);
    println!("Total Analysis Time: {:.2}s", total_time.as_secs_f64());
    println!("Average Query Time: {:.2}ms", total_time.as_millis() as f64 / query_count as f64);
    
    // RPC comparison
    let estimated_rpc_time = query_count * 150; // ~150ms per RPC call (conservative)
    let speedup = estimated_rpc_time as f64 / total_time.as_millis() as f64;
    
    println!();
    println!("🔄 RPC API Replacement Comparison:");
    println!("Original RPC Implementation: ~{}ms ({} calls × 150ms)", estimated_rpc_time, query_count);
    println!("New Direct DB Implementation: {}ms", total_time.as_millis());
    println!("Performance Improvement: {:.0}x faster", speedup);
    
    let cost_savings = query_count as f64 * 0.001; // Assume $0.001 per RPC call
    println!("Cost Savings: ~${:.3} per analysis", cost_savings);
    
    println!();
    println!("✅ Stablecoin market share analysis complete!");
    println!("🎯 This implementation can replace the existing RPC-based API endpoint");
    println!("💡 Perfect for real-time dashboards and trading applications!");
    
    Ok(())
}