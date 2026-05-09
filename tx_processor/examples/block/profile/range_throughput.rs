use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use clap::{Parser, ValueEnum};
use eyre::{bail, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reth_chain_query::{provider::BlockDataFetcher, RethQueryProvider};
use tx_processor::{
    load_processed_block_range_with_options, BlockProcessor, ProcessedBlock,
    ProcessedBlockDiskCacheStore, ProcessedBlockRangeLoadOptions, ProcessedBlockReplayStoreWriter,
    DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS,
    DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY, DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH,
};
use tx_simulator::{
    block_simulation::{BlockReplayProfile, BlockTraceEngine, ReplayProfileConfig},
    TxSimulator,
};

#[derive(Debug, Parser)]
#[command(about = "100K-oriented processed-block throughput investigation harness")]
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

    /// Harness mode to run.
    #[arg(long, value_enum, default_value_t = Mode::ColdSweep)]
    mode: Mode,

    /// Comma-separated cold-processing concurrency sweep.
    #[arg(long, default_value = "1,4,8,16,24,32,48")]
    concurrencies: String,

    /// Comma-separated representative matrix range sizes.
    #[arg(long, default_value = "20,2000,20000,100000")]
    range_sizes: String,

    /// Blocks per outer range chunk.
    #[arg(long, default_value_t = DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH)]
    chunk_size: u64,

    /// Processed-block disk cache directory.
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// Missing blocks per traced cache-fill batch.
    #[arg(long, default_value_t = DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS)]
    fill_batch_blocks: usize,

    /// Parallel traced block processing jobs per cache-fill batch.
    #[arg(long, default_value_t = DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY)]
    fill_concurrency: usize,

    /// Parallel blocks for replay-only state-access rows.
    #[arg(long, default_value_t = 24)]
    replay_concurrency: usize,

    /// Trace engine used for cold processed-block runs.
    #[arg(long, value_enum, default_value_t = TraceEngineArg::Default)]
    trace_engine: TraceEngineArg,

    /// Skip replay-only state-access rows in matrix mode.
    #[arg(long)]
    skip_replay_only: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    Matrix,
    ColdSweep,
    CacheRefresh,
    CacheReadOnly,
    ReplayOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum TraceEngineArg {
    Default,
    Fresh,
    Fused,
}

impl TraceEngineArg {
    fn as_engine(self) -> BlockTraceEngine {
        match self {
            Self::Default => BlockTraceEngine::default(),
            Self::Fresh => BlockTraceEngine::FreshInspector,
            Self::Fused => BlockTraceEngine::RethFusedCallTracer,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Fresh => "fresh",
            Self::Fused => "fused",
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    validate_args(&args)?;

    let datadir = args
        .datadir
        .clone()
        .map(Ok)
        .unwrap_or_else(tx_simulator::config::repo::reth_datadir)?;
    let provider = Arc::new(RethQueryProvider::new(&datadir)?);
    let latest = provider.get_latest_block()?;
    let fetcher = Arc::new(BlockDataFetcher::new(provider.clone()));
    let processor = BlockProcessor::with_block_fetcher(fetcher);
    let cache_store = args
        .cache_dir
        .as_ref()
        .map(ProcessedBlockDiskCacheStore::open)
        .transpose()?;
    let cache_replay_store = cache_store.as_ref().map(|store| {
        ProcessedBlockReplayStoreWriter::new(store.clone(), provider.chain_id(), None)
    });

    print_header();
    match args.mode {
        Mode::Matrix => {
            let ranges = matrix_ranges(&args, latest)?;
            for (start, end) in ranges {
                run_cold_sweep(&args, &processor, start, end).await?;
                if let Some(replay_store) = cache_replay_store.as_ref() {
                    run_cache_refresh(
                        &args,
                        &processor,
                        provider.as_ref(),
                        replay_store,
                        start,
                        end,
                    )
                    .await?;
                }
                if let Some(store) = cache_store.as_ref() {
                    run_cache_read_only(&args, provider.as_ref(), store, start, end).await?;
                }
                if !args.skip_replay_only {
                    run_replay_only(&args, &datadir, start, end).await?;
                }
            }
        }
        Mode::ColdSweep => {
            let (start, end) = selected_range(&args, latest)?;
            run_cold_sweep(&args, &processor, start, end).await?;
        }
        Mode::CacheRefresh => {
            let store = required_cache_replay_store(cache_replay_store.as_ref(), args.mode)?;
            let (start, end) = selected_range(&args, latest)?;
            run_cache_refresh(&args, &processor, provider.as_ref(), store, start, end).await?;
        }
        Mode::CacheReadOnly => {
            let store = required_cache_store(cache_store.as_ref(), args.mode)?;
            let (start, end) = selected_range(&args, latest)?;
            run_cache_read_only(&args, provider.as_ref(), store, start, end).await?;
        }
        Mode::ReplayOnly => {
            let (start, end) = selected_range(&args, latest)?;
            run_replay_only(&args, &datadir, start, end).await?;
        }
    }

    Ok(())
}

fn validate_args(args: &Args) -> Result<()> {
    if args.chunk_size == 0 {
        bail!("--chunk-size must be greater than zero");
    }
    if args.fill_batch_blocks == 0 {
        bail!("--fill-batch-blocks must be greater than zero");
    }
    if args.fill_concurrency == 0 {
        bail!("--fill-concurrency must be greater than zero");
    }
    if args.replay_concurrency == 0 {
        bail!("--replay-concurrency must be greater than zero");
    }
    if parse_usize_list(&args.concurrencies)?.is_empty() {
        bail!("--concurrencies must contain at least one value");
    }
    if parse_u64_list(&args.range_sizes)?.is_empty() {
        bail!("--range-sizes must contain at least one value");
    }
    Ok(())
}

async fn run_cold_sweep(
    args: &Args,
    processor: &BlockProcessor,
    start: u64,
    end: u64,
) -> Result<()> {
    let concurrencies = parse_usize_list(&args.concurrencies)?;
    let blocks = block_numbers(start, end);
    for iteration in 0..args.iterations.max(1) {
        for concurrency in &concurrencies {
            let started = Instant::now();
            let mut timed_blocks = Vec::with_capacity(blocks.len());
            for chunk in blocks.chunks(args.chunk_size as usize) {
                timed_blocks.extend(
                    process_blocks_timed(
                        processor,
                        chunk.to_vec(),
                        *concurrency,
                        args.trace_engine.as_engine(),
                    )
                    .await?,
                );
            }
            let total_ms = ms(started);
            SummaryRow::from_timed_blocks(
                "cold_no_cache",
                args.trace_engine.label(),
                iteration,
                start,
                end,
                *concurrency,
                args.chunk_size,
                args.fill_batch_blocks,
                args.fill_concurrency,
                total_ms,
                &timed_blocks,
            )
            .print();
        }
    }
    Ok(())
}

async fn process_blocks_timed(
    processor: &BlockProcessor,
    blocks: Vec<u64>,
    concurrency: usize,
    trace_engine: BlockTraceEngine,
) -> Result<Vec<TimedProcessedBlock>> {
    stream::iter(blocks)
        .map(|block_number| {
            let processor = processor.clone();
            async move {
                let started = Instant::now();
                let block = processor
                    .process_block_with_trace_engine(block_number, true, trace_engine)
                    .await?;
                Ok(TimedProcessedBlock {
                    elapsed_ms: ms(started),
                    block,
                })
            }
        })
        .buffer_unordered(concurrency.max(1))
        .try_collect()
        .await
}

async fn run_cache_refresh(
    args: &Args,
    processor: &BlockProcessor,
    provider: &RethQueryProvider,
    store: &ProcessedBlockReplayStoreWriter,
    start: u64,
    end: u64,
) -> Result<()> {
    let options = ProcessedBlockRangeLoadOptions::default()
        .with_fill_batch_blocks(args.fill_batch_blocks)
        .with_fill_concurrency(args.fill_concurrency);
    let blocks = block_numbers(start, end);
    for iteration in 0..args.iterations.max(1) {
        let started = Instant::now();
        let mut loaded = Vec::with_capacity(blocks.len());
        for chunk in blocks.chunks(args.chunk_size as usize) {
            let chunk_start = chunk[0];
            let chunk_end = *chunk.last().unwrap_or(&chunk_start);
            loaded.extend(
                load_processed_block_range_with_options(
                    processor,
                    provider,
                    chunk_start,
                    chunk_end,
                    Some(store),
                    options,
                )
                .await?,
            );
        }
        let total_ms = ms(started);
        SummaryRow::from_loaded_blocks(
            "cache_refresh",
            "processed-block",
            iteration,
            start,
            end,
            args.fill_concurrency,
            args.chunk_size,
            args.fill_batch_blocks,
            args.fill_concurrency,
            total_ms,
            &loaded,
        )
        .print();
    }
    Ok(())
}

async fn run_cache_read_only(
    args: &Args,
    provider: &RethQueryProvider,
    store: &ProcessedBlockDiskCacheStore,
    start: u64,
    end: u64,
) -> Result<()> {
    let reader = store.reader();
    for iteration in 0..args.iterations.max(1) {
        let plan_started = Instant::now();
        let plan = reader.plan_range(provider, start, end).await?;
        let plan_ms = ms(plan_started);
        if !plan.is_complete() {
            bail!(
                "read-only cache is missing {} blocks in {}..={}",
                plan.missing_keys.len(),
                start,
                end
            );
        }

        let read_started = Instant::now();
        let keys = plan.keys.clone();
        let reader_for_task = reader.clone();
        let reads = tokio::task::spawn_blocking(move || reader_for_task.get_many_parallel(&keys))
            .await
            .map_err(|error| {
                eyre::eyre!("processed block disk cache reader task failed: {error}")
            })??;
        let read_wall_ms = ms(read_started);
        let total_ms = plan_ms + read_wall_ms;
        SummaryRow::from_cache_reads(
            "cache_read_only",
            "processed-block",
            iteration,
            start,
            end,
            args.chunk_size,
            args.fill_batch_blocks,
            args.fill_concurrency,
            total_ms,
            plan_ms,
            read_wall_ms,
            plan.missing_keys.len(),
            &reads,
        )?
        .print();
    }
    Ok(())
}

async fn run_replay_only(args: &Args, datadir: &str, start: u64, end: u64) -> Result<()> {
    let simulator = TxSimulator::new(datadir)?;
    let blocks = block_numbers(start, end);
    for iteration in 0..args.iterations.max(1) {
        let started = Instant::now();
        let mut profiles = Vec::with_capacity(blocks.len());
        for chunk in blocks.chunks(args.chunk_size as usize) {
            profiles.extend(
                replay_blocks_profiled(
                    simulator.clone(),
                    chunk.to_vec(),
                    args.replay_concurrency,
                    args.trace_engine.as_engine(),
                )
                .await?,
            );
        }
        let total_ms = ms(started);
        SummaryRow::from_replay_profiles(
            "replay_state_access",
            args.trace_engine.label(),
            iteration,
            start,
            end,
            args.replay_concurrency,
            args.chunk_size,
            args.fill_batch_blocks,
            args.fill_concurrency,
            total_ms,
            &profiles,
        )
        .print();
    }
    Ok(())
}

async fn replay_blocks_profiled(
    simulator: TxSimulator,
    blocks: Vec<u64>,
    concurrency: usize,
    trace_engine: BlockTraceEngine,
) -> Result<Vec<BlockReplayProfile>> {
    stream::iter(blocks)
        .map(|block_number| {
            let simulator = simulator.clone();
            async move {
                let profile = simulator
                    .block_replay_session(block_number)
                    .with_engine(trace_engine)
                    .with_profile_config(ReplayProfileConfig {
                        record_keys: false,
                        prewarm_keys: None,
                    })
                    .profile()
                    .await?
                    .profile;
                Ok(profile)
            }
        })
        .buffer_unordered(concurrency.max(1))
        .try_collect()
        .await
}

fn required_cache_store<'a>(
    store: Option<&'a ProcessedBlockDiskCacheStore>,
    mode: Mode,
) -> Result<&'a ProcessedBlockDiskCacheStore> {
    store.ok_or_else(|| eyre::eyre!("--cache-dir is required for {mode:?}"))
}

fn required_cache_replay_store<'a>(
    store: Option<&'a ProcessedBlockReplayStoreWriter>,
    mode: Mode,
) -> Result<&'a ProcessedBlockReplayStoreWriter> {
    store.ok_or_else(|| eyre::eyre!("--cache-dir is required for {mode:?}"))
}

fn selected_range(args: &Args, latest: u64) -> Result<(u64, u64)> {
    let (start, end) = match (args.start, args.end) {
        (Some(start), Some(end)) => (start, end),
        (Some(start), None) => (start, start),
        (None, Some(end)) => (end.saturating_sub(args.latest_count.saturating_sub(1)), end),
        (None, None) => (
            latest.saturating_sub(args.latest_count.saturating_sub(1)),
            latest,
        ),
    };
    validate_range(start, end)?;
    Ok((start, end))
}

fn matrix_ranges(args: &Args, latest: u64) -> Result<Vec<(u64, u64)>> {
    if args.start.is_some() || args.end.is_some() {
        return selected_range(args, latest).map(|range| vec![range]);
    }

    parse_u64_list(&args.range_sizes)?
        .into_iter()
        .map(|size| {
            let end = latest;
            let start = end.saturating_sub(size.saturating_sub(1));
            validate_range(start, end)?;
            Ok((start, end))
        })
        .collect()
}

fn validate_range(start: u64, end: u64) -> Result<()> {
    if start > end {
        bail!("invalid range: start block {start} is after end block {end}");
    }
    Ok(())
}

fn block_numbers(start: u64, end: u64) -> Vec<u64> {
    (start..=end).collect()
}

fn parse_usize_list(value: &str) -> Result<Vec<usize>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<usize>()
                .map_err(|error| eyre::eyre!("invalid usize value '{part}': {error}"))
        })
        .collect()
}

fn parse_u64_list(value: &str) -> Result<Vec<u64>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<u64>()
                .map_err(|error| eyre::eyre!("invalid u64 value '{part}': {error}"))
        })
        .collect()
}

fn print_header() {
    println!(
        "scenario,profile_kind,iteration,start_block,end_block,blocks,concurrency,chunk_size,fill_batch_blocks,fill_concurrency,total_ms,plan_ms,read_wall_ms,missing_count,invalid_count,avg_ms,p50_ms,p95_ms,max_ms,cache_hit_rate,cache_read_ms,cache_write_ms,evm_exec_ms,provider_read_ms,account_reads,storage_reads,code_reads,preload_ms,trace_build_ms,commit_ms,processing_errors,internal_txs"
    );
}

fn ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

struct TimedProcessedBlock {
    elapsed_ms: f64,
    block: ProcessedBlock,
}

struct SummaryRow {
    scenario: &'static str,
    profile_kind: &'static str,
    iteration: usize,
    start_block: u64,
    end_block: u64,
    blocks: usize,
    concurrency: usize,
    chunk_size: u64,
    fill_batch_blocks: usize,
    fill_concurrency: usize,
    total_ms: f64,
    plan_ms: f64,
    read_wall_ms: f64,
    missing_count: usize,
    invalid_count: usize,
    avg_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    cache_hit_rate: f64,
    cache_read_ms: f64,
    cache_write_ms: f64,
    evm_exec_ms: f64,
    provider_read_ms: f64,
    account_reads: u64,
    storage_reads: u64,
    code_reads: u64,
    preload_ms: f64,
    trace_build_ms: f64,
    commit_ms: f64,
    processing_errors: usize,
    internal_txs: usize,
}

impl SummaryRow {
    fn from_timed_blocks(
        scenario: &'static str,
        profile_kind: &'static str,
        iteration: usize,
        start_block: u64,
        end_block: u64,
        concurrency: usize,
        chunk_size: u64,
        fill_batch_blocks: usize,
        fill_concurrency: usize,
        total_ms: f64,
        timed_blocks: &[TimedProcessedBlock],
    ) -> Self {
        let mut block_ms = timed_blocks
            .iter()
            .map(|timed| timed.elapsed_ms)
            .collect::<Vec<_>>();
        block_ms.sort_by(f64::total_cmp);
        let blocks = timed_blocks.len();
        Self {
            scenario,
            profile_kind,
            iteration,
            start_block,
            end_block,
            blocks,
            concurrency,
            chunk_size,
            fill_batch_blocks,
            fill_concurrency,
            total_ms,
            plan_ms: 0.0,
            read_wall_ms: 0.0,
            missing_count: 0,
            invalid_count: 0,
            avg_ms: total_ms / blocks.max(1) as f64,
            p50_ms: percentile(&block_ms, 0.50),
            p95_ms: percentile(&block_ms, 0.95),
            max_ms: block_ms.last().copied().unwrap_or_default(),
            cache_hit_rate: 0.0,
            cache_read_ms: 0.0,
            cache_write_ms: 0.0,
            evm_exec_ms: 0.0,
            provider_read_ms: 0.0,
            account_reads: 0,
            storage_reads: 0,
            code_reads: 0,
            preload_ms: 0.0,
            trace_build_ms: 0.0,
            commit_ms: 0.0,
            processing_errors: timed_blocks
                .iter()
                .map(|timed| processing_error_count(&timed.block))
                .sum(),
            internal_txs: timed_blocks
                .iter()
                .map(|timed| internal_tx_count(&timed.block))
                .sum(),
        }
    }

    fn from_loaded_blocks(
        scenario: &'static str,
        profile_kind: &'static str,
        iteration: usize,
        start_block: u64,
        end_block: u64,
        concurrency: usize,
        chunk_size: u64,
        fill_batch_blocks: usize,
        fill_concurrency: usize,
        total_ms: f64,
        loaded: &[tx_processor::LoadedProcessedBlockWithMetrics],
    ) -> Self {
        let mut block_ms = loaded
            .iter()
            .map(|loaded| loaded.upstream_ms as f64)
            .collect::<Vec<_>>();
        block_ms.sort_by(f64::total_cmp);
        let blocks = loaded.len();
        let hits = loaded
            .iter()
            .filter(|loaded| loaded.disk_cache_metrics.disk_cache_hit)
            .count();
        Self {
            scenario,
            profile_kind,
            iteration,
            start_block,
            end_block,
            blocks,
            concurrency,
            chunk_size,
            fill_batch_blocks,
            fill_concurrency,
            total_ms,
            plan_ms: 0.0,
            read_wall_ms: 0.0,
            missing_count: loaded
                .iter()
                .filter(|loaded| !loaded.disk_cache_metrics.disk_cache_hit)
                .count(),
            invalid_count: 0,
            avg_ms: total_ms / blocks.max(1) as f64,
            p50_ms: percentile(&block_ms, 0.50),
            p95_ms: percentile(&block_ms, 0.95),
            max_ms: block_ms.last().copied().unwrap_or_default(),
            cache_hit_rate: hits as f64 / blocks.max(1) as f64,
            cache_read_ms: loaded
                .iter()
                .map(|loaded| loaded.disk_cache_metrics.disk_cache_read_ms as f64)
                .sum(),
            cache_write_ms: loaded
                .iter()
                .map(|loaded| loaded.disk_cache_metrics.disk_cache_write_ms as f64)
                .sum(),
            evm_exec_ms: 0.0,
            provider_read_ms: 0.0,
            account_reads: 0,
            storage_reads: 0,
            code_reads: 0,
            preload_ms: 0.0,
            trace_build_ms: 0.0,
            commit_ms: 0.0,
            processing_errors: loaded
                .iter()
                .map(|loaded| processing_error_count(&loaded.block))
                .sum(),
            internal_txs: loaded
                .iter()
                .map(|loaded| internal_tx_count(&loaded.block))
                .sum(),
        }
    }

    fn from_cache_reads(
        scenario: &'static str,
        profile_kind: &'static str,
        iteration: usize,
        start_block: u64,
        end_block: u64,
        chunk_size: u64,
        fill_batch_blocks: usize,
        fill_concurrency: usize,
        total_ms: f64,
        plan_ms: f64,
        read_wall_ms: f64,
        missing_count: usize,
        reads: &[tx_processor::ProcessedBlockDiskCacheRead],
    ) -> Result<Self> {
        let mut block_ms = reads.iter().map(|read| read.read_ms).collect::<Vec<_>>();
        block_ms.sort_by(f64::total_cmp);
        let blocks = reads.len();
        let mut processing_errors = 0;
        let mut internal_txs = 0;
        let mut invalid_count = 0;
        for read in reads {
            let Some(block) = read.block.as_ref() else {
                invalid_count += 1;
                continue;
            };
            processing_errors += processing_error_count(block);
            internal_txs += internal_tx_count(block);
        }
        if invalid_count > 0 {
            eyre::bail!("read-only cache returned {invalid_count} invalid blocks");
        }
        Ok(Self {
            scenario,
            profile_kind,
            iteration,
            start_block,
            end_block,
            blocks,
            concurrency: 0,
            chunk_size,
            fill_batch_blocks,
            fill_concurrency,
            total_ms,
            plan_ms,
            read_wall_ms,
            missing_count,
            invalid_count,
            avg_ms: total_ms / blocks.max(1) as f64,
            p50_ms: percentile(&block_ms, 0.50),
            p95_ms: percentile(&block_ms, 0.95),
            max_ms: block_ms.last().copied().unwrap_or_default(),
            cache_hit_rate: 1.0,
            cache_read_ms: reads.iter().map(|read| read.read_ms).sum(),
            cache_write_ms: 0.0,
            evm_exec_ms: 0.0,
            provider_read_ms: 0.0,
            account_reads: 0,
            storage_reads: 0,
            code_reads: 0,
            preload_ms: 0.0,
            trace_build_ms: 0.0,
            commit_ms: 0.0,
            processing_errors,
            internal_txs,
        })
    }

    fn from_replay_profiles(
        scenario: &'static str,
        profile_kind: &'static str,
        iteration: usize,
        start_block: u64,
        end_block: u64,
        concurrency: usize,
        chunk_size: u64,
        fill_batch_blocks: usize,
        fill_concurrency: usize,
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
            chunk_size,
            fill_batch_blocks,
            fill_concurrency,
            total_ms,
            plan_ms: 0.0,
            read_wall_ms: 0.0,
            missing_count: 0,
            invalid_count: 0,
            avg_ms: total_ms / blocks.max(1) as f64,
            p50_ms: percentile(&block_ms, 0.50),
            p95_ms: percentile(&block_ms, 0.95),
            max_ms: block_ms.last().copied().unwrap_or_default(),
            cache_hit_rate: 0.0,
            cache_read_ms: 0.0,
            cache_write_ms: 0.0,
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
            internal_txs: 0,
        }
    }

    fn print(&self) {
        println!(
            "{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{},{},{:.3},{:.3},{:.3},{:.3},{:.4},{:.3},{:.3},{:.3},{:.3},{},{},{},{:.3},{:.3},{:.3},{},{}",
            self.scenario,
            self.profile_kind,
            self.iteration,
            self.start_block,
            self.end_block,
            self.blocks,
            self.concurrency,
            self.chunk_size,
            self.fill_batch_blocks,
            self.fill_concurrency,
            self.total_ms,
            self.plan_ms,
            self.read_wall_ms,
            self.missing_count,
            self.invalid_count,
            self.avg_ms,
            self.p50_ms,
            self.p95_ms,
            self.max_ms,
            self.cache_hit_rate,
            self.cache_read_ms,
            self.cache_write_ms,
            self.evm_exec_ms,
            self.provider_read_ms,
            self.account_reads,
            self.storage_reads,
            self.code_reads,
            self.preload_ms,
            self.trace_build_ms,
            self.commit_ms,
            self.processing_errors,
            self.internal_txs,
        );
    }
}

fn processing_error_count(block: &ProcessedBlock) -> usize {
    block
        .transactions
        .iter()
        .filter(|tx| tx.processing_error.is_some())
        .count()
}

fn internal_tx_count(block: &ProcessedBlock) -> usize {
    block
        .transactions
        .iter()
        .map(|tx| tx.processed.internal_transactions.len())
        .sum()
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}
