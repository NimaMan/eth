/// Test the BuySellSimulator with known tokens and print state changes
/// 
/// This example tests the refactored buy_sell_sequence_simulator module with:
/// - AITAI token (should work - no honeypot)
/// - 0xT token (should fail sell - honeypot)
/// 
/// It prints the raw state changes from buy and sell transactions

use mempool_processor::simulator::{
    SequentialBuySellSimulator,
    BuySellSimulatorConfig,
};
use mempool_processor::token_parameter_extraction::{calculate_buy_tax, calculate_sell_tax};
use alloy_primitives::{Address, U256, I256};
use std::str::FromStr;
use eyre::Result;

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

fn print_state_changes(state_changes: &std::collections::HashMap<Address, reth_tx_simulator::AddressStateChange>, title: &str) {
    println!("\n   {} State Changes:", title);
    println!("   {}", "=".repeat(60));
    
    for (address, changes) in state_changes {
        println!("   Address: {}", address);
        
        // ETH changes
        if changes.eth_net != I256::ZERO {
            let (sign, amount) = if changes.eth_net.is_negative() {
                ("-", changes.eth_net.unsigned_abs())
            } else {
                ("+", changes.eth_net.unsigned_abs())
            };
            println!("     ETH: {}{} ETH", sign, format_token_amount(amount, 18));
        }
        
        // Token changes
        for (token_addr, token_change) in &changes.token_net {
            if *token_change != I256::ZERO {
                let (sign, amount) = if token_change.is_negative() {
                    ("-", token_change.unsigned_abs())
                } else {
                    ("+", token_change.unsigned_abs())
                };
                println!("     Token {}: {}{}", token_addr, sign, amount);
            }
        }
        
        // Show movements summary
        if !changes.movements.tokens.is_empty() || 
           !changes.movements.denom.incoming.is_empty() || 
           !changes.movements.denom.outgoing.is_empty() {
            println!("     Movements:");
            
            // ETH movements
            if !changes.movements.denom.incoming.is_empty() {
                println!("       ETH incoming: {} transfers", changes.movements.denom.incoming.len());
            }
            if !changes.movements.denom.outgoing.is_empty() {
                println!("       ETH outgoing: {} transfers", changes.movements.denom.outgoing.len());
            }
            
            // Token movements
            for (token_addr, token_movements) in &changes.movements.tokens {
                if !token_movements.incoming.is_empty() {
                    println!("       Token {} incoming: {} transfers", token_addr, token_movements.incoming.len());
                }
                if !token_movements.outgoing.is_empty() {
                    println!("       Token {} outgoing: {} transfers", token_addr, token_movements.outgoing.len());
                }
            }
        }
    }
    println!("   {}", "=".repeat(60));
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simulator with default config
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let config = BuySellSimulatorConfig::default();
    let simulator = SequentialBuySellSimulator::with_config(reth_datadir, config.clone())?;
    
    println!("🚀 Testing BuySellSimulator with known tokens\n");
    println!("Configuration:");
    println!("  Buyer Address: {}", config.buyer_address);
    println!("  Test Buy Amount: {} ETH", format_token_amount(config.test_buy_amount, 18));
    println!("  Router: {}", config.router_address);
    
    // Test 1: AITAI token (should work)
    println!("\n1️⃣ Testing AITAI token:");
    let aitai_token = Address::from_str("0xBCb2479ae9F271BB1561b4823dCaAc8B02860B1E")?;
    let aitai_pool = Address::from_str("0xa32d14c0d48ed4835179f33bc00d1bd7acea4aff")?;
    let aitai_block = 23005264;
    
    match simulator.simulate_sequence(aitai_token, aitai_pool, Some(aitai_block)).await {
        Ok(result) => {
            println!("   ✅ Simulation completed in {:.1}ms", result.simulation_time_ms);
            println!("   Block: {}", result.block_number);
            
            // Buy transaction results
            println!("\n   Buy Transaction:");
            println!("     Success: {}", result.buy_result.success);
            println!("     Gas Used: {}", result.buy_result.gas_used);
            if let Some(reason) = &result.buy_result.revert_reason {
                println!("     Revert Reason: {}", reason);
            }
            
            // Calculate and show buy tax
            if result.buy_result.success {
                if let Some(buy_tax) = calculate_buy_tax(
                    &result.buy_result.state_changes,
                    &aitai_pool,
                    &config.buyer_address,
                    &aitai_token,
                ) {
                    println!("     Buy Tax: {:.2}%", buy_tax);
                }
            }
            
            // Print buy state changes
            print_state_changes(&result.buy_result.state_changes, "Buy");
            
            // Sell transaction results
            println!("\n   Sell Transaction:");
            println!("     Success: {}", result.sell_result.success);
            println!("     Gas Used: {}", result.sell_result.gas_used);
            if let Some(reason) = &result.sell_result.revert_reason {
                println!("     Revert Reason: {}", reason);
            }
            
            // Calculate and show sell tax
            if result.sell_result.success {
                if let Some(sell_tax) = calculate_sell_tax(
                    &result.sell_result.state_changes,
                    &aitai_pool,
                    &config.buyer_address,
                ) {
                    println!("     Sell Tax: {:.2}%", sell_tax);
                }
            }
            
            // Print sell state changes
            print_state_changes(&result.sell_result.state_changes, "Sell");
            
            // Summary
            let is_honeypot = result.buy_result.success && !result.sell_result.success;
            println!("\n   Summary: {}", if is_honeypot { "🍯 HONEYPOT DETECTED" } else { "✅ Trading Enabled" });
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    println!("\n2️⃣ Testing 0xT token (known honeypot):");
    let zerot_token = Address::from_str("0x0Ca5f8f96C3a2C84190a415b658AaFc8501AaeFF")?;
    let zerot_pool = Address::from_str("0x885cf65E1511D50Bb49e488839525fDE44cDE36b")?;
    let zerot_block = 22954920;
    
    match simulator.simulate_sequence(zerot_token, zerot_pool, Some(zerot_block)).await {
        Ok(result) => {
            println!("   ✅ Simulation completed in {:.1}ms", result.simulation_time_ms);
            println!("   Block: {}", result.block_number);
            
            // Buy transaction results
            println!("\n   Buy Transaction:");
            println!("     Success: {}", result.buy_result.success);
            println!("     Gas Used: {}", result.buy_result.gas_used);
            if let Some(reason) = &result.buy_result.revert_reason {
                println!("     Revert Reason: {}", reason);
            }
            
            // Calculate and show buy tax
            if result.buy_result.success {
                if let Some(buy_tax) = calculate_buy_tax(
                    &result.buy_result.state_changes,
                    &zerot_pool,
                    &config.buyer_address,
                    &zerot_token,
                ) {
                    println!("     Buy Tax: {:.2}%", buy_tax);
                }
            }
            
            // Print buy state changes
            print_state_changes(&result.buy_result.state_changes, "Buy");
            
            // Sell transaction results
            println!("\n   Sell Transaction:");
            println!("     Success: {}", result.sell_result.success);
            println!("     Gas Used: {}", result.sell_result.gas_used);
            if let Some(reason) = &result.sell_result.revert_reason {
                println!("     Revert Reason: {}", reason);
            }
            
            // Calculate and show sell tax
            if result.sell_result.success {
                if let Some(sell_tax) = calculate_sell_tax(
                    &result.sell_result.state_changes,
                    &zerot_pool,
                    &config.buyer_address,
                ) {
                    println!("     Sell Tax: {:.2}%", sell_tax);
                }
            }
            
            // Print sell state changes
            print_state_changes(&result.sell_result.state_changes, "Sell");
            
            // Summary
            let is_honeypot = result.buy_result.success && !result.sell_result.success;
            println!("\n   Summary: {}", if is_honeypot { "🍯 HONEYPOT DETECTED" } else { "✅ Trading Enabled" });
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    println!("\n3️⃣ Testing at a later block for 0xT (where tax changed):");
    let zerot_block_later = 22954923;
    
    match simulator.simulate_sequence(zerot_token, zerot_pool, Some(zerot_block_later)).await {
        Ok(result) => {
            println!("   ✅ Simulation completed in {:.1}ms", result.simulation_time_ms);
            println!("   Block: {}", result.block_number);
            
            // Buy transaction results (show summary only for brevity)
            println!("\n   Buy Transaction: {}", if result.buy_result.success { "Success" } else { "Failed" });
            if result.buy_result.success {
                if let Some(buy_tax) = calculate_buy_tax(
                    &result.buy_result.state_changes,
                    &zerot_pool,
                    &config.buyer_address,
                    &zerot_token,
                ) {
                    println!("     Buy Tax: {:.2}%", buy_tax);
                }
            }
            
            // Sell transaction results
            println!("   Sell Transaction: {}", if result.sell_result.success { "Success" } else { "Failed" });
            if result.sell_result.success {
                if let Some(sell_tax) = calculate_sell_tax(
                    &result.sell_result.state_changes,
                    &zerot_pool,
                    &config.buyer_address,
                ) {
                    println!("     Sell Tax: {:.2}%", sell_tax);
                }
            }
            
            // Summary
            let is_honeypot = result.buy_result.success && !result.sell_result.success;
            println!("\n   Summary: {}", if is_honeypot { "🍯 HONEYPOT DETECTED" } else { "✅ Trading Enabled" });
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    Ok(())
}