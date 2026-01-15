//! Risk Check Demo
//!
//! Demonstrates the simplified risk management checks

use eth_kartal::risk::{RiskManager, RiskConfig, RiskDecision};
use ethers::types::U256;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("=== Risk Check Demo ===");

    // Create risk manager with custom config
    let config = RiskConfig {
        max_gas_cost_percent: 5.0,    // 5% max gas cost
        max_slippage_percent: 3.0,    // 3% max slippage  
        min_eth_balance: 0.01,        // Keep 0.01 ETH buffer
    };
    
    let risk_manager = RiskManager::new(config);

    // Scenario 1: Check sufficient funds
    info!("\n📊 Scenario 1: Sufficient Funds Check");
    
    let available_balance = U256::from((0.05 * 1e18) as u128); // 0.05 ETH
    let required_amount = U256::from((0.01 * 1e18) as u128);   // 0.01 ETH  
    let gas_cost_eth = 0.0002; // 0.0002 ETH
    
    match risk_manager.check_sufficient_funds(available_balance, required_amount, gas_cost_eth) {
        RiskDecision::Allow => {
            info!("✅ Sufficient funds check PASSED");
            info!("   Available: 0.05 ETH");
            info!("   Required: 0.01 ETH + 0.0002 ETH gas + 0.01 ETH buffer = 0.0202 ETH");
        }
        RiskDecision::Block { reason } => {
            info!("❌ Blocked: {}", reason);
        }
    }

    // Scenario 2: Insufficient funds
    info!("\n📊 Scenario 2: Insufficient Funds");
    
    let small_balance = U256::from((0.015 * 1e18) as u128); // Only 0.015 ETH
    
    match risk_manager.check_sufficient_funds(small_balance, required_amount, gas_cost_eth) {
        RiskDecision::Allow => {
            info!("✅ Passed (unexpected)");
        }
        RiskDecision::Block { reason } => {
            info!("❌ Blocked: {}", reason);
            info!("   This is expected - we need 0.0202 ETH but only have 0.015 ETH");
        }
    }

    // Scenario 3: Gas cost check
    info!("\n📊 Scenario 3: Gas Cost Check");
    
    let trade_amount = 0.01; // 0.01 ETH trade
    let normal_gas = 0.0002; // 0.0002 ETH (2% of trade)
    
    match risk_manager.check_gas_cost(trade_amount, normal_gas) {
        RiskDecision::Allow => {
            info!("✅ Gas cost acceptable: {} ETH ({:.1}% of trade)", 
                normal_gas, (normal_gas / trade_amount) * 100.0);
        }
        RiskDecision::Block { reason } => {
            info!("❌ Blocked: {}", reason);
        }
    }
    
    // High gas scenario
    let high_gas = 0.003; // 0.003 ETH (30% of trade!)
    
    match risk_manager.check_gas_cost(trade_amount, high_gas) {
        RiskDecision::Allow => {
            info!("✅ Passed (unexpected)");
        }
        RiskDecision::Block { reason } => {
            info!("❌ Blocked: {}", reason);
            info!("   Gas would be {:.1}% of trade value!", (high_gas / trade_amount) * 100.0);
        }
    }

    // Scenario 4: Slippage check
    info!("\n📊 Scenario 4: Slippage Check");
    
    let expected_tokens = U256::from(1000);
    let actual_tokens = U256::from(980); // 2% slippage
    
    match risk_manager.check_slippage(expected_tokens, actual_tokens) {
        RiskDecision::Allow => {
            info!("✅ Slippage acceptable: {:.1}%", 
                ((1000.0 - 980.0) / 1000.0) * 100.0);
        }
        RiskDecision::Block { reason } => {
            info!("❌ Blocked: {}", reason);
        }
    }
    
    // High slippage scenario
    let low_tokens = U256::from(850); // 15% slippage!
    
    match risk_manager.check_slippage(expected_tokens, low_tokens) {
        RiskDecision::Allow => {
            info!("✅ Passed (unexpected)");
        }
        RiskDecision::Block { reason } => {
            info!("❌ Blocked: {}", reason);
            info!("   Would receive 15% less tokens than expected!");
        }
    }

    info!("\n📝 Summary:");
    info!("- Risk checks focus on preventing obvious failures");
    info!("- No complex portfolio management or daily limits");
    info!("- Simple Allow/Block decisions with clear reasons");

    Ok(())
}