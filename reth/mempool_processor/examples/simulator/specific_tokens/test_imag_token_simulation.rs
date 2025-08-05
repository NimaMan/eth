/// Test Buy/Approve/Sell Simulation for IMAG Token
/// 
/// This tests the simulation of buy/approve/sell sequence for the IMAG token
/// at block 23031190 where it was created with liquidity.
///
/// Token: 0x7EAa8d0DdeC2B0427cca190C8c360ff49c88d257 (IMAG)
/// Pool: 0x3032a580cb8b3368160400cc489845d0dd8b5646 (Uniswap V2)
/// Block: 23031190

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

    info!("=== Testing Buy/Approve/Sell for IMAG Token ===");
    
    // Token and pool addresses from the transaction
    let token_address = "0x7EAa8d0DdeC2B0427cca190C8c360ff49c88d257"
        .parse::<Address>()?;
    let pool_address = "0x3032a580cb8b3368160400cc489845d0dd8b5646"
        .parse::<Address>()?;
    let block_number = 23031190u64;
    
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
    
    info!("\n🚀 Starting sequential simulation at block {}...", block_number);
    
    // Run the simulation sequence: Buy -> Approve -> Sell
    match unified_simulator.simulate_buy_sell_sequence(
        token_address,
        pool_address,
        Some(block_number)
    ).await {
        Ok(result) => {
            info!("\n✅ Simulation completed successfully!");
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
            info!("  Block Number: {}", result.block_number);
            
            // Check state changes
            let buyer_address = unified_simulator.get_buyer_address();
            info!("\n  Buyer Address: {:?}", buyer_address);
            
            // Check buy state changes
            if let Some(buyer_changes) = result.buy_result.state_changes.get(&buyer_address) {
                info!("\n  Buy State Changes for Buyer:");
                let token_addr_str = format!("{:#x}", token_address);
                if let Some(token_balance) = buyer_changes.token_net.get(&token_addr_str) {
                    info!("    - Token Balance Change: {}", token_balance);
                }
                info!("    - ETH Balance Change: {}", buyer_changes.eth_net);
            }
            
            // Check sell state changes
            if let Some(buyer_changes) = result.sell_result.state_changes.get(&buyer_address) {
                info!("\n  Sell State Changes for Buyer:");
                let token_addr_str = format!("{:#x}", token_address);
                if let Some(token_balance) = buyer_changes.token_net.get(&token_addr_str) {
                    info!("    - Token Balance Change: {}", token_balance);
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
            } else {
                error!("\n❌ Token is not tradeable");
            }
            
        }
        Err(e) => {
            error!("❌ Simulation failed: {}", e);
            error!("Error details: {:?}", e);
        }
    }
    
    Ok(())
}