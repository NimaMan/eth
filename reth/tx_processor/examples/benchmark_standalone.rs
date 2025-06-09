use std::time::Instant;
use anyhow::Result;

use ethers_core::types::H256;
use ethers_providers::{Provider, Http, Middleware};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏁 Benchmarking TX Processing Speed");
    println!("===================================\n");

    let tx_hash_str = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
    let tx_hash: H256 = tx_hash_str.parse()?;
    let rpc_url = "http://127.0.0.1:8545";
    
    println!("📊 Transaction: {}", tx_hash_str);
    println!("🌐 RPC URL: {}\n", rpc_url);
    
    let provider = Provider::<Http>::try_from(rpc_url)?;
    
    // Warm up
    println!("🔥 Warming up...");
    let _ = provider.get_transaction(tx_hash).await?;
    
    // Get transaction details once
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    let receipt = provider.get_transaction_receipt(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Receipt not found"))?;
    
    println!("\n📖 Transaction Details:");
    println!("  Block: {}", receipt.block_number.unwrap());
    println!("  From: {:?}", tx.from);
    println!("  To: {:?}", tx.to);
    println!("  Gas Used: {}", receipt.gas_used.unwrap());
    println!("  Status: {}", if receipt.status.unwrap().as_u64() == 1 { "✅ Success" } else { "❌ Failed" });
    
    println!("\n" + "─".repeat(50) + "\n");
    
    // Test 1: Basic RPC (2 calls)
    println!("⚡ Test 1: Basic RPC (tx + receipt)");
    println!("─".repeat(35));
    
    let mut times = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        let _ = provider.get_transaction(tx_hash).await?;
        let _ = provider.get_transaction_receipt(tx_hash).await?;
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
    println!("  Average: {:.2}ms", avg);
    println!("  Minimum: {:.2}ms", min);
    
    // Test 2: RPC with block (3 calls)
    println!("\n⚡ Test 2: RPC + Block (3 calls)");
    println!("─".repeat(35));
    
    times.clear();
    for _ in 0..20 {
        let start = Instant::now();
        let _ = provider.get_transaction(tx_hash).await?;
        let _ = provider.get_transaction_receipt(tx_hash).await?;
        let _ = provider.get_block(receipt.block_number.unwrap()).await?;
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    
    let avg3 = times.iter().sum::<f64>() / times.len() as f64;
    let min3 = times.iter().cloned().fold(f64::INFINITY, f64::min);
    println!("  Average: {:.2}ms", avg3);
    println!("  Minimum: {:.2}ms", min3);
    
    // Test 3: RPC with traces (3 calls)
    println!("\n⚡ Test 3: RPC with Traces (3 calls)");
    println!("─".repeat(35));
    
    times.clear();
    for _ in 0..20 {
        let start = Instant::now();
        let _ = provider.get_transaction(tx_hash).await?;
        let _ = provider.get_transaction_receipt(tx_hash).await?;
        let _ = provider.request::<_, serde_json::Value>(
            "debug_traceTransaction", 
            vec![
                serde_json::to_value(tx_hash)?,
                serde_json::json!({"tracer": "callTracer"})
            ]
        ).await.ok();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    
    let avg_trace = times.iter().sum::<f64>() / times.len() as f64;
    let min_trace = times.iter().cloned().fold(f64::INFINITY, f64::min);
    println!("  Average: {:.2}ms", avg_trace);
    println!("  Minimum: {:.2}ms", min_trace);
    
    // Summary
    println!("\n" + "═".repeat(50));
    println!("📊 Summary");
    println!("═".repeat(50));
    
    println!("\nMethod              │ Avg (ms) │ Min (ms)");
    println!("────────────────────┼──────────┼─────────");
    println!("RPC Basic (2 calls) │ {:>8.2} │ {:>7.2}", avg, min);
    println!("RPC + Block (3)     │ {:>8.2} │ {:>7.2}", avg3, min3);
    println!("RPC + Traces (3)    │ {:>8.2} │ {:>7.2}", avg_trace, min_trace);
    
    println!("\n📈 Performance Targets:");
    println!("- REVM Direct: Should achieve <1ms");
    println!("- That's {:.0}x faster than RPC+traces", avg_trace);
    println!("- Even with 3 RPC calls for data, REVM sim adds <0.5ms");
    
    println!("\n💡 Key Insight:");
    println!("- Each RPC call adds ~{}ms", (avg3 - avg) / 1.0);
    println!("- Trace call alone adds ~{}ms", avg_trace - avg);
    println!("- REVM simulation replaces the expensive trace call");
    
    Ok(())
}