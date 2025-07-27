/// Test the BuySellSimulator with known tokens
/// 
/// This example tests the new buy_sell_simulator module with:
/// - AITAI token (should work - no honeypot)
/// - 0xT token (should fail sell - honeypot)

use mempool_processor::signal_engine::simulator::buy_sell_simulator::SequentialBuySellSimulator;
use mempool_processor::signal_engine::simulator::BuySellSimulator;
use alloy_primitives::Address;
use std::str::FromStr;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simulator
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let simulator = SequentialBuySellSimulator::new(reth_datadir)?;
    
    println!("🚀 Testing BuySellSimulator with known tokens\n");
    
    // Test 1: AITAI token (should work)
    println!("1️⃣ Testing AITAI token:");
    let aitai_token = Address::from_str("0xBCb2479ae9F271BB1561b4823dCaAc8B02860B1E")?;
    let aitai_pool = Address::from_str("0xa32d14c0d48ed4835179f33bc00d1bd7acea4aff")?;
    let aitai_block = 23005264;
    
    match simulator.simulate_buy_sell_sequence(aitai_token, aitai_pool, Some(aitai_block)).await {
        Ok(result) => {
            println!("   ✅ Simulation completed in {:.1}ms", result.simulation_time_ms);
            println!("   Can Buy: {}", result.can_buy);
            println!("   Can Sell: {}", result.can_sell);
            println!("   Buy Tax: {:.1}%", result.buy_tax);
            println!("   Sell Tax: {:.1}%", result.sell_tax);
            println!("   Is Honeypot: {}", result.is_honeypot);
            println!("   Tokens Received: {} (raw units)", result.tokens_received);
            if let Some(error) = result.error {
                println!("   Error: {}", error);
            }
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    println!("\n2️⃣ Testing 0xT token (known honeypot):");
    let zerot_token = Address::from_str("0x0Ca5f8f96C3a2C84190a415b658AaFc8501AaeFF")?;
    let zerot_pool = Address::from_str("0x885cf65E1511D50Bb49e488839525fDE44cDE36b")?;
    let zerot_block = 22954920;
    
    match simulator.simulate_buy_sell_sequence(zerot_token, zerot_pool, Some(zerot_block)).await {
        Ok(result) => {
            println!("   ✅ Simulation completed in {:.1}ms", result.simulation_time_ms);
            println!("   Can Buy: {}", result.can_buy);
            println!("   Can Sell: {}", result.can_sell);
            println!("   Buy Tax: {:.1}%", result.buy_tax);
            println!("   Sell Tax: {:.1}%", result.sell_tax);
            println!("   Is Honeypot: {}", result.is_honeypot);
            println!("   Tokens Received: {} (raw units)", result.tokens_received);
            if let Some(error) = result.error {
                println!("   Error: {}", error);
            }
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    println!("\n3️⃣ Testing at a later block for 0xT (where tax changed):");
    let zerot_block_later = 22954923;
    
    match simulator.simulate_buy_sell_sequence(zerot_token, zerot_pool, Some(zerot_block_later)).await {
        Ok(result) => {
            println!("   ✅ Simulation completed in {:.1}ms", result.simulation_time_ms);
            println!("   Can Buy: {}", result.can_buy);
            println!("   Can Sell: {}", result.can_sell);
            println!("   Buy Tax: {:.1}%", result.buy_tax);
            println!("   Sell Tax: {:.1}%", result.sell_tax);
            println!("   Is Honeypot: {}", result.is_honeypot);
            println!("   Tokens Received: {} (raw units)", result.tokens_received);
            if let Some(error) = result.error {
                println!("   Error: {}", error);
            }
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    Ok(())
}