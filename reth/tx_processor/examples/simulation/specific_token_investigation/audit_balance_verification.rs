/// Audit Balance Verification Logic
/// 
/// This example audits whether balanceOf() contract calls give the exact same results
/// as address balance changes from ProcessedTransaction. We'll examine:
/// 1. How address balance changes are calculated in ProcessedTransaction
/// 2. How balanceOf() calls work in simulation
/// 3. Whether both methods truly return identical values
/// 4. Any potential discrepancies between the two approaches

use eyre::Result;
use alloy_primitives::{Address, U256, Bytes};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::{TxSimulator, UnsignedTransaction};
use std::str::FromStr;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";
const FLOKI_ADDRESS: &str = "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E";
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Auditing Balance Verification Logic");
    println!("=====================================");
    
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    let tx_processor = TxProcessor::new();
    
    // Use latest block for simulation
    let latest_block = simulator.get_latest_block()?;
    println!("📦 Using block: {}", latest_block);
    
    // Create a simulation chain to persist state
    let mut chain = simulator.start_simulation_chain(Some(latest_block)).await?;
    
    // Test address (will be our "buyer")
    let buyer = Address::from_str("0x1000000000000000000000000000000000000001")?;
    
    // Note: Using existing on-chain balance; ensure buyer has sufficient ETH in state
    
    println!("\n=== PHASE 1: Pre-Buy State ===");
    
    // Check initial FLOKI balance
    let initial_balance = check_floki_balance(&mut chain, buyer).await?;
    println!("Initial FLOKI balance: {} raw", initial_balance);
    
    println!("\n=== PHASE 2: Execute Buy Transaction ===");
    
    // Create a buy transaction (swap ETH for FLOKI)
    let router = Address::from_str(UNISWAP_V2_ROUTER)?;
    let floki_addr = Address::from_str(FLOKI_ADDRESS)?;
    let weth_addr = Address::from_str(WETH_ADDRESS)?;
    
    // swapExactETHForTokensSupportingFeeOnTransferTokens
    let swap_selector = [0xb6, 0xf9, 0xde, 0x95]; // swapExactETHForTokensSupportingFeeOnTransferTokens selector
    
    // Parameters for the swap
    let amount_out_min = U256::ZERO; // Accept any amount
    let deadline = U256::from(2000000000u64); // Far future timestamp
    let eth_to_spend = U256::from(1u64) * U256::from(10u64).pow(U256::from(17)); // 0.1 ETH
    
    // Path: [WETH, FLOKI]
    let path_data = {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 32 - 20]); // Padding for address
        data.extend_from_slice(weth_addr.as_slice());
        data.extend_from_slice(&[0u8; 32 - 20]); // Padding for address  
        data.extend_from_slice(floki_addr.as_slice());
        data
    };
    
    // ABI encode the function call
    let mut call_data = Vec::new();
    call_data.extend_from_slice(&swap_selector);
    
    // amountOutMin (uint256)
    let amount_out_min_bytes = amount_out_min.to_be_bytes::<32>();
    call_data.extend_from_slice(&amount_out_min_bytes);
    
    // path offset (uint256) - points to start of path array
    call_data.extend_from_slice(&[0u8; 31]);
    call_data.push(0x80); // 128 bytes offset
    
    // to address (address)
    call_data.extend_from_slice(&[0u8; 12]);
    call_data.extend_from_slice(buyer.as_slice());
    
    // deadline (uint256)
    let deadline_bytes = deadline.to_be_bytes::<32>();
    call_data.extend_from_slice(&deadline_bytes);
    
    // Path array: [length, weth_address, floki_address]
    call_data.extend_from_slice(&[0u8; 31]);
    call_data.push(0x02); // Array length = 2
    call_data.extend_from_slice(&path_data);
    
    let buy_tx = UnsignedTransaction {
        from: Some(buyer),
        to: Some(router),
        value: Some(eth_to_spend),
        data: Some(Bytes::from(call_data)),
        gas: Some(300_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    // Execute the buy transaction
    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;
    
    if !buy_result.success {
        println!("❌ Buy transaction failed: {:?}", buy_result.revert_reason);
        return Ok(());
    }
    
    println!("✅ Buy transaction succeeded!");
    println!("  Gas used: {}", buy_result.gas_used);
    
    println!("\n=== PHASE 3: Process Transaction ===");
    
    // Generate ProcessedTransaction from the simulation result
    let processed_tx = tx_processor.process_transaction_from_simulation_result(
        &buy_tx,
        &buy_result, 
        latest_block,
        0, // tx_index
    ).await?;
    
    // Extract balance changes from ProcessedTransaction
    let floki_address = Address::from_str(FLOKI_ADDRESS)?;
    let balance_change_floki = processed_tx.get_address_token_balance_change(&buyer, &floki_address)
        .unwrap_or(U256::ZERO);
    
    let balance_change_eth = processed_tx.get_address_currency_balance_change(&buyer, "ETH")
        .unwrap_or(U256::ZERO);
    
    println!("\n📊 ProcessedTransaction Balance Changes:");
    println!("  FLOKI balance change: {} raw", balance_change_floki);
    println!("  ETH balance change: {} raw", balance_change_eth);
    
    // Also examine the raw address_balance_changes data
    println!("\n🔍 Raw address_balance_changes data:");
    if let Some(changes) = processed_tx.address_balance_changes.get(&buyer) {
        println!("  Currency changes: {:?}", changes.currency_net);
        println!("  Token changes: {:?}", changes.token_net);
    } else {
        println!("  No balance changes found for buyer address!");
    }
    
    // Check ERC20 transfers as comparison
    println!("\n📤 ERC20 Transfers to buyer:");
    for transfer in &processed_tx.erc20_transfers {
        if transfer.to_address == buyer {
            println!("  Token: {}", transfer.token_address);
            println!("  Amount: {} raw", transfer.amount);
            
            if transfer.token_address.to_string().to_lowercase() == FLOKI_ADDRESS.to_lowercase() {
                println!("  ✅ This is FLOKI transfer to buyer");
            }
        }
    }
    
    println!("\n=== PHASE 4: Verify with balanceOf() Call ===");
    
    // Check FLOKI balance using balanceOf() 
    let actual_balance = check_floki_balance(&mut chain, buyer).await?;
    let net_balance_gained = actual_balance - initial_balance;
    
    println!("\n💰 Balance Verification Results:");
    println!("  Initial balance: {} raw", initial_balance);
    println!("  Final balance: {} raw", actual_balance);
    println!("  Net gained: {} raw", net_balance_gained);
    println!("  ProcessedTransaction says: {} raw", balance_change_floki);
    
    println!("\n🔍 Detailed Comparison:");
    if net_balance_gained == balance_change_floki {
        println!("  ✅ PERFECT MATCH: Both methods agree exactly");
        println!("     balanceOf() net change: {}", net_balance_gained);
        println!("     ProcessedTransaction:    {}", balance_change_floki);
    } else {
        println!("  ❌ DISCREPANCY DETECTED:");
        println!("     balanceOf() net change: {}", net_balance_gained);
        println!("     ProcessedTransaction:    {}", balance_change_floki);
        println!("     Difference: {}", balance_change_floki.abs_diff(net_balance_gained));
        
        // Calculate percentage difference if both values are non-zero
        if balance_change_floki > U256::ZERO && net_balance_gained > U256::ZERO {
            let diff_pct = if balance_change_floki > net_balance_gained {
                (balance_change_floki - net_balance_gained) * U256::from(10000) / balance_change_floki
            } else {
                (net_balance_gained - balance_change_floki) * U256::from(10000) / net_balance_gained
            };
            println!("     Difference: {}.{}% of larger value", diff_pct / U256::from(100), diff_pct % U256::from(100));
        }
    }
    
    println!("\n=== PHASE 5: Deep Analysis ===");
    
    // Check if the FLOKI token has any special behavior
    println!("\n🔍 Analyzing FLOKI token characteristics:");
    
    // Check total supply
    let total_supply = check_floki_total_supply(&mut chain).await?;
    println!("  Total supply: {} raw", total_supply);
    
    // Check decimals
    let decimals = check_floki_decimals(&mut chain).await?;
    println!("  Decimals: {}", decimals);
    
    // Format the amounts with correct decimals
    let format_floki = |amount: U256| {
        let decimal_divisor = U256::from(10u64).pow(U256::from(decimals));
        let formatted = amount.to_string().parse::<f64>().unwrap_or(0.0) / decimal_divisor.to_string().parse::<f64>().unwrap_or(1.0);
        format!("{:.6}", formatted)
    };
    
    println!("\n📈 Formatted Amounts:");
    println!("  balanceOf() net change: {} FLOKI", format_floki(net_balance_gained));
    println!("  ProcessedTransaction:   {} FLOKI", format_floki(balance_change_floki));
    
    // Final assessment
    println!("\n=== FINAL ASSESSMENT ===");
    if net_balance_gained == balance_change_floki {
        println!("🎉 CONCLUSION: The balance verification is working correctly!");
        println!("   Both balanceOf() and ProcessedTransaction address balance changes");
        println!("   return identical values with perfect precision.");
    } else {
        println!("⚠️  CONCLUSION: There is a discrepancy between the two methods!");
        println!("   This suggests either:");
        println!("   1. ProcessedTransaction balance change calculation has a bug");
        println!("   2. balanceOf() simulation has an issue");
        println!("   3. There's some edge case with FLOKI token behavior");
        println!("   4. State differences between simulation steps");
    }
    
    Ok(())
}

/// Check FLOKI balance using balanceOf() contract call
async fn check_floki_balance(chain: &mut tx_simulator::UnsignedTxChainSimulation, address: Address) -> Result<U256> {
    let balance_check_data = {
        let mut data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(address.as_slice());
        data
    };
    
    let balance_call = UnsignedTransaction {
        from: Some(address),
        to: Some(Address::from_str(FLOKI_ADDRESS)?),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(balance_check_data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    let balance_result = chain.step_with_trace(balance_call).await?;
    if let Some(output) = balance_result.call_trace.output {
        if output.len() >= 32 {
            return Ok(U256::from_be_slice(&output[0..32]));
        }
    }
    
    Ok(U256::ZERO)
}

/// Check FLOKI total supply
async fn check_floki_total_supply(chain: &mut tx_simulator::UnsignedTxChainSimulation) -> Result<U256> {
    let total_supply_data = vec![0x18, 0x16, 0x0d, 0xdd]; // totalSupply selector
    
    let call = UnsignedTransaction {
        from: Some(Address::ZERO),
        to: Some(Address::from_str(FLOKI_ADDRESS)?),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(total_supply_data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    let result = chain.step_with_trace(call).await?;
    if let Some(output) = result.call_trace.output {
        if output.len() >= 32 {
            return Ok(U256::from_be_slice(&output[0..32]));
        }
    }
    
    Ok(U256::ZERO)
}

/// Check FLOKI decimals
async fn check_floki_decimals(chain: &mut tx_simulator::UnsignedTxChainSimulation) -> Result<u8> {
    let decimals_data = vec![0x31, 0x3c, 0xe5, 0x67]; // decimals selector
    
    let call = UnsignedTransaction {
        from: Some(Address::ZERO),
        to: Some(Address::from_str(FLOKI_ADDRESS)?),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(decimals_data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    let result = chain.step_with_trace(call).await?;
    if let Some(output) = result.call_trace.output {
        if output.len() >= 32 {
            return Ok(output[31]);
        }
    }
    
    Ok(18) // Default to 18 if call fails
}
