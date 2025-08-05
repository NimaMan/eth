/// Test IMAG Token Sell Transaction Simulation
/// 
/// This tests the simulation of the actual sell transaction that happened at block 23032100
/// Transaction: 0xe269d8e9d2ec58c28824ee2fa9974c13cde1f2b9f1e484c878662260d4350d04
///
/// The transaction shows two sells:
/// - 493,119.42 IMAG → 0.055149209 ETH
/// - 473,394.64 IMAG → 0.051572481 ETH

use mempool_processor::simulator::UnifiedSimulator;
use std::sync::Arc;
use tracing::{info, error};
use alloy_primitives::{Address, U256, Bytes};
use reth_tx_simulator::CallRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("=== Testing IMAG Token Sell Transaction at Block 23032100 ===");
    
    // Token and pool addresses
    let token_address = "0x7EAa8d0DdeC2B0427cca190C8c360ff49c88d257"
        .parse::<Address>()?;
    let pool_address = "0x3032a580cb8b3368160400cc489845d0dd8b5646"
        .parse::<Address>()?;
    let seller_address = "0x4bB39f71e3154A495442A0dd65B9504f8C69F8cE"
        .parse::<Address>()?;
    
    // Two different blocks to test
    let block_before_sell = 23032099u64; // Right before the sell
    let block_of_sell = 23032100u64;     // The sell transaction block
    
    info!("Token Address: {:?}", token_address);
    info!("Pool Address: {:?}", pool_address);
    info!("Seller Address: {:?}", seller_address);
    
    // Initialize simulator
    let datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    let unified_simulator = Arc::new(
        UnifiedSimulator::new(&datadir)?
    );
    
    // First, let's test if our test buyer can buy/sell at block 23032099 (before the actual sell)
    info!("\n📊 Test 1: Buy/Sell simulation at block {} (before the actual sell)", block_before_sell);
    
    match unified_simulator.simulate_buy_sell_sequence(
        token_address,
        pool_address,
        Some(block_before_sell)
    ).await {
        Ok(result) => {
            info!("  Buy: {} (Gas: {})", 
                if result.buy_result.success { "✅ SUCCESS" } else { "❌ FAILED" },
                result.buy_result.gas_used
            );
            info!("  Sell: {} (Gas: {})", 
                if result.sell_result.success { "✅ SUCCESS" } else { "❌ FAILED" },
                result.sell_result.gas_used
            );
            if let Some(reason) = &result.sell_result.revert_reason {
                info!("  Sell Revert: {}", reason);
            }
        }
        Err(e) => error!("Simulation failed: {}", e),
    }
    
    // Now let's simulate the actual sell transaction
    info!("\n📊 Test 2: Simulating the actual sell transaction");
    info!("  Creating sell transaction for the seller address...");
    
    // Create a sell transaction similar to what happened
    // This would be swapExactTokensForETH
    let sell_tx = create_sell_transaction(
        seller_address,
        token_address,
        U256::from(493119418366027u64), // Amount of IMAG to sell
    );
    
    // Test buy/sell with this transaction at block 23032100
    match unified_simulator.simulate_sequence_with_tx(
        Some(sell_tx),
        token_address,
        pool_address,
        Some(block_of_sell)
    ).await {
        Ok(result) => {
            info!("\n  Results at block {}:", block_of_sell);
            
            // Check the given transaction (the sell)
            if let Some(given_tx) = &result.given_tx_result {
                info!("  Given TX (Sell): {} (Gas: {})",
                    if given_tx.success { "✅ SUCCESS" } else { "❌ FAILED" },
                    given_tx.gas_used
                );
                if let Some(reason) = &given_tx.revert_reason {
                    info!("  Given TX Revert: {}", reason);
                }
            }
            
            info!("  Buy: {} (Gas: {})", 
                if result.buy_result.success { "✅ SUCCESS" } else { "❌ FAILED" },
                result.buy_result.gas_used
            );
            info!("  Sell: {} (Gas: {})", 
                if result.sell_result.success { "✅ SUCCESS" } else { "❌ FAILED" },
                result.sell_result.gas_used
            );
        }
        Err(e) => error!("Simulation failed: {}", e),
    }
    
    // Also test at the exact block but with just buy/sell
    info!("\n📊 Test 3: Buy/Sell simulation at block {} (the sell transaction block)", block_of_sell);
    
    match unified_simulator.simulate_buy_sell_sequence(
        token_address,
        pool_address,
        Some(block_of_sell)
    ).await {
        Ok(result) => {
            info!("  Buy: {} (Gas: {})", 
                if result.buy_result.success { "✅ SUCCESS" } else { "❌ FAILED" },
                result.buy_result.gas_used
            );
            info!("  Sell: {} (Gas: {})", 
                if result.sell_result.success { "✅ SUCCESS" } else { "❌ FAILED" },
                result.sell_result.gas_used
            );
            if let Some(reason) = &result.sell_result.revert_reason {
                info!("  Sell Revert: {}", reason);
            }
            
            // Check if it's still a honeypot
            if result.buy_result.success && !result.sell_result.success {
                error!("\n🚨 Still a honeypot at block {} despite real sells happening!", block_of_sell);
            } else if result.buy_result.success && result.sell_result.success {
                info!("\n✅ Token is tradeable at block {}!", block_of_sell);
            }
        }
        Err(e) => error!("Simulation failed: {}", e),
    }
    
    Ok(())
}

/// Create a sell transaction (swapExactTokensForETH)
fn create_sell_transaction(
    from: Address,
    token_address: Address,
    amount_in: U256,
) -> CallRequest {
    let router_address = Address::from([0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 
                                       0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6, 
                                       0x59, 0xF2, 0x48, 0x8D]); // Uniswap V2 Router
    let weth_address = Address::from([0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 
                                      0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 
                                      0x3C, 0x75, 0x6C, 0xc2]); // WETH
    
    // swapExactTokensForETH function selector: 0x18cbafe5
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5];
    
    // amountIn
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    
    // amountOutMin (0 for this test)
    data.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
    
    // path offset
    data.extend_from_slice(&U256::from(160).to_be_bytes::<32>());
    
    // to address (seller)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(from.as_slice());
    
    // deadline
    data.extend_from_slice(&U256::from(9999999999u64).to_be_bytes::<32>());
    
    // path array
    data.extend_from_slice(&U256::from(2).to_be_bytes::<32>());
    
    // path[0] = token
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(token_address.as_slice());
    
    // path[1] = WETH
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(weth_address.as_slice());
    
    CallRequest {
        from: Some(from),
        to: Some(router_address),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(300_000),
        gas_price: Some(20_000_000_000u128),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: None,
    }
}