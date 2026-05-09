use std::time::Instant;

use alloy_rpc_types_trace::geth::{
    GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
};
use clap::{Parser, ValueEnum};
use eyre::{bail, Result};
use futures::{stream, StreamExt, TryStreamExt};
use tx_simulator::{
    block_simulation::{BlockReplayProfile, BlockTraceEngine, ReplayProfileConfig},
    TxSimulator,
};

#[derive(Debug, Parser)]
#[command(about = "Range-level state-access replay profiler for tx_simulator")]
struct Args {
    /// Reth datadir. Defaults to tx_simulator config/env resolution.
    #[arg(long)]
    datadir: Option<String>,

    /// First block in the range.
    #[arg(long)]
    start: Option<u64>,

    /// Last block in the range. Defaults to --start.
    #[arg(long)]
    end: Option<u64>,

    /// Profile the latest N persisted DB blocks when no range is provided.
    #[arg(long, default_value_t = 20)]
    latest_count: u64,

    /// Number of measured iterations.
    #[arg(long, default_value_t = 1)]
    iterations: usize,

    /// Parallel blocks to replay.
    #[arg(long, default_value_t = 1)]
    concurrency: usize,

    /// Replay engine/profile kind.
    #[arg(long, value_enum, default_value_t = ProfileKind::TracingFused)]
    profile_kind: ProfileKind,

    /// Record provider-miss account/storage/code/block-hash keys.
    #[arg(long)]
    record_keys: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ProfileKind {
    BaselineFresh,
    TracingFused,
    RethDebug,
    ExecuteOnly,
}

impl ProfileKind {
    fn label(self) -> &'static str {
        match self {
            Self::BaselineFresh => "baseline-fresh",
            Self::TracingFused => "tracing-fused",
            Self::RethDebug => "reth-debug",
            Self::ExecuteOnly => "execute-only",
        }
    }

    fn trace_engine(self) -> Option<BlockTraceEngine> {
        match self {
            Self::BaselineFresh => Some(BlockTraceEngine::FreshInspector),
            Self::TracingFused => Some(BlockTraceEngine::RethFusedCallTracer),
            Self::RethDebug => Some(BlockTraceEngine::RethDebug),
            Self::ExecuteOnly => None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if args.concurrency == 0 {
        bail!("--concurrency must be greater than zero");
    }

    let datadir = args
        .datadir
        .clone()
        .map(Ok)
        .unwrap_or_else(tx_simulator::config::repo::reth_datadir)?;
    let simulator = TxSimulator::new(&datadir)?;
    let latest = simulator.get_latest_block()?;
    let blocks = selected_blocks(&args, latest)?;

    print_header();
    for iteration in 0..args.iterations.max(1) {
        let started = Instant::now();
        let profiles = run_profiles(
            simulator.clone(),
            blocks.clone(),
            args.profile_kind,
            args.concurrency,
            args.record_keys,
        )
        .await?;
        let total_ms = started.elapsed().as_secs_f64() * 1000.0;
        let row = RangeSummary::from_replay_profiles(
            "replay_state_access",
            args.profile_kind.label(),
            iteration,
            blocks[0],
            *blocks.last().unwrap_or(&blocks[0]),
            args.concurrency,
            total_ms,
            &profiles,
        );
        row.print();
    }

    Ok(())
}

async fn run_profiles(
    simulator: TxSimulator,
    blocks: Vec<u64>,
    profile_kind: ProfileKind,
    concurrency: usize,
    record_keys: bool,
) -> Result<Vec<BlockReplayProfile>> {
    stream::iter(blocks)
        .map(|block_number| {
            let simulator = simulator.clone();
            async move { profile_block(simulator, block_number, profile_kind, record_keys).await }
        })
        .buffer_unordered(concurrency)
        .try_collect()
        .await
}

async fn profile_block(
    simulator: TxSimulator,
    block_number: u64,
    profile_kind: ProfileKind,
    record_keys: bool,
) -> Result<BlockReplayProfile> {
    let config = ReplayProfileConfig {
        record_keys,
        prewarm_keys: None,
    };

    let profiled = match profile_kind.trace_engine() {
        Some(engine) => {
            simulator
                .block_replay_session(block_number)
                .with_tracing_options(call_tracer_options())
                .with_engine(engine)
                .with_profile_config(config)
                .profile()
                .await?
        }
        None => {
            simulator
                .block_replay_session(block_number)
                .with_profile_config(config)
                .execute_only_profile()
                .await?
        }
    };
    Ok(profiled.profile)
}

fn call_tracer_options() -> GethDebugTracingOptions {
    GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::CallTracer,
        )),
        ..Default::default()
    }
}

fn selected_blocks(args: &Args, latest: u64) -> Result<Vec<u64>> {
    let (start, end) = match (args.start, args.end) {
        (Some(start), Some(end)) => (start, end),
        (Some(start), None) => (start, start),
        (None, Some(end)) => (end, end),
        (None, None) => (
            latest.saturating_sub(args.latest_count.saturating_sub(1)),
            latest,
        ),
    };

    if start > end {
        bail!("invalid range: start block {start} is after end block {end}");
    }
    Ok((start..=end).collect())
}

fn print_header() {
    println!(
        "scenario,profile_kind,iteration,start_block,end_block,blocks,concurrency,chunk_size,fill_batch_blocks,fill_concurrency,total_ms,avg_ms,p50_ms,p95_ms,max_ms,cache_hit_rate,cache_read_ms,cache_write_ms,evm_exec_ms,provider_read_ms,account_reads,storage_reads,code_reads,preload_ms,trace_build_ms,commit_ms,processing_errors,internal_txs"
    );
}

#[derive(Debug)]
struct RangeSummary {
    scenario: &'static str,
    profile_kind: &'static str,
    iteration: usize,
    start_block: u64,
    end_block: u64,
    blocks: usize,
    concurrency: usize,
    total_ms: f64,
    avg_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    evm_exec_ms: f64,
    provider_read_ms: f64,
    account_reads: u64,
    storage_reads: u64,
    code_reads: u64,
    preload_ms: f64,
    trace_build_ms: f64,
    commit_ms: f64,
    processing_errors: usize,
}

impl RangeSummary {
    fn from_replay_profiles(
        scenario: &'static str,
        profile_kind: &'static str,
        iteration: usize,
        start_block: u64,
        end_block: u64,
        concurrency: usize,
        total_ms: f64,
        profiles: &[BlockReplayProfile],
    ) -> Self {
        let mut block_ms = profiles
            .iter()
            .map(|profile| profile.total_ms)
            .collect::<Vec<_>>();
        block_ms.sort_by(f64::total_cmp);
        let blocks = profiles.len();
        Self {
            scenario,
            profile_kind,
            iteration,
            start_block,
            end_block,
            blocks,
            concurrency,
            total_ms,
            avg_ms: total_ms / blocks.max(1) as f64,
            p50_ms: percentile(&block_ms, 0.50),
            p95_ms: percentile(&block_ms, 0.95),
            max_ms: block_ms.last().copied().unwrap_or_default(),
            evm_exec_ms: profiles.iter().map(|profile| profile.evm_exec_ms).sum(),
            provider_read_ms: profiles
                .iter()
                .map(|profile| profile.state_reads.provider_read_ms)
                .sum(),
            account_reads: profiles
                .iter()
                .map(|profile| profile.state_reads.account_reads)
                .sum(),
            storage_reads: profiles
                .iter()
                .map(|profile| profile.state_reads.storage_reads)
                .sum(),
            code_reads: profiles
                .iter()
                .map(|profile| profile.state_reads.code_reads)
                .sum(),
            preload_ms: profiles.iter().map(|profile| profile.preload_ms).sum(),
            trace_build_ms: profiles.iter().map(|profile| profile.trace_build_ms).sum(),
            commit_ms: profiles.iter().map(|profile| profile.db_commit_ms).sum(),
            processing_errors: profiles.iter().map(|profile| profile.errors).sum(),
        }
    }

    fn print(&self) {
        println!(
            "{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.4},{:.3},{:.3},{:.3},{:.3},{},{},{},{:.3},{:.3},{:.3},{},{}",
            self.scenario,
            self.profile_kind,
            self.iteration,
            self.start_block,
            self.end_block,
            self.blocks,
            self.concurrency,
            0,
            0,
            0,
            self.total_ms,
            self.avg_ms,
            self.p50_ms,
            self.p95_ms,
            self.max_ms,
            0.0,
            0.0,
            0.0,
            self.evm_exec_ms,
            self.provider_read_ms,
            self.account_reads,
            self.storage_reads,
            self.code_reads,
            self.preload_ms,
            self.trace_build_ms,
            self.commit_ms,
            self.processing_errors,
            0,
        );
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}
