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
use tx_simulator::{
    TxSimulator, 
    CallRequest,
    SequentialSimulationOptions,
};
use alloy_primitives::{Address, U256, address};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 Batch Sequence Example");
    println!("===========================\n");
    
    // Initialize simulator
    let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Create a sequence of transactions
    // In a real scenario, these might be:
    // 1. Enable trading on a token
    // 2. Add liquidity
    // 3. Perform a swap
    let transactions = vec![
        // Transaction 1: Simple ETH transfer
        CallRequest {
            from: Some(address!("0000000000000000000000000000000000000001")),
            to: Some(address!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef")),
            value: Some(U256::from(100_000_000_000_000_000u128)), // 0.1 ETH
            gas: Some(21_000),
            data: None,
            gas_price: None,
            max_fee_per_gas: Some(30_000_000_000), // 30 gwei
            max_priority_fee_per_gas: Some(1_000_000_000), // 1 gwei
            nonce: None, // Will be auto-detected
        },
        // Transaction 2: Another transfer (builds on tx1's state)
        CallRequest {
            from: Some(address!("0000000000000000000000000000000000000001")),
            to: Some(address!("beefdeadbeefdeadbeefdeadbeefdeadbeefdead")),
            value: Some(U256::from(200_000_000_000_000_000u128)), // 0.2 ETH
            gas: Some(21_000),
            data: None,
            gas_price: None,
            max_fee_per_gas: Some(30_000_000_000),
            max_priority_fee_per_gas: Some(1_000_000_000),
            nonce: None, // Will auto-increment from tx1
        },
        // Transaction 3: Final transfer
        CallRequest {
            from: Some(address!("0000000000000000000000000000000000000001")),
            to: Some(address!("cafebabecafebabecafebabecafebabecafebabe")),
            value: Some(U256::from(300_000_000_000_000_000u128)), // 0.3 ETH
            gas: Some(21_000),
            data: None,
            gas_price: None,
            max_fee_per_gas: Some(30_000_000_000),
            max_priority_fee_per_gas: Some(1_000_000_000),
            nonce: None, // Will auto-increment from tx2
        },
    ];
    
    println!("Setting up {} transactions:", transactions.len());
    for (i, tx) in transactions.iter().enumerate() {
        println!("  - Tx {}: Transfer {} ETH to {:?}", 
            i, 
            tx.value.unwrap_or_default().to_string(),
            tx.to.unwrap_or_default()
        );
    }
    
    // Simulate the sequence with default options (stop on failure)
    println!("\nRunning sequential simulation...");
    let options = SequentialSimulationOptions::default();
    
    let result = simulator
        .simulate_transaction_sequence(transactions.clone(), options)
        .await?;
    
    if result.sequence_success {
        println!("✅ Sequence complete: {}/{} successful", 
            result.successful_transactions, 
            result.total_transactions
        );
    } else {
        println!("❌ Sequence failed: {}/{} successful", 
            result.successful_transactions,
            result.total_transactions
        );
    }
    
    println!("  - Total gas used: {:?}", result.total_gas_used);
    for (i, tx_result) in result.results.iter().enumerate() {
        let status = if tx_result.success { "✓" } else { "✗" };
        println!("  - Transaction {}: {} ({} gas)", 
            i, 
            status, 
            tx_result.gas_used
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
    failing_sequence[1] = CallRequest {
        from: Some(address!("0000000000000000000000000000000000000002")), // Different sender with no balance
        to: Some(address!("beefdeadbeefdeadbeefdeadbeefdeadbeefdead")),
        value: Some(U256::from(999_000_000_000_000_000_000u128)), // 999 ETH (will fail)
        gas: Some(21_000),
        data: None,
        gas_price: None,
        max_fee_per_gas: Some(30_000_000_000),
        max_priority_fee_per_gas: Some(1_000_000_000),
        nonce: None,
    };
    
    let result2 = simulator
        .simulate_transaction_sequence(failing_sequence, options_continue)
        .await?;
    
    println!("  - Completed: {}/{} transactions", 
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
    
    println!("\n💡 Key Features:");
    println!("  - State persists between transactions");
    println!("  - Nonces auto-increment for same sender");
    println!("  - Can stop on first failure or continue");
    println!("  - Perfect for MEV bundle simulation");
    
    Ok(())
}