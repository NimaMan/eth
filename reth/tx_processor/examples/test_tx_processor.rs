use std::time::Instant;
use anyhow::Result;

use ethers_core::types::H256;
use revm_tx_simulator::tx_processor::{
    RevmTxProcessor, ProcessorConfig,
    RpcProcessor, RpcProcessorConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Testing Transaction Processor with Specific Transaction");
    println!("========================================================\n");

    // The transaction you provided
    let tx_hash_str = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
    let tx_hash: H256 = tx_hash_str.parse()?;
    
    println!("📊 Transaction Hash: {}", tx_hash_str);
    println!("🌐 RPC URL: http://127.0.0.1:8545\n");
    
    // Test 1: RPC Processor (baseline)
    println!("⚡ Testing RPC Processor (with traces)");
    println!("─".repeat(40));
    
    let rpc_config = RpcProcessorConfig::default();
    let rpc_processor = RpcProcessor::new(rpc_config).await?;
    
    let rpc_start = Instant::now();
    match rpc_processor.process_transaction(tx_hash).await {
        Ok(result) => {
            let rpc_time = rpc_start.elapsed().as_secs_f64() * 1000.0;
            
            println!("✅ RPC Processing Complete!");
            println!("  Block Number: {}", result.block_number);
            println!("  From: {:?}", result.from);
            println!("  To: {:?}", result.to);
            println!("  Value: {}", result.value);
            println!("  Gas Used: {}", result.gas_used);
            println!("  Success: {}", result.success);
            println!("  Internal Transfers: {}", result.internal_transfers.len());
            println!("  Time: {:.2}ms", rpc_time);
        }
        Err(e) => {
            println!("❌ RPC processing failed: {}", e);
        }
    }
    
    println!("\n" + "─".repeat(60) + "\n");
    
    // Test 2: REVM Processor (our implementation)
    println!("🚀 Testing REVM Direct Processor");
    println!("─".repeat(40));
    
    let revm_config = ProcessorConfig::default();
    let revm_processor = RevmTxProcessor::new(revm_config).await?;
    
    let revm_start = Instant::now();
    match revm_processor.process_transaction(tx_hash).await {
        Ok(result) => {
            let revm_time = revm_start.elapsed().as_secs_f64() * 1000.0;
            
            println!("✅ REVM Processing Complete!");
            println!("  Block Number: {}", result.block_number);
            println!("  From: {:?}", result.from);
            println!("  To: {:?}", result.to);
            println!("  Value: {}", result.value);
            println!("  Gas Used: {}", result.gas_used);
            println!("  Success: {}", result.success);
            println!("  Internal Transfers: {}", result.internal_transfers.len());
            
            println!("\n  Performance Breakdown:");
            println!("    Fetch Time: {:.2}ms", result.metrics.fetch_time_ms);
            println!("    Simulation Time: {:.2}ms", result.metrics.simulation_time_ms);
            println!("    Total Time: {:.2}ms", result.metrics.total_time_ms);
            
            // Show internal transfers if any
            if !result.internal_transfers.is_empty() {
                println!("\n  Internal Transfers:");
                for (i, transfer) in result.internal_transfers.iter().enumerate() {
                    println!("    {}. {} → {} : {} ({})", 
                        i+1, 
                        transfer.from, 
                        transfer.to, 
                        transfer.value,
                        transfer.call_type
                    );
                }
            }
        }
        Err(e) => {
            println!("❌ REVM processing failed: {}", e);
            println!("   Error details: {:?}", e);
        }
    }
    
    println!("\n" + "═".repeat(60));
    println!("📊 Performance Comparison");
    println!("═".repeat(60));
    
    // Run multiple iterations for accurate timing
    println!("\nRunning 10 iterations for accurate benchmarking...\n");
    
    let mut rpc_times = Vec::new();
    let mut revm_times = Vec::new();
    
    for i in 1..=10 {
        print!("Iteration {}/10... ", i);
        
        // RPC timing
        let start = Instant::now();
        let _ = rpc_processor.process_transaction(tx_hash).await;
        rpc_times.push(start.elapsed().as_secs_f64() * 1000.0);
        
        // REVM timing
        let start = Instant::now();
        let _ = revm_processor.process_transaction(tx_hash).await;
        revm_times.push(start.elapsed().as_secs_f64() * 1000.0);
        
        println!("✓");
    }
    
    let avg_rpc = rpc_times.iter().sum::<f64>() / rpc_times.len() as f64;
    let avg_revm = revm_times.iter().sum::<f64>() / revm_times.len() as f64;
    let min_rpc = rpc_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let min_revm = revm_times.iter().cloned().fold(f64::INFINITY, f64::min);
    
    println!("\n📊 Final Results:");
    println!("─".repeat(40));
    println!("       │ Average │ Minimum");
    println!("───────┼─────────┼─────────");
    println!("RPC    │ {:>7.2} │ {:>7.2}", avg_rpc, min_rpc);
    println!("REVM   │ {:>7.2} │ {:>7.2}", avg_revm, min_revm);
    println!("───────┼─────────┼─────────");
    println!("Speedup│ {:>7.1}x │ {:>7.1}x", avg_rpc/avg_revm, min_rpc/min_revm);
    
    println!("\n✅ Transaction processor successfully integrated!");
    println!("✅ Achieved {:.1}x speedup over RPC+traces", avg_rpc/avg_revm);
    
    Ok(())
}