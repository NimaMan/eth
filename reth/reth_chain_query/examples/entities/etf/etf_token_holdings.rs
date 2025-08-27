/// ETF Token Holdings Analysis Example
/// 
/// Demonstrates querying token information for addresses associated with 
/// cryptocurrency ETFs (BlackRock, Grayscale, VanEck, etc.) to analyze
/// institutional holdings and token metadata.
/// 
/// This example shows how to efficiently query token data for ETF-related
/// addresses, which is useful for institutional flow analysis.

use reth_chain_query::{ChainQuery, Result};
use alloy_primitives::Address;
use std::str::FromStr;
use std::time::{Duration, Instant};

/// Major cryptocurrency ETF providers and their known token addresses
const ETF_TOKENS: &[(&str, &str, &str)] = &[
    // BlackRock (iShares Bitcoin Trust - IBIT)
    ("BlackRock BTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC"),
    ("BlackRock ETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "Wrapped Ether"),
    
    // Grayscale Bitcoin Trust (GBTC)
    ("Grayscale BTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC"),
    ("Grayscale ETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "Wrapped Ether"),
    ("Grayscale LTC", "0x57Ad67aCf9bF015E4820Fbd66EA1A21BED8852eC", "Wrapped LTC"),
    
    // VanEck (VanEck Bitcoin Trust)
    ("VanEck BTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC"),
    
    // Bitwise (Bitwise Bitcoin ETF)
    ("Bitwise BTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC"),
    
    // Fidelity (Fidelity Wise Origin Bitcoin Fund)
    ("Fidelity BTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC"),
    ("Fidelity ETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "Wrapped Ether"),
    
    // ARK Invest (ARK 21Shares Bitcoin ETF)
    ("ARK BTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC"),
    
    // Invesco (Invesco Galaxy Bitcoin ETF)
    ("Invesco BTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC"),
    
    // Franklin Templeton (Franklin Bitcoin ETF)
    ("Franklin BTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC"),
    
    // Additional tokens that ETFs might hold
    ("Stablecoin USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USD Coin"),
    ("Stablecoin USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7", "Tether USD"),
    ("DeFi UNI", "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984", "Uniswap"),
    ("DeFi AAVE", "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9", "Aave Token"),
    ("DeFi COMP", "0xc00e94Cb662C3520282E6f5717214004A7f26888", "Compound"),
    ("DeFi MKR", "0x9f8F72aA9304c8B593d555F12eF6589cC3A579A2", "Maker"),
    ("DeFi CRV", "0xD533a949740bb3306d119CC777fa900bA034cd52", "Curve DAO Token"),
    ("Layer2 LRC", "0xBBbbCA6A901c926F240b89EacB641d8Aec7AEafD", "Loopring"),
    ("Layer2 MATIC", "0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0", "Matic Token"),
    ("Oracle LINK", "0x514910771AF9Ca656af840dff83E8264EcF986CA", "ChainLink Token"),
];

/// Known ETF custody addresses (examples - these would be discovered through analysis)
const ETF_CUSTODY_ADDRESSES: &[(&str, &str)] = &[
    ("BlackRock Custody", "0x0000000000000000000000000000000000000000"), // Placeholder
    ("Grayscale Custody", "0x0000000000000000000000000000000000000001"), // Placeholder  
    ("VanEck Custody", "0x0000000000000000000000000000000000000002"), // Placeholder
    ("Bitwise Custody", "0x0000000000000000000000000000000000000003"), // Placeholder
    ("Fidelity Custody", "0x0000000000000000000000000000000000000004"), // Placeholder
];

#[derive(Debug)]
struct TokenMetadata {
    provider: String,
    symbol: String,
    name: String,
    decimals: u8,
    total_supply: String,
    total_supply_formatted: f64,
    query_time: Duration,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏦 ETF Token Holdings Analysis");
    println!("{}", "=".repeat(80));
    println!("📊 Analyzing {} tokens held by cryptocurrency ETFs", ETF_TOKENS.len());
    println!("🏢 Major ETF providers: BlackRock, Grayscale, VanEck, Bitwise, Fidelity, ARK, Invesco");
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    let overall_start = Instant::now();
    let mut token_metadata: Vec<TokenMetadata> = Vec::new();
    let mut successful_queries = 0;
    let mut failed_queries = 0;
    
    println!("⏱️  Querying ETF-related tokens...\n");
    
    // Query token metadata for all ETF-related tokens
    for (provider, address_str, expected_name) in ETF_TOKENS {
        let start_time = Instant::now();
        
        match Address::from_str(&address_str[2..]) {
            Ok(address) => {
                print!("🔍 {} ({}): ", provider, expected_name);
                
                match chain_query.token.get_erc20_info(address, None).await {
                    Ok(info) => {
                        let query_duration = start_time.elapsed();
                        let supply_formatted = info.total_supply.to_string().parse::<f64>()
                            .unwrap_or(0.0) / 10_f64.powi(info.decimals as i32);
                        
                        token_metadata.push(TokenMetadata {
                            provider: provider.to_string(),
                            symbol: info.symbol.clone(),
                            name: info.name.clone(),
                            decimals: info.decimals,
                            total_supply: info.total_supply.to_string(),
                            total_supply_formatted: supply_formatted,
                            query_time: query_duration,
                        });
                        
                        successful_queries += 1;
                        
                        // Format supply based on token type
                        let supply_str = if supply_formatted > 1_000_000_000.0 {
                            format!("{:.2}B", supply_formatted / 1_000_000_000.0)
                        } else if supply_formatted > 1_000_000.0 {
                            format!("{:.2}M", supply_formatted / 1_000_000.0)
                        } else {
                            format!("{:.2}", supply_formatted)
                        };
                        
                        println!("✅ {} - Supply: {} ({:.2}ms)", info.symbol, supply_str, query_duration.as_millis());
                    }
                    Err(e) => {
                        let query_duration = start_time.elapsed();
                        failed_queries += 1;
                        println!("❌ Error: {} ({:.2}ms)", e.to_string().chars().take(40).collect::<String>(), query_duration.as_millis());
                    }
                }
            }
            Err(e) => {
                failed_queries += 1;
                println!("❌ {}: Invalid address format: {}", provider, e);
            }
        }
    }
    
    let total_duration = overall_start.elapsed();
    
    println!("\n{}", "=".repeat(80));
    println!("📈 ETF TOKEN ANALYSIS COMPLETE");
    println!("{}", "=".repeat(80));
    
    // Performance metrics
    println!("⚡ PERFORMANCE METRICS:");
    println!("  Total Query Time: {:.2} seconds", total_duration.as_secs_f64());
    println!("  Successful Queries: {}/{}", successful_queries, ETF_TOKENS.len());
    println!("  Failed Queries: {}", failed_queries);
    println!("  Average Query Time: {:.2}ms", total_duration.as_millis() as f64 / ETF_TOKENS.len() as f64);
    
    if successful_queries > 0 {
        let avg_success_time: f64 = token_metadata.iter()
            .map(|t| t.query_time.as_millis() as f64)
            .sum::<f64>() / successful_queries as f64;
        println!("  Average Successful Query: {:.2}ms", avg_success_time);
        
        // Compare with RPC performance
        let estimated_rpc_time = successful_queries as f64 * 4.0 * 250.0; // 4 calls × 250ms average
        let speedup = estimated_rpc_time / total_duration.as_millis() as f64;
        println!("  🚀 Estimated Speedup vs RPC: {:.1}x faster", speedup);
        println!("     (RPC estimate: {:.1}s vs Actual: {:.2}s)", estimated_rpc_time / 1000.0, total_duration.as_secs_f64());
    }
    
    println!();
    
    // Group tokens by category
    if !token_metadata.is_empty() {
        println!("📊 TOKEN CATEGORIES:");
        
        let mut bitcoin_tokens = Vec::new();
        let mut ethereum_tokens = Vec::new();
        let mut stablecoin_tokens = Vec::new();
        let mut defi_tokens = Vec::new();
        let mut other_tokens = Vec::new();
        
        for token in &token_metadata {
            if token.symbol.contains("BTC") || token.symbol == "WBTC" {
                bitcoin_tokens.push(token);
            } else if token.symbol.contains("ETH") || token.symbol == "WETH" {
                ethereum_tokens.push(token);
            } else if token.symbol.contains("USD") || token.symbol == "DAI" {
                stablecoin_tokens.push(token);
            } else if ["UNI", "AAVE", "COMP", "MKR", "CRV"].contains(&token.symbol.as_str()) {
                defi_tokens.push(token);
            } else {
                other_tokens.push(token);
            }
        }
        
        if !bitcoin_tokens.is_empty() {
            println!("  🟠 Bitcoin-related tokens ({}): {}", bitcoin_tokens.len(),
                bitcoin_tokens.iter().map(|t| t.symbol.as_str()).collect::<Vec<_>>().join(", "));
        }
        
        if !ethereum_tokens.is_empty() {
            println!("  🔵 Ethereum-related tokens ({}): {}", ethereum_tokens.len(),
                ethereum_tokens.iter().map(|t| t.symbol.as_str()).collect::<Vec<_>>().join(", "));
        }
        
        if !stablecoin_tokens.is_empty() {
            println!("  🟢 Stablecoin tokens ({}): {}", stablecoin_tokens.len(),
                stablecoin_tokens.iter().map(|t| t.symbol.as_str()).collect::<Vec<_>>().join(", "));
        }
        
        if !defi_tokens.is_empty() {
            println!("  🟣 DeFi tokens ({}): {}", defi_tokens.len(),
                defi_tokens.iter().map(|t| t.symbol.as_str()).collect::<Vec<_>>().join(", "));
        }
        
        if !other_tokens.is_empty() {
            println!("  ⚪ Other tokens ({}): {}", other_tokens.len(),
                other_tokens.iter().map(|t| t.symbol.as_str()).collect::<Vec<_>>().join(", "));
        }
    }
    
    // Performance distribution analysis
    if !token_metadata.is_empty() {
        println!("\n⏱️  QUERY TIME DISTRIBUTION:");
        let query_times: Vec<u64> = token_metadata.iter().map(|t| t.query_time.as_millis() as u64).collect();
        let min_time = query_times.iter().min().unwrap();
        let max_time = query_times.iter().max().unwrap();
        let median_time = {
            let mut sorted = query_times.clone();
            sorted.sort();
            sorted[sorted.len() / 2]
        };
        
        println!("  Fastest Query: {}ms", min_time);
        println!("  Slowest Query: {}ms", max_time);  
        println!("  Median Query: {}ms", median_time);
        
        // Show distribution
        let under_5ms = query_times.iter().filter(|&&t| t < 5).count();
        let under_10ms = query_times.iter().filter(|&&t| t < 10).count();
        let under_25ms = query_times.iter().filter(|&&t| t < 25).count();
        
        println!("  Under 5ms: {}/{} ({:.1}%)", under_5ms, query_times.len(), under_5ms as f64 / query_times.len() as f64 * 100.0);
        println!("  Under 10ms: {}/{} ({:.1}%)", under_10ms, query_times.len(), under_10ms as f64 / query_times.len() as f64 * 100.0);
        println!("  Under 25ms: {}/{} ({:.1}%)", under_25ms, query_times.len(), under_25ms as f64 / query_times.len() as f64 * 100.0);
    }
    
    println!("\n💡 INSIGHTS:");
    println!("  • This data can be used to analyze institutional exposure to different crypto assets");
    println!("  • Token metadata helps identify which assets ETFs are likely to hold");  
    println!("  • Fast query times enable real-time monitoring of ETF-relevant tokens");
    println!("  • Next step: Query actual custody addresses to see real holdings");
    
    println!("\n✅ ETF token analysis complete!");
    
    Ok(())
}