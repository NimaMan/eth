use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use alloy_rpc_types_trace::geth::{
    GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
};
use clap::{Parser, ValueEnum};
use eyre::{bail, Result};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use serde_json::{json, Value};
use tx_simulator::block_simulation::{
    BlockReplayProfile, BlockTraceEngine, BlockTracer, TransactionReplayProfile,
};
use tx_simulator::TxSimulator;

#[derive(Debug, Parser)]
#[command(about = "Cold replay profiler for tx_simulator block tracing")]
struct Args {
    /// Reth datadir. Defaults to tx_simulator config/env resolution.
    #[arg(long)]
    datadir: Option<String>,

    /// Comma-separated block numbers.
    #[arg(long)]
    blocks: Option<String>,

    /// First block in a contiguous range.
    #[arg(long)]
    start: Option<u64>,

    /// Last block in a contiguous range. Defaults to --start.
    #[arg(long)]
    end: Option<u64>,

    /// Number of repeated iterations per scenario.
    #[arg(long, default_value_t = 3)]
    iterations: usize,

    /// Candidate engine for stage-breakdown mode.
    #[arg(long, value_enum, default_value_t = EngineArg::BaselineFresh)]
    engine: EngineArg,

    /// Profiler mode.
    #[arg(long, value_enum, default_value_t = Mode::EngineSweep)]
    mode: Mode,

    /// Optional CSV output path. Defaults to stdout.
    #[arg(long)]
    output: Option<PathBuf>,

    /// Also emit one CSV row per transaction.
    #[arg(long)]
    per_tx: bool,

    /// Optional local Reth RPC URL for correctness comparison.
    #[arg(long)]
    rpc_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    EngineSweep,
    StageBreakdown,
    Correctness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum EngineArg {
    BaselineFresh,
    TracingFused,
    RethDebug,
}

impl EngineArg {
    const fn as_engine(self) -> BlockTraceEngine {
        match self {
            Self::BaselineFresh => BlockTraceEngine::FreshInspector,
            Self::TracingFused => BlockTraceEngine::RethFusedCallTracer,
            Self::RethDebug => BlockTraceEngine::RethDebug,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let datadir = args
        .datadir
        .clone()
        .map(Ok)
        .unwrap_or_else(tx_simulator::config::repo::reth_datadir)?;
    let simulator = TxSimulator::new(&datadir)?;
    let tracer = BlockTracer::new(&simulator);
    let blocks = selected_blocks(&args, simulator.get_latest_block()?)?;

    match args.mode {
        Mode::Correctness => run_correctness(&tracer, &blocks, args.rpc_url.as_deref()).await,
        Mode::EngineSweep | Mode::StageBreakdown => run_profile(&args, &tracer, &blocks).await,
    }
}

async fn run_profile(args: &Args, tracer: &BlockTracer<'_>, blocks: &[u64]) -> Result<()> {
    let mut writer = output_writer(args.output.as_ref())?;
    print_header(&mut writer)?;

    let engines: Vec<BlockTraceEngine> = match args.mode {
        Mode::EngineSweep => vec![
            BlockTraceEngine::FreshInspector,
            BlockTraceEngine::RethFusedCallTracer,
            BlockTraceEngine::RethDebug,
        ],
        Mode::StageBreakdown => vec![args.engine.as_engine()],
        Mode::Correctness => unreachable!("correctness is handled separately"),
    };

    let mut summaries: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
    for iteration in 0..args.iterations.max(1) {
        for engine in &engines {
            for block_number in blocks.iter().copied() {
                let profiled = tracer
                    .trace_block_by_number_profiled(
                        block_number,
                        Some(call_tracer_options()),
                        *engine,
                    )
                    .await?;
                let profile = &profiled.profile;
                print_block_row(&mut writer, "cold_replay", iteration, profile)?;
                if args.per_tx {
                    for tx in &profile.tx_profiles {
                        print_tx_row(&mut writer, "cold_replay", iteration, profile, tx)?;
                    }
                }
                summaries
                    .entry(profile.engine)
                    .or_default()
                    .push(profile.total_ms);
            }
        }
    }

    for (engine, values) in summaries {
        let summary = Summary::from_values(&values);
        writeln!(
            writer,
            "# summary engine={} count={} avg_ms={:.3} median_ms={:.3} p95_ms={:.3} max_ms={:.3}",
            engine, summary.count, summary.avg, summary.median, summary.p95, summary.max
        )?;
    }
    Ok(())
}

async fn run_correctness(
    tracer: &BlockTracer<'_>,
    blocks: &[u64],
    rpc_url: Option<&str>,
) -> Result<()> {
    for block_number in blocks.iter().copied() {
        let baseline = tracer
            .trace_block_by_number_profiled(
                block_number,
                Some(call_tracer_options()),
                BlockTraceEngine::FreshInspector,
            )
            .await?;
        let baseline_value = normalize_traces(&baseline.traces)?;

        for engine in [
            BlockTraceEngine::RethFusedCallTracer,
            BlockTraceEngine::RethDebug,
        ] {
            let candidate = tracer
                .trace_block_by_number_profiled(block_number, Some(call_tracer_options()), engine)
                .await?;
            let candidate_value = normalize_traces(&candidate.traces)?;
            if baseline_value != candidate_value {
                let diff = first_json_diff("$", &baseline_value, &candidate_value)
                    .unwrap_or_else(|| "values differ but no focused diff was found".to_string());
                bail!(
                    "trace mismatch block={} engine={}: {}",
                    block_number,
                    engine.as_str(),
                    diff
                );
            }
            println!(
                "# correctness block={} engine={} ok txs={} trace_nodes={}",
                block_number,
                engine.as_str(),
                candidate.profile.tx_count,
                candidate.profile.trace_node_count
            );
        }

        if let Some(rpc_url) = rpc_url {
            compare_reth_debug_to_rpc(tracer, rpc_url, block_number).await?;
        }
    }
    Ok(())
}

async fn compare_reth_debug_to_rpc(
    tracer: &BlockTracer<'_>,
    rpc_url: &str,
    block_number: u64,
) -> Result<()> {
    let local = tracer
        .trace_block_by_number_profiled(
            block_number,
            Some(call_tracer_options()),
            BlockTraceEngine::RethDebug,
        )
        .await?;
    let local_value = normalize_traces(&local.traces)?;

    let client = HttpClientBuilder::default().build(rpc_url)?;
    let block_hex = format!("0x{block_number:x}");
    let tracer_config = json!({
        "tracer": "callTracer",
        "tracerConfig": {}
    });
    let rpc_response: Vec<Value> = client
        .request(
            "debug_traceBlockByNumber",
            rpc_params![block_hex, tracer_config],
        )
        .await?;
    let rpc_value = Value::Array(rpc_response);

    if local_value != rpc_value {
        let diff = first_json_diff("$", &local_value, &rpc_value)
            .unwrap_or_else(|| "values differ but no focused diff was found".to_string());
        bail!("reth-debug RPC mismatch block={block_number}: {diff}");
    }

    println!(
        "# rpc_correctness block={} engine=reth-debug ok txs={} trace_nodes={}",
        block_number, local.profile.tx_count, local.profile.trace_node_count
    );
    Ok(())
}

fn selected_blocks(args: &Args, latest: u64) -> Result<Vec<u64>> {
    if let Some(blocks) = args.blocks.as_deref() {
        let parsed = blocks
            .split(',')
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().parse::<u64>().map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        if parsed.is_empty() {
            bail!("--blocks did not contain any block numbers");
        }
        return Ok(parsed);
    }

    if let Some(start) = args.start {
        let end = args.end.unwrap_or(start);
        if end < start {
            bail!("--end {} is before --start {}", end, start);
        }
        return Ok((start..=end).collect());
    }

    Ok(vec![latest])
}

fn output_writer(path: Option<&PathBuf>) -> Result<Box<dyn Write>> {
    match path {
        Some(path) => Ok(Box::new(BufWriter::new(File::create(path)?))),
        None => Ok(Box::new(BufWriter::new(io::stdout()))),
    }
}

fn print_header(writer: &mut dyn Write) -> Result<()> {
    writeln!(
        writer,
        "row_type,scenario,iteration,block,tx_index,tx_hash,engine,txs,gas_used,trace_nodes,total_ms,block_hash_lookup_ms,block_load_ms,state_open_ms,sender_recovery_ms,evm_env_ms,tx_env_ms,inspector_build_ms,evm_exec_ms,trace_build_ms,db_commit_ms,account_reads,storage_reads,code_reads,block_hash_reads,provider_read_ms,errors"
    )?;
    Ok(())
}

fn print_block_row(
    writer: &mut dyn Write,
    scenario: &str,
    iteration: usize,
    profile: &BlockReplayProfile,
) -> Result<()> {
    writeln!(
        writer,
        "block,{},{},{},,,{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{:.3},{}",
        scenario,
        iteration,
        profile.block_number,
        profile.engine,
        profile.tx_count,
        profile.gas_used,
        profile.trace_node_count,
        profile.total_ms,
        profile.block_hash_lookup_ms,
        profile.block_load_ms,
        profile.state_open_ms,
        profile.sender_recovery_ms,
        profile.evm_env_ms,
        profile.tx_env_ms,
        profile.inspector_build_ms,
        profile.evm_exec_ms,
        profile.trace_build_ms,
        profile.db_commit_ms,
        profile.state_reads.account_reads,
        profile.state_reads.storage_reads,
        profile.state_reads.code_reads,
        profile.state_reads.block_hash_reads,
        profile.state_reads.provider_read_ms,
        profile.errors
    )?;
    Ok(())
}

fn print_tx_row(
    writer: &mut dyn Write,
    scenario: &str,
    iteration: usize,
    block: &BlockReplayProfile,
    tx: &TransactionReplayProfile,
) -> Result<()> {
    let total_ms = tx.sender_recovery_ms
        + tx.evm_env_ms
        + tx.tx_env_ms
        + tx.inspector_build_ms
        + tx.evm_exec_ms
        + tx.trace_build_ms
        + tx.db_commit_ms;
    writeln!(
        writer,
        "tx,{},{},{},{},{:#x},{},,{},{},{:.3},0.000,0.000,0.000,{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},0,0,0,0,0.000,{}",
        scenario,
        iteration,
        block.block_number,
        tx.tx_index,
        tx.tx_hash,
        block.engine,
        tx.gas_used,
        tx.trace_nodes,
        total_ms,
        tx.sender_recovery_ms,
        tx.evm_env_ms,
        tx.tx_env_ms,
        tx.inspector_build_ms,
        tx.evm_exec_ms,
        tx.trace_build_ms,
        tx.db_commit_ms,
        tx.errors
    )?;
    Ok(())
}

fn call_tracer_options() -> GethDebugTracingOptions {
    GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::CallTracer,
        )),
        ..Default::default()
    }
}

fn normalize_traces(traces: &[alloy_rpc_types_trace::geth::TraceResult]) -> Result<Value> {
    Ok(serde_json::to_value(traces)?)
}

fn first_json_diff(path: &str, left: &Value, right: &Value) -> Option<String> {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            for key in left.keys().chain(right.keys()) {
                let child_path = format!("{path}.{key}");
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => {
                        if let Some(diff) = first_json_diff(&child_path, left, right) {
                            return Some(diff);
                        }
                    }
                    (Some(_), None) => return Some(format!("{child_path} missing on right")),
                    (None, Some(_)) => return Some(format!("{child_path} missing on left")),
                    (None, None) => {}
                }
            }
            None
        }
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                return Some(format!(
                    "{path} array length differs: {} != {}",
                    left.len(),
                    right.len()
                ));
            }
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                if let Some(diff) = first_json_diff(&format!("{path}[{index}]"), left, right) {
                    return Some(diff);
                }
            }
            None
        }
        _ if left == right => None,
        _ => Some(format!("{path} differs: {left:?} != {right:?}")),
    }
}

#[derive(Debug)]
struct Summary {
    count: usize,
    avg: f64,
    median: f64,
    p95: f64,
    max: f64,
}

impl Summary {
    fn from_values(values: &[f64]) -> Self {
        let mut sorted = values.to_vec();
        sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        let count = sorted.len();
        let avg = if count == 0 {
            0.0
        } else {
            sorted.iter().sum::<f64>() / count as f64
        };
        Self {
            count,
            avg,
            median: percentile(&sorted, 0.50),
            p95: percentile(&sorted, 0.95),
            max: sorted.last().copied().unwrap_or_default(),
        }
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}
