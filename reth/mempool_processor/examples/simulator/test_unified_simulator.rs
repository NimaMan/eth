/// Test the unified simulator to verify it solves the database lock issue
///
/// This example demonstrates how to use the unified simulator to:
/// 1. Simulate single transactions
/// 2. Simulate buy/sell sequences
/// All using the same database connection (no error code 11!)

use mempool_processor::simulator::{UnifiedSimulator, BuySellSimulatorConfig};
use alloy_primitives::{Address, U256};
use std::str::FromStr;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("info")
        .init();
    
    println!("🚀 Testing Unified Simulator\n");
    
    // Create unified simulator - single database connection!
    let datadir = "/home/nima/.local/share/reth/mainnet";
    let simulator = UnifiedSimulator::new(datadir)?;
    
    let latest_block = simulator.get_latest_block()?;
    println!("📦 Latest block: {}", latest_block);
    
    // Test parameters from the working example
    let token_address = Address::from_str("0xaAd355C579d85432D0a31A3801D267dEaC111071")?;
    let pool_address = Address::from_str("0xDbDCBee13F7a0af5F5dD233553C055f5f0627F98")?;
    let block_number = 23046144u64;
    
    println!("\n🧪 Test 1: Buy/sell simulation WITHOUT original transaction");
    println!("Token: {}", token_address);
    println!("Pool: {}", pool_address);
    println!("Block: {}", block_number);
    
    match simulator.simulate_buy_sell_sequence(
        token_address,
        pool_address,
        Some(block_number)
    ).await {
        Ok(result) => {
            println!("✅ SUCCESS!");
            println!("  Buy success: {}", result.buy_result.success);
            println!("  Sell success: {}", result.sell_result.success);
            println!("  Simulation time: {:.2}ms", result.simulation_time_ms);
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            if e.to_string().contains("error code: 11") {
                println!("⚠️  ERROR CODE 11 DETECTED!");
            }
        }
    }
    
    // Test 2: With original transaction
    println!("\n🧪 Test 2: Buy/sell simulation WITH original transaction");
    
    use reth_tx_simulator::CallRequest;
    use alloy_primitives::Bytes;
    
    let original_tx = CallRequest {
        from: Some(Address::from_str("0xCC03d17aC866fdeb8B9679e1b082409e2659Dd6b")?),
        to: Some(Address::from_str("0x4a098251e89B42cA59666B33b4Aa5D1Af4F3C650")?),
        value: Some(U256::from_str("1924247168819762189")?), // 1.924 ETH
        data: Some(Bytes::new()), // Empty data for ETH transfer
        gas: Some(21000),
        gas_price: Some(740951535), // 0.740951535 Gwei
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: Some(5),
    };
    
    match simulator.simulate_sequence_with_tx(
        Some(original_tx),
        token_address,
        pool_address,
        Some(block_number)
    ).await {
        Ok(result) => {
            println!("✅ SUCCESS!");
            if let Some(tx_result) = &result.given_tx_result {
                println!("  Original tx success: {}", tx_result.success);
            }
            println!("  Buy success: {}", result.buy_result.success);
            println!("  Sell success: {}", result.sell_result.success);
            println!("  Simulation time: {:.2}ms", result.simulation_time_ms);
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            if e.to_string().contains("error code: 11") {
                println!("⚠️  ERROR CODE 11 DETECTED!");
            }
        }
    }
    
    println!("\n✨ Unified simulator test complete!");
    
    Ok(())
}