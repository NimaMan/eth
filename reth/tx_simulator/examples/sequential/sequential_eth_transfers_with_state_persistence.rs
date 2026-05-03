use alloy_primitives::{address, U256};
/// Batch Sequence Example
///
/// Demonstrates simulating a sequence of transactions where each builds on the previous state.
/// This is crucial for MEV bundle simulation, protocol testing, and complex DeFi interactions.
///
/// # Example Output:
/// ```
/// 🔄 Sequential Simulation Demo
/// ===========================
///
/// Setting up 3 transactions:
///   - Tx 0: Transfer 0.1 ETH to 0xdead...beef
///   - Tx 1: Transfer 0.2 ETH to 0xbeef...dead  
///   - Tx 2: Transfer 0.3 ETH to 0xcafe...babe
///
/// Running sequential simulation...
/// ✅ Sequence complete: 3/3 successful
///   - Total gas used: 63,000
///   - Transaction 0: ✓ (21,000 gas)
///   - Transaction 1: ✓ (21,000 gas)
///   - Transaction 2: ✓ (21,000 gas)
///
/// Testing with failure in middle:
///   - Transaction 0: ✓
///   - Transaction 1: ✗ (insufficient balance)
///   - Sequence stopped due to failure
/// ```
use eyre::Result;
use tx_simulator::{SequentialSimulationOptions, TxSimulator, UnsignedTransaction};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 Batch Sequence Example");
    println!("===========================\n");

    // Initialize simulator
    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let simulator = TxSimulator::new(&reth_datadir)?;

    // Create a sequence of transactions
    // In a real scenario, these might be:
    // 1. Enable trading on a token
    // 2. Add liquidity
    // 3. Perform a swap
    let transactions = vec![
        // Transaction 1: Simple ETH transfer
        UnsignedTransaction {
            from: Some(address!("0C96c602b1b332B8AB2093E5d72D804a24bd5689")),
            to: Some(address!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef")),
            value: Some(U256::from(1_000_000_000_000_000u128)), // 0.001 ETH
            gas: Some(21_000),
            data: None,
            gas_price: None,
            max_fee_per_gas: Some(30_000_000_000), // 30 gwei
            max_priority_fee_per_gas: Some(1_000_000_000), // 1 gwei
            nonce: None,                           // Will be auto-detected
            ..Default::default()
        },
        // Transaction 2: Another transfer (builds on tx1's state)
        UnsignedTransaction {
            from: Some(address!("0C96c602b1b332B8AB2093E5d72D804a24bd5689")),
            to: Some(address!("beefdeadbeefdeadbeefdeadbeefdeadbeefdead")),
            value: Some(U256::from(2_000_000_000_000_000u128)), // 0.002 ETH
            gas: Some(21_000),
            data: None,
            gas_price: None,
            max_fee_per_gas: Some(30_000_000_000),
            max_priority_fee_per_gas: Some(1_000_000_000),
            nonce: None, // Will auto-increment from tx1
            ..Default::default()
        },
        // Transaction 3: Final transfer
        UnsignedTransaction {
            from: Some(address!("0C96c602b1b332B8AB2093E5d72D804a24bd5689")),
            to: Some(address!("cafebabecafebabecafebabecafebabecafebabe")),
            value: Some(U256::from(3_000_000_000_000_000u128)), // 0.003 ETH
            gas: Some(21_000),
            data: None,
            gas_price: None,
            max_fee_per_gas: Some(30_000_000_000),
            max_priority_fee_per_gas: Some(1_000_000_000),
            nonce: None, // Will auto-increment from tx2
            ..Default::default()
        },
    ];

    println!("Setting up {} transactions:", transactions.len());
    for (i, tx) in transactions.iter().enumerate() {
        let eth_value = wei_to_eth(&tx.value.unwrap_or_default());
        println!(
            "  - Tx {}: Transfer {:.6} ETH to {:?}",
            i,
            eth_value,
            tx.to.unwrap_or_default()
        );
    }

    // Simulate the sequence with default options (stop on failure)
    println!("\nRunning sequential simulation...");
    let options = SequentialSimulationOptions::default();

    let result = simulator
        .simulate_unsigned_tx_sequence(transactions.clone(), options)
        .await?;

    if result.sequence_success {
        println!(
            "✅ Sequence complete: {}/{} successful",
            result.successful_transactions, result.total_transactions
        );
    } else {
        println!(
            "❌ Sequence failed: {}/{} successful",
            result.successful_transactions, result.total_transactions
        );
    }

    println!("  - Total gas used: {:?}", result.total_gas_used);
    for (i, tx_result) in result.results.iter().enumerate() {
        let status = if tx_result.success { "✓" } else { "✗" };
        println!(
            "  - Transaction {}: {} ({} gas)",
            i, status, tx_result.gas_used
        );
        if let Some(reason) = &tx_result.revert_reason {
            println!("    Revert: {}", reason);
        }
    }

    // Test with continue-on-failure option
    println!("\nTesting with continue-on-failure option:");
    let mut options_continue = SequentialSimulationOptions::default();
    options_continue.stop_on_failure = false;

    // Add a transaction that will fail
    let mut failing_sequence = transactions.clone();
    failing_sequence[1] = UnsignedTransaction {
        from: Some(address!("0C96c602b1b332B8AB2093E5d72D804a24bd5689")),
        to: Some(address!("beefdeadbeefdeadbeefdeadbeefdeadbeefdead")),
        value: Some(U256::from(2_000_000_000_000_000u128)),
        gas: Some(5_000), // intentionally too low, will OOG
        data: None,
        gas_price: None,
        max_fee_per_gas: Some(30_000_000_000),
        max_priority_fee_per_gas: Some(1_000_000_000),
        nonce: None,
        ..Default::default()
    };

    match simulator
        .simulate_unsigned_tx_sequence(failing_sequence, options_continue)
        .await
    {
        Ok(result2) => {
            println!(
                "  - Completed: {}/{} transactions",
                result2.successful_transactions + result2.failed_transactions,
                result2.total_transactions
            );
            println!("  - Successful: {}", result2.successful_transactions);
            println!("  - Failed: {}", result2.failed_transactions);

            for (i, tx_result) in result2.results.iter().enumerate() {
                let status = if tx_result.success { "✓" } else { "✗" };
                println!("  - Transaction {}: {}", i, status);
                if let Some(reason) = &tx_result.revert_reason {
                    println!("    Reason: {}", reason);
                }
            }
        }
        Err(err) => {
            println!("  - Sequence failed before completion: {}", err);
        }
    }

    println!("\n💡 Key Features:");
    println!("  - State persists between transactions");
    println!("  - Nonces auto-increment for same sender");
    println!("  - Can stop on first failure or continue");
    println!("  - Perfect for MEV bundle simulation");

    fn wei_to_eth(value: &U256) -> f64 {
        let wei: f64 = value.to::<u128>() as f64;
        wei / 1e18
    }

    Ok(())
}
