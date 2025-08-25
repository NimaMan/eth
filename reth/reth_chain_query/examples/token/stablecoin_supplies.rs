/// Stablecoin Market Share Analysis Example
/// 
/// Demonstrates querying all major stablecoins to calculate market share
/// and total supply information. This example shows the performance improvement
/// over RPC calls for bulk token analysis.
/// 
/// Performance expectations:
/// - RPC method: ~30 tokens × 4 calls × 100-500ms = 12-60 seconds  
/// - reth_chain_query: ~30 tokens × 4 calls × 2-5ms = 240-600ms (20-100x faster)

use reth_chain_query::{ChainQuery, Result};
use alloy_primitives::Address;
use std::str::FromStr;
use std::time::{Duration, Instant};

/// Major stablecoins with their addresses and expected symbols
const STABLECOINS: &[(&str, &str, &str)] = &[
    // Major USD stablecoins
    ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USD Coin"),
    ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7", "Tether USD"),
    ("DAI", "0x6B175474E89094C44Da98b954EedeAC495271d0F", "Dai Stablecoin"),
    ("BUSD", "0x4Fabb145d64652a948d72533023f6E7A623C7C53", "Binance USD"),
    ("FRAX", "0x853d955aCEf822Db058eb8505911ED77F175b99e", "Frax"),
    ("TUSD", "0x0000000000085d4780B73119b644AE5ecd22b376", "TrueUSD"),
    ("USDP", "0x8E870D67F660D95d5be530380D0eC0bd388289E1", "Pax Dollar"),
    ("GUSD", "0x056Fd409E1d7A124BD7017459dFEa2F387b6d5Cd", "Gemini Dollar"),
    ("USDD", "0x0C10bF8FcB7Bf5412187A595ab97a3609160b5c6", "Decentralized USD"),
    ("LUSD", "0x5f98805A4E8be255a32880FDeC7F6728C6568bA0", "Liquity USD"),
    
    // Yield-bearing stablecoins
    ("sDAI", "0x83F20F44975D03b1b09e64809B757c47f942BEeA", "Savings Dai"),
    ("PYUSD", "0x6c3ea9036406852006290770BEdFcAbA0e23A0e8", "PayPal USD"),
    ("FDUSD", "0xc5f0f7b66764F6ec8C8Dff7BA683102295E16409", "First Digital USD"),
    
    // Euro stablecoins
    ("EURS", "0xdB25f211AB05b1c97D595516F45794528a807ad8", "STASIS EURS"),
    ("EURT", "0xC581b735A1688071A1746c968e0798D642EDE491", "Tether EURt"),
    ("AGEUR", "0x1a7e4e63778B4f12a199C062f3eFdD288afCBce8", "agEUR"),
    
    // Other fiat stablecoins  
    ("XSGD", "0x70e8dE73cE538DA2bEEd35d14187F6959a8ecA96", "XSGD"),
    ("GYEN", "0xC08512927D12348F6620a698105e1BAac6EcD911", "GYEN"),
    ("ZUSD", "0xc56c2b7e71B54d38Aab6d52E94a04Cb71C1c81AE", "Zero USD"),
    
    // Algorithmic/crypto-backed
    ("FRXETH", "0x5E8422345238F34275888049021821E8E08CAa1f", "Frax Ether"),
    ("alETH", "0x0100546F2cD4C9D97f798fFC9755E47865FF7Ee6", "Alchemix ETH"),
    ("alUSD", "0xBC6DA0FE9aD5f3b0d58160288917AA56653660E9", "Alchemix USD"),
    ("MIM", "0x99D8a9C45b2ecA8864373A26D1459e3Dff1e17F3", "Magic Internet Money"),
    ("DOLA", "0x865377367054516e17014CcdED1e7d814EDC9ce4", "Dola USD Stablecoin"),
    ("USDe", "0x4c9EDD5852cd905f086C759E8383e09bff1E68B3", "Ethena USDe"),
    
    // Deprecated/Legacy (for completeness)
    ("USDC.e", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USD Coin (Ethereum)"),
    ("UST", "0xa693B19d2931d498c5B318dF961919BB4aee87a5", "TerraUSD (Classic)"),
];

#[derive(Debug)]
struct StablecoinData {
    symbol: String,
    name: String,
    decimals: u8,
    total_supply_raw: String,
    total_supply_formatted: f64,
    query_time: Duration,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Stablecoin Market Share Analysis");
    println!("{}", "=".repeat(80));
    println!("📊 Analyzing {} stablecoins using reth_chain_query", STABLECOINS.len());
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    let overall_start = Instant::now();
    let mut stablecoin_data: Vec<StablecoinData> = Vec::new();
    let mut total_supply_usd: f64 = 0.0;
    let mut successful_queries = 0;
    let mut failed_queries = 0;
    
    println!("⏱️  Querying tokens...\n");
    
    for (expected_symbol, address_str, expected_name) in STABLECOINS {
        let start_time = Instant::now();
        
        match Address::from_str(&address_str[2..]) {
            Ok(address) => {
                print!("🔍 {}: ", expected_symbol);
                
                match chain_query.token.get_erc20_info(address, None).await {
                    Ok(info) => {
                        let query_duration = start_time.elapsed();
                        let supply_formatted = info.total_supply.to_string().parse::<f64>()
                            .unwrap_or(0.0) / 10_f64.powi(info.decimals as i32);
                        
                        // Only count USD stablecoins in total (skip EUR, GBP, etc.)
                        if expected_symbol.contains("USD") || *expected_symbol == "DAI" || 
                           *expected_symbol == "FRAX" || *expected_symbol == "USDC" || 
                           *expected_symbol == "USDT" || *expected_symbol == "BUSD" {
                            total_supply_usd += supply_formatted;
                        }
                        
                        stablecoin_data.push(StablecoinData {
                            symbol: info.symbol.clone(),
                            name: info.name.clone(),
                            decimals: info.decimals,
                            total_supply_raw: info.total_supply.to_string(),
                            total_supply_formatted: supply_formatted,
                            query_time: query_duration,
                        });
                        
                        successful_queries += 1;
                        println!("✅ {:.2}B {} ({:.2}ms)", supply_formatted / 1_000_000_000.0, info.symbol, query_duration.as_millis());
                    }
                    Err(e) => {
                        let query_duration = start_time.elapsed();
                        failed_queries += 1;
                        println!("❌ Error: {} ({:.2}ms)", e.to_string().chars().take(50).collect::<String>(), query_duration.as_millis());
                    }
                }
            }
            Err(e) => {
                failed_queries += 1;
                println!("❌ {}: Invalid address format: {}", expected_symbol, e);
            }
        }
    }
    
    let total_duration = overall_start.elapsed();
    
    println!("\n{}", "=".repeat(80));
    println!("📈 STABLECOIN MARKET ANALYSIS COMPLETE");
    println!("{}", "=".repeat(80));
    
    // Performance metrics
    println!("⚡ PERFORMANCE METRICS:");
    println!("  Total Query Time: {:.2} seconds", total_duration.as_secs_f64());
    println!("  Successful Queries: {}/{}", successful_queries, STABLECOINS.len());
    println!("  Failed Queries: {}", failed_queries);
    println!("  Average Query Time: {:.2}ms", total_duration.as_millis() as f64 / STABLECOINS.len() as f64);
    
    if successful_queries > 0 {
        let avg_success_time: f64 = stablecoin_data.iter()
            .map(|s| s.query_time.as_millis() as f64)
            .sum::<f64>() / successful_queries as f64;
        println!("  Average Successful Query: {:.2}ms", avg_success_time);
        
        // Compare with RPC performance
        let estimated_rpc_time = successful_queries as f64 * 4.0 * 300.0; // 4 calls × 300ms average
        let speedup = estimated_rpc_time / total_duration.as_millis() as f64;
        println!("  🚀 Estimated Speedup vs RPC: {:.1}x faster", speedup);
        println!("     (RPC estimate: {:.1}s vs Actual: {:.2}s)", estimated_rpc_time / 1000.0, total_duration.as_secs_f64());
    }
    
    println!();
    
    // Market share analysis
    if !stablecoin_data.is_empty() {
        println!("💰 USD STABLECOIN MARKET SHARE:");
        
        // Sort by total supply
        let mut usd_stablecoins: Vec<_> = stablecoin_data.iter()
            .filter(|s| s.symbol.contains("USD") || s.symbol == "DAI" || 
                       s.symbol == "FRAX" || s.symbol == "USDC" || 
                       s.symbol == "USDT" || s.symbol == "BUSD")
            .collect();
        usd_stablecoins.sort_by(|a, b| b.total_supply_formatted.partial_cmp(&a.total_supply_formatted).unwrap());
        
        println!("  Total USD Stablecoin Supply: ${:.2}B", total_supply_usd / 1_000_000_000.0);
        println!();
        
        for (i, coin) in usd_stablecoins.iter().take(10).enumerate() {
            let market_share = (coin.total_supply_formatted / total_supply_usd) * 100.0;
            let supply_billions = coin.total_supply_formatted / 1_000_000_000.0;
            
            println!("  {}. {} ({}):", i + 1, coin.symbol, coin.name);
            println!("     Supply: ${:.2}B ({:.1}% of market)", supply_billions, market_share);
        }
        
        println!();
    }
    
    // Query time distribution
    if !stablecoin_data.is_empty() {
        println!("⏱️  QUERY TIME DISTRIBUTION:");
        let query_times: Vec<u64> = stablecoin_data.iter().map(|s| s.query_time.as_millis() as u64).collect();
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
        let under_50ms = query_times.iter().filter(|&&t| t < 50).count();
        
        println!("  Under 5ms: {}/{} ({:.1}%)", under_5ms, query_times.len(), under_5ms as f64 / query_times.len() as f64 * 100.0);
        println!("  Under 10ms: {}/{} ({:.1}%)", under_10ms, query_times.len(), under_10ms as f64 / query_times.len() as f64 * 100.0);
        println!("  Under 50ms: {}/{} ({:.1}%)", under_50ms, query_times.len(), under_50ms as f64 / query_times.len() as f64 * 100.0);
    }
    
    println!("\n✅ Analysis complete! This data can replace RPC calls in stablecoin market share calculations.");
    
    Ok(())
}