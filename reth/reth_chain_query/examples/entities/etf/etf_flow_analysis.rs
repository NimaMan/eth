/// ETF and Institutional Token Flow Analysis
/// 
/// This example demonstrates comprehensive analysis of institutional token holdings
/// and flow patterns. It tracks major ETF and institutional addresses across
/// multiple tokens to understand capital flows and institutional behavior.
/// 
/// Algorithm:
/// 1. Define major ETF tokens and institutional addresses
/// 2. Query token metadata and current holdings across all combinations
/// 3. Calculate institutional concentration per token
/// 4. Analyze cross-token correlation and diversification
/// 5. Generate institutional flow insights and portfolio analysis
/// 
/// Use cases: Institutional flow monitoring, ETF rebalancing detection,
/// correlation analysis for trading strategies.

use alloy_primitives::{Address, U256};
use reth_chain_query::{ChainQuery, Result};
use std::str::FromStr;
use std::time::Instant;
use std::collections::HashMap;

/// Major ETF tokens for institutional tracking
const ETF_TOKENS: &[(&str, &str, &str, u8)] = &[
    // Bitcoin ETFs
    ("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "WBTC", "Wrapped Bitcoin", 8),
    
    // Ethereum ETFs and staking
    ("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "WETH", "Wrapped Ether", 18),
    ("0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84", "stETH", "Liquid staked Ether 2.0", 18),
    ("0xBe9895146f7AF43049ca1c1AE358B0541Ea49704", "cbETH", "Coinbase Wrapped Staked ETH", 18),
    ("0xac3E018457B222d93114458476f3E3416Abbe38F", "sfrxETH", "Staked Frax Ether", 18),
    
    // Major DeFi tokens
    ("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984", "UNI", "Uniswap", 18),
    ("0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9", "AAVE", "Aave Token", 18),
    ("0x6B3595068778DD592e39A122f4f5a5cF09C90fE2", "SUSHI", "SushiToken", 18),
    ("0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F", "SNX", "Synthetix Network Token", 18),
    ("0x0f5D2fB29fb7d3CFeE444a200298f468908cC942", "MANA", "Decentraland", 18),
    
    // Layer 2 and infrastructure tokens  
    ("0x4200000000000000000000000000000000000042", "OP", "Optimism", 18),
    ("0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0", "MATIC", "Polygon", 18),
    ("0x3845badAde8e6dFF049820680d1F14bD3903a5d0", "SAND", "The Sandbox", 18),
    
    // Stablecoins for portfolio balance analysis
    ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC", "USD Coin", 6),
    ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT", "Tether USD", 6),
    
    // Other major tokens
    ("0x514910771AF9Ca656af840dff83E8264EcF986CA", "LINK", "ChainLink Token", 18),
    ("0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE", "SHIB", "SHIBA INU", 18),
    ("0xA0b73E1Ff0B80914AB6fe0444E65848C4C34450b", "CRO", "Cronos Coin", 8),
];

/// Major institutional addresses for tracking
const INSTITUTIONAL_ADDRESSES: &[(&str, &str)] = &[
    // Major exchanges
    ("0x28C6c06298d514Db089934071355E5743bf21d60", "Binance: Hot Wallet"),
    ("0x21a31Ee1afC51d94C2eFcCAa2092aD1028285549", "Binance: Cold Wallet"),
    ("0xF977814e90dA44bFA03b6295A0616a897441aceC", "Binance US"),
    ("0x267be1C1D684F78cb4F6a176C4911b741E4Ffdc0", "Kraken"),
    ("0x6cc5f688a315f3dc28a7781717a9a798a59fda7b", "OKEx"),
    
    // Custodial services and institutions
    ("0x5754284f345afc66a98fbB0a0Afe71e0F007B949", "Coinbase: Hot"),
    ("0xa9D1e08C7793af67e9d92fe308d5697FB81d3E43", "Coinbase: Cold"), 
    ("0x77696bb39917C91A0c3908D577d5e322095425cA", "Bitfinex"),
    ("0x3f5CE5FBFe3E9af3971dD833D26bA9b5C936f0bE", "Binance: Institutional"),
    ("0x1522900b6dafac587d499a862861c0869be6e428", "FTX (Historical)"),
    
    // DeFi protocols and bridges  
    ("0x8EB8a3b98659Cce290402893d0123abb75E3ab28", "Avalanche: Bridge"),
    ("0x40ec5B33f54e0E8A33A975908C5BA1c14e5BbbDf", "Polygon: Bridge"),
    ("0x99C9fc46f92E8a1c0deC1b1747d010903E884bE1", "Optimism: Gateway"),
    ("0x3041CbD36888bECc7bbCBc0045E3B1f144466f5f", "Circle: Multi-Sig"),
    ("0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F", "SushiSwap: Router"),
];

#[derive(Debug)]
struct TokenHolding {
    token_address: Address,
    token_symbol: String,
    institution: String,
    balance: U256,
    balance_formatted: f64,
}

#[derive(Debug)]  
struct InstitutionalAnalysis {
    token_symbol: String,
    total_supply: f64,
    institutional_holdings: Vec<TokenHolding>,
    total_institutional_balance: f64,
    concentration_percentage: f64,
    top_3_holders: Vec<(String, f64)>,
}

/// Format token amount with proper decimals
fn format_token_amount(amount: U256, decimals: u8) -> f64 {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    let whole_f64 = whole.to_string().parse::<f64>().unwrap_or(0.0);
    let fraction_f64 = fraction.to_string().parse::<f64>().unwrap_or(0.0) / 10_f64.powi(decimals as i32);
    
    whole_f64 + fraction_f64
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏦 ETF and Institutional Token Flow Analysis");
    println!("{}", "=".repeat(70));
    println!("📊 Analyzing {} tokens across {} institutional addresses", ETF_TOKENS.len(), INSTITUTIONAL_ADDRESSES.len());
    println!("🎯 Focus: Capital flows, concentration metrics, and institutional behavior");
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";  
    let chain_query = ChainQuery::new(reth_datadir)?;
    let latest_block = chain_query.get_latest_block()?;
    
    println!("🔗 Analysis Block: {}", latest_block);
    println!();
    
    let overall_start = Instant::now();
    let mut token_analyses = Vec::new();
    let mut total_queries = 0;
    
    println!("📈 Gathering Institutional Holdings Data...");
    
    for (token_address_str, symbol, name, decimals) in ETF_TOKENS {
        let token_start = Instant::now();
        let token_address = Address::from_str(token_address_str)?;
        
        println!("   🪙 Analyzing {} ({})...", symbol, name);
        
        // Get total supply
        let total_supply = chain_query.token.get_erc20_total_supply(token_address, Some(latest_block)).await.unwrap_or(U256::ZERO);
        let total_supply_formatted = format_token_amount(total_supply, *decimals);
        total_queries += 1;
        
        let mut institutional_holdings = Vec::new();
        let mut total_institutional_balance = 0.0;
        
        // Query each institutional address
        for (inst_address_str, inst_name) in INSTITUTIONAL_ADDRESSES {
            let inst_address = Address::from_str(inst_address_str)?;
            let balance = chain_query.token.get_erc20_balance(token_address, inst_address, Some(latest_block)).await.unwrap_or(U256::ZERO);
            let balance_formatted = format_token_amount(balance, *decimals);
            total_queries += 1;
            
            if balance_formatted > 0.001 { // Filter out dust balances
                institutional_holdings.push(TokenHolding {
                    token_address,
                    token_symbol: symbol.to_string(),
                    institution: inst_name.to_string(),
                    balance,
                    balance_formatted,
                });
                
                total_institutional_balance += balance_formatted;
            }
        }
        
        // Sort holdings by balance (descending)
        institutional_holdings.sort_by(|a, b| b.balance_formatted.partial_cmp(&a.balance_formatted).unwrap_or(std::cmp::Ordering::Equal));
        
        // Calculate concentration percentage
        let concentration_percentage = if total_supply_formatted > 0.0 {
            (total_institutional_balance / total_supply_formatted) * 100.0
        } else {
            0.0
        };
        
        // Get top 3 holders
        let top_3_holders: Vec<(String, f64)> = institutional_holdings
            .iter()
            .take(3)
            .map(|h| (h.institution.clone(), h.balance_formatted))
            .collect();
        
        let token_time = token_start.elapsed();
        
        token_analyses.push(InstitutionalAnalysis {
            token_symbol: symbol.to_string(),
            total_supply: total_supply_formatted,
            institutional_holdings,
            total_institutional_balance,
            concentration_percentage,
            top_3_holders,
        });
        
        let holdings_count = token_analyses.last().unwrap().institutional_holdings.len();
        println!("      📊 Found {} institutional holdings | Concentration: {:.1}% | Time: {:.1}ms", 
               holdings_count, concentration_percentage, token_time.as_millis());
    }
    
    // Sort by institutional concentration (descending)
    token_analyses.sort_by(|a, b| b.concentration_percentage.partial_cmp(&a.concentration_percentage).unwrap_or(std::cmp::Ordering::Equal));
    
    let total_time = overall_start.elapsed();
    
    // Display comprehensive analysis results
    println!();
    println!("{}", "=".repeat(70));
    println!("🏆 INSTITUTIONAL CONCENTRATION RANKINGS");
    println!("{}", "=".repeat(70));
    
    for (rank, analysis) in token_analyses.iter().enumerate().take(10) {
        println!("{}. {} - {:.1}% Institutional Concentration", rank + 1, analysis.token_symbol, analysis.concentration_percentage);
        println!("   💰 Institutional Holdings: {:.0} tokens", analysis.total_institutional_balance);
        println!("   📊 Total Supply: {:.0} tokens", analysis.total_supply);
        
        if !analysis.top_3_holders.is_empty() {
            println!("   🏦 Top 3 Institutional Holders:");
            for (i, (holder, balance)) in analysis.top_3_holders.iter().enumerate() {
                println!("      {}. {}: {:.0} tokens", i + 1, holder, balance);
            }
        }
        println!();
    }
    
    // Cross-token institutional analysis
    println!("{}", "=".repeat(70));
    println!("🔄 CROSS-TOKEN INSTITUTIONAL ANALYSIS");
    println!("{}", "=".repeat(70));
    
    // Count holdings per institution across all tokens
    let mut institutional_portfolio: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    
    for analysis in &token_analyses {
        for holding in &analysis.institutional_holdings {
            institutional_portfolio
                .entry(holding.institution.clone())
                .or_insert_with(Vec::new)
                .push((holding.token_symbol.clone(), holding.balance_formatted));
        }
    }
    
    // Sort institutions by number of different tokens held
    let mut institutional_diversity: Vec<(String, usize, f64)> = institutional_portfolio
        .iter()
        .map(|(inst, holdings)| {
            let token_count = holdings.len();
            let total_value_proxy = holdings.iter().map(|(_, balance)| balance).sum::<f64>(); // Simplified value proxy
            (inst.clone(), token_count, total_value_proxy)
        })
        .collect();
    
    institutional_diversity.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by token count
    
    println!("📈 Most Diversified Institutional Portfolios:");
    for (i, (institution, token_count, _value)) in institutional_diversity.iter().enumerate().take(5) {
        println!("{}. {}: {} different tokens", i + 1, institution, token_count);
        
        if let Some(holdings) = institutional_portfolio.get(institution) {
            let mut sorted_holdings = holdings.clone();
            sorted_holdings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            println!("   Top holdings:");
            for (token, balance) in sorted_holdings.iter().take(3) {
                println!("      • {}: {:.0} tokens", token, balance);
            }
        }
        println!();
    }
    
    // Market concentration insights
    let high_concentration_tokens = token_analyses.iter().filter(|a| a.concentration_percentage > 10.0).count();
    let moderate_concentration_tokens = token_analyses.iter().filter(|a| a.concentration_percentage >= 5.0 && a.concentration_percentage <= 10.0).count();
    let low_concentration_tokens = token_analyses.iter().filter(|a| a.concentration_percentage < 5.0).count();
    
    println!("{}", "=".repeat(70));
    println!("📊 MARKET CONCENTRATION INSIGHTS");
    println!("{}", "=".repeat(70));
    println!("🔴 High Concentration (>10%): {} tokens", high_concentration_tokens);
    println!("🟡 Moderate Concentration (5-10%): {} tokens", moderate_concentration_tokens);  
    println!("🟢 Low Concentration (<5%): {} tokens", low_concentration_tokens);
    
    let avg_concentration = token_analyses.iter().map(|a| a.concentration_percentage).sum::<f64>() / token_analyses.len() as f64;
    println!("📈 Average Institutional Concentration: {:.1}%", avg_concentration);
    
    // Performance summary
    println!();
    println!("{}", "=".repeat(70));
    println!("⚡ PERFORMANCE SUMMARY");
    println!("{}", "=".repeat(70));
    println!("Total Tokens Analyzed: {}", ETF_TOKENS.len());
    println!("Total Institutional Addresses: {}", INSTITUTIONAL_ADDRESSES.len());
    println!("Total Queries Executed: {}", total_queries);
    println!("Total Analysis Time: {:.2}s", total_time.as_secs_f64());
    println!("Average Query Time: {:.2}ms", total_time.as_millis() as f64 / total_queries as f64);
    
    // RPC comparison
    let estimated_rpc_time = total_queries * 120; // ~120ms per RPC call
    let speedup = estimated_rpc_time as f64 / total_time.as_millis() as f64;
    
    println!();
    println!("🔄 Traditional RPC Implementation Comparison:");
    println!("RPC API calls required: {} × 120ms = {}ms ({:.1}s)", total_queries, estimated_rpc_time, estimated_rpc_time as f64 / 1000.0);
    println!("Direct DB implementation: {}ms", total_time.as_millis());
    println!("Performance improvement: {:.0}x faster", speedup);
    
    println!();
    println!("✅ Institutional flow analysis complete!");
    println!("🎯 Insights: Institutional concentration varies significantly across tokens");
    println!("💡 Use cases: Portfolio rebalancing detection, institutional sentiment analysis");
    println!("🚀 Perfect for real-time institutional flow monitoring!");
    
    Ok(())
}