use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use std::str::FromStr;
/// Unsigned Transaction Simulation Example
///
/// This example demonstrates various ways to simulate unsigned transactions,
/// mirroring Ethereum's `eth_call` semantics while reading state directly from
/// the local reth database for better performance.
use tx_simulator::{TxSimulator, UnsignedTransaction};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simulator
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let simulator = TxSimulator::new(reth_datadir)?;

    println!("🔄 Unsigned Transaction Simulation Example");
    println!("==========================================");

    // Get latest block for testing
    let latest_block = simulator.get_latest_block()?;
    println!("Latest block: {}", latest_block);

    // Example 1: Simple ETH transfer simulation
    println!("\n📝 Example 1: ETH Transfer Simulation");
    println!("{}", "-".repeat(40));

    let eth_call = UnsignedTransaction {
        from: Some("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689".parse()?),
        to: Some("0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?),
        value: Some(U256::from(1000000000000000u64)), // 0.001 ETH
        gas: Some(21000),
        gas_price: Some(20_000_000_000), // 20 gwei
        ..Default::default()
    };

    // Method 1: Basic simulation at latest block
    let result = simulator
        .simulate_unsigned_transaction(eth_call.clone())
        .await?;
    println!("✅ Basic simulation result:");
    println!("   Success: {}", result.success);
    println!("   Gas used: {}", result.gas_used);
    if let Some(reason) = &result.revert_reason {
        println!("   Revert reason: {}", reason);
    }

    // Method 2: Simulation at specific block
    let test_block = latest_block - 50;
    let result = simulator
        .simulate_unsigned_transaction_at_block(eth_call.clone(), test_block)
        .await?;
    println!("✅ Historical simulation (block {}):", test_block);
    println!("   Success: {}", result.success);
    println!("   Gas used: {}", result.gas_used);

    // Example 2: Contract interaction simulation
    println!("\n📝 Example 2: ERC20 Token Balance Query");
    println!("{}", "-".repeat(45));

    // balanceOf(address) function call
    let balance_of_selector = [0x70, 0xa0, 0x82, 0x31]; // balanceOf(address)
    let account_to_check: Address = "0x742d35cc6548c5b8a9f63c4c81d0e90e3e1d3d9e".parse()?;

    let mut call_data = Vec::new();
    call_data.extend_from_slice(&balance_of_selector);
    call_data.extend_from_slice(&[0u8; 12]); // Pad to 32 bytes
    call_data.extend_from_slice(account_to_check.as_slice());

    let balance_call = UnsignedTransaction {
        from: Some("0x0000000000000000000000000000000000000000".parse()?),
        to: Some("0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?), // USDC contract
        data: Some(Bytes::from(call_data)),
        gas: Some(50000),
        ..Default::default()
    };

    let result = simulator
        .simulate_unsigned_transaction(balance_call)
        .await?;
    println!("✅ Contract call result:");
    println!("   Success: {}", result.success);
    println!("   Gas used: {}", result.gas_used);

    // Example 3: EIP-1559 transaction simulation
    println!("\n📝 Example 3: EIP-1559 Transaction");
    println!("{}", "-".repeat(35));

    let eip1559_call = UnsignedTransaction {
        from: Some("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689".parse()?),
        to: Some("0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?),
        value: Some(U256::from(1000000000000000u64)), // 0.001 ETH
        gas: Some(21000),
        max_fee_per_gas: Some(30_000_000_000), // 30 gwei max fee
        max_priority_fee_per_gas: Some(2_000_000_000), // 2 gwei tip
        ..Default::default()
    };

    let result = simulator
        .simulate_unsigned_transaction(eip1559_call)
        .await?;
    println!("✅ EIP-1559 simulation result:");
    println!("   Success: {}", result.success);
    println!("   Gas used: {}", result.gas_used);

    // Example 4: Failed transaction simulation
    println!("\n📝 Example 4: Failed Transaction Analysis");
    println!("{}", "-".repeat(42));

    // Try to send more ETH than available
    let failed_call = UnsignedTransaction {
        from: Some("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689".parse()?), // Funded account
        to: Some("0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?),
        value: Some(U256::from_str("5000000000000000000").unwrap()), // 5 ETH
        gas: Some(21000),
        gas_price: Some(20_000_000_000),
        ..Default::default()
    };

    let result = simulator
        .simulate_unsigned_transaction(failed_call)
        .await
        .unwrap_or_else(|e| {
            println!("❌ Transaction validation failed: {}", e);
            // Return a failed result
            tx_simulator::SimulationResult {
                success: false,
                gas_used: 0,
                revert_reason: Some(e.to_string()),
                revert_context: None,
            }
        });
    println!("❌ Failed transaction analysis:");
    println!("   Success: {}", result.success);
    println!("   Gas used: {}", result.gas_used);
    if let Some(reason) = &result.revert_reason {
        println!("   Revert reason: {}", reason);
    }

    // Example 5: Gas limit testing
    println!("\n📝 Example 5: Gas Limit Optimization");
    println!("{}", "-".repeat(40));

    let base_call = UnsignedTransaction {
        from: Some("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689".parse()?),
        to: Some("0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?),
        value: Some(U256::from(100000000000000000u64)), // 0.1 ETH
        gas_price: Some(20_000_000_000),
        ..Default::default()
    };

    // Test different gas limits
    let gas_limits = [21000, 25000, 30000];

    for gas_limit in gas_limits {
        let mut test_call = base_call.clone();
        test_call.gas = Some(gas_limit);

        let result = simulator.simulate_unsigned_transaction(test_call).await?;
        println!(
            "   Gas limit {}: Success={}, Used={}",
            gas_limit, result.success, result.gas_used
        );
    }

    // Example 6: Nonce handling
    println!("\n📝 Example 6: Nonce Management");
    println!("{}", "-".repeat(32));

    let nonce_call = UnsignedTransaction {
        from: Some("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689".parse()?),
        to: Some("0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?),
        value: Some(U256::from(100000000000000000u64)), // 0.1 ETH
        gas: Some(21000),
        gas_price: Some(20_000_000_000),
        nonce: Some(999999), // Likely too high
        ..Default::default()
    };

    let result = simulator
        .simulate_unsigned_transaction(nonce_call)
        .await
        .unwrap_or_else(|e| {
            println!("   Expected error with wrong nonce: {}", e);
            tx_simulator::SimulationResult {
                success: false,
                gas_used: 0,
                revert_reason: Some(e.to_string()),
                revert_context: None,
            }
        });
    println!("   High nonce test:");
    println!("   Success: {}", result.success);
    println!("   Gas used: {}", result.gas_used);
    if !result.success {
        println!("   Expected failure - nonce likely too high");
    }

    // Summary
    println!("\n📊 Unsigned Transaction Simulation Summary");
    println!("==========================================");
    println!("Unsigned transaction simulation provides:");
    println!("• Pre-execution validation without broadcasting");
    println!("• Gas usage estimation and optimization");
    println!("• Contract interaction testing");
    println!("• EIP-1559 transaction support");
    println!("• Historical block simulation");
    println!("• Detailed failure analysis");
    println!();
    println!("Key use cases:");
    println!("• Transaction pre-flight checks");
    println!("• Gas optimization");
    println!("• Contract testing and debugging");
    println!("• MEV research and analysis");
    println!("• DeFi interaction simulation");

    Ok(())
}
