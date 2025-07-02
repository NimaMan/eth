//! Test REVM simulation performance for mempool transactions (tx_index = 0)
//! This simulates how fast REVM would be for mempool transactions that don't need replay

use anyhow::Result;
use std::time::Instant;
use ethers_providers::{Provider as EthersProvider, Http, Middleware};
use ethers_core::types::H256;
use revm_tx_simulator_lib::simulate_signed_tx::lib::{simulate_signed_tx_bytes};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Testing REVM Simulation Performance for Mempool Transactions");
    println!("{}", "=".repeat(80));
    
    let rpc_url = "http://localhost:8545";
    
    // Use a known simple ETH transfer transaction for consistent testing
    // Transaction hash: 0x21aeb75078488051edcdce4b55e4a9bc6c1fd217bb19c6654921b2a4aed8e3db
    let raw_tx_hex = "02f87401808501193a82298501193a82298252089431385527fe03ca642f043a9e8a1c8bacbd9addac8805e19dbc7b6438a480c001a0c42862561ee35aa615dfa57228a64100bfda08752bbf8eccfe3a36f4a088371aa003cdff0088d91e73933656ecceb122337956a7c30eab96fa4642217d1f81be21";
    let tx_bytes = hex::decode(raw_tx_hex)?;
    let block_number = 22687107u64; // Block where this tx was mined
    
    // Get provider to fetch transaction details for display
    let provider = EthersProvider::<Http>::try_from(rpc_url)?;
    let tx_hash: H256 = "0x21aeb75078488051edcdce4b55e4a9bc6c1fd217bb19c6654921b2a4aed8e3db".parse()?;
    let test_tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    
    println!("📦 Test Transaction:");
    println!("   Hash: {:?}", test_tx.hash);
    println!("   From: {:?}", test_tx.from);
    println!("   To: {:?}", test_tx.to);
    println!("   Value: {} ETH", ethers_core::utils::format_units(test_tx.value, "ether")?);
    println!("   Gas: {}", test_tx.gas);
    println!("   Data Length: {} bytes", test_tx.input.len());
    println!("   Encoded Size: {} bytes", tx_bytes.len());
    
    println!("\n⚡ Running REVM Simulations (as if tx_index = 0)...\n");
    
    // Run multiple simulations to get average timing
    let mut simulation_times = Vec::new();
    let num_simulations = 10;
    
    for i in 1..=num_simulations {
        let start = Instant::now();
        
        // Simulate at current block (as if this is a mempool tx)
        match simulate_signed_tx_bytes(&tx_bytes, block_number, rpc_url).await {
            Ok(output) => {
                let elapsed = start.elapsed();
                simulation_times.push(elapsed.as_secs_f64() * 1000.0); // Convert to ms
                
                println!("Simulation {}: {:.2}ms (Result: {:?}, Gas: {})",
                    i,
                    elapsed.as_secs_f64() * 1000.0,
                    output.result_type,
                    output.gas_used
                );
            }
            Err(e) => {
                println!("Simulation {} failed: {:?}", i, e);
            }
        }
        
        // Small delay between simulations
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    if !simulation_times.is_empty() {
        let avg_time = simulation_times.iter().sum::<f64>() / simulation_times.len() as f64;
        let min_time = simulation_times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time = simulation_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        println!("\n📊 REVM Simulation Performance Summary:");
        println!("   Simulations: {}", simulation_times.len());
        println!("   Average: {:.2}ms", avg_time);
        println!("   Min: {:.2}ms", min_time);
        println!("   Max: {:.2}ms", max_time);
        
        println!("\n💡 Comparison with debug_traceCall:");
        println!("   debug_traceCall avg: ~2-5ms");
        println!("   REVM simulation avg: {:.2}ms", avg_time);
        
        if avg_time < 2.0 {
            println!("   ✅ REVM is FASTER than debug_traceCall!");
        } else if avg_time < 5.0 {
            println!("   ⚡ REVM performance is comparable to debug_traceCall");
        } else {
            println!("   ⚠️  REVM is slower, likely due to RPC state fetching");
        }
    }
    
    Ok(())
}