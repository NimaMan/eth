use std::collections::BTreeSet;
use std::time::Instant;

use clap::{Parser, ValueEnum};
use eyre::{eyre, Result};
use serde::Serialize;
use tx_simulator::TxSimulator;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BenchmarkMode {
    Raw,
    Both,
}

#[derive(Debug, Parser)]
#[command(about = "Profile block-scoped tx replay branches from one warmed prefix state")]
struct Args {
    #[arg(long, env = "RETH_DATADIR")]
    datadir: String,
    #[arg(long)]
    block: u64,
    #[arg(long, value_delimiter = ',')]
    tx_indexes: Vec<usize>,
    #[arg(long, default_value_t = 5)]
    iterations: usize,
    #[arg(long, default_value_t = 1)]
    warmup_iterations: usize,
    #[arg(long, value_enum, default_value_t = BenchmarkMode::Raw)]
    mode: BenchmarkMode,
    #[arg(long)]
    json_summary: bool,
}

#[derive(Debug, Clone, Serialize)]
struct MeasurementRow {
    iteration: usize,
    warmup: bool,
    block_number: u64,
    tx_index: usize,
    tx_hash: String,
    session_load_ms: f64,
    prefix_advance_ms: f64,
    prefix_applied_txs: usize,
    branch_trace_clone_ms: f64,
    trace_ms: f64,
    branch_execute_clone_ms: f64,
    execute_ms: f64,
    trace_gas_used: u64,
    execute_gas_used: u64,
    trace_success: bool,
    execute_success: bool,
    raw_correct: bool,
}

#[derive(Debug, Clone, Serialize)]
struct Summary {
    block_number: u64,
    tx_count: usize,
    measured_rows: usize,
    warmed_branch_trace_p50_ms: f64,
    warmed_branch_trace_p95_ms: f64,
    warmed_branch_trace_max_ms: f64,
    warmed_branch_execute_p50_ms: f64,
    warmed_branch_execute_p95_ms: f64,
    warmed_branch_execute_max_ms: f64,
    correctness_failures: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if matches!(args.mode, BenchmarkMode::Both) {
        eprintln!(
            "--mode both currently measures raw tx branches in tx_simulator; token pool branch timing is exercised through tx_processor/eth_token integration"
        );
    }

    let simulator = TxSimulator::new(&args.datadir)?;
    let total_iterations = args.warmup_iterations + args.iterations;
    let mut rows = Vec::new();
    let mut tx_count = 0usize;

    for iteration in 0..total_iterations {
        let warmup = iteration < args.warmup_iterations;
        let load_started = Instant::now();
        let mut session = simulator.block_tx_state_session(args.block).await?;
        let session_load_ms = elapsed_ms(load_started);
        tx_count = session.transaction_count();
        let indexes = resolve_tx_indexes(&args.tx_indexes, tx_count)?;

        for tx_index in indexes {
            let advance = session.advance_to_before_tx(tx_index)?;
            let trace = session.trace_mined_tx_at_index(tx_index)?;
            let execute = session.execute_mined_tx_at_index(tx_index)?;
            let raw_correct = trace.result.success == execute.result.success
                && trace.result.gas_used == execute.result.gas_used;

            rows.push(MeasurementRow {
                iteration,
                warmup,
                block_number: session.block_number(),
                tx_index,
                tx_hash: format!("{:#x}", trace.tx_hash),
                session_load_ms,
                prefix_advance_ms: advance.elapsed_ms,
                prefix_applied_txs: advance.applied_txs,
                branch_trace_clone_ms: trace.branch_clone_ms,
                trace_ms: trace.trace_ms,
                branch_execute_clone_ms: execute.branch_clone_ms,
                execute_ms: execute.execute_ms,
                trace_gas_used: trace.result.gas_used,
                execute_gas_used: execute.result.gas_used,
                trace_success: trace.result.success,
                execute_success: execute.result.success,
                raw_correct,
            });
        }
    }

    let measured: Vec<_> = rows.iter().filter(|row| !row.warmup).cloned().collect();
    let summary = summarize(args.block, tx_count, &measured);

    if args.json_summary {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    print_rows(&measured);
    print_summary(&summary);
    Ok(())
}

fn resolve_tx_indexes(requested: &[usize], tx_count: usize) -> Result<Vec<usize>> {
    if tx_count == 0 {
        return Err(eyre!("block has no transactions"));
    }

    let mut indexes = BTreeSet::new();
    if requested.is_empty() {
        indexes.extend([0, 1, 2, 3].into_iter().filter(|idx| *idx < tx_count));
        indexes.insert(tx_count / 4);
        indexes.insert(tx_count / 2);
        indexes.insert(tx_count - 1);
    } else {
        for index in requested {
            if *index >= tx_count {
                return Err(eyre!(
                    "tx index {} out of range for block with {} txs",
                    index,
                    tx_count
                ));
            }
            indexes.insert(*index);
        }
    }

    Ok(indexes.into_iter().collect())
}

fn summarize(block_number: u64, tx_count: usize, rows: &[MeasurementRow]) -> Summary {
    let mut branch_trace_ms: Vec<_> = rows
        .iter()
        .map(|row| row.branch_trace_clone_ms + row.trace_ms)
        .collect();
    let mut branch_execute_ms: Vec<_> = rows
        .iter()
        .map(|row| row.branch_execute_clone_ms + row.execute_ms)
        .collect();
    let correctness_failures = rows.iter().filter(|row| !row.raw_correct).count();

    Summary {
        block_number,
        tx_count,
        measured_rows: rows.len(),
        warmed_branch_trace_p50_ms: percentile(&mut branch_trace_ms, 0.50),
        warmed_branch_trace_p95_ms: percentile(&mut branch_trace_ms, 0.95),
        warmed_branch_trace_max_ms: max(&branch_trace_ms),
        warmed_branch_execute_p50_ms: percentile(&mut branch_execute_ms, 0.50),
        warmed_branch_execute_p95_ms: percentile(&mut branch_execute_ms, 0.95),
        warmed_branch_execute_max_ms: max(&branch_execute_ms),
        correctness_failures,
    }
}

fn percentile(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let index = ((values.len() - 1) as f64 * q).ceil() as usize;
    values[index]
}

fn max(values: &[f64]) -> f64 {
    values.iter().copied().fold(0.0, f64::max)
}

fn print_rows(rows: &[MeasurementRow]) {
    println!(
        "block,tx_index,tx_hash,session_load_ms,prefix_advance_ms,prefix_applied_txs,branch_trace_ms,branch_execute_ms,trace_gas,execute_gas,correct"
    );
    for row in rows {
        println!(
            "{},{},{},{:.3},{:.3},{},{:.3},{:.3},{},{},{}",
            row.block_number,
            row.tx_index,
            row.tx_hash,
            row.session_load_ms,
            row.prefix_advance_ms,
            row.prefix_applied_txs,
            row.branch_trace_clone_ms + row.trace_ms,
            row.branch_execute_clone_ms + row.execute_ms,
            row.trace_gas_used,
            row.execute_gas_used,
            row.raw_correct
        );
    }
}

fn print_summary(summary: &Summary) {
    println!();
    println!(
        "summary block={} tx_count={} rows={}",
        summary.block_number, summary.tx_count, summary.measured_rows
    );
    println!(
        "warmed_branch_trace_ms p50={:.3} p95={:.3} max={:.3}",
        summary.warmed_branch_trace_p50_ms,
        summary.warmed_branch_trace_p95_ms,
        summary.warmed_branch_trace_max_ms
    );
    println!(
        "warmed_branch_execute_ms p50={:.3} p95={:.3} max={:.3}",
        summary.warmed_branch_execute_p50_ms,
        summary.warmed_branch_execute_p95_ms,
        summary.warmed_branch_execute_max_ms
    );
    println!("correctness_failures={}", summary.correctness_failures);
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}
