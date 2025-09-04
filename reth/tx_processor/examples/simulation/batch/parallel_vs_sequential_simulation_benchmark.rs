/// Parallel vs Sequential Simulation Benchmark
/// 
/// This example demonstrates the performance difference between sequential and parallel
/// transaction simulation using the tx_simulator.
///
/// WHAT IT DOES:
/// 1. Creates 5 independent test transactions (simple ETH transfers)
/// 2. Simulates them sequentially (one after another), measures total time
/// 3. Simulates them in parallel (all at once using futures::join_all), measures total time  
/// 4. Compares performance and shows speedup ratio (e.g., "2.26x faster")
///
/// USE CASE:
/// - Performance benchmarking of transaction simulation approaches
/// - Understanding concurrency benefits for independent transaction processing
/// - Optimizing bulk transaction analysis workflows
///
/// OUTPUT:
/// - Sequential processing time and success count
/// - Parallel processing time and success count  
/// - Speedup ratio comparison
///
/// EXAMPLE OUTPUT:
/// ```
/// === Parallel vs Sequential Simulation Benchmark ===
///
/// ✓ Simulator initialized
///
/// 1. Sequential Processing:
///    Call 0 ✓
///    Call 1 ✓
///    Call 2 ✓
///    Call 3 ✓
///    Call 4 ✓
///    Total: 5 successful in 592.839µs
///
/// 2. Parallel Processing:
///    Total: 5 successful in 262.382µs
///    Speedup: 2.26x
///
/// ✓ Benchmark complete!
/// ```
use eyre::Result;
use tx_simulator::{TxSimulator, CallRequest};
use tx_simulator::batch_simulator::BatchSimulationOptions;
use tokio::time::Instant;
use alloy_primitives::{Address, U256};

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n=== Parallel vs Sequential Simulation Benchmark ===\n");
    
    let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    println!("✓ Simulator initialized");
    
    // Create test call requests
    let mut calls = Vec::new();
    for i in 0..5 {
        let call = CallRequest {
            from: Some(Address::default()),
            to: Some(Address::default()),
            value: Some(U256::from(i * 1000)),
            data: None,
            gas: Some(21000),
            gas_price: None,
            max_fee_per_gas: Some(30_000_000_000),
            max_priority_fee_per_gas: Some(1_000_000_000),
            nonce: None,
        };
        calls.push(call);
    }
    
    // Sequential processing
    println!("\n1. Sequential Processing:");
    let start = Instant::now();
    let mut sequential_success = 0;
    for (i, call) in calls.iter().enumerate() {
        match simulator.simulate_call(call.clone()).await {
            Ok(_) => {
                sequential_success += 1;
                println!("   Call {} ✓", i);
            }
            Err(e) => println!("   Call {} ✗: {}", i, e),
        }
    }
    let sequential_time = start.elapsed();
    println!("   Total: {} successful in {:?}", sequential_success, sequential_time);
    
    // Parallel processing using futures
    println!("\n2. Parallel Processing:");
    let start = Instant::now();
    
    use futures::future::join_all;
    let futures = calls.iter().map(|call| {
        let sim = simulator.clone();
        let call = call.clone();
        async move {
            sim.simulate_call(call).await
        }
    });
    
    let results: Vec<_> = join_all(futures).await;
    let parallel_success = results.iter().filter(|r| r.is_ok()).count();
    let parallel_time = start.elapsed();
    
    println!("   Total: {} successful in {:?}", parallel_success, parallel_time);
    println!("   Speedup: {:.2}x", sequential_time.as_secs_f64() / parallel_time.as_secs_f64());
    
    println!("\n✓ Benchmark complete!");
    
    Ok(())
}