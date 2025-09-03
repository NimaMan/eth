/// Process all transactions in a block
/// 
/// This example shows how to:
/// 1. Get all transactions from a block
/// 2. Process transactions with receipts
/// 3. Extract events and logs
/// 4. Calculate gas usage and fees
/// 5. Stream large blocks efficiently
/// 
/// Run with: cargo run --example block_transactions

use reth_chain_query::{RethQueryProvider, Result, BlockTransactionOptions, CallFrame};
use alloy_primitives::{Address, U256, utils::format_ether};
use std::str::FromStr;
use tx_simulator::TxSimulator;
use alloy_rpc_types_trace::geth::{GethDebugTracingOptions, GethDebugTracerType, GethDebugBuiltInTracerType, TraceResult, GethTrace};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Block Transaction Processing ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // === Get Latest Block Transactions ===
    println!("1. Latest Block Transactions");
    println!("-" .repeat(60));
    
    let latest_block = provider.get_latest_block()?;
    println!("Latest block: #{}", latest_block);
    
    // Get block with transactions
    let block = provider.get_block_with_txs(latest_block).await?;
    
    println!("Block #{} summary:", block.header.number);
    println!("  Timestamp: {}", block.header.timestamp);
    println!("  Gas used: {} / {}", block.header.gas_used, block.header.gas_limit);
    println!("  Base fee: {} gwei", block.header.base_fee_per_gas.unwrap_or(U256::ZERO) / U256::from(1_000_000_000));
    println!("  Transactions: {}", block.transactions.len());
    
    // Process first few transactions
    println!("\nFirst 5 transactions:");
    for (i, tx) in block.transactions.iter().take(5).enumerate() {
        println!("  {}. Hash: 0x{:x}", i + 1, tx.hash());
        println!("     From: 0x{}", tx.sender());
        println!("     To: {}", 
            tx.to().map(|a| format!("0x{}", a)).unwrap_or("Contract Creation".to_string())
        );
        println!("     Value: {} ETH", format_ether(tx.value()));
        println!("     Gas: {} @ {} gwei", 
            tx.gas_limit(),
            tx.max_fee_per_gas().unwrap_or(U256::ZERO) / U256::from(1_000_000_000)
        );
    }
    
    println!();
    
    // === Process Block with Receipts ===
    println!("2. Block Transactions with Receipts");
    println!("-" .repeat(60));
    
    // Get a specific historical block known to have interesting transactions
    let block_number = 20_000_000; // A round number block
    
    let block_txs = provider.get_block_transactions(block_number).await?;
    
    println!("Block #{} transaction analysis:", block_number);
    
    // Analyze transaction types
    let mut transfer_count = 0;
    let mut contract_creation = 0;
    let mut failed_count = 0;
    let mut total_gas_used = U256::ZERO;
    let mut total_value = U256::ZERO;
    
    for tx in &block_txs.transactions {
        if tx.to().is_none() {
            contract_creation += 1;
        }
        
        total_value = total_value + tx.value();
    }
    
    // Get receipts for all transactions
    let receipts = provider.get_block_receipts(block_number).await?;
    
    for receipt in &receipts {
        if !receipt.success {
            failed_count += 1;
        }
        total_gas_used = total_gas_used + U256::from(receipt.gas_used);
        
        // Check for Transfer events (ERC20)
        for log in &receipt.logs {
            if log.topics.len() > 0 {
                // Transfer event topic: 0xddf252ad...
                let transfer_topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
                if format!("0x{:x}", log.topics[0]) == transfer_topic {
                    transfer_count += 1;
                }
            }
        }
    }
    
    println!("  Total transactions: {}", block_txs.transactions.len());
    println!("  Contract creations: {}", contract_creation);
    println!("  Failed transactions: {}", failed_count);
    println!("  ERC20 transfers: {}", transfer_count);
    println!("  Total value moved: {} ETH", format_ether(total_value));
    println!("  Total gas used: {}", total_gas_used);
    
    println!();
    
    // === Find Specific Transaction Types ===
    println!("3. Find Specific Transaction Types");
    println!("-" .repeat(60));
    
    // Find DEX trades (transactions to Uniswap routers)
    let uniswap_v2_router = Address::from_str("7a250d5630B4cF539739dF2C5dAcb4c659F2488D")?;
    let uniswap_v3_router = Address::from_str("E592427A0AEce92De3Edee1F18E0157C05861564")?;
    
    let mut dex_trades = Vec::new();
    
    for tx in &block_txs.transactions {
        if let Some(to) = tx.to() {
            if to == uniswap_v2_router || to == uniswap_v3_router {
                dex_trades.push(tx);
            }
        }
    }
    
    println!("Found {} DEX trades in block:", dex_trades.len());
    for (i, tx) in dex_trades.iter().take(3).enumerate() {
        println!("  {}. 0x{:x}", i + 1, tx.hash());
        println!("     Value: {} ETH", format_ether(tx.value()));
        println!("     Router: {}", 
            if tx.to() == Some(uniswap_v2_router) { "Uniswap V2" } else { "Uniswap V3" }
        );
    }
    
    println!();
    
    // === Large Block Streaming ===
    println!("4. Large Block Processing");
    println!("-" .repeat(60));
    
    // Find a recent block
    let large_block = latest_block - 100; // 100 blocks ago
    
    println!("Processing block #{} in chunks...", large_block);
    
    // Process in batches for memory efficiency
    let block_data = provider.get_block_with_txs(large_block).await?;
    let chunk_size = 50;
    let total_txs = block_data.transactions.len();
    
    for (chunk_idx, chunk) in block_data.transactions.chunks(chunk_size).enumerate() {
        let start_idx = chunk_idx * chunk_size;
        let end_idx = std::cmp::min(start_idx + chunk_size, total_txs);
        
        println!("  Processing transactions {} to {}", start_idx + 1, end_idx);
        
        // Process chunk
        let mut chunk_value = U256::ZERO;
        for tx in chunk {
            chunk_value = chunk_value + tx.value();
        }
        
        println!("    Chunk value: {} ETH", format_ether(chunk_value));
    }
    
    println!();
    
    // === Transaction Ordering ===
    println!("5. Transaction Ordering Analysis");
    println!("-" .repeat(60));
    
    // Check for MEV patterns by analyzing transaction ordering
    let block_for_mev = latest_block - 10;
    let mev_block = provider.get_block_with_txs(block_for_mev).await?;
    
    println!("Analyzing block #{} for MEV patterns:", block_for_mev);
    
    // Look for sandwich attacks (same sender with transactions around another)
    let mut sender_positions: std::collections::HashMap<Address, Vec<usize>> = std::collections::HashMap::new();
    
    for (idx, tx) in mev_block.transactions.iter().enumerate() {
        sender_positions.entry(tx.sender()).or_insert_with(Vec::new).push(idx);
    }
    
    // Find potential sandwiches
    let mut potential_sandwiches = 0;
    for (sender, positions) in sender_positions {
        if positions.len() >= 2 {
            // Check if transactions are close together
            for window in positions.windows(2) {
                if window[1] - window[0] <= 3 {
                    potential_sandwiches += 1;
                    println!("  Potential sandwich by 0x{}: positions {} and {}", 
                        sender, window[0], window[1]);
                    break;
                }
            }
        }
    }
    
    if potential_sandwiches == 0 {
        println!("  No obvious sandwich patterns detected");
    }
    
    println!();
    
    // === Block with Traces ===
    println!("6. Block Transactions with Execution Traces");
    println!("-" .repeat(60));
    
    // Get a recent block with traces (only for blocks with contract interactions)
    let trace_block = 20_000_000;
    
    // Check if block needs traces (has contract interactions)
    if provider.block_needs_traces(trace_block).await? {
        println!("Block #{} has contract interactions, fetching with traces...", trace_block);
        
        // Get full block data including traces
        let block_with_traces = provider.get_block_transactions_with_options(
            trace_block,
            BlockTransactionOptions {
                include_receipts: true,
                include_traces: true,  // Enable trace fetching
            }
        ).await?;
        
        // Find transactions with interesting traces
        let mut complex_txs = 0;
        for tx_data in &block_with_traces.transactions {
            if let Some(trace) = &tx_data.trace {
                // Count transactions with multiple internal calls
                let call_count = count_calls(&trace.call_frame);
                if call_count > 1 {
                    complex_txs += 1;
                    if complex_txs <= 2 {
                        println!("\n  Transaction 0x{:x}:", tx_data.transaction.hash());
                        println!("    Internal calls: {}", call_count);
                        println!("    Gas used: {}", tx_data.receipt.as_ref().unwrap().gas_used);
                        print_trace_summary(&trace.call_frame, 2);
                    }
                }
            }
        }
        
        println!("\n  Total complex transactions (>1 internal call): {}", complex_txs);
    } else {
        println!("Block #{} has no contract interactions (simple transfers only)", trace_block);
    }
    
    println!();
    
    // === Block Tracing with tx_simulator ===
    println!("7. Block Tracing with tx_simulator (RPC Equivalent)");
    println!("-" .repeat(60));
    
    // Initialize tx_simulator for direct tracing
    let tx_sim = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    let block_tracer = tx_simulator::block_simulation::BlockTracer::new(&tx_sim);
    
    // Trace a recent block with CallTracer
    let trace_block_num = latest_block - 5; // Recent block
    println!("Tracing block #{} with tx_simulator...", trace_block_num);
    
    let tracer_config = GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::CallTracer
        )),
        ..Default::default()
    };
    
    let start = std::time::Instant::now();
    let traces = block_tracer.trace_block_by_number(trace_block_num, Some(tracer_config)).await?;
    let trace_time = start.elapsed();
    
    println!("  Traced {} transactions in {:.2}s", traces.len(), trace_time.as_secs_f64());
    
    // Analyze trace results
    let mut successful = 0;
    let mut failed = 0;
    let mut total_internal_calls = 0;
    
    for trace in &traces {
        match trace {
            TraceResult::Success { result, .. } => {
                successful += 1;
                if let GethTrace::CallTracer(frame) = result {
                    total_internal_calls += count_calls_in_geth_frame(frame);
                }
            }
            TraceResult::Error { .. } => {
                failed += 1;
            }
        }
    }
    
    println!("  Successful: {}, Failed: {}", successful, failed);
    println!("  Total internal calls: {}", total_internal_calls);
    
    // Show details of first complex transaction
    for (i, trace) in traces.iter().enumerate() {
        if let TraceResult::Success { result: GethTrace::CallTracer(frame), tx_hash } = trace {
            let call_count = count_calls_in_geth_frame(frame);
            if call_count > 5 {
                println!("\n  Complex transaction found:");
                println!("    Hash: 0x{:x}", tx_hash.unwrap_or_default());
                println!("    Position: #{}", i);
                println!("    Internal calls: {}", call_count);
                println!("    Gas used: {}", frame.gas_used);
                println!("    Type: {:?}", frame.typ);
                if let Some(to) = &frame.to {
                    println!("    To: 0x{:x}", to);
                }
                break;
            }
        }
    }
    
    println!("\n✅ Block transaction processing complete!");
    println!("\nNote: tx_simulator provides RPC-equivalent traces with direct DB access");
    
    Ok(())
}

fn count_calls(frame: &CallFrame) -> usize {
    1 + frame.calls.iter().map(|c| count_calls(c)).sum::<usize>()
}

fn count_calls_in_geth_frame(frame: &alloy_rpc_types_trace::geth::CallFrame) -> usize {
    1 + frame.calls.iter().map(|c| count_calls_in_geth_frame(c)).sum::<usize>()
}

fn print_trace_summary(frame: &CallFrame, indent: usize) {
    let prefix = " ".repeat(indent);
    println!("{}→ {} to 0x{}", 
        prefix,
        frame.call_type,
        frame.to.map(|a| format!("{}", a)).unwrap_or("CREATE".to_string())
    );
    
    // Show first few subcalls
    for (i, call) in frame.calls.iter().take(2).enumerate() {
        print_trace_summary(call, indent + 2);
        if i == 1 && frame.calls.len() > 2 {
            println!("{}  ... and {} more calls", prefix, frame.calls.len() - 2);
        }
    }
}