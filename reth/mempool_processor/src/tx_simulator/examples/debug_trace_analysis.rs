use ethers::prelude::*;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to local node
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    
    // Get a recent transaction that likely has complex interactions
    let block = provider.get_block(BlockNumber::Latest).await?.unwrap();
    println!("Latest block: {}", block.number.unwrap());
    
    // Get first few transactions from the block
    let txs = &block.transactions;
    
    for (i, tx_hash) in txs.iter().take(5).enumerate() {
        println!("\n--- Transaction {} ---", i + 1);
        println!("Hash: {:#x}", tx_hash);
        
        // Get transaction details
        let tx = provider.get_transaction(*tx_hash).await?.unwrap();
        println!("From: {:#x}", tx.from);
        println!("To: {:?}", tx.to.map(|a| format!("{:#x}", a)));
        println!("Value: {} ETH", ethers::utils::format_ether(tx.value));
        
        // Create call request
        let call_request = serde_json::json!({
            "from": format!("{:#x}", tx.from),
            "to": tx.to.map(|addr| format!("{:#x}", addr)),
            "value": format!("{:#x}", tx.value),
            "data": format!("0x{}", hex::encode(&tx.input)),
            "gas": format!("{:#x}", tx.gas),
            "gasPrice": format!("{:#x}", tx.gas_price.unwrap_or_default())
        });
        
        // Execute debug_traceCall
        match provider.request::<_, serde_json::Value>(
            "debug_traceCall",
            (call_request, format!("{:#x}", block.number.unwrap()), serde_json::json!({"tracer": "callTracer", "tracerConfig": {"withLog": true}}))
        ).await {
            Ok(trace) => {
                // Pretty print the trace
                let pretty = serde_json::to_string_pretty(&trace)?;
                
                // Count different elements
                let logs_count = trace.get("logs")
                    .and_then(|l| l.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                    
                let has_calls = trace.get("calls").is_some();
                let calls_count = trace.get("calls")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                
                println!("\nTrace summary:");
                println!("- Logs: {}", logs_count);
                println!("- Has internal calls: {}", has_calls);
                println!("- Number of calls: {}", calls_count);
                
                // Show first 500 chars of trace
                if pretty.len() > 500 {
                    println!("\nTrace (first 500 chars):\n{}", &pretty[..500]);
                } else {
                    println!("\nFull trace:\n{}", pretty);
                }
            }
            Err(e) => {
                println!("Failed to trace: {}", e);
            }
        }
    }
    
    Ok(())
}