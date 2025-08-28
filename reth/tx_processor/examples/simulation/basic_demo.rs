/// Sequential Transaction Simulation Demo
/// 
/// This example demonstrates how to simulate a sequence of transactions where each
/// transaction builds on the state changes from previous ones. This is essential for:
/// 
/// - MEV simulation (bundle validation)
/// - Protocol testing (enable trading -> swap scenarios)
/// - Complex DeFi interactions
/// - Transaction dependency analysis
/// 
/// The example shows:
/// 1. Basic sequential simulation
/// 2. How state changes carry forward
/// 3. Nonce management across the sequence
/// 4. Handling transaction failures in sequences

use tx_processor::{RethTxSimulator, CallRequest, SequentialSimulationOptions, TxProcessor};
use alloy_primitives::{Address, U256, Bytes};
use eyre::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n🔄 Sequential Transaction Simulation Demo");
    println!("=========================================\n");
    
    // Initialize the simulator
    let simulator = RethTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    println!("✅ Simulator initialized");
    
    let latest_block = simulator.get_latest_block()?;
    println!("📊 Latest block: {}", latest_block);
    
    // Example 1: Simple ETH transfer sequence
    println!("\n📋 Example 1: Simple ETH Transfer Sequence");
    println!("-------------------------------------------");
    
    let whale_address = Address::from_str("0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5")?; // Known whale
    let recipient1 = Address::from_str("0x388C818CA8B9251b393131C08a736A67ccB19297")?; // Lido treasury
    let recipient2 = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?; // USDC contract
    
    let sequence1 = vec![
        CallRequest {
            from: Some(whale_address),
            to: Some(recipient1),
            value: Some(U256::from(100_000_000_000_000_000u64)), // 0.1 ETH
            gas: Some(21000),
            gas_price: Some(20_000_000_000), // 20 gwei
            data: None,
            nonce: None, // Auto-detected
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        },
        CallRequest {
            from: Some(whale_address),
            to: Some(recipient2),
            value: Some(U256::from(200_000_000_000_000_000u64)), // 0.2 ETH
            gas: Some(21000),
            gas_price: Some(20_000_000_000), // 20 gwei
            data: None,
            nonce: None, // Auto-incremented from previous
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        },
    ];
    
    println!("🔄 Simulating sequence of {} transactions...", sequence1.len());
    
    let result1 = simulator.simulate_transaction_sequence(
        sequence1,
        SequentialSimulationOptions::default()
    ).await?;
    
    println!("\n📊 Sequence Results:");
    println!("  Total transactions: {}", result1.total_transactions);
    println!("  Successful: {}", result1.successful_transactions);
    println!("  Failed: {}", result1.failed_transactions);
    println!("  Total gas used: {}", result1.total_gas_used);
    println!("  Sequence success: {}", result1.sequence_success);
    
    // Show individual transaction results
    for (i, tx_result) in result1.results.iter().enumerate() {
        println!("\n  Transaction {}: {}", 
            i + 1, 
            if tx_result.success { "✅ SUCCESS" } else { "❌ FAILED" }
        );
        println!("    Gas used: {}", tx_result.gas_used);
        println!("    Cumulative gas: {}", tx_result.cumulative_gas_used);
        if let Some(reason) = &tx_result.revert_reason {
            println!("    Revert reason: {}", reason);
        }
        
        // Show state changes for each transaction
        println!("    Address balance changes: {} addresses affected", tx_result.address_balance_changes.len());
        for (addr, changes) in &tx_result.address_balance_changes {
            if let Some(eth_net) = changes.currency_net.get("ETH") {
                let eth_value = eth_net.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                println!("      {}: ETH {:+.18}", addr, eth_value);
            }
        }
    }
    
    // Example 2: Contract interaction sequence (approve + transfer pattern)
    println!("\n\n📋 Example 2: ERC20 Approve + Transfer Sequence");
    println!("-----------------------------------------------");
    
    let usdc_address = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?; // USDC
    let spender = Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D")?; // Uniswap V2 Router
    let amount_to_approve = U256::from(1000_000_000u64); // 1000 USDC (6 decimals)
    
    // Encode approve(spender, amount) call
    let approve_calldata = {
        let function_selector = "095ea7b3"; // approve(address,uint256)
        let spender_padded = format!("{:0>64}", hex::encode(spender.as_slice()));
        let amount_padded = format!("{:0>64x}", amount_to_approve);
        format!("{}{}{}", function_selector, spender_padded, amount_padded)
    };
    
    // Encode transfer(to, amount) call
    let transfer_calldata = {
        let function_selector = "a9059cbb"; // transfer(address,uint256)
        let recipient_padded = format!("{:0>64}", hex::encode(recipient1.as_slice()));
        let amount_padded = format!("{:0>64x}", U256::from(500_000_000u64)); // 500 USDC
        format!("{}{}{}", function_selector, recipient_padded, amount_padded)
    };
    
    let sequence2 = vec![
        CallRequest {
            from: Some(whale_address),
            to: Some(usdc_address),
            value: Some(U256::ZERO),
            gas: Some(100000),
            gas_price: Some(20_000_000_000),
            data: Some(Bytes::from_str(&format!("0x{}", approve_calldata))?),
            nonce: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        },
        CallRequest {
            from: Some(whale_address),
            to: Some(usdc_address),
            value: Some(U256::ZERO),
            gas: Some(100000),
            gas_price: Some(20_000_000_000),
            data: Some(Bytes::from_str(&format!("0x{}", transfer_calldata))?),
            nonce: None, // Auto-incremented
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        },
    ];
    
    println!("🔄 Simulating ERC20 approve + transfer sequence...");
    
    let result2 = simulator.simulate_transaction_sequence(
        sequence2,
        SequentialSimulationOptions::default()
    ).await?;
    
    println!("\n📊 ERC20 Sequence Results:");
    println!("  Total transactions: {}", result2.total_transactions);
    println!("  Successful: {}", result2.successful_transactions);
    println!("  Failed: {}", result2.failed_transactions);
    println!("  Sequence success: {}", result2.sequence_success);
    
    for (i, tx_result) in result2.results.iter().enumerate() {
        let tx_type = if i == 0 { "APPROVE" } else { "TRANSFER" };
        println!("\n  {} Transaction: {}", 
            tx_type,
            if tx_result.success { "✅ SUCCESS" } else { "❌ FAILED" }
        );
        println!("    Gas used: {}", tx_result.gas_used);
        if let Some(reason) = &tx_result.revert_reason {
            println!("    Revert reason: {}", reason);
        }
        
        // Show token movements
        for (addr, changes) in &tx_result.address_balance_changes {
            if !changes.token_net.is_empty() {
                println!("    {}: Token changes:", addr);
                for (token, amount) in &changes.token_net {
                    println!("      {}: {:+.6}", token, amount);
                }
            }
        }
    }
    
    // Example 3: Demonstrate stop_on_failure option
    println!("\n\n📋 Example 3: Stop on Failure Behavior");
    println!("-------------------------------------");
    
    let failing_sequence = vec![
        CallRequest {
            from: Some(whale_address),
            to: Some(recipient1),
            value: Some(U256::from(1000_000_000_000_000_000u64)), // 1 ETH - should succeed
            gas: Some(21000),
            gas_price: Some(20_000_000_000),
            data: None,
            nonce: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        },
        CallRequest {
            from: Some(whale_address),
            to: Some(recipient1),
            value: U256::from_str("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")?.into(), // Max uint256 - should fail
            gas: Some(21000),
            gas_price: Some(20_000_000_000),
            data: None,
            nonce: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        },
        CallRequest {
            from: Some(whale_address),
            to: Some(recipient2),
            value: Some(U256::from(100_000_000_000_000_000u64)), // 0.1 ETH - would succeed but won't be reached
            gas: Some(21000),
            gas_price: Some(20_000_000_000),
            data: None,
            nonce: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        },
    ];
    
    let options_stop_on_failure = SequentialSimulationOptions {
        stop_on_failure: true,
        ..Default::default()
    };
    
    println!("🔄 Simulating sequence with stop_on_failure=true...");
    
    let result3 = simulator.simulate_transaction_sequence(
        failing_sequence.clone(),
        options_stop_on_failure
    ).await?;
    
    println!("\n📊 Stop on Failure Results:");
    println!("  Total transactions processed: {}", result3.total_transactions);
    println!("  Successful: {}", result3.successful_transactions);
    println!("  Failed: {}", result3.failed_transactions);
    println!("  Sequence success: {}", result3.sequence_success);
    
    // Compare with continue_on_failure
    let options_continue_on_failure = SequentialSimulationOptions {
        stop_on_failure: false,
        ..Default::default()
    };
    
    println!("\n🔄 Simulating same sequence with stop_on_failure=false...");
    
    let result4 = simulator.simulate_transaction_sequence(
        failing_sequence,
        options_continue_on_failure
    ).await?;
    
    println!("\n📊 Continue on Failure Results:");
    println!("  Total transactions processed: {}", result4.total_transactions);
    println!("  Successful: {}", result4.successful_transactions);
    println!("  Failed: {}", result4.failed_transactions);
    println!("  Sequence success: {}", result4.sequence_success);
    
    println!("\n✅ Sequential simulation demo completed!");
    println!("\n💡 Key Takeaways:");
    println!("  - Each transaction builds on the state of previous ones");
    println!("  - Nonces are automatically managed across the sequence");
    println!("  - You can control whether to stop or continue on failures");
    println!("  - State changes accumulate throughout the sequence");
    println!("  - Perfect for MEV bundle validation and protocol testing");
    
    Ok(())
}