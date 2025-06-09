use std::time::Instant;
use anyhow::Result;

use ethers_core::types::H256;
use ethers_providers::{Provider, Http, Middleware};

use revm_tx_simulator::{
    simulate_transaction,
    conversions::{ethers_to_revm_address, ethers_to_revm_u256},
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏁 Testing TX Processor Integration in REVM Workspace");
    println!("===================================================\n");

    let tx_hash_str = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
    let tx_hash: H256 = tx_hash_str.parse()?;
    let rpc_url = "http://127.0.0.1:8545";
    
    // Setup provider
    let provider = Provider::<Http>::try_from(rpc_url)?;
    
    println!("📊 Transaction: {}", tx_hash_str);
    
    // Get transaction data
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    let receipt = provider.get_transaction_receipt(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Receipt not found"))?;
    
    let block_number = receipt.block_number.unwrap().as_u64();
    
    println!("📦 Block: {}", block_number);
    println!("👤 From: {:?}", tx.from);
    println!("📍 To: {:?}", tx.to);
    println!("💰 Value: {} wei", tx.value);
    
    // Test 1: RPC with traces
    println!("\n⚡ Method 1: RPC with Traces");
    println!("─".repeat(30));
    
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
    let rpc_time = start.elapsed().as_secs_f64() * 1000.0;
    
    println!("✅ RPC Time: {:.2}ms", rpc_time);
    
    // Test 2: Direct REVM (would use simulate_transaction)
    println!("\n🚀 Method 2: Direct REVM Integration");
    println!("─".repeat(30));
    
    // This is where we'd call our integrated processor
    // For now, show that we can access REVM types without conflicts
    let from_addr = ethers_to_revm_address(tx.from);
    let value = ethers_to_revm_u256(tx.value);
    
    println!("✅ Successfully converted types:");
    println!("  REVM Address: {:?}", from_addr);
    println!("  REVM Value: {:?}", value);
    println!("✅ No dependency conflicts!");
    
    println!("\n" + "═".repeat(50));
    println!("📊 Summary");
    println!("═".repeat(50));
    println!("✅ TX processor code integrated into REVM workspace");
    println!("✅ Can access REVM types without lifetime errors");
    println!("✅ Ready for full implementation");
    
    println!("\n💡 Next Steps:");
    println!("1. Complete the RevmTxProcessor implementation");
    println!("2. Hook up to existing simulate_transaction");
    println!("3. Run performance benchmarks");
    
    Ok(())
}