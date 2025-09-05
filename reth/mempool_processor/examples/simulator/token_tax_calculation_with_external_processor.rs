/// Test the new PoolBuySellSimulator from tx_processor with known tokens
/// 
/// This example tests the migration from mempool_processor's own simulator to
/// tx_processor's proven PoolBuySellSimulator:
/// - AITAI token (should work - no honeypot)
/// - 0xT token (should fail sell - honeypot)
/// 
/// The new simulator returns PoolViabilityResult with tax percentages already calculated

use std::sync::Arc;
use alloy_primitives::{Address, U256};
use std::str::FromStr;
use eyre::Result;

// Import from tx_processor
use tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::{
    pool_buy_sell_simulator::check_can_buy_sell_pool,
    config::PoolViabilityConfig,
    types::{PoolType, PoolViabilityResult},
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

fn format_token_amount(amount: U256, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }
    
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    if fraction == U256::ZERO {
        format!("{}", whole)
    } else {
        let fraction_str = format!("{:0>width$}", fraction, width = decimals as usize);
        let trimmed = fraction_str.trim_end_matches('0');
        if trimmed.is_empty() {
            format!("{}", whole)
        } else {
            format!("{}.{}", whole, trimmed)
        }
    }
}

fn print_pool_results(result: &PoolViabilityResult, token_name: &str, decimals: u8) {
    println!("\n🔍 Testing {}", token_name);
    println!("=" .repeat(60));
    
    // Trading status
    println!("📊 Trading Status:");
    println!("  Can Buy: {}", if result.can_buy { "✅" } else { "❌" });
    println!("  Can Approve: {}", if result.can_approve { "✅" } else { "❌" });
    println!("  Can Sell: {}", if result.can_sell { "✅" } else { "❌" });
    println!("  Overall Tradeable: {}", if result.is_tradeable { "✅" } else { "❌" });
    
    // Tax information
    println!("\n💰 Tax Analysis:");
    if result.buy_tax_percent >= 0.0 {
        println!("  Buy Tax: {:.2}%", result.buy_tax_percent);
    } else {
        println!("  Buy Tax: Failed to calculate");
    }
    if result.sell_tax_percent >= 0.0 {
        println!("  Sell Tax: {:.2}%", result.sell_tax_percent);
    } else {
        println!("  Sell Tax: Failed to calculate");
    }
    
    // Trade details
    println!("\n📈 Trade Details:");
    if result.tokens_received > U256::ZERO {
        println!("  Tokens Received: {} {}", 
            format_token_amount(result.tokens_received, decimals),
            token_name
        );
    }
    println!("  ETH Spent: {} ETH", format_token_amount(result.eth_spent, 18));
    if result.eth_received > U256::ZERO {
        println!("  ETH Received Back: {} ETH", format_token_amount(result.eth_received, 18));
        
        // Calculate net loss
        if result.eth_spent > result.eth_received {
            let loss = result.eth_spent - result.eth_received;
            let loss_percent = (loss * U256::from(10000) / result.eth_spent).to::<u64>() as f64 / 100.0;
            println!("  Net Loss: {} ETH ({:.2}%)", 
                format_token_amount(loss, 18),
                loss_percent
            );
        }
    }
    
    // Summary
    let is_honeypot = result.can_buy && !result.can_sell;
    println!("\n   Summary: {}", if is_honeypot { "🍯 HONEYPOT DETECTED" } else { "✅ Trading Enabled" });
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    
    println!("🚀 Testing PoolBuySellSimulator from tx_processor");
    println!("=" .repeat(60));
    
    // Create shared simulator and processor
    let simulator = Arc::new(TxSimulator::new(reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());
    
    // Test configuration
    let buyer_address = Address::from_str("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")
        .map_err(|e| eyre::eyre!("Failed to parse buyer address: {}", e))?;
    let test_amount = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
    
    println!("Configuration:");
    println!("  Buyer Address: {}", buyer_address);
    println!("  Test Buy Amount: {} ETH", format_token_amount(test_amount, 18));
    
    // Test 1: AITAI token (should work)
    {
        let aitai_token = Address::from_str("0xBCb2479ae9F271BB1561b4823dCaAc8B02860B1E")
            .map_err(|e| eyre::eyre!("Failed to parse AITAI address: {}", e))?;
        let aitai_pool = Address::from_str("0xa32d14c0d48ed4835179f33bc00d1bd7acea4aff")
            .map_err(|e| eyre::eyre!("Failed to parse AITAI pool: {}", e))?;
        
        let config = PoolViabilityConfig {
            token_address: aitai_token,
            pool_address: aitai_pool,
            pool_type: PoolType::UniswapV2,
            test_amount,
            buyer_address,
            block_number: Some(23005264),
            gas_limit: 500_000,
            gas_price: 30_000_000_000,
            prior_tx: None,
            block_delay: 0,
        };
        
        match check_can_buy_sell_pool(
            simulator.clone(),
            tx_processor.clone(),
            config
        ).await {
            Ok(result) => print_pool_results(&result, "AITAI", 18),
            Err(e) => println!("❌ AITAI simulation failed: {}", e),
        }
    }
    
    // Test 2: 0xT token (known honeypot)
    {
        let zerot_token = Address::from_str("0x0Ca5f8f96C3a2C84190a415b658AaFc8501AaeFF")
            .map_err(|e| eyre::eyre!("Failed to parse 0xT address: {}", e))?;
        let zerot_pool = Address::from_str("0x885cf65E1511D50Bb49e488839525fDE44cDE36b")
            .map_err(|e| eyre::eyre!("Failed to parse 0xT pool: {}", e))?;
        
        let config = PoolViabilityConfig {
            token_address: zerot_token,
            pool_address: zerot_pool,
            pool_type: PoolType::UniswapV2,
            test_amount,
            buyer_address,
            block_number: Some(22954920),
            gas_limit: 500_000,
            gas_price: 30_000_000_000,
            prior_tx: None,
            block_delay: 0,
        };
        
        match check_can_buy_sell_pool(
            simulator.clone(),
            tx_processor.clone(),
            config
        ).await {
            Ok(result) => print_pool_results(&result, "0xT", 18),
            Err(e) => println!("❌ 0xT simulation failed: {}", e),
        }
    }
    
    // Test 3: FLOKI (has buy tax) 
    {
        let floki_address = Address::from_str("0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E")
            .map_err(|e| eyre::eyre!("Failed to parse FLOKI address: {}", e))?;
        let floki_pool = Address::from_str("0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0")
            .map_err(|e| eyre::eyre!("Failed to parse FLOKI pool: {}", e))?;
        
        let config = PoolViabilityConfig {
            token_address: floki_address,
            pool_address: floki_pool,
            pool_type: PoolType::UniswapV2,
            test_amount,
            buyer_address,
            block_number: None, // Use latest block
            gas_limit: 500_000,
            gas_price: 30_000_000_000,
            prior_tx: None,
            block_delay: 0,
        };
        
        match check_can_buy_sell_pool(
            simulator.clone(),
            tx_processor.clone(),
            config
        ).await {
            Ok(result) => print_pool_results(&result, "FLOKI", 9),
            Err(e) => println!("❌ FLOKI simulation failed: {}", e),
        }
    }
    
    println!("\n" .repeat(1));
    println!("✅ Testing complete!");
    println!("\nThis demonstrates how mempool_processor can use tx_processor's");
    println!("proven PoolBuySellSimulator instead of its own tax calculation.");
    
    Ok(())
}