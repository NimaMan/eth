use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use alloy_rpc_types_trace::geth::{
    GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
};
use clap::{Parser, ValueEnum};
use eyre::{bail, Result};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use serde::Serialize;
use serde_json::{json, Value};
use tx_simulator::block_simulation::{
    BlockReplayProfile, BlockTraceEngine, BlockTracer, ReplayProfileConfig,
    TransactionReplayProfile,
};
use tx_simulator::TxSimulator;

const TARGET_MS: f64 = 25.0;

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

    /// Number of measured iterations per scenario.
    #[arg(long, default_value_t = 3)]
    iterations: usize,

    /// Extra warmup iterations emitted as rows but excluded from summaries.
    #[arg(long, default_value_t = 0)]
    warmup_iterations: usize,

    /// Discard the first post-warmup iteration from summaries.
    #[arg(long)]
    discard_first: bool,

    /// Stable label written to every CSV/JSON row.
    #[arg(long, default_value = "cold")]
    label: String,

    /// Candidate engine/scenario for stage-breakdown mode.
    #[arg(long, value_enum, default_value_t = EngineArg::BaselineFresh)]
    engine: EngineArg,

    /// Profiler mode.
    #[arg(long, value_enum, default_value_t = Mode::EngineSweep)]
    mode: Mode,

    /// Optional CSV output path. Defaults to stdout.
    #[arg(long)]
    output: Option<PathBuf>,

    /// Optional JSON summary output path.
    #[arg(long)]
    json_summary: Option<PathBuf>,

    /// Also emit one CSV row per transaction.
    #[arg(long)]
    per_tx: bool,

    /// Record provider-miss account/storage/code/block-hash keys.
    #[arg(long)]
    record_keys: bool,

    /// Optional local Reth RPC URL for correctness comparison.
    #[arg(long)]
    rpc_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    EngineSweep,
    StageBreakdown,
    Feasibility,
    Correctness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum EngineArg {
    BaselineFresh,
    TracingFused,
    RethDebug,
    ExecuteOnly,
    OraclePrewarm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scenario {
    Trace(BlockTraceEngine),
    ExecuteOnly,
    OraclePrewarm,
}

impl Scenario {
    fn engine(self) -> &'static str {
        match self {
            Self::Trace(engine) => engine.as_str(),
            Self::ExecuteOnly => "execute-only",
            Self::OraclePrewarm => BlockTraceEngine::RethFusedCallTracer.as_str(),
        }
    }

    fn profile_kind(self) -> &'static str {
        match self {
            Self::Trace(_) => "full-trace",
            Self::ExecuteOnly => "execute-only",
            Self::OraclePrewarm => "oracle-prewarm",
        }
    }

    fn is_full_trace_candidate(self) -> bool {
        matches!(self, Self::Trace(_) | Self::OraclePrewarm)
    }
}

impl EngineArg {
    const fn as_scenario(self) -> Scenario {
        match self {
            Self::BaselineFresh => Scenario::Trace(BlockTraceEngine::FreshInspector),
            Self::TracingFused => Scenario::Trace(BlockTraceEngine::RethFusedCallTracer),
            Self::RethDebug => Scenario::Trace(BlockTraceEngine::RethDebug),
            Self::ExecuteOnly => Scenario::ExecuteOnly,
            Self::OraclePrewarm => Scenario::OraclePrewarm,
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
    let run_id = build_run_id();
    let sample = sample_label(&blocks);

    match args.mode {
        Mode::Correctness => run_correctness(&tracer, &blocks, args.rpc_url.as_deref()).await,
        Mode::EngineSweep | Mode::StageBreakdown | Mode::Feasibility => {
            run_profile(&args, &tracer, &blocks, &run_id, &sample).await
        }
    }
}

async fn run_profile(
    args: &Args,
    tracer: &BlockTracer<'_>,
    blocks: &[u64],
    run_id: &str,
    sample: &str,
) -> Result<()> {
    let mut writer = output_writer(args.output.as_ref())?;
    print_header(&mut writer)?;

    let scenarios = match args.mode {
        Mode::EngineSweep => vec![
            Scenario::Trace(BlockTraceEngine::FreshInspector),
            Scenario::Trace(BlockTraceEngine::RethFusedCallTracer),
            Scenario::Trace(BlockTraceEngine::RethDebug),
        ],
        Mode::StageBreakdown => vec![args.engine.as_scenario()],
        Mode::Feasibility => vec![
            Scenario::Trace(BlockTraceEngine::FreshInspector),
            Scenario::Trace(BlockTraceEngine::RethFusedCallTracer),
            Scenario::Trace(BlockTraceEngine::RethDebug),
            Scenario::ExecuteOnly,
            Scenario::OraclePrewarm,
        ],
        Mode::Correctness => unreachable!("correctness is handled separately"),
    };

    let total_iterations = args
        .warmup_iterations
        .saturating_add(args.iterations.max(1))
        .saturating_add(usize::from(args.discard_first));
    let discard_iteration = args.discard_first.then_some(args.warmup_iterations);

    let mut summaries: BTreeMap<SummaryKey, Vec<f64>> = BTreeMap::new();
    let mut preload_summaries: BTreeMap<SummaryKey, Vec<f64>> = BTreeMap::new();
    let mut exec_after_summaries: BTreeMap<SummaryKey, Vec<f64>> = BTreeMap::new();
    for iteration in 0..total_iterations {
        let is_warmup = iteration < args.warmup_iterations || discard_iteration == Some(iteration);

        for scenario in &scenarios {
            for block_number in blocks.iter().copied() {
                let state_mode = state_mode_for(*scenario, is_warmup, args);
                let profiled =
                    run_scenario(*scenario, tracer, block_number, args.record_keys).await?;
                let profile = &profiled.profile;
                print_block_row(
                    &mut writer,
                    run_id,
                    &args.label,
                    sample,
                    state_mode,
                    *scenario,
                    iteration,
                    is_warmup,
                    profile,
                )?;

                if args.per_tx {
                    for tx in &profile.tx_profiles {
                        print_tx_row(
                            &mut writer,
                            run_id,
                            &args.label,
                            sample,
                            state_mode,
                            *scenario,
                            iteration,
                            is_warmup,
                            profile,
                            tx,
                        )?;
                    }
                }

                if !is_warmup {
                    summaries
                        .entry(SummaryKey::new(
                            *scenario,
                            state_mode_for(*scenario, false, args),
                        ))
                        .or_default()
                        .push(profile.total_ms);
                    if matches!(scenario, Scenario::OraclePrewarm) {
                        preload_summaries
                            .entry(SummaryKey::custom(
                                "prewarmed-state",
                                "oracle-prewarm-preload",
                                scenario.engine(),
                                false,
                            ))
                            .or_default()
                            .push(profile.preload_ms);
                        exec_after_summaries
                            .entry(SummaryKey::custom(
                                "prewarmed-state",
                                "oracle-prewarm-exec-after",
                                scenario.engine(),
                                true,
                            ))
                            .or_default()
                            .push(profile.exec_after_prewarm_ms);
                    }
                }
            }
        }
    }

    let mut summary_rows = Vec::new();
    for (key, values) in summaries {
        let summary = Summary::from_values(&values);
        let row = SummaryRow::new(run_id, &args.label, sample, key, summary);
        print_summary_row(&mut writer, &row)?;
        summary_rows.push(row);
    }
    for (key, values) in preload_summaries.into_iter().chain(exec_after_summaries) {
        let summary = Summary::from_values(&values);
        let row = SummaryRow::new(run_id, &args.label, sample, key, summary);
        print_summary_row(&mut writer, &row)?;
        summary_rows.push(row);
    }

    if let Some(path) = args.json_summary.as_ref() {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &summary_rows)?;
    }

    Ok(())
}

async fn run_scenario(
    scenario: Scenario,
    tracer: &BlockTracer<'_>,
    block_number: u64,
    record_keys: bool,
) -> Result<tx_simulator::block_simulation::ProfiledBlockTrace> {
    match scenario {
        Scenario::Trace(engine) => {
            tracer
                .trace_block_by_number_profiled_with_config(
                    block_number,
                    Some(call_tracer_options()),
                    engine,
                    ReplayProfileConfig {
                        record_keys,
                        prewarm_keys: None,
                    },
                )
                .await
        }
        Scenario::ExecuteOnly => {
            tracer
                .execute_block_by_number_profiled(
                    block_number,
                    ReplayProfileConfig {
                        record_keys,
                        prewarm_keys: None,
                    },
                )
                .await
        }
        Scenario::OraclePrewarm => {
            let key_source = tracer
                .trace_block_by_number_profiled_with_config(
                    block_number,
                    Some(call_tracer_options()),
                    BlockTraceEngine::FreshInspector,
                    ReplayProfileConfig {
                        record_keys: true,
                        prewarm_keys: None,
                    },
                )
                .await?;
            tracer
                .trace_block_by_number_profiled_with_config(
                    block_number,
                    Some(call_tracer_options()),
                    BlockTraceEngine::RethFusedCallTracer,
                    ReplayProfileConfig {
                        record_keys,
                        prewarm_keys: Some(key_source.profile.state_keys),
                    },
                )
                .await
        }
    }
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

        let key_source = tracer
            .trace_block_by_number_profiled_with_config(
                block_number,
                Some(call_tracer_options()),
                BlockTraceEngine::FreshInspector,
                ReplayProfileConfig {
                    record_keys: true,
                    prewarm_keys: None,
                },
            )
            .await?;
        let oracle = tracer
            .trace_block_by_number_profiled_with_config(
                block_number,
                Some(call_tracer_options()),
                BlockTraceEngine::RethFusedCallTracer,
                ReplayProfileConfig {
                    record_keys: false,
                    prewarm_keys: Some(key_source.profile.state_keys),
                },
            )
            .await?;
        let oracle_value = normalize_traces(&oracle.traces)?;
        if baseline_value != oracle_value {
            let diff = first_json_diff("$", &baseline_value, &oracle_value)
                .unwrap_or_else(|| "values differ but no focused diff was found".to_string());
            bail!("trace mismatch block={block_number} engine=oracle-prewarm: {diff}");
        }
        println!(
            "# correctness block={} engine=oracle-prewarm ok txs={} trace_nodes={}",
            block_number, oracle.profile.tx_count, oracle.profile.trace_node_count
        );

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
        "row_type,run_id,label,sample,state_mode,profile_kind,engine,iteration,is_warmup,block,tx_index,tx_hash,count,txs,gas_used,trace_nodes,total_ms,block_hash_lookup_ms,block_load_ms,state_open_ms,sender_recovery_ms,evm_env_ms,tx_env_ms,inspector_build_ms,evm_exec_ms,trace_build_ms,db_commit_ms,preload_ms,exec_after_prewarm_ms,account_reads,storage_reads,code_reads,block_hash_reads,provider_read_ms,errors,avg_ms,median_ms,p90_ms,p95_ms,p99_ms,min_ms,max_ms,stddev_ms,meets_25ms"
    )?;
    Ok(())
}

fn print_block_row(
    writer: &mut dyn Write,
    run_id: &str,
    label: &str,
    sample: &str,
    state_mode: &'static str,
    scenario: Scenario,
    iteration: usize,
    is_warmup: bool,
    profile: &BlockReplayProfile,
) -> Result<()> {
    writeln!(
        writer,
        "block,{},{},{},{},{},{},{},{},{},,,,{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{:.3},{},,,,,,,,,{}",
        run_id,
        label,
        sample,
        state_mode,
        scenario.profile_kind(),
        profile.engine,
        iteration,
        is_warmup,
        profile.block_number,
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
        profile.preload_ms,
        profile.exec_after_prewarm_ms,
        profile.state_reads.account_reads,
        profile.state_reads.storage_reads,
        profile.state_reads.code_reads,
        profile.state_reads.block_hash_reads,
        profile.state_reads.provider_read_ms,
        profile.errors,
        scenario.is_full_trace_candidate() && profile.total_ms <= TARGET_MS,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn print_tx_row(
    writer: &mut dyn Write,
    run_id: &str,
    label: &str,
    sample: &str,
    state_mode: &'static str,
    scenario: Scenario,
    iteration: usize,
    is_warmup: bool,
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
        "tx,{},{},{},{},{},{},{},{},{},{},{:#x},,,{},,{:.3},0.000,0.000,0.000,{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},0.000,0.000,{},{},{},{},{:.3},{},,,,,,,,,",
        run_id,
        label,
        sample,
        state_mode,
        scenario.profile_kind(),
        block.engine,
        iteration,
        is_warmup,
        block.block_number,
        tx.tx_index,
        tx.tx_hash,
        tx.gas_used,
        total_ms,
        tx.sender_recovery_ms,
        tx.evm_env_ms,
        tx.tx_env_ms,
        tx.inspector_build_ms,
        tx.evm_exec_ms,
        tx.trace_build_ms,
        tx.db_commit_ms,
        tx.state_reads.account_reads,
        tx.state_reads.storage_reads,
        tx.state_reads.code_reads,
        tx.state_reads.block_hash_reads,
        tx.state_reads.provider_read_ms,
        tx.errors
    )?;
    Ok(())
}

fn print_summary_row(writer: &mut dyn Write, row: &SummaryRow) -> Result<()> {
    writeln!(
        writer,
        "summary,{},{},{},{},{},{},,,,,,{},{},,,,,,,,,,,,,,,,,,,,,,{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{}",
        row.run_id,
        row.label,
        row.sample,
        row.state_mode,
        row.profile_kind,
        row.engine,
        row.count,
        row.count,
        row.avg_ms,
        row.median_ms,
        row.p90_ms,
        row.p95_ms,
        row.p99_ms,
        row.min_ms,
        row.max_ms,
        row.stddev_ms,
        row.meets_25ms
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
            let mut keys = left.keys().chain(right.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SummaryKey {
    state_mode: &'static str,
    profile_kind: &'static str,
    engine: &'static str,
    full_trace_candidate: bool,
}

impl SummaryKey {
    fn new(scenario: Scenario, state_mode: &'static str) -> Self {
        Self {
            state_mode,
            profile_kind: scenario.profile_kind(),
            engine: scenario.engine(),
            full_trace_candidate: scenario.is_full_trace_candidate(),
        }
    }

    const fn custom(
        state_mode: &'static str,
        profile_kind: &'static str,
        engine: &'static str,
        full_trace_candidate: bool,
    ) -> Self {
        Self {
            state_mode,
            profile_kind,
            engine,
            full_trace_candidate,
        }
    }
}

#[derive(Debug)]
struct Summary {
    count: usize,
    avg: f64,
    median: f64,
    p90: f64,
    p95: f64,
    p99: f64,
    min: f64,
    max: f64,
    stddev: f64,
}

impl Summary {
    fn from_values(values: &[f64]) -> Self {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let count = sorted.len();
        let avg = if count == 0 {
            0.0
        } else {
            sorted.iter().sum::<f64>() / count as f64
        };
        let variance = if count == 0 {
            0.0
        } else {
            sorted
                .iter()
                .map(|value| {
                    let diff = value - avg;
                    diff * diff
                })
                .sum::<f64>()
                / count as f64
        };
        Self {
            count,
            avg,
            median: median(&sorted),
            p90: percentile(&sorted, 0.90),
            p95: percentile(&sorted, 0.95),
            p99: percentile(&sorted, 0.99),
            min: sorted.first().copied().unwrap_or_default(),
            max: sorted.last().copied().unwrap_or_default(),
            stddev: variance.sqrt(),
        }
    }
}

#[derive(Debug, Serialize)]
struct SummaryRow {
    run_id: String,
    label: String,
    sample: String,
    state_mode: &'static str,
    profile_kind: &'static str,
    engine: &'static str,
    count: usize,
    avg_ms: f64,
    median_ms: f64,
    p90_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    min_ms: f64,
    max_ms: f64,
    stddev_ms: f64,
    meets_25ms: bool,
}

impl SummaryRow {
    fn new(run_id: &str, label: &str, sample: &str, key: SummaryKey, summary: Summary) -> Self {
        Self {
            run_id: run_id.to_string(),
            label: label.to_string(),
            sample: sample.to_string(),
            state_mode: key.state_mode,
            profile_kind: key.profile_kind,
            engine: key.engine,
            count: summary.count,
            avg_ms: summary.avg,
            median_ms: summary.median,
            p90_ms: summary.p90,
            p95_ms: summary.p95,
            p99_ms: summary.p99,
            min_ms: summary.min,
            max_ms: summary.max,
            stddev_ms: summary.stddev,
            meets_25ms: key.full_trace_candidate && summary.median <= TARGET_MS,
        }
    }
}

fn median(sorted: &[f64]) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = percentile.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = rank - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

fn sample_label(blocks: &[u64]) -> String {
    match (blocks.first(), blocks.last(), blocks.len()) {
        (Some(first), Some(last), len) if len > 1 => format!("{first}..{last}"),
        (Some(block), _, _) => block.to_string(),
        _ => "empty".to_string(),
    }
}

fn build_run_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{}-{}", millis, std::process::id())
}

fn state_mode_for(scenario: Scenario, is_warmup: bool, args: &Args) -> &'static str {
    match scenario {
        Scenario::OraclePrewarm => "prewarmed-state",
        _ if is_warmup => "warmup",
        _ if args.warmup_iterations > 0 || args.discard_first => "warm-os-cache",
        _ => "cold",
    }
}
