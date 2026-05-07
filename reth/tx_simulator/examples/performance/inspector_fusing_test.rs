use alloy_primitives::{address, U256};
/// Sequential No-Trace Performance Test
///
/// This example exercises the fast sequential simulation path. Plain sequence
/// execution does not allocate tracing inspectors; tracing-specific benchmarks
/// live under `examples/replay/profile`.
use eyre::Result;
use std::time::Instant;
use tx_simulator::{SequentialSimulationOptions, TxSimulator, UnsignedTransaction};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Sequential No-Trace Performance Test");
    println!("====================================\n");

    // Initialize simulator
    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let simulator = TxSimulator::new(&reth_datadir)?;

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

    println!(
        "Running sequential simulation with {} transactions...\n",
        transactions.len()
    );

    // Test the no-trace execution path.
    let start = Instant::now();

    let options = SequentialSimulationOptions {
        at_block: None,
        stop_on_failure: false,
        auto_increment_nonces: false,
        gas_limit_per_tx: Some(30_000),
    };

    let result = simulator
        .simulate_unsigned_tx_sequence(transactions.clone(), options.clone())
        .await?;

    let cold_duration = start.elapsed();

    println!("No-trace sequence:");
    println!("   - Total transactions: {}", result.total_transactions);
    println!("   - Successful: {}", result.successful_transactions);
    println!("   - Failed: {}", result.failed_transactions);
    println!("   - Total gas: {}", result.total_gas_used);
    println!("   - Time taken: {:?}", cold_duration);
    println!(
        "   - Avg per tx: {:?}",
        cold_duration / result.total_transactions as u32
    );

    // Run again to test warm cache
    println!("\nSecond run (warm cache):");
    let start = Instant::now();

    let result2 = simulator
        .simulate_unsigned_tx_sequence(transactions, options)
        .await?;

    let warm_duration = start.elapsed();

    println!("   - Time taken: {:?}", warm_duration);
    println!(
        "   - Avg per tx: {:?}",
        warm_duration / result2.total_transactions as u32
    );

    println!("\nPerformance Analysis:");
    println!("   - Plain sequence execution avoids inspector allocation");
    println!("   - Warm cache run shows additional speedup from database caching");

    if warm_duration < cold_duration {
        let speedup = cold_duration.as_secs_f64() / warm_duration.as_secs_f64();
        println!("   - Warm cache speedup: {:.2}x faster", speedup);
    }

    Ok(())
}
