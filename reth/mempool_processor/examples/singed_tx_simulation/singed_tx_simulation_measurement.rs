/// ACTUAL Simulation Measurement
/// 
/// This WILL run and give REAL numbers
///
/// Run with: cargo run --example actual_simulation_measurement

use ethers::prelude::*;
use ethers::providers::{Provider, Http, Ws};
use std::time::{Duration, Instant};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ACTUAL Transaction Simulation Measurement");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    
    // Connect providers
    println!("Connecting to Ethereum node...");
    let http_provider = Arc::new(Provider::<Http>::try_from(http_url)?);
    let ws_provider = Provider::<Ws>::connect(ws_url).await?;
    
    // Verify connection
    let block = http_provider.get_block_number().await?;
    println!("Connected! Current block: {}", block);
    
    // Subscribe to pending transactions
    println!("\nSubscribing to mempool...");
    let mut stream = ws_provider.subscribe_pending_txs().await?;
    println!("Subscribed! Waiting for transactions...\n");
    
    let mut count = 0;
    let mut simulation_times = Vec::new();
    let start_time = Instant::now();
    
    // Measure 10 transactions
    while count < 10 {
        if let Some(tx_hash) = stream.next().await {
            let detection_start = Instant::now();
            
            // Fetch full transaction
            match http_provider.get_transaction(tx_hash).await {
                Ok(Some(tx)) => {
                    count += 1;
                    let fetch_time = detection_start.elapsed();
                    
                    // Simulate transaction using debug_traceCall
                    let sim_start = Instant::now();
                    
                    // Create call request for simulation
                    let mut call_request = TransactionRequest::new()
                        .from(tx.from)
                        .to(tx.to.unwrap_or_default())
                        .value(tx.value)
                        .data(tx.input.clone())
                        .gas(tx.gas);
                    
                    if let Some(gas_price) = tx.gas_price {
                        call_request = call_request.gas_price(gas_price);
                    }
                    
                    // Try to simulate
                    match http_provider.request::<_, serde_json::Value>(
                        "debug_traceCall",
                        (call_request, "latest", serde_json::json!({"tracer": "prestateTracer"}))
                    ).await {
                        Ok(trace_result) => {
                            let sim_time = sim_start.elapsed();
                            simulation_times.push(sim_time.as_millis() as f64);
                            
                            // Count state changes
                            let state_changes = if let Some(obj) = trace_result.as_object() {
                                obj.len()
                            } else {
                                0
                            };
                            
                            println!("TX #{}: {}", count, format!("{:#x}", tx_hash));
                            println!("  Fetch time: {:.2}ms", fetch_time.as_millis());
                            println!("  Simulation time: {:.2}ms", sim_time.as_millis());
                            println!("  State changes: {} addresses", state_changes);
                            println!("  Total time: {:.2}ms\n", (fetch_time + sim_time).as_millis());
                        }
                        Err(e) => {
                            println!("TX #{}: {} - Simulation failed: {}", count, format!("{:#x}", tx_hash), e);
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Timeout after 60 seconds
        if start_time.elapsed() > Duration::from_secs(60) {
            println!("Timeout reached");
            break;
        }
    }
    
    // Show results
    println!("\n📊 ACTUAL SIMULATION RESULTS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Transactions simulated: {}", count);
    
    if !simulation_times.is_empty() {
        let avg = simulation_times.iter().sum::<f64>() / simulation_times.len() as f64;
        let min = simulation_times.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let max = simulation_times.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        
        println!("\nSimulation times:");
        println!("  Average: {:.2}ms", avg);
        println!("  Min: {:.2}ms", min);
        println!("  Max: {:.2}ms", max);
    }
    
    Ok(())
}