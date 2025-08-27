/// Ethereum Whale Analysis Example
/// 
/// Demonstrates querying ETH balances for known whale addresses
/// and analyzing their account information including contract vs EOA status.
/// This is useful for tracking large ETH movements and whale behavior.

use reth_chain_query::{ChainQuery, Result};
use alloy_primitives::{Address, U256};
use std::str::FromStr;
use std::time::{Duration, Instant};

/// Known Ethereum whale addresses and their descriptions
const ETH_WHALES: &[(&str, &str)] = &[
    // Exchanges (Cold Storage)
    ("Binance 1", "0x3f5CE5FBFe3E9af3971dD833D26bA9b5C936f0bE"),
    ("Binance 2", "0xD551234Ae421e3BCBA99A0Da6d736074f22192FF"),
    ("Binance 3", "0x564286362092D8e7936f0549571a803B203aAceD"),
    ("Coinbase 1", "0x71660c4005BA85c37ccec55d0C4493E66Fe775d3"),
    ("Coinbase 2", "0x503828976D22510aad0201ac7EC88293211D23Da"),
    ("Coinbase 3", "0xddfAbCdc4D8FdF6d5beaF154f18B778f892A0740"),
    ("Kraken 1", "0x2910543af39aba0cd09dbb2d50200b3e800a63d2"),
    ("Kraken 2", "0x0a869d79a7052c7f1b55a8ebabbea3420f0d1e13"),
    ("OKEx 1", "0x6cC5F688a315f3dC28A7781717a9A798a59fDA7b"),
    ("OKEx 2", "0x236f9F97e0E62388479bf9E5BA4889e46B0273C3"),
    
    // DeFi Protocols
    ("Uniswap V3", "0x1F98431c8aD98523631AE4a59f267346ea31F984"),
    ("Uniswap V2", "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
    ("Compound", "0x3d9819210a31b4961b30ef54be2aed79b9c9cd3b"),
    ("Aave V3", "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2"),
    ("MakerDAO", "0x9f8f72aa9304c8b593d555f12ef6589cc3a579a2"),
    ("Curve", "0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7"),
    
    // MEV/Arbitrage Bots
    ("MEV Bot 1", "0x000000000dfde7deaf24138722987c9a6991e2d4"),
    ("MEV Bot 2", "0x6b75d8AF000000e20B7a7DDf000Ba900b4009A80"),
    ("Searcher 1", "0x0000000000007F150Bd6f54c40A34d7C3d5e9f56"),
    
    // Bridge Contracts
    ("Arbitrum Bridge", "0x8315177aB297bA92A06054cE80a67Ed4DBd7ed3a"),
    ("Polygon Bridge", "0x40ec5B33f54e0E8A33A975908C5BA1c14e5BbbDf"),
    ("Optimism Bridge", "0x99C9fc46f92E8a1c0deC1b1747d010903E884bE1"),
    
    // Layer 2 Sequencers  
    ("Arbitrum Sequencer", "0x1c479675ad559DC151F6Ec7ed3FbF8ceE79582B6"),
    ("Optimism Sequencer", "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985"),
    
    // Known Individual Whales (Vitalik, etc.)
    ("Vitalik Buterin", "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
    ("Ethereum Foundation 1", "0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"),
    ("Ethereum Foundation 2", "0x4750C43867EF5F89869132eaCf9B13263Aab7093"),
    
    // Staking Contracts
    ("Lido", "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84"),
    ("Rocket Pool", "0xae78736Cd615f374D3085123A210448E74Fc6393"),
    ("Coinbase Staking", "0xa9D1e08C7793af67e9d92fe308d5697FB81d3E43"),
    
    // Large Holders  
    ("Wrapped ETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
    ("0x Relay", "0x0000000000001fF3684f28c67538d4D072C22734"),
    ("Large Holder 1", "0x220866B1A2219f40e72f5c628B65D54268cA3A9D"),
    ("Large Holder 2", "0x8103683202aa8DA10536036EDEf04CDd865C225E"),
];

#[derive(Debug, Clone)]
struct WhaleData {
    name: String,
    address: String,
    balance_wei: U256,
    balance_eth: f64,
    nonce: u64,
    is_contract: bool,
    query_time: Duration,
}

fn wei_to_eth(wei: U256) -> f64 {
    let wei_str = wei.to_string();
    let wei_f64: f64 = wei_str.parse().unwrap_or(0.0);
    wei_f64 / 1e18
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐋 Ethereum Whale Analysis");
    println!("{}", "=".repeat(80));
    println!("📊 Analyzing ETH balances for {} known whale addresses", ETH_WHALES.len());
    println!("💰 Including exchanges, DeFi protocols, MEV bots, and large holders");
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    let overall_start = Instant::now();
    let mut whale_data: Vec<WhaleData> = Vec::new();
    let mut total_eth_tracked: f64 = 0.0;
    let mut successful_queries = 0;
    let mut failed_queries = 0;
    
    println!("⏱️  Querying whale addresses...\n");
    
    for (name, address_str) in ETH_WHALES {
        let start_time = Instant::now();
        
        match Address::from_str(&address_str[2..]) {
            Ok(address) => {
                print!("🔍 {}: ", name);
                
                // Get complete account info (balance, nonce, code check)
                match chain_query.account.get_account_info(address, None).await {
                    Ok(info) => {
                        let query_duration = start_time.elapsed();
                        let balance_eth = wei_to_eth(info.balance);
                        total_eth_tracked += balance_eth;
                        
                        whale_data.push(WhaleData {
                            name: name.to_string(),
                            address: address_str.to_string(),
                            balance_wei: info.balance,
                            balance_eth,
                            nonce: info.nonce,
                            is_contract: info.has_code,
                            query_time: query_duration,
                        });
                        
                        successful_queries += 1;
                        
                        let account_type = if info.has_code { "Contract" } else { "EOA" };
                        let balance_str = if balance_eth > 1_000_000.0 {
                            format!("{:.2}M ETH", balance_eth / 1_000_000.0)
                        } else if balance_eth > 1_000.0 {
                            format!("{:.2}K ETH", balance_eth / 1_000.0)
                        } else {
                            format!("{:.2} ETH", balance_eth)
                        };
                        
                        println!("✅ {} {} | Nonce: {} | {} ({:.2}ms)", 
                            balance_str, account_type, info.nonce, 
                            if balance_eth > 10000.0 { "🐋" } else if balance_eth > 1000.0 { "🐟" } else { "🦐" },
                            query_duration.as_millis());
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
                println!("❌ {}: Invalid address format: {}", name, e);
            }
        }
    }
    
    let total_duration = overall_start.elapsed();
    
    println!("\n{}", "=".repeat(80));
    println!("📈 WHALE ANALYSIS COMPLETE");
    println!("{}", "=".repeat(80));
    
    // Performance metrics
    println!("⚡ PERFORMANCE METRICS:");
    println!("  Total Query Time: {:.2} seconds", total_duration.as_secs_f64());
    println!("  Successful Queries: {}/{}", successful_queries, ETH_WHALES.len());
    println!("  Failed Queries: {}", failed_queries);
    println!("  Average Query Time: {:.2}ms", total_duration.as_millis() as f64 / ETH_WHALES.len() as f64);
    
    if successful_queries > 0 {
        let avg_success_time: f64 = whale_data.iter()
            .map(|w| w.query_time.as_millis() as f64)
            .sum::<f64>() / successful_queries as f64;
        println!("  Average Successful Query: {:.2}ms", avg_success_time);
        
        // Compare with RPC performance
        let estimated_rpc_time = successful_queries as f64 * 3.0 * 150.0; // 3 calls × 150ms average
        let speedup = estimated_rpc_time / total_duration.as_millis() as f64;
        println!("  🚀 Estimated Speedup vs RPC: {:.1}x faster", speedup);
        println!("     (RPC estimate: {:.1}s vs Actual: {:.2}s)", estimated_rpc_time / 1000.0, total_duration.as_secs_f64());
    }
    
    println!();
    
    // Whale analysis
    if !whale_data.is_empty() {
        println!("🐋 TOP ETH HOLDINGS:");
        
        // Sort by balance
        let mut sorted_whales = whale_data.clone();
        sorted_whales.sort_by(|a, b| b.balance_eth.partial_cmp(&a.balance_eth).unwrap());
        
        let total_eth_millions = total_eth_tracked / 1_000_000.0;
        println!("  Total ETH Tracked: {:.2}M ETH", total_eth_millions);
        println!();
        
        for (i, whale) in sorted_whales.iter().take(15).enumerate() {
            let percentage = (whale.balance_eth / total_eth_tracked) * 100.0;
            let balance_str = if whale.balance_eth > 1_000_000.0 {
                format!("{:.2}M ETH", whale.balance_eth / 1_000_000.0)
            } else if whale.balance_eth > 1_000.0 {
                format!("{:.2}K ETH", whale.balance_eth / 1_000.0)
            } else {
                format!("{:.1} ETH", whale.balance_eth)
            };
            
            let whale_emoji = if whale.balance_eth > 1_000_000.0 { "🐋" } 
                            else if whale.balance_eth > 100_000.0 { "🐟" } 
                            else { "🦐" };
            
            let account_type = if whale.is_contract { "Contract" } else { "EOA" };
            
            println!("  {}. {} {} ({}):", i + 1, whale_emoji, whale.name, account_type);
            println!("     {} ({:.1}% of tracked)", balance_str, percentage);
        }
        
        println!();
    }
    
    // Contract vs EOA analysis
    if !whale_data.is_empty() {
        let contracts = whale_data.iter().filter(|w| w.is_contract).count();
        let eoas = whale_data.iter().filter(|w| !w.is_contract).count();
        
        println!("🏗️  ACCOUNT TYPE DISTRIBUTION:");
        println!("  Contracts: {}/{} ({:.1}%)", contracts, whale_data.len(), contracts as f64 / whale_data.len() as f64 * 100.0);
        println!("  EOAs: {}/{} ({:.1}%)", eoas, whale_data.len(), eoas as f64 / whale_data.len() as f64 * 100.0);
        
        // ETH distribution by account type
        let contract_eth: f64 = whale_data.iter().filter(|w| w.is_contract).map(|w| w.balance_eth).sum();
        let eoa_eth: f64 = whale_data.iter().filter(|w| !w.is_contract).map(|w| w.balance_eth).sum();
        
        println!("  Contract ETH: {:.2}M ({:.1}%)", contract_eth / 1_000_000.0, contract_eth / total_eth_tracked * 100.0);
        println!("  EOA ETH: {:.2}M ({:.1}%)", eoa_eth / 1_000_000.0, eoa_eth / total_eth_tracked * 100.0);
    }
    
    // Query performance distribution
    if !whale_data.is_empty() {
        println!("\n⏱️  QUERY TIME DISTRIBUTION:");
        let query_times: Vec<u64> = whale_data.iter().map(|w| w.query_time.as_millis() as u64).collect();
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
    println!("  • Fast account queries enable real-time whale tracking");
    println!("  • Contract vs EOA classification helps identify address types");  
    println!("  • Nonce information shows transaction activity levels");
    println!("  • This data supports whale movement alerts and market analysis");
    
    println!("\n✅ Whale analysis complete!");
    
    Ok(())
}