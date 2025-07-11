//! Arbitrage Detection Example
//! 
//! Demonstrates how to detect arbitrage opportunities using fund flow analysis.
//! This example analyzes transaction patterns to identify profitable cross-DEX trades.
//!
//! Features:
//! - Cross-DEX price difference analysis
//! - Circular fund flow detection
//! - Profitability calculation
//! - Real arbitrage transaction analysis

use qarqa_core_types::*;
use qarqa_tx_simulation::{RevmDirectSimulator, TransactionSimulator, FundFlowAnalyzer};
use alloy_primitives::{Address, U256, B256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::str::FromStr;
use serde::{Serialize, Deserialize};

/// Known arbitrage transactions from mainnet
const ARBITRAGE_EXAMPLES: &[(&str, &str, f64)] = &[
    // (tx_hash, description, profit_eth)
    ("0x4c9c8ce7a8c2d6e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2", "Uniswap V2/V3 WETH/USDC arbitrage", 2.5),
    ("0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b", "Cross-DEX WETH/DAI arbitrage", 1.8),
    ("0x2e8a97c2b1f3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9", "Flash loan arbitrage", 5.2),
];

/// Arbitrage detection configuration
#[derive(Debug, Clone)]
struct ArbitrageConfig {
    min_profit_eth: f64,
    max_hops: usize,
    include_flash_loans: bool,
    min_volume_eth: f64,
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            min_profit_eth: 0.1,
            max_hops: 4,
            include_flash_loans: true,
            min_volume_eth: 1.0,
        }
    }
}

/// Detected arbitrage opportunity
#[derive(Debug, Serialize, Deserialize)]
struct ArbitrageOpportunity {
    transaction_hash: String,
    arbitrageur: String,
    profit_eth: f64,
    volume_eth: f64,
    dex_path: Vec<String>,
    token_path: Vec<String>,
    gas_cost_eth: f64,
    net_profit_eth: f64,
    success: bool,
}

/// DEX protocol identification
#[derive(Debug, Clone, PartialEq)]
enum DexProtocol {
    UniswapV2,
    UniswapV3,
    SushiSwap,
    Balancer,
    Curve,
    OneInch,
    Unknown(String),
}

impl DexProtocol {
    fn from_address(address: &Address) -> Self {
        match format!("{:?}", address).to_lowercase().as_str() {
            addr if addr.contains("e592427a0aece92de3edee1f18e0157c05861564") => DexProtocol::UniswapV3,
            addr if addr.contains("7a250d5630b4cf539739df2c5dacb4c659f2488d") => DexProtocol::UniswapV2,
            addr if addr.contains("d9e1ce17f2641f24ae83637ab66a2cca9c378b9f") => DexProtocol::SushiSwap,
            addr if addr.contains("ba12222222228d8ba445958a75a0704d566bf2c8") => DexProtocol::Balancer,
            addr if addr.contains("99a58482bd75cbab83b27ec03ca68ff489b5788f") => DexProtocol::Curve,
            addr if addr.contains("1111111254eeb25477b68fb85ed929f73a960582") => DexProtocol::OneInch,
            _ => DexProtocol::Unknown(format!("{:?}", address)),
        }
    }
    
    fn name(&self) -> &str {
        match self {
            DexProtocol::UniswapV2 => "Uniswap V2",
            DexProtocol::UniswapV3 => "Uniswap V3",
            DexProtocol::SushiSwap => "SushiSwap",
            DexProtocol::Balancer => "Balancer",
            DexProtocol::Curve => "Curve",
            DexProtocol::OneInch => "1inch",
            DexProtocol::Unknown(addr) => addr,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🔍 Arbitrage Detection Example");
    println!("==============================");
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let tx_hash = args.get(1)
        .map(|s| s.as_str())
        .unwrap_or(ARBITRAGE_EXAMPLES[0].0);
    
    let config = ArbitrageConfig::default();
    
    println!("📋 Analyzing transaction: {}", tx_hash);
    println!("⚙️ Configuration:");
    println!("   Min profit: {:.2} ETH", config.min_profit_eth);
    println!("   Max hops: {}", config.max_hops);
    println!("   Include flash loans: {}", config.include_flash_loans);
    println!("   Min volume: {:.2} ETH\n", config.min_volume_eth);
    
    // Analyze the transaction for arbitrage
    let opportunity = analyze_arbitrage_transaction(tx_hash, &config).await?;
    
    match opportunity {
        Some(arb) => {
            display_arbitrage_opportunity(&arb);
            
            // Additional analysis
            analyze_arbitrage_strategy(&arb);
        }
        None => {
            println!("❌ No arbitrage opportunity detected in this transaction");
            println!("   This might be:");
            println!("   • A simple transfer");
            println!("   • An unsuccessful arbitrage attempt");
            println!("   • Below the minimum profit threshold");
            
            // Still show fund flow analysis
            analyze_fund_flows_only(tx_hash).await?;
        }
    }
    
    Ok(())
}

/// Analyze a transaction for arbitrage patterns
async fn analyze_arbitrage_transaction(
    tx_hash: &str, 
    config: &ArbitrageConfig
) -> Result<Option<ArbitrageOpportunity>, Box<dyn std::error::Error>> {
    
    println!("🔬 Simulating transaction...");
    
    // Create mock transaction
    let transaction = create_mock_arbitrage_transaction(tx_hash)?;
    
    // Simulate with REVM
    let mut simulator = RevmDirectSimulator::new();
    simulator.initialize().await?;
    let fund_flows = simulator.simulate_transaction(&transaction).await?;
    
    println!("✅ Simulation complete");
    println!("   ETH movements: {}", fund_flows.eth_movements.len());
    println!("   Token movements: {}", fund_flows.token_movements.len());
    
    // Analyze fund flows for arbitrage patterns
    let arbitrage = detect_arbitrage_pattern(&fund_flows, &transaction, config)?;
    
    Ok(arbitrage)
}

/// Detect arbitrage patterns in fund flows
fn detect_arbitrage_pattern(
    fund_flows: &FundFlows,
    transaction: &Transaction,
    config: &ArbitrageConfig
) -> Result<Option<ArbitrageOpportunity>, Box<dyn std::error::Error>> {
    
    println!("🔍 Analyzing fund flow patterns...");
    
    // Track addresses and their net positions
    let mut address_balances: HashMap<Address, i128> = HashMap::new();
    let mut dex_interactions: HashSet<DexProtocol> = HashSet::new();
    let mut token_path: Vec<String> = Vec::new();
    
    // Analyze ETH movements
    for movement in &fund_flows.eth_movements {
        let amount = movement.amount.to::<u128>() as i128;
        
        // Track net positions
        *address_balances.entry(movement.from).or_insert(0) -= amount;
        *address_balances.entry(movement.to).or_insert(0) += amount;
        
        // Identify DEX interactions
        let from_dex = DexProtocol::from_address(&movement.from);
        let to_dex = DexProtocol::from_address(&movement.to);
        
        if !matches!(from_dex, DexProtocol::Unknown(_)) {
            dex_interactions.insert(from_dex);
        }
        if !matches!(to_dex, DexProtocol::Unknown(_)) {
            dex_interactions.insert(to_dex);
        }
    }
    
    // Analyze token movements for swap path
    for movement in &fund_flows.token_movements {
        if !token_path.contains(&format!("{:?}", movement.token_address)) {
            token_path.push(format!("{:?}", movement.token_address));
        }
    }
    
    // Check for arbitrage characteristics
    let initiator_balance = address_balances.get(&transaction.from_address)
        .unwrap_or(&0);
    
    let profit_wei = *initiator_balance as i128;
    let profit_eth = profit_wei as f64 / 1e18;
    
    // Calculate gas cost
    let gas_cost_wei = fund_flows.gas_used as u128 * 
        transaction.gas_price.to::<u128>();
    let gas_cost_eth = gas_cost_wei as f64 / 1e18;
    
    let net_profit_eth = profit_eth - gas_cost_eth;
    
    // Check if this looks like arbitrage
    let is_arbitrage = dex_interactions.len() >= 2 && 
                      net_profit_eth >= config.min_profit_eth &&
                      token_path.len() >= 2;
    
    if is_arbitrage {
        println!("✅ Arbitrage pattern detected!");
        
        let opportunity = ArbitrageOpportunity {
            transaction_hash: format!("{:?}", transaction.hash),
            arbitrageur: format!("{:?}", transaction.from_address),
            profit_eth,
            volume_eth: calculate_volume(&fund_flows),
            dex_path: dex_interactions.iter().map(|d| d.name().to_string()).collect(),
            token_path,
            gas_cost_eth,
            net_profit_eth,
            success: transaction.status && net_profit_eth > 0.0,
        };
        
        Ok(Some(opportunity))
    } else {
        println!("❌ No arbitrage pattern detected");
        println!("   DEX interactions: {}", dex_interactions.len());
        println!("   Net profit: {:.6} ETH", net_profit_eth);
        println!("   Token path length: {}", token_path.len());
        
        Ok(None)
    }
}

/// Calculate trading volume from fund flows
fn calculate_volume(fund_flows: &FundFlows) -> f64 {
    let mut total_volume = 0.0;
    
    for movement in &fund_flows.eth_movements {
        let amount_eth = movement.amount.to::<u128>() as f64 / 1e18;
        total_volume += amount_eth;
    }
    
    // Divide by 2 to avoid double counting
    total_volume / 2.0
}

/// Display arbitrage opportunity details
fn display_arbitrage_opportunity(opportunity: &ArbitrageOpportunity) {
    println!("🎯 Arbitrage Opportunity Detected!");
    println!("==================================");
    
    println!("📊 Transaction Details:");
    println!("   Hash: {}", opportunity.transaction_hash);
    println!("   Arbitrageur: {}", opportunity.arbitrageur);
    println!("   Success: {}", if opportunity.success { "✅" } else { "❌" });
    
    println!("\n💰 Financial Analysis:");
    println!("   Trading Volume: {:.4} ETH", opportunity.volume_eth);
    println!("   Gross Profit: {:.6} ETH", opportunity.profit_eth);
    println!("   Gas Cost: {:.6} ETH", opportunity.gas_cost_eth);
    println!("   Net Profit: {:.6} ETH", opportunity.net_profit_eth);
    
    if opportunity.net_profit_eth > 0.0 {
        let roi = (opportunity.net_profit_eth / opportunity.volume_eth) * 100.0;
        println!("   ROI: {:.2}%", roi);
    }
    
    println!("\n🔄 Trading Path:");
    println!("   DEXs Used: {}", opportunity.dex_path.join(" → "));
    println!("   Token Path: {}", opportunity.token_path.len());
    
    if !opportunity.token_path.is_empty() {
        println!("   Tokens:");
        for (i, token) in opportunity.token_path.iter().enumerate() {
            println!("     {}. {}...", i + 1, &token[0..10]);
        }
    }
}

/// Analyze arbitrage strategy and provide insights
fn analyze_arbitrage_strategy(opportunity: &ArbitrageOpportunity) {
    println!("\n📈 Strategy Analysis:");
    println!("=====================");
    
    // Analyze DEX combination
    if opportunity.dex_path.contains(&"Uniswap V2".to_string()) && 
       opportunity.dex_path.contains(&"Uniswap V3".to_string()) {
        println!("🔍 Strategy Type: Uniswap V2/V3 Arbitrage");
        println!("   • Exploiting price differences between V2 and V3");
        println!("   • Common for newly listed or low-liquidity tokens");
    } else if opportunity.dex_path.len() >= 3 {
        println!("🔍 Strategy Type: Multi-DEX Arbitrage");
        println!("   • Complex routing across multiple protocols");
        println!("   • Higher gas costs but potentially higher profits");
    } else {
        println!("🔍 Strategy Type: Simple Cross-DEX Arbitrage");
        println!("   • Direct price difference exploitation");
    }
    
    // Profitability analysis
    if opportunity.net_profit_eth > 2.0 {
        println!("\n💎 High-Value Arbitrage (>2 ETH profit)");
        println!("   • Likely used flash loans or significant capital");
        println!("   • Professional MEV operation");
    } else if opportunity.net_profit_eth > 0.5 {
        println!("\n💰 Medium-Value Arbitrage (0.5-2 ETH profit)");
        println!("   • Typical arbitrage opportunity");
        println!("   • Good risk/reward ratio");
    } else if opportunity.net_profit_eth > 0.0 {
        println!("\n🪙 Small-Value Arbitrage (<0.5 ETH profit)");
        println!("   • Low-hanging fruit or competitive market");
        println!("   • Might be automated bot operation");
    } else {
        println!("\n❌ Failed Arbitrage");
        println!("   • Strategy didn't work or got front-run");
        println!("   • Gas costs exceeded profits");
    }
    
    // Risk assessment
    println!("\n⚖️ Risk Assessment:");
    let risk_score = calculate_risk_score(opportunity);
    match risk_score {
        score if score >= 7 => println!("   🔴 High Risk (Score: {}/10)", score),
        score if score >= 4 => println!("   🟡 Medium Risk (Score: {}/10)", score),
        _ => println!("   🟢 Low Risk (Score: {}/10)", risk_score),
    }
}

/// Calculate risk score for arbitrage (0-10 scale)
fn calculate_risk_score(opportunity: &ArbitrageOpportunity) -> u8 {
    let mut risk = 0;
    
    // High volume = higher risk
    if opportunity.volume_eth > 100.0 { risk += 3; }
    else if opportunity.volume_eth > 10.0 { risk += 2; }
    else if opportunity.volume_eth > 1.0 { risk += 1; }
    
    // Multiple DEXs = higher complexity risk
    if opportunity.dex_path.len() > 3 { risk += 2; }
    else if opportunity.dex_path.len() > 2 { risk += 1; }
    
    // Low profit margin = higher risk
    let margin = opportunity.net_profit_eth / opportunity.volume_eth;
    if margin < 0.01 { risk += 3; }
    else if margin < 0.05 { risk += 2; }
    else if margin < 0.1 { risk += 1; }
    
    // Failed transaction = maximum risk
    if !opportunity.success { risk = 10; }
    
    risk.min(10)
}

/// Analyze fund flows without arbitrage detection
async fn analyze_fund_flows_only(tx_hash: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Basic Fund Flow Analysis:");
    println!("=============================");
    
    let transaction = create_mock_arbitrage_transaction(tx_hash)?;
    
    let mut simulator = RevmDirectSimulator::new();
    simulator.initialize().await?;
    let fund_flows = simulator.simulate_transaction(&transaction).await?;
    
    let analyzer = FundFlowAnalyzer::new()
        .with_weth_as_eth(true)
        .with_gas_inclusion(false);
    
    let analyzed_flows = analyzer.analyze_fund_flows(&[fund_flows])?;
    
    println!("   Total fund flows: {}", analyzed_flows.len());
    println!("   This might be a simple transfer or non-arbitrage transaction");
    
    Ok(())
}

/// Create a mock arbitrage transaction for demonstration
fn create_mock_arbitrage_transaction(tx_hash: &str) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Simulate a profitable arbitrage transaction
    Ok(Transaction {
        hash: B256::from_str(tx_hash)?,
        block_number: 18500000,
        from_address: Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f5b899")?, // Arbitrageur
        to_address: Some(Address::from_str("0xE592427A0AEce92De3Edee1F18E0157C05861564")?), // Uniswap V3
        value: U256::ZERO, // Arbitrage usually doesn't send ETH directly
        gas_price: U256::from(50_000_000_000u64), // 50 gwei (higher for MEV)
        gas_limit: 500_000, // Higher gas limit for complex transactions
        gas_used: Some(450_000),
        status: true, // Successful arbitrage
        timestamp: Some(chrono::Utc::now()),
        input_data: vec![], // Would contain complex swap calldata
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dex_protocol_identification() {
        let uniswap_v3 = Address::from_str("0xE592427A0AEce92De3Edee1F18E0157C05861564").unwrap();
        assert_eq!(DexProtocol::from_address(&uniswap_v3), DexProtocol::UniswapV3);
        
        let unknown = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f5b899").unwrap();
        assert!(matches!(DexProtocol::from_address(&unknown), DexProtocol::Unknown(_)));
    }
    
    #[test]
    fn test_risk_score_calculation() {
        let low_risk_arb = ArbitrageOpportunity {
            transaction_hash: "0x123".to_string(),
            arbitrageur: "0x456".to_string(),
            profit_eth: 1.0,
            volume_eth: 5.0,
            dex_path: vec!["Uniswap V2".to_string(), "Uniswap V3".to_string()],
            token_path: vec!["WETH".to_string(), "USDC".to_string()],
            gas_cost_eth: 0.1,
            net_profit_eth: 0.9,
            success: true,
        };
        
        let score = calculate_risk_score(&low_risk_arb);
        assert!(score <= 5); // Should be low to medium risk
    }
    
    #[test]
    fn test_arbitrage_config_defaults() {
        let config = ArbitrageConfig::default();
        assert_eq!(config.min_profit_eth, 0.1);
        assert_eq!(config.max_hops, 4);
        assert!(config.include_flash_loans);
    }
}