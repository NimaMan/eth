use std::time::Instant;
use anyhow::Result;

use ethers_core::types::H256;
use ethers_providers::{Provider, Http, Middleware};

// Import our minimal processor
mod minimal_processor;
use minimal_processor::MinimalProcessor;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏁 Benchmarking Minimal TX Processor vs RPC");
    println!("==========================================\n");

    let tx_hash_str = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
    let tx_hash: H256 = tx_hash_str.parse()?;
    let rpc_url = "http://127.0.0.1:8545";
    
    println!("📊 Transaction: {}", tx_hash_str);
    println!("🌐 RPC URL: {}\n", rpc_url);
    
    // Setup
    let provider = Provider::<Http>::try_from(rpc_url)?;
    let processor = MinimalProcessor::new(rpc_url).await?;
    
    // Warm up
    println!("🔥 Warming up...");
    let _ = provider.get_transaction(tx_hash).await?;
    let _ = processor.process_tx_minimal(tx_hash).await?;
    
    println!("\n" + "─".repeat(50) + "\n");
    
    // Benchmark 1: Basic RPC (no traces)
    println!("⚡ Method 1: Basic RPC (tx + receipt)");
    println!("─".repeat(35));
    
    let mut rpc_times = Vec::new();
    
    for i in 1..=20 {
        let start = Instant::now();
        
        let tx = provider.get_transaction(tx_hash).await?;
        let receipt = provider.get_transaction_receipt(tx_hash).await?;
        
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        rpc_times.push(duration);
        
        if i == 1 {
            println!("✅ Transaction found");
            println!("  Gas used: {}", receipt.unwrap().gas_used.unwrap());
            println!("  First call: {:.2}ms", duration);
        }
    }
    
    let avg_rpc = rpc_times.iter().sum::<f64>() / rpc_times.len() as f64;
    let min_rpc = rpc_times.iter().cloned().fold(f64::INFINITY, f64::min);
    
    println!("\nRPC Results (20 iterations):");
    println!("  Average: {:.2}ms", avg_rpc);
    println!("  Minimum: {:.2}ms", min_rpc);
    
    println!("\n" + "─".repeat(50) + "\n");
    
    // Benchmark 2: Minimal REVM Processor
    println!("🚀 Method 2: Minimal REVM Processor");
    println!("─".repeat(35));
    
    let mut revm_times = Vec::new();
    let mut fetch_times = Vec::new();
    let mut sim_times = Vec::new();
    
    for i in 1..=20 {
        let result = processor.process_tx_minimal(tx_hash).await?;
        
        revm_times.push(result.total_time_ms);
        fetch_times.push(result.fetch_time_ms);
        sim_times.push(result.sim_time_ms);
        
        if i == 1 {
            println!("✅ Processing complete");
            println!("  Gas used: {}", result.gas_used);
            println!("  Success: {}", result.success);
            println!("  First call breakdown:");
            println!("    Fetch: {:.2}ms", result.fetch_time_ms);
            println!("    Sim: {:.2}ms", result.sim_time_ms);
            println!("    Total: {:.2}ms", result.total_time_ms);
        }
    }
    
    let avg_revm = revm_times.iter().sum::<f64>() / revm_times.len() as f64;
    let min_revm = revm_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let avg_fetch = fetch_times.iter().sum::<f64>() / fetch_times.len() as f64;
    let avg_sim = sim_times.iter().sum::<f64>() / sim_times.len() as f64;
    
    println!("\nREVM Results (20 iterations):");
    println!("  Average: {:.2}ms", avg_revm);
    println!("  Minimum: {:.2}ms", min_revm);
    println!("  Breakdown:");
    println!("    Avg Fetch: {:.2}ms", avg_fetch);
    println!("    Avg Sim: {:.2}ms", avg_sim);
    
    // Summary
    println!("\n" + "═".repeat(50));
    println!("📈 Performance Comparison");
    println!("═".repeat(50));
    
    println!("\n         │ Average │ Minimum");
    println!("─────────┼─────────┼─────────");
    println!("RPC      │ {:>7.2} │ {:>7.2}", avg_rpc, min_rpc);
    println!("REVM     │ {:>7.2} │ {:>7.2}", avg_revm, min_revm);
    println!("─────────┼─────────┼─────────");
    
    if avg_revm < avg_rpc {
        let speedup = avg_rpc / avg_revm;
        println!("\n🎉 REVM is {:.1}x faster than RPC!", speedup);
        println!("   RPC makes 2 network calls");
        println!("   REVM makes 3 calls but simulation is local");
    } else {
        println!("\n🤔 REVM appears slower - likely due to:");
        println!("   - Extra block fetch for simulation");
        println!("   - Simulation overhead not optimized yet");
    }
    
    println!("\n💡 Note: This is WITHOUT actual REVM simulation");
    println!("   Once we hook up real simulation, we'll also get:");
    println!("   - Internal transfers");
    println!("   - Complete logs");
    println!("   - State changes");
    
    Ok(())
}

// Include the minimal processor inline for now
mod minimal_processor {
    use super::*;
    
    pub struct MinimalProcessor {
        provider: Arc<Provider<Http>>,
    }

    #[derive(Debug)]
    pub struct MinimalResult {
        pub hash: H256,
        pub gas_used: u64,
        pub success: bool,
        pub fetch_time_ms: f64,
        pub sim_time_ms: f64,
        pub total_time_ms: f64,
    }

    impl MinimalProcessor {
        pub async fn new(rpc_url: &str) -> Result<Self> {
            let provider = Provider::<Http>::try_from(rpc_url)?;
            Ok(Self {
                provider: Arc::new(provider),
            })
        }

        pub async fn process_tx_minimal(&self, tx_hash: H256) -> Result<MinimalResult> {
            let total_start = Instant::now();
            
            // Fetch minimal data needed
            let fetch_start = Instant::now();
            let tx = self.provider.get_transaction(tx_hash).await?
                .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
            let receipt = self.provider.get_transaction_receipt(tx_hash).await?
                .ok_or_else(|| anyhow::anyhow!("Receipt not found"))?;
            let block = self.provider.get_block(receipt.block_number.unwrap()).await?
                .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
            let fetch_time = fetch_start.elapsed().as_secs_f64() * 1000.0;
            
            // TODO: Actual REVM simulation here
            let sim_start = Instant::now();
            // For now, just use receipt gas
            let gas_used = receipt.gas_used.unwrap().as_u64();
            let sim_time = sim_start.elapsed().as_secs_f64() * 1000.0;
            
            let total_time = total_start.elapsed().as_secs_f64() * 1000.0;
            
            Ok(MinimalResult {
                hash: tx_hash,
                gas_used,
                success: receipt.status.unwrap().as_u64() == 1,
                fetch_time_ms: fetch_time,
                sim_time_ms: sim_time,
                total_time_ms: total_time,
            })
        }
    }
    
    use std::sync::Arc;
}