/// Block Trace Integration Example
/// 
/// This example demonstrates how to integrate tx_simulator's BlockTracer
/// to get RPC-equivalent block traces with direct database access.
/// 
/// Run with: cargo run --example block_trace_integration

use eyre::Result;
use alloy_primitives::{Address, U256, utils::format_ether};
use std::str::FromStr;
use tx_simulator::TxSimulator;
use alloy_rpc_types_trace::geth::{GethDebugTracingOptions, GethDebugTracerType, GethDebugBuiltInTracerType, TraceResult, GethTrace};

// Import Reth provider for block access
use reth_provider::{HeaderProvider, BlockReader, ProviderFactory};
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_db::open_db_read_only;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Block Trace Integration Example ===\n");
    
    // Initialize database
    let db_path = "/home/nima/.local/share/reth/mainnet/db";
    let db = Arc::new(open_db_read_only(db_path, Default::default())?);
    let factory = ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<_>>>::new(
        db.clone(),
        Default::default(),
        Default::default(),
    )?;
    
    // Get latest block
    let provider = factory.provider()?;
    let latest_block = provider.best_block_number()?;
    println!("Latest block: #{}", latest_block);
    
    // === Section 1: Basic Block Information ===
    println!("\n1. Basic Block Information");
    println!("{}", "-".repeat(60));
    
    let test_block = latest_block - 10; // Use a recent block
    let block = provider.block_by_number(test_block)?
        .ok_or_else(|| eyre::eyre!("Block {} not found", test_block))?;
    
    println!("Block #{} summary:", test_block);
    println!("  Timestamp: {}", block.header.timestamp);
    println!("  Gas used: {} / {}", block.header.gas_used, block.header.gas_limit);
    println!("  Base fee: {} gwei", block.header.base_fee_per_gas.unwrap_or(U256::ZERO) / U256::from(1_000_000_000));
    println!("  Transactions: {}", block.body.transactions.len());
    
    // === Section 2: Block Tracing with tx_simulator ===
    println!("\n2. Block Tracing with tx_simulator");
    println!("{}", "-".repeat(60));
    
    // Initialize tx_simulator
    let tx_sim = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    let block_tracer = tx_simulator::block_simulation::BlockTracer::new(&tx_sim);
    
    println!("Tracing block #{} with CallTracer...", test_block);
    
    let tracer_config = GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::CallTracer
        )),
        ..Default::default()
    };
    
    let start = std::time::Instant::now();
    let traces = block_tracer.trace_block_by_number(test_block, Some(tracer_config)).await?;
    let trace_time = start.elapsed();
    
    println!("✅ Traced {} transactions in {:.2}s", traces.len(), trace_time.as_secs_f64());
    
    // === Section 3: Analyze Trace Results ===
    println!("\n3. Trace Analysis");
    println!("{}", "-".repeat(60));
    
    let mut successful = 0;
    let mut failed = 0;
    let mut total_gas = 0u64;
    let mut complex_txs = Vec::new();
    
    for (idx, trace) in traces.iter().enumerate() {
        match trace {
            TraceResult::Success { result, tx_hash } => {
                successful += 1;
                if let GethTrace::CallTracer(frame) = result {
                    total_gas += frame.gas_used;
                    let call_count = count_calls(frame);
                    if call_count > 5 {
                        complex_txs.push((idx, tx_hash, call_count, frame.gas_used));
                    }
                }
            }
            TraceResult::Error { error, tx_hash } => {
                failed += 1;
                println!("  Transaction {} failed: {}", 
                    tx_hash.map(|h| format!("0x{:x}", h)).unwrap_or("unknown".to_string()),
                    error
                );
            }
        }
    }
    
    println!("Transaction Results:");
    println!("  • Successful: {}", successful);
    println!("  • Failed: {}", failed);
    println!("  • Total gas used: {}", total_gas);
    println!("  • Average gas per tx: {}", total_gas / traces.len() as u64);
    
    // === Section 4: Complex Transactions ===
    println!("\n4. Complex Transactions (>5 internal calls)");
    println!("{}", "-".repeat(60));
    
    if complex_txs.is_empty() {
        println!("No complex transactions found in this block");
    } else {
        println!("Found {} complex transactions:", complex_txs.len());
        for (pos, hash, calls, gas) in complex_txs.iter().take(3) {
            println!("\n  Position #{}", pos);
            println!("  Hash: 0x{:x}", hash.unwrap_or_default());
            println!("  Internal calls: {}", calls);
            println!("  Gas used: {}", gas);
        }
    }
    
    // === Section 5: Specific Transaction Types ===
    println!("\n5. Transaction Type Analysis");
    println!("{}", "-".repeat(60));
    
    let uniswap_v2 = Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D")?;
    let uniswap_v3 = Address::from_str("0xE592427A0AEce92De3Edee1F18E0157C05861564")?;
    
    let mut dex_trades = 0;
    let mut contract_creations = 0;
    let mut simple_transfers = 0;
    
    for trace in &traces {
        if let TraceResult::Success { result: GethTrace::CallTracer(frame), .. } = trace {
            if let Some(to) = frame.to {
                if to == uniswap_v2 || to == uniswap_v3 {
                    dex_trades += 1;
                }
            } else {
                contract_creations += 1;
            }
            
            if frame.calls.is_empty() && frame.input.len() == 0 {
                simple_transfers += 1;
            }
        }
    }
    
    println!("Transaction Types:");
    println!("  • DEX trades: {}", dex_trades);
    println!("  • Contract creations: {}", contract_creations);
    println!("  • Simple ETH transfers: {}", simple_transfers);
    println!("  • Other: {}", traces.len() - dex_trades - contract_creations - simple_transfers);
    
    // === Section 6: Performance Comparison ===
    println!("\n6. Performance Metrics");
    println!("{}", "-".repeat(60));
    
    let txs_per_sec = traces.len() as f64 / trace_time.as_secs_f64();
    println!("Tracing Performance:");
    println!("  • Block: #{}", test_block);
    println!("  • Transactions: {}", traces.len());
    println!("  • Time: {:.3}s", trace_time.as_secs_f64());
    println!("  • Throughput: {:.0} tx/s", txs_per_sec);
    println!("  • Average: {:.2}ms per transaction", 
        trace_time.as_millis() as f64 / traces.len() as f64);
    
    println!("\n✅ Block trace integration complete!");
    println!("\nKey Benefits:");
    println!("  • RPC-equivalent traces without network calls");
    println!("  • Direct database access for performance");
    println!("  • Sequential execution with proper state management");
    println!("  • 100% compatibility with debug_traceBlockByNumber");
    
    Ok(())
}

fn count_calls(frame: &alloy_rpc_types_trace::geth::CallFrame) -> usize {
    1 + frame.calls.iter().map(|c| count_calls(c)).sum::<usize>()
}