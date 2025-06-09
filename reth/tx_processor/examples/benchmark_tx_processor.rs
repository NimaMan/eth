use std::time::Instant;
use eyre::Result;

use ethers_core::types::H256;
use revm_tx_simulator::tx_processor::{
    RevmTxProcessor, ProcessorConfig,
    RpcProcessor, RpcProcessorConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏁 Benchmarking Integrated TX Processor");
    println!("======================================\n");

    let tx_hash_str = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
    let tx_hash: H256 = tx_hash_str.parse()?;
    
    // Setup processors
    let revm_config = ProcessorConfig::default();
    let revm_processor = RevmTxProcessor::new(revm_config).await?;
    
    let rpc_config = RpcProcessorConfig::default();
    let rpc_processor = RpcProcessor::new(rpc_config).await?;
    
    println!("📊 Transaction: {}", tx_hash_str);
    println!("🌐 RPC URL: http://127.0.0.1:8545\n");
    
    // Warm up
    println!("🔥 Warming up processors...");
    let _ = rpc_processor.process_transaction(tx_hash).await?;
    
    println!("\n" + "─".repeat(50) + "\n");
    
    // Benchmark RPC processor
    println!("⚡ Method 1: RPC with Traces");
    println!("─".repeat(30));
    
    let mut rpc_times = Vec::new();
    for i in 1..=10 {
        let result = rpc_processor.process_transaction(tx_hash).await?;
        rpc_times.push(result.metrics.total_time_ms);
        
        if i == 1 {
            println!("✅ Transaction processed");
            println!("  Block: {}", result.block_number);
            println!("  Gas used: {}", result.gas_used);
            println!("  Internal transfers: {}", result.internal_transfers.len());
            println!("  Time: {:.2}ms", result.metrics.total_time_ms);
        }
    }
    
    let avg_rpc = rpc_times.iter().sum::<f64>() / rpc_times.len() as f64;
    println!("\nAverage time: {:.2}ms", avg_rpc);
    
    println!("\n" + "─".repeat(50) + "\n");
    
    // Benchmark REVM processor
    println!("🚀 Method 2: Direct REVM Integration");
    println!("─".repeat(30));
    
    let mut revm_times = Vec::new();
    for i in 1..=10 {
        let result = revm_processor.process_transaction(tx_hash).await?;
        revm_times.push(result.metrics.total_time_ms);
        
        if i == 1 {
            println!("✅ Transaction processed");
            println!("  Block: {}", result.block_number);
            println!("  Gas used: {}", result.gas_used);
            println!("  Internal transfers: {}", result.internal_transfers.len());
            println!("  Fetch time: {:.2}ms", result.metrics.fetch_time_ms);
            println!("  Simulation time: {:.2}ms", result.metrics.simulation_time_ms);
            println!("  Total time: {:.2}ms", result.metrics.total_time_ms);
        }
    }
    
    let avg_revm = revm_times.iter().sum::<f64>() / revm_times.len() as f64;
    println!("\nAverage time: {:.2}ms", avg_revm);
    
    // Summary
    println!("\n" + "═".repeat(50));
    println!("📈 Performance Summary");
    println!("═".repeat(50));
    
    println!("RPC with traces:  {:.2}ms", avg_rpc);
    println!("Direct REVM:      {:.2}ms", avg_revm);
    println!("Speedup:          {:.1}x", avg_rpc / avg_revm);
    
    println!("\n✅ Success! TX processor integrated within REVM workspace.");
    println!("✅ No more dependency conflicts!");
    
    Ok(())
}