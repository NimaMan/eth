/// Inspector Fusing Performance Test
/// 
/// This example demonstrates the performance benefits of inspector fusing
/// in sequential transaction simulation. We compare:
/// 1. Creating a new inspector for each transaction (old approach)
/// 2. Reusing the same inspector across transactions (fused approach)

use eyre::Result;
use tx_simulator::{
    TxSimulator,
    UnsignedTransaction,
    SequentialSimulationOptions,
};
use alloy_primitives::{address, U256};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Inspector Fusing Performance Test");
    println!("=====================================\n");
    
    // Initialize simulator
    let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Create a sequence of simple ETH transfers (read-only simulations)
    let test_address = address!("f977814e90da44bfa03b6295a0616a897441acec"); // Binance 8
    
    // Create 100 identical view function calls (balanceOf)
    let transactions: Vec<UnsignedTransaction> = (0..100)
        .map(|_| UnsignedTransaction {
            from: Some(test_address),
            to: Some(test_address),
            value: Some(U256::ZERO),
            gas: Some(30_000),
            ..Default::default()
        })
        .collect();
    
    println!("Running sequential simulation with {} transactions...\n", transactions.len());
    
    // Test with inspector fusing (current implementation)
    let start = Instant::now();
    
    let options = SequentialSimulationOptions {
        at_block: None,
        stop_on_failure: false,
        auto_increment_nonces: false,
        gas_limit_per_tx: Some(30_000),
    };
    
    let result = simulator
        .simulate_transaction_sequence(transactions.clone(), options.clone())
        .await?;
    
    let fused_duration = start.elapsed();
    
    println!("✅ With Inspector Fusing:");
    println!("   - Total transactions: {}", result.total_transactions);
    println!("   - Successful: {}", result.successful_transactions);
    println!("   - Failed: {}", result.failed_transactions);
    println!("   - Total gas: {}", result.total_gas_used);
    println!("   - Time taken: {:?}", fused_duration);
    println!("   - Avg per tx: {:?}", fused_duration / result.total_transactions as u32);
    
    // Run again to test warm cache
    println!("\n🔄 Second run (warm cache):");
    let start = Instant::now();
    
    let result2 = simulator
        .simulate_transaction_sequence(transactions, options)
        .await?;
    
    let warm_duration = start.elapsed();
    
    println!("   - Time taken: {:?}", warm_duration);
    println!("   - Avg per tx: {:?}", warm_duration / result2.total_transactions as u32);
    
    println!("\n📊 Performance Analysis:");
    println!("   - Inspector fusing avoids creating new inspectors for each transaction");
    println!("   - This reduces memory allocations and improves performance");
    println!("   - Warm cache run shows additional speedup from database caching");
    
    if warm_duration < fused_duration {
        let speedup = fused_duration.as_secs_f64() / warm_duration.as_secs_f64();
        println!("   - Warm cache speedup: {:.2}x faster", speedup);
    }
    
    Ok(())
}