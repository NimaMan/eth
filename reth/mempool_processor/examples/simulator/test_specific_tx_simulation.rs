/// Test simulation with a specific transaction
/// Tests transaction 0xc32f7fd1e0a3561da3a686555f3e3ed97cdfd0b0369da0f8bf6f5471f7f0643d

use mempool_processor::simulator::{
    SequentialBuySellSimulator,
    BuySellSimulatorConfig,
};
use alloy_primitives::{Address, U256, Bytes};
use std::str::FromStr;
use eyre::Result;
use reth_tx_simulator::CallRequest;

#[tokio::main]
async fn main() -> Result<()> {
    // Transaction details from etherscan
    let tx_hash = "0xc32f7fd1e0a3561da3a686555f3e3ed97cdfd0b0369da0f8bf6f5471f7f0643d";
    let from = Address::from_str("0xCC03d17aC866fdeb8B9679e1b082409e2659Dd6b")?;
    let to = Address::from_str("0x4a098251e89B42cA59666B33b4Aa5D1Af4F3C650")?;
    let value = U256::from_str("1924247168819762189")?; // 1.924247168819762189 ETH
    let tx_mined_at_block = 23046145u64;
    let block_number = tx_mined_at_block - 1; // Simulate at block before tx was mined
    
    // Token info from the logs
    let token_address = Address::from_str("0xaAd355C579d85432D0a31A3801D267dEaC111071")?;
    let pool_address = Address::from_str("0xDbDCBee13F7a0af5F5dA233553C055f5f0627F98")?;
    
    println!("🔍 Testing simulation with transaction: {}", tx_hash);
    println!("Transaction was mined at block: {}", tx_mined_at_block);
    println!("Simulating at block: {} (one before mined)", block_number);
    println!("From: {}", from);
    println!("To: {}", to);
    println!("Value: {} ETH", format!("{:.18}", value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18));
    println!("Token: {}", token_address);
    println!("Pool: {}", pool_address);
    
    // Initialize simulator
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let config = BuySellSimulatorConfig::default();
    let simulator = SequentialBuySellSimulator::with_config(reth_datadir, config.clone())?;
    
    println!("\nConfiguration:");
    println!("  Buyer Address: {}", config.buyer_address);
    println!("  Test Buy Amount: {} ETH", format!("{:.18}", config.test_buy_amount.to_string().parse::<f64>().unwrap_or(0.0) / 1e18));
    
    // Create the original transaction as a CallRequest
    let original_tx = CallRequest {
        from: Some(from),
        to: Some(to),
        value: Some(value),
        data: Some(Bytes::new()), // Empty data for ETH transfer
        gas: Some(21000),
        gas_price: Some(740951535), // 0.740951535 Gwei
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: Some(5),
    };
    
    // Test 1: Simulate WITHOUT the original transaction (just buy/sell)
    println!("\n1️⃣ Testing buy/sell simulation WITHOUT original transaction:");
    match simulator.simulate_sequence(token_address, pool_address, Some(block_number)).await {
        Ok(result) => {
            println!("   ✅ Simulation completed in {:.1}ms", result.simulation_time_ms);
            println!("   Buy success: {}", result.buy_result.success);
            println!("   Sell success: {}", result.sell_result.success);
            if let Some(reason) = &result.buy_result.revert_reason {
                println!("   Buy revert: {}", reason);
            }
            if let Some(reason) = &result.sell_result.revert_reason {
                println!("   Sell revert: {}", reason);
            }
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    // Test 2: Simulate WITH the original transaction
    println!("\n2️⃣ Testing buy/sell simulation WITH original transaction:");
    match simulator.simulate_sequence_with_tx(
        Some(original_tx.clone()),
        token_address,
        pool_address,
        Some(block_number)
    ).await {
        Ok(result) => {
            println!("   ✅ Simulation completed in {:.1}ms", result.simulation_time_ms);
            if let Some(given_tx_result) = &result.given_tx_result {
                println!("   Original tx success: {}", given_tx_result.success);
                if let Some(reason) = &given_tx_result.revert_reason {
                    println!("   Original tx revert: {}", reason);
                }
            }
            println!("   Buy success: {}", result.buy_result.success);
            println!("   Sell success: {}", result.sell_result.success);
            if let Some(reason) = &result.buy_result.revert_reason {
                println!("   Buy revert: {}", reason);
            }
            if let Some(reason) = &result.sell_result.revert_reason {
                println!("   Sell revert: {}", reason);
            }
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
            println!("   This is the error we're seeing in the logs!");
        }
    }
    
    
    Ok(())
}