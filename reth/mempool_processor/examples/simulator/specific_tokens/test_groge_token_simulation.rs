/// Test Buy/Approve/Sell Simulation for GROGE Token (Grok Doge)
/// 
/// This tests the simulation of buy/approve/sell sequence for the GROGE token
/// at block 23066897 where it was created with liquidity.
///
/// Token: 0x01d103f117C99E5c99C32aa6C19F4861EF7A60b7 (GROGE)
/// Pool: 0xcd9461b220a3b872ef2d82c34578f273f97f78ae (Uniswap V2)
/// Block: 23066897
/// Liquidity: 980M GROGE + 1.5 ETH

use mempool_processor::simulator::UnifiedSimulator;
use std::sync::Arc;
use tracing::{info, error};
use alloy_primitives::Address;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("=== Testing Buy/Approve/Sell for GROGE Token ===");
    info!("Token created 4 hours ago with 980M GROGE + 1.5 ETH liquidity");
    
    // Token and pool addresses from the transaction
    let token_address = "0x01d103f117C99E5c99C32aa6C19F4861EF7A60b7"
        .parse::<Address>()?;
    let pool_address = "0xcd9461b220a3b872ef2d82c34578f273f97f78ae"
        .parse::<Address>()?;
    let block_number = 23066897u64; // Creation block
    
    info!("Token Address: {:?}", token_address);
    info!("Pool Address: {:?}", pool_address);
    info!("Block Number: {}", block_number);
    
    // Initialize simulator
    let datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    info!("Using Reth datadir: {}", datadir);
    
    // Create unified simulator
    let unified_simulator = Arc::new(
        UnifiedSimulator::new(&datadir)?
    );
    
    // Test at creation block
    info!("\n🚀 Test 1: Buy/Sell at creation block {}...", block_number);
    
    match unified_simulator.simulate_buy_sell_sequence(
        token_address,
        pool_address,
        Some(block_number)
    ).await {
        Ok(result) => {
            display_results(&result, "Creation Block");
        }
        Err(e) => {
            error!("❌ Simulation failed: {}", e);
        }
    }
    
    // Test at a later block (current - 1000)
    let latest_block = unified_simulator.get_latest_block()?;
    let recent_block = latest_block.saturating_sub(1000);
    
    info!("\n🚀 Test 2: Buy/Sell at recent block {}...", recent_block);
    
    match unified_simulator.simulate_buy_sell_sequence(
        token_address,
        pool_address,
        Some(recent_block)
    ).await {
        Ok(result) => {
            display_results(&result, "Recent Block");
        }
        Err(e) => {
            error!("❌ Simulation failed: {}", e);
        }
    }
    
    Ok(())
}

fn display_results(result: &mempool_processor::simulator::SequenceSimulationResult, test_name: &str) {
    info!("\n✅ {} - Simulation completed!", test_name);
    info!("\n📊 Results:");
    info!("  Buy Transaction:");
    info!("    - Success: {}", result.buy_result.success);
    info!("    - Gas Used: {}", result.buy_result.gas_used);
    if let Some(reason) = &result.buy_result.revert_reason {
        info!("    - Revert Reason: {}", reason);
    }
    
    info!("\n  Approve Transaction:");
    info!("    - Success: {}", result.approve_result.success);
    info!("    - Gas Used: {}", result.approve_result.gas_used);
    if let Some(reason) = &result.approve_result.revert_reason {
        info!("    - Revert Reason: {}", reason);
    }
    
    info!("\n  Sell Transaction:");
    info!("    - Success: {}", result.sell_result.success);
    info!("    - Gas Used: {}", result.sell_result.gas_used);
    if let Some(reason) = &result.sell_result.revert_reason {
        info!("    - Revert Reason: {}", reason);
    }
    
    info!("\n  Simulation Time: {:.2}ms", result.simulation_time_ms);
    
    // Check state changes
    let buyer_address = Address::from([0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 0xAB, 0x20, 
                                      0x93, 0xE5, 0xd7, 0x2D, 0x80, 0x4a, 0x24, 0xbd, 0x56, 0x89]);
    
    // Check buy state changes
    if let Some(buyer_changes) = result.buy_result.state_changes.get(&buyer_address) {
        info!("\n  Buy State Changes:");
        // Find GROGE token balance (use checksummed address comparison)
        let groge_token_checksum = "0x01D103F117C99E5C99c32Aa6C19F4861ef7a60b7";
        for (token_addr, balance) in &buyer_changes.token_net {
            if token_addr == groge_token_checksum {
                info!("    - GROGE Balance Change: {}", balance);
            }
        }
        info!("    - ETH Balance Change: {}", buyer_changes.eth_net);
    }
    
    // Summary
    info!("\n📈 Summary:");
    info!("  Can Buy: {}", result.buy_result.success);
    info!("  Can Sell: {}", result.sell_result.success);
    
    if result.buy_result.success && result.sell_result.success {
        info!("\n✅ Token is tradeable - both buy and sell work!");
    } else if result.buy_result.success && !result.sell_result.success {
        error!("\n⚠️  Honeypot detected - can buy but cannot sell!");
    } else if !result.buy_result.success {
        error!("\n❌ Cannot even buy the token!");
    }
}