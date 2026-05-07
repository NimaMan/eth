use alloy_rpc_types_trace::geth::{CallConfig, GethDebugTracingOptions, GethTrace, TraceResult};
use eyre::Result;
use reth_provider::{BlockReader, HeaderProvider};
use std::time::Instant;
use tx_simulator::block_trace::block_tracer::{BlockTraceEngine, BlockTracer};
use tx_simulator::{CallFrame, TxSimulator};

/// Block transaction tracer.
///
/// This example replays all transactions in a persisted block sequentially from
/// the parent state and returns callTracer results, matching the semantics of
/// `debug_traceBlockByNumber` with `"tracer": "callTracer"`.
#[tokio::main]
async fn main() -> Result<()> {
    println!("Block tracer - debug_traceBlockByNumber callTracer equivalent");
    println!("===========================================================");

    let reth_db_path = tx_simulator::config::repo::reth_datadir()?;
    let simulator = TxSimulator::new(&reth_db_path)?;

    let latest_block = simulator.get_latest_block()?;
    let block_to_trace = latest_block;

    let provider = simulator.provider_factory().provider()?;
    let block = provider
        .block_by_number(block_to_trace)?
        .ok_or_else(|| eyre::eyre!("block {} not found", block_to_trace))?;
    let header = provider
        .header_by_number(block_to_trace)?
        .ok_or_else(|| eyre::eyre!("header {} not found", block_to_trace))?;

    println!("Block: {}", block_to_trace);
    println!("Transactions: {}", block.body.transactions.len());
    println!("Gas used: {}", header.gas_used);
    if let Some(base_fee) = header.base_fee_per_gas {
        println!("Base fee: {:.3} gwei", base_fee as f64 / 1e9);
    }

    let opts = GethDebugTracingOptions::call_tracer(CallConfig::default().with_log());
    let tracer = BlockTracer::new(&simulator);

    let started = Instant::now();
    let traces = tracer
        .trace_block_by_number_with_engine(
            block_to_trace,
            Some(opts),
            BlockTraceEngine::RethFusedCallTracer,
        )
        .await?;
    let elapsed = started.elapsed();

    let successes = traces
        .iter()
        .filter(|trace| matches!(trace, TraceResult::Success { .. }))
        .count();
    let errors = traces.len().saturating_sub(successes);

    println!();
    println!("Trace complete");
    println!("Successful traces: {}", successes);
    println!("Failed traces: {}", errors);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    if !traces.is_empty() {
        println!(
            "Average per tx: {:.2}ms",
            elapsed.as_millis() as f64 / traces.len() as f64
        );
    }

    println!();
    println!("Sample traces");
    for (idx, trace) in traces.iter().take(3).enumerate() {
        print_trace(idx, trace);
    }

    Ok(())
}

fn print_trace(index: usize, trace: &TraceResult) {
    match trace {
        TraceResult::Success {
            result: GethTrace::CallTracer(frame),
            tx_hash,
        } => {
            println!();
            println!("[{}] {:?}", index, tx_hash);
            print_call_frame(frame, 2);
        }
        TraceResult::Success { result, tx_hash } => {
            println!();
            println!(
                "[{}] {:?}: unexpected trace type {:?}",
                index, tx_hash, result
            );
        }
        TraceResult::Error { error, tx_hash } => {
            println!();
            println!("[{}] {:?}: {}", index, tx_hash, error);
        }
    }
}

fn print_call_frame(frame: &CallFrame, indent: usize) {
    let prefix = " ".repeat(indent);
    println!("{}type: {:?}", prefix, frame.typ);
    println!("{}from: {:?}", prefix, frame.from);
    println!("{}to: {:?}", prefix, frame.to);
    println!("{}gas: {}", prefix, frame.gas);
    println!("{}gas used: {}", prefix, frame.gas_used);

    if let Some(value) = &frame.value {
        println!("{}value: {} wei", prefix, value);
    }
    if let Some(error) = &frame.error {
        println!("{}error: {}", prefix, error);
    }
    if !frame.calls.is_empty() {
        println!("{}subcalls: {}", prefix, frame.calls.len());
        for subcall in frame.calls.iter().take(2) {
            print_call_frame(subcall, indent + 2);
        }
    }
}
