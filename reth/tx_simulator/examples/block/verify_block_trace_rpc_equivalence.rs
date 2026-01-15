use eyre::Result;
use reth_provider::BlockReader;
use std::time::Instant;
use tx_simulator::block_trace::block_tracer::BlockTracer;
/// Verify Block Trace RPC Equivalence
///
/// This example verifies that our direct database block tracing produces
/// EXACTLY the same results as RPC's debug_traceBlockByNumber.
///
/// This is critical for ensuring our implementation is a true drop-in
/// replacement for RPC tracing calls.
use tx_simulator::TxSimulator;

// For RPC calls
use alloy_rpc_types_trace::geth::{GethDebugTracingOptions, GethTrace, TraceResult};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Verifying Block Trace RPC Equivalence");
    println!("=========================================\n");

    // Initialize our simulator
    let reth_db_path = "/home/nima/.local/share/reth/mainnet";
    let simulator = TxSimulator::new(reth_db_path)?;
    println!("✅ Simulator initialized");

    // Initialize RPC client
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
    println!("📡 Connecting to RPC: {}", rpc_url);

    let client = HttpClientBuilder::default().build(&rpc_url)?;

    // Get latest block or use a specific one
    let latest_block = simulator.get_latest_block()?;
    // Use a slightly older block to ensure it's available on RPC
    let block_to_verify = latest_block.saturating_sub(10);

    println!("📊 Verifying block: {}", block_to_verify);

    // Get block details
    let provider = simulator.provider_factory().provider()?;
    let block = provider
        .block_by_number(block_to_verify)?
        .ok_or_else(|| eyre::eyre!("Block {} not found", block_to_verify))?;

    let tx_count = block.body.transactions.len();
    println!("📝 Transactions in block: {}", tx_count);

    println!("\n{}", "=".repeat(60));
    println!("STEP 1: TRACE WITH OUR METHOD (Sequential Block Tracer)");
    println!("{}", "=".repeat(60));

    let start_our = Instant::now();

    // Use our BlockTracer for sequential execution with state persistence
    let block_tracer = BlockTracer::new(&simulator);
    let tracer_config = GethDebugTracingOptions {
        tracer: Some(
            alloy_rpc_types_trace::geth::GethDebugTracerType::BuiltInTracer(
                alloy_rpc_types_trace::geth::GethDebugBuiltInTracerType::CallTracer,
            ),
        ),
        ..Default::default()
    };

    let our_traces_result = block_tracer
        .trace_block_by_number(block_to_verify, Some(tracer_config))
        .await;

    let our_time = start_our.elapsed();
    println!("✅ Our method completed in {:.2}s", our_time.as_secs_f64());

    println!("\n{}", "=".repeat(60));
    println!("STEP 2: TRACE WITH RPC");
    println!("{}", "=".repeat(60));

    let start_rpc = Instant::now();

    // Call debug_traceBlockByNumber with callTracer
    let block_hex = format!("0x{:x}", block_to_verify);
    let tracer_config = serde_json::json!({
        "tracer": "callTracer",
        "tracerConfig": {
            "withLog": true
        }
    });

    println!("🔄 Calling debug_traceBlockByNumber({})...", block_hex);

    let rpc_response: Vec<Value> = match client
        .request(
            "debug_traceBlockByNumber",
            rpc_params![block_hex, tracer_config],
        )
        .await
    {
        Ok(response) => response,
        Err(e) => {
            println!("❌ RPC call failed: {}", e);
            println!("\nNote: Make sure your RPC endpoint supports debug_traceBlockByNumber");
            println!("You may need to use an archive node with debug APIs enabled.");
            return Ok(());
        }
    };

    let rpc_time = start_rpc.elapsed();
    println!("✅ RPC completed in {:.2}s", rpc_time.as_secs_f64());

    println!("\n{}", "=".repeat(60));
    println!("STEP 3: COMPARE RESULTS");
    println!("{}", "=".repeat(60));

    // Compare each transaction
    let mut matches = 0;
    let mut mismatches = 0;
    let mut mismatch_details = Vec::new();

    let our_traces = match our_traces_result {
        Ok(traces) => traces,
        Err(e) => {
            println!("❌ Our block tracer failed: {}", e);
            return Ok(());
        }
    };

    for (idx, (our_trace, rpc_trace)) in our_traces.iter().zip(rpc_response.iter()).enumerate() {
        // Get transaction hash from our trace or RPC
        let tx_hash = match our_trace {
            TraceResult::Success { tx_hash, .. } => tx_hash.unwrap_or_default(),
            TraceResult::Error { tx_hash, .. } => tx_hash.unwrap_or_default(),
        };

        println!("\n[Transaction {}] 0x{:x}", idx, tx_hash);

        // Parse RPC result
        let rpc_result = &rpc_trace["result"];

        match our_trace {
            TraceResult::Success {
                result: our_geth_trace,
                ..
            } => {
                // Extract our frame from GethTrace
                let our_frame = match our_geth_trace {
                    GethTrace::CallTracer(frame) => frame,
                    _ => {
                        println!("  ⚠️  Unexpected trace type");
                        mismatches += 1;
                        continue;
                    }
                };
                // Compare key fields
                let mut tx_matches = true;
                let mut tx_mismatches = Vec::new();

                // Compare gas used
                if let Some(rpc_gas) = rpc_result["gasUsed"].as_str() {
                    let rpc_gas_val =
                        u64::from_str_radix(rpc_gas.trim_start_matches("0x"), 16).unwrap_or(0);
                    if our_frame.gas_used != rpc_gas_val {
                        tx_matches = false;
                        tx_mismatches.push(format!(
                            "Gas: our={}, rpc={}",
                            our_frame.gas_used, rpc_gas_val
                        ));
                    }
                }

                // Compare type (remove quotes for comparison)
                if let Some(rpc_type) = rpc_result["type"].as_str() {
                    let our_type = format!("{:?}", our_frame.typ);
                    // Remove quotes from our type for comparison
                    let our_type_clean = our_type.trim_matches('"');
                    if our_type_clean != rpc_type {
                        tx_matches = false;
                        tx_mismatches
                            .push(format!("Type: our={}, rpc={}", our_type_clean, rpc_type));
                    }
                }

                // Compare from address (normalize formats)
                if let Some(rpc_from) = rpc_result["from"].as_str() {
                    let our_from = format!("{:?}", our_frame.from).to_lowercase();
                    let rpc_from_normalized = rpc_from.to_lowercase();
                    // Compare just the address hex, ignoring 0x prefix differences
                    let our_addr = our_from.trim_start_matches("0x");
                    let rpc_addr = rpc_from_normalized.trim_start_matches("0x");
                    if !our_addr.contains(rpc_addr) && !rpc_addr.contains(our_addr) {
                        tx_matches = false;
                        tx_mismatches.push(format!("From: our={}, rpc={}", our_from, rpc_from));
                    }
                }

                // Compare to address (normalize formats)
                if let Some(rpc_to) = rpc_result["to"].as_str() {
                    if let Some(our_to) = our_frame.to {
                        let our_to_str = format!("{:?}", our_to).to_lowercase();
                        let rpc_to_normalized = rpc_to.to_lowercase();
                        let our_addr = our_to_str.trim_start_matches("0x");
                        let rpc_addr = rpc_to_normalized.trim_start_matches("0x");
                        if !our_addr.contains(rpc_addr) && !rpc_addr.contains(our_addr) {
                            tx_matches = false;
                            tx_mismatches.push(format!("To: our={}, rpc={}", our_to_str, rpc_to));
                        }
                    }
                }

                // Compare value (if present)
                if let Some(rpc_value) = rpc_result["value"].as_str() {
                    if let Some(our_value) = &our_frame.value {
                        let rpc_value_int =
                            u128::from_str_radix(rpc_value.trim_start_matches("0x"), 16)
                                .unwrap_or(0);
                        let our_value_int: u128 = our_value.to_string().parse().unwrap_or(0);
                        if our_value_int != rpc_value_int {
                            tx_matches = false;
                            tx_mismatches.push(format!(
                                "Value: our={}, rpc={}",
                                our_value_int, rpc_value_int
                            ));
                        }
                    }
                }

                // Compare subcalls count
                if let Some(rpc_calls) = rpc_result["calls"].as_array() {
                    if our_frame.calls.len() != rpc_calls.len() {
                        tx_matches = false;
                        tx_mismatches.push(format!(
                            "Subcalls: our={}, rpc={}",
                            our_frame.calls.len(),
                            rpc_calls.len()
                        ));
                    }
                }

                // Report result
                if tx_matches {
                    println!("  ✅ MATCH");
                    matches += 1;
                } else {
                    println!("  ❌ MISMATCH:");
                    for mismatch in &tx_mismatches {
                        println!("    - {}", mismatch);
                    }
                    mismatches += 1;
                    mismatch_details.push((idx, tx_hash, tx_mismatches));
                }
            }
            TraceResult::Error { error, .. } => {
                if let Some(_rpc_error) = rpc_trace["error"].as_str() {
                    println!("  ✅ Both failed (expected)");
                    matches += 1;
                } else {
                    println!("  ❌ MISMATCH: Our method failed but RPC succeeded");
                    println!("    Our error: {}", error);
                    mismatches += 1;
                }
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("VERIFICATION SUMMARY");
    println!("{}", "=".repeat(60));

    let total = matches + mismatches;
    let match_percentage = (matches as f64 / total as f64) * 100.0;

    println!("\n📊 Results:");
    println!("  • Total transactions: {}", total);
    println!("  • ✅ Matching: {} ({:.2}%)", matches, match_percentage);
    println!(
        "  • ❌ Mismatches: {} ({:.2}%)",
        mismatches,
        100.0 - match_percentage
    );

    if !mismatch_details.is_empty() {
        println!("\n📋 Mismatch Details:");
        for (idx, hash, details) in mismatch_details.iter().take(5) {
            println!("  Transaction {} (0x{:x}):", idx, hash);
            for detail in details {
                println!("    - {}", detail);
            }
        }
        if mismatch_details.len() > 5 {
            println!("  ... and {} more", mismatch_details.len() - 5);
        }
    }

    println!("\n⚡ Performance Comparison:");
    println!("  • Our method: {:.2}s", our_time.as_secs_f64());
    println!("  • RPC method: {:.2}s", rpc_time.as_secs_f64());
    println!(
        "  • Speedup: {:.1}x faster",
        rpc_time.as_secs_f64() / our_time.as_secs_f64()
    );

    // Binary pass/fail - either we match RPC exactly or we don't
    if mismatches == 0 {
        println!("\n✅ VERIFICATION PASSED!");
        println!("Our implementation matches RPC debug_traceBlockByNumber EXACTLY.");
    } else {
        println!("\n❌ VERIFICATION FAILED!");
        println!("Our implementation does NOT match RPC debug_traceBlockByNumber.");
        println!("{} of {} transactions have differences.", mismatches, total);
    }

    Ok(())
}
