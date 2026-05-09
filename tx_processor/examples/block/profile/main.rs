use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

use alloy_rpc_types_trace::geth::{GethTrace, TraceResult};
use clap::{Parser, ValueEnum};
use eyre::{bail, Result};
use reth_chain_query::{
    provider::{BlockDataFetcher, CallFrame, RpcBlockDataFetcher, TransactionTrace},
    RethQueryProvider,
};
use serde_json::{json, Value};
use tx_processor::{
    processed_block_trace_config_hash, BlockBatchOptions, BlockProcessor,
    PersistentProcessedBlockCacheMode, ProcessedBlock, ProcessedBlockCacheKey,
    ProcessedBlockCacheStore,
};
use tx_simulator::block_simulation::{BlockTraceEngine, BlockTracer};

const FULL_TRACES: bool = true;

#[derive(Debug, Parser)]
#[command(about = "Rust-only processed block profiling and correctness harness")]
struct Args {
    /// Comma-separated block numbers.
    #[arg(long)]
    blocks: Option<String>,

    /// First block in a contiguous range.
    #[arg(long)]
    start: Option<u64>,

    /// Last block in a contiguous range. Defaults to --start.
    #[arg(long)]
    end: Option<u64>,

    /// Profile the latest N persisted DB blocks when no block selector is provided.
    #[arg(long, default_value_t = 20)]
    latest_count: u64,

    /// Reth datadir. Defaults to RETH_DATADIR, RETH_DATA_DIR, then local default.
    #[arg(long)]
    datadir: Option<String>,

    /// Local execution RPC URL for RPC trace comparisons.
    #[arg(long)]
    rpc_url: Option<String>,

    /// Harness mode to run.
    #[arg(long, value_enum, default_value_t = Mode::All)]
    mode: Mode,

    /// Trace engine for single-engine modes.
    #[arg(long, value_enum, default_value_t = TraceEngineArg::Fresh)]
    engine: TraceEngineArg,

    /// Number of repeated iterations per scenario.
    #[arg(long, default_value_t = 3)]
    iterations: usize,

    /// Comma-separated max concurrency values for batch sweep.
    #[arg(long, default_value = "1,2,4,8,10,16,24,32")]
    concurrencies: String,

    /// Comma-separated batch sizes for batch sweep.
    #[arg(long, default_value = "20,40,80,120,240")]
    batch_sizes: String,

    /// Persistent processed-block cache directory.
    #[arg(long)]
    cache_dir: Option<String>,

    /// Persistent processed-block cache mode for DB processing modes.
    #[arg(long, value_enum, default_value_t = CacheModeArg::Off)]
    cache_mode: CacheModeArg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    All,
    DbCurrent,
    DbBaselineFresh,
    TraceOnlyFresh,
    TraceOnlyFused,
    FetchOnly,
    ProcessRawOnly,
    BatchSweep,
    Correctness,
    CacheCorrectness,
    RpcTrace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum TraceEngineArg {
    Fresh,
    Fused,
}

impl TraceEngineArg {
    fn as_engine(self) -> BlockTraceEngine {
        match self {
            Self::Fresh => BlockTraceEngine::FreshInspector,
            Self::Fused => BlockTraceEngine::RethFusedCallTracer,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CacheModeArg {
    Off,
    ReadWrite,
    ReadOnly,
    Refresh,
}

impl CacheModeArg {
    fn as_persistent(self) -> Option<PersistentProcessedBlockCacheMode> {
        match self {
            Self::Off => None,
            Self::ReadWrite => Some(PersistentProcessedBlockCacheMode::ReadWrite),
            Self::ReadOnly => Some(PersistentProcessedBlockCacheMode::ReadOnly),
            Self::Refresh => Some(PersistentProcessedBlockCacheMode::Refresh),
        }
    }
}

#[derive(Debug)]
struct TimedRow {
    scenario: &'static str,
    iteration: usize,
    block: Option<u64>,
    engine: Option<BlockTraceEngine>,
    blocks: usize,
    txs: usize,
    gas_used: u64,
    trace_nodes: usize,
    fetch_ms: f64,
    trace_ms: f64,
    process_raw_ms: f64,
    stages: StageTimings,
    cache_hit: bool,
    cache_read_ms: f64,
    cache_write_ms: f64,
    source: &'static str,
    total_ms: f64,
    processing_errors: usize,
    internal_txs: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct StageTimings {
    block_load_ms: f64,
    tx_load_ms: f64,
    receipt_load_ms: f64,
    raw_tx_process_ms: f64,
    trace_convert_ms: f64,
    internal_extract_ms: f64,
    balance_calc_ms: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    let datadir = args.datadir.clone().unwrap_or_else(default_datadir);
    let provider = Arc::new(RethQueryProvider::new(&datadir)?);
    let latest = provider.get_latest_block()?;
    let blocks = selected_blocks(&args, latest)?;
    let fetcher = Arc::new(BlockDataFetcher::new(provider.clone()));
    let processor = BlockProcessor::with_block_fetcher(fetcher.clone());
    let needs_cache = args.cache_mode != CacheModeArg::Off || args.mode == Mode::CacheCorrectness;
    let cache_store = match (needs_cache, args.cache_dir.as_deref()) {
        (false, _) => None,
        (true, Some(path)) => Some(ProcessedBlockCacheStore::open(path)?),
        (true, None) => bail!("--cache-dir is required for persistent cache modes"),
    };

    print_header();

    match args.mode {
        Mode::All => {
            run_db_process(
                "db_current",
                &processor,
                &blocks,
                BlockTraceEngine::default(),
                args.iterations,
                cache_store.as_ref(),
                args.cache_mode,
            )
            .await?;
            run_db_process(
                "db_baseline_fresh",
                &processor,
                &blocks,
                BlockTraceEngine::FreshInspector,
                args.iterations,
                cache_store.as_ref(),
                args.cache_mode,
            )
            .await?;
            run_trace_only(
                "trace_only_fused",
                &provider,
                &blocks,
                BlockTraceEngine::RethFusedCallTracer,
                args.iterations,
            )
            .await?;
            run_trace_only(
                "trace_only_fresh",
                &provider,
                &blocks,
                BlockTraceEngine::FreshInspector,
                args.iterations,
            )
            .await?;
            run_fetch_only(&provider, &blocks, args.iterations).await?;
            run_process_raw_only(&fetcher, &processor, &blocks, args.iterations).await?;
            run_batch_sweep(
                &processor,
                &blocks,
                parse_usizes(&args.batch_sizes)?,
                parse_usizes(&args.concurrencies)?,
                args.iterations,
            )
            .await?;
            run_correctness(&processor, &blocks).await?;
            if let Some(rpc_url) = args.rpc_url.as_deref() {
                run_rpc_trace(rpc_url, &blocks, args.iterations).await?;
            }
        }
        Mode::DbCurrent => {
            run_db_process(
                "db_current",
                &processor,
                &blocks,
                args.engine.as_engine(),
                args.iterations,
                cache_store.as_ref(),
                args.cache_mode,
            )
            .await?;
        }
        Mode::DbBaselineFresh => {
            run_db_process(
                "db_baseline_fresh",
                &processor,
                &blocks,
                BlockTraceEngine::FreshInspector,
                args.iterations,
                cache_store.as_ref(),
                args.cache_mode,
            )
            .await?;
        }
        Mode::TraceOnlyFresh => {
            run_trace_only(
                "trace_only_fresh",
                &provider,
                &blocks,
                BlockTraceEngine::FreshInspector,
                args.iterations,
            )
            .await?;
        }
        Mode::TraceOnlyFused => {
            run_trace_only(
                "trace_only_fused",
                &provider,
                &blocks,
                BlockTraceEngine::RethFusedCallTracer,
                args.iterations,
            )
            .await?;
        }
        Mode::FetchOnly => {
            run_fetch_only(&provider, &blocks, args.iterations).await?;
        }
        Mode::ProcessRawOnly => {
            run_process_raw_only(&fetcher, &processor, &blocks, args.iterations).await?;
        }
        Mode::BatchSweep => {
            run_batch_sweep(
                &processor,
                &blocks,
                parse_usizes(&args.batch_sizes)?,
                parse_usizes(&args.concurrencies)?,
                args.iterations,
            )
            .await?;
        }
        Mode::Correctness => {
            run_correctness(&processor, &blocks).await?;
        }
        Mode::CacheCorrectness => {
            let cache_store = cache_store
                .as_ref()
                .ok_or_else(|| eyre::eyre!("--cache-dir is required for cache-correctness"))?;
            run_cache_correctness(&processor, &blocks, cache_store, args.engine.as_engine())
                .await?;
        }
        Mode::RpcTrace => {
            let Some(rpc_url) = args.rpc_url.as_deref() else {
                bail!("--rpc-url is required for --mode rpc-trace");
            };
            run_rpc_trace(rpc_url, &blocks, args.iterations).await?;
        }
    }

    Ok(())
}

async fn run_db_process(
    scenario: &'static str,
    processor: &BlockProcessor,
    blocks: &[u64],
    engine: BlockTraceEngine,
    iterations: usize,
    cache_store: Option<&ProcessedBlockCacheStore>,
    cache_mode: CacheModeArg,
) -> Result<()> {
    let mut totals = Vec::new();
    for iteration in 0..iterations.max(1) {
        for block_number in blocks.iter().copied() {
            let started = Instant::now();
            let mut cache_hit = false;
            let mut cache_read_ms = 0.0;
            let mut cache_write_ms = 0.0;
            let mut source = "processed";
            let processed = if let (Some(cache_store), Some(cache_mode)) =
                (cache_store, cache_mode.as_persistent())
            {
                let cached = processor
                    .process_block_cached_with_mode(
                        block_number,
                        FULL_TRACES,
                        engine,
                        cache_store,
                        cache_mode,
                    )
                    .await?;
                cache_hit = cached.cache_hit;
                cache_read_ms = ms(cached.cache_read);
                cache_write_ms = ms(cached.cache_write);
                source = cached.source.as_str();
                cached.block
            } else {
                processor
                    .process_block_with_trace_engine(block_number, FULL_TRACES, engine)
                    .await?
            };
            ensure_full_traces(&processed)?;
            let total = started.elapsed();
            totals.push(ms(total));
            print_row(TimedRow {
                scenario,
                iteration,
                block: Some(block_number),
                engine: Some(engine),
                blocks: 1,
                txs: processed.transactions.len(),
                gas_used: processed.header.gas_used,
                trace_nodes: processed_block_trace_nodes(&processed),
                fetch_ms: ms(total),
                trace_ms: 0.0,
                process_raw_ms: 0.0,
                stages: StageTimings::default(),
                cache_hit,
                cache_read_ms,
                cache_write_ms,
                source,
                total_ms: ms(total),
                processing_errors: processing_error_count(&processed),
                internal_txs: internal_tx_count(&processed),
            });
        }
    }
    print_summary(scenario, blocks.len() * iterations.max(1), &totals);
    Ok(())
}

async fn run_trace_only(
    scenario: &'static str,
    provider: &Arc<RethQueryProvider>,
    blocks: &[u64],
    engine: BlockTraceEngine,
    iterations: usize,
) -> Result<()> {
    let tracer = BlockTracer::new(provider.simulator());
    let mut totals = Vec::new();
    for iteration in 0..iterations.max(1) {
        for block_number in blocks.iter().copied() {
            let started = Instant::now();
            let traces = tracer
                .trace_block_by_number_with_engine(
                    block_number,
                    Some(call_tracer_options()),
                    engine,
                )
                .await?;
            let elapsed = started.elapsed();
            totals.push(ms(elapsed));
            let (trace_nodes, gas_used) = trace_result_stats(&traces);
            print_row(TimedRow {
                scenario,
                iteration,
                block: Some(block_number),
                engine: Some(engine),
                blocks: 1,
                txs: traces.len(),
                gas_used,
                trace_nodes,
                fetch_ms: 0.0,
                trace_ms: ms(elapsed),
                process_raw_ms: 0.0,
                stages: StageTimings::default(),
                cache_hit: false,
                cache_read_ms: 0.0,
                cache_write_ms: 0.0,
                source: "trace",
                total_ms: ms(elapsed),
                processing_errors: 0,
                internal_txs: 0,
            });
        }
    }
    print_summary(scenario, blocks.len() * iterations.max(1), &totals);
    Ok(())
}

async fn run_fetch_only(
    provider: &Arc<RethQueryProvider>,
    blocks: &[u64],
    iterations: usize,
) -> Result<()> {
    let mut totals = Vec::new();
    for iteration in 0..iterations.max(1) {
        for block_number in blocks.iter().copied() {
            let total_started = Instant::now();
            let header_started = Instant::now();
            let header = provider.fetch_block_header_only(block_number).await?;
            let header_ms = ms(header_started.elapsed());

            let metadata_started = Instant::now();
            let metadata = provider.fetch_block_tx_metadata_only(block_number).await?;
            let metadata_ms = ms(metadata_started.elapsed());

            let receipts_started = Instant::now();
            let receipts = provider.fetch_block_receipts_only(block_number).await?;
            let receipts_ms = ms(receipts_started.elapsed());
            if receipts.len() != metadata.len() {
                bail!(
                    "fetch_with_traces block {} has {} transactions but {} receipts",
                    block_number,
                    metadata.len(),
                    receipts.len()
                );
            }

            let tracer = BlockTracer::new(provider.simulator());
            let trace_started = Instant::now();
            let traces = tracer
                .trace_block_by_number_with_engine(
                    block_number,
                    Some(call_tracer_options()),
                    BlockTraceEngine::default(),
                )
                .await?;
            if traces.len() != metadata.len() {
                bail!(
                    "fetch_with_traces block {} has {} transactions but {} traces",
                    block_number,
                    metadata.len(),
                    traces.len()
                );
            }
            let trace_ms = ms(trace_started.elapsed());
            let (trace_nodes, trace_gas_used) = trace_result_stats(&traces);

            let total_ms = ms(total_started.elapsed());
            totals.push(total_ms);
            print_row(TimedRow {
                scenario: "fetch_with_traces",
                iteration,
                block: Some(block_number),
                engine: Some(BlockTraceEngine::default()),
                blocks: 1,
                txs: metadata.len(),
                gas_used: trace_gas_used.max(header.gas_used),
                trace_nodes,
                fetch_ms: header_ms + metadata_ms + receipts_ms,
                trace_ms,
                process_raw_ms: 0.0,
                stages: StageTimings {
                    block_load_ms: header_ms,
                    tx_load_ms: metadata_ms,
                    receipt_load_ms: receipts_ms,
                    ..Default::default()
                },
                cache_hit: false,
                cache_read_ms: 0.0,
                cache_write_ms: 0.0,
                source: "fetch",
                total_ms,
                processing_errors: 0,
                internal_txs: 0,
            });
        }
    }
    print_summary(
        "fetch_with_traces",
        blocks.len() * iterations.max(1),
        &totals,
    );
    Ok(())
}

async fn run_process_raw_only(
    fetcher: &Arc<BlockDataFetcher>,
    processor: &BlockProcessor,
    blocks: &[u64],
    iterations: usize,
) -> Result<()> {
    let mut raw_blocks = Vec::with_capacity(blocks.len());
    for block_number in blocks.iter().copied() {
        raw_blocks.push(
            fetcher
                .fetch_db_block_with_traces(block_number, FULL_TRACES)
                .await?,
        );
    }

    let mut totals = Vec::new();
    for iteration in 0..iterations.max(1) {
        for raw in raw_blocks.iter().cloned() {
            let block_number = raw.header.number;
            let started = Instant::now();
            let (processed, profile) = processor.process_raw_block_profiled(raw).await?;
            ensure_full_traces(&processed)?;
            let elapsed = started.elapsed();
            totals.push(ms(elapsed));
            print_row(TimedRow {
                scenario: "process_raw_with_traces",
                iteration,
                block: Some(block_number),
                engine: None,
                blocks: 1,
                txs: processed.transactions.len(),
                gas_used: processed.header.gas_used,
                trace_nodes: processed_block_trace_nodes(&processed),
                fetch_ms: 0.0,
                trace_ms: 0.0,
                process_raw_ms: ms(profile.total),
                stages: StageTimings {
                    raw_tx_process_ms: ms(profile.tx_processing),
                    trace_convert_ms: ms(profile.trace_conversion),
                    internal_extract_ms: ms(profile.internal_extraction),
                    balance_calc_ms: ms(profile.balance_calculation),
                    ..Default::default()
                },
                cache_hit: false,
                cache_read_ms: 0.0,
                cache_write_ms: 0.0,
                source: "raw",
                total_ms: ms(elapsed),
                processing_errors: processing_error_count(&processed),
                internal_txs: internal_tx_count(&processed),
            });
        }
    }
    print_summary(
        "process_raw_with_traces",
        blocks.len() * iterations.max(1),
        &totals,
    );
    Ok(())
}

async fn run_batch_sweep(
    processor: &BlockProcessor,
    blocks: &[u64],
    batch_sizes: Vec<usize>,
    concurrencies: Vec<usize>,
    iterations: usize,
) -> Result<()> {
    for iteration in 0..iterations.max(1) {
        for batch_size in batch_sizes.iter().copied().filter(|value| *value > 0) {
            for concurrency in concurrencies.iter().copied().filter(|value| *value > 0) {
                let started = Instant::now();
                let mut txs = 0usize;
                let mut gas_used = 0u64;
                let mut trace_nodes = 0usize;
                let mut processing_errors = 0usize;
                let mut internal_txs = 0usize;
                for chunk in blocks.chunks(batch_size) {
                    let processed = processor
                        .process_block_batch(
                            chunk.iter().copied(),
                            BlockBatchOptions::default()
                                .with_traces(FULL_TRACES)
                                .with_trace_engine(BlockTraceEngine::default())
                                .with_max_concurrency(concurrency),
                        )
                        .await?;
                    for block in &processed {
                        ensure_full_traces(block)?;
                        txs += block.transactions.len();
                        gas_used += block.header.gas_used;
                        trace_nodes += processed_block_trace_nodes(block);
                        processing_errors += processing_error_count(block);
                        internal_txs += internal_tx_count(block);
                    }
                }
                let elapsed = started.elapsed();
                print_row(TimedRow {
                    scenario: "batch_sweep",
                    iteration,
                    block: None,
                    engine: Some(BlockTraceEngine::default()),
                    blocks: blocks.len(),
                    txs,
                    gas_used,
                    trace_nodes,
                    fetch_ms: 0.0,
                    trace_ms: 0.0,
                    process_raw_ms: 0.0,
                    stages: StageTimings::default(),
                    cache_hit: false,
                    cache_read_ms: 0.0,
                    cache_write_ms: 0.0,
                    source: "processed",
                    total_ms: ms(elapsed),
                    processing_errors,
                    internal_txs,
                });
                println!(
                    "# batch_sweep iteration={} batch_size={} concurrency={} blocks={} total_ms={:.3} avg_block_ms={:.3}",
                    iteration,
                    batch_size,
                    concurrency,
                    blocks.len(),
                    ms(elapsed),
                    ms(elapsed) / blocks.len().max(1) as f64,
                );
            }
        }
    }
    Ok(())
}

async fn run_correctness(processor: &BlockProcessor, blocks: &[u64]) -> Result<()> {
    for block_number in blocks.iter().copied() {
        let baseline = processor
            .process_block_with_trace_engine(
                block_number,
                FULL_TRACES,
                BlockTraceEngine::FreshInspector,
            )
            .await?;
        let candidate = processor
            .process_block_with_trace_engine(
                block_number,
                FULL_TRACES,
                BlockTraceEngine::RethFusedCallTracer,
            )
            .await?;
        ensure_full_traces(&baseline)?;
        ensure_full_traces(&candidate)?;
        let baseline_norm = normalize_processed_block(&baseline)?;
        let candidate_norm = normalize_processed_block(&candidate)?;
        if baseline_norm != candidate_norm {
            let diff = first_json_diff("$", &baseline_norm, &candidate_norm)
                .unwrap_or_else(|| "values differ but no focused diff was found".to_string());
            bail!("correctness mismatch for block {}: {}", block_number, diff);
        }
        println!(
            "# correctness block={} ok txs={} trace_nodes={} internal_txs={}",
            block_number,
            candidate.transactions.len(),
            processed_block_trace_nodes(&candidate),
            internal_tx_count(&candidate),
        );
    }
    Ok(())
}

async fn run_cache_correctness(
    processor: &BlockProcessor,
    blocks: &[u64],
    cache_store: &ProcessedBlockCacheStore,
    trace_engine: BlockTraceEngine,
) -> Result<()> {
    let provider = processor
        .provider()
        .ok_or_else(|| eyre::eyre!("cache correctness requires MDBX provider access"))?;
    let chain_id = provider.chain_id();
    let trace_config_hash = processed_block_trace_config_hash(FULL_TRACES);

    for block_number in blocks.iter().copied() {
        let fresh = processor
            .process_block_with_trace_engine(block_number, FULL_TRACES, trace_engine)
            .await?;
        ensure_full_traces(&fresh)?;
        let key = ProcessedBlockCacheKey::new(
            chain_id,
            fresh.header.number,
            fresh.header.hash,
            trace_engine,
            trace_config_hash,
        );
        cache_store.put(&key, &fresh)?;
        let cached = cache_store
            .get(&key)?
            .ok_or_else(|| eyre::eyre!("cache miss immediately after write for {block_number}"))?;
        ensure_full_traces(&cached)?;

        let fresh_norm = normalize_processed_block(&fresh)?;
        let cached_norm = normalize_processed_block(&cached)?;
        if fresh_norm != cached_norm {
            let diff = first_json_diff("$", &fresh_norm, &cached_norm)
                .unwrap_or_else(|| "values differ but no focused diff was found".to_string());
            bail!(
                "cache correctness mismatch for block {}: {}",
                block_number,
                diff
            );
        }
        println!(
            "# cache_correctness block={} ok txs={} trace_nodes={} internal_txs={}",
            block_number,
            cached.transactions.len(),
            processed_block_trace_nodes(&cached),
            internal_tx_count(&cached),
        );
    }
    Ok(())
}

async fn run_rpc_trace(rpc_url: &str, blocks: &[u64], iterations: usize) -> Result<()> {
    let rpc = RpcBlockDataFetcher::new(rpc_url)?;
    let mut totals = Vec::new();
    for iteration in 0..iterations.max(1) {
        for block_number in blocks.iter().copied() {
            let started = Instant::now();
            let traces = rpc.trace_block_by_number(block_number).await?;
            let elapsed = started.elapsed();
            totals.push(ms(elapsed));
            print_row(TimedRow {
                scenario: "rpc_trace",
                iteration,
                block: Some(block_number),
                engine: None,
                blocks: 1,
                txs: traces.len(),
                gas_used: 0,
                trace_nodes: 0,
                fetch_ms: 0.0,
                trace_ms: ms(elapsed),
                process_raw_ms: 0.0,
                stages: StageTimings::default(),
                cache_hit: false,
                cache_read_ms: 0.0,
                cache_write_ms: 0.0,
                source: "rpc",
                total_ms: ms(elapsed),
                processing_errors: 0,
                internal_txs: 0,
            });
        }
    }
    print_summary("rpc_trace", blocks.len() * iterations.max(1), &totals);
    Ok(())
}

fn selected_blocks(args: &Args, latest: u64) -> Result<Vec<u64>> {
    if let Some(blocks) = args.blocks.as_ref() {
        let parsed: Result<Vec<_>, _> = blocks
            .split(',')
            .map(str::trim)
            .filter(|block| !block.is_empty())
            .map(str::parse::<u64>)
            .collect();
        let parsed = parsed?;
        if parsed.is_empty() {
            bail!("--blocks did not contain any block numbers");
        }
        return Ok(parsed);
    }

    if let Some(start) = args.start {
        let end = args.end.unwrap_or(start);
        if end < start {
            bail!("--end must be greater than or equal to --start");
        }
        return Ok((start..=end).collect());
    }

    let count = args.latest_count.max(1);
    let start = latest.saturating_sub(count - 1);
    Ok((start..=latest).collect())
}

fn parse_usizes(input: &str) -> Result<Vec<usize>> {
    let parsed: Result<Vec<_>, _> = input
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::parse::<usize>)
        .collect();
    let parsed = parsed?;
    if parsed.is_empty() {
        bail!("expected at least one comma-separated usize value");
    }
    Ok(parsed)
}

fn call_tracer_options() -> alloy_rpc_types_trace::geth::GethDebugTracingOptions {
    use alloy_rpc_types_trace::geth::{GethDebugBuiltInTracerType, GethDebugTracerType};

    alloy_rpc_types_trace::geth::GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::CallTracer,
        )),
        ..Default::default()
    }
}

fn processed_block_trace_nodes(block: &ProcessedBlock) -> usize {
    block
        .transactions
        .iter()
        .filter_map(|tx| tx.trace.as_ref())
        .map(transaction_trace_nodes)
        .sum()
}

fn transaction_trace_nodes(trace: &TransactionTrace) -> usize {
    call_frame_nodes(&trace.call_frame)
}

fn call_frame_nodes(frame: &CallFrame) -> usize {
    1 + frame.subcalls.iter().map(call_frame_nodes).sum::<usize>()
}

fn internal_tx_count(block: &ProcessedBlock) -> usize {
    block
        .transactions
        .iter()
        .map(|tx| tx.processed.internal_transactions.len())
        .sum()
}

fn processing_error_count(block: &ProcessedBlock) -> usize {
    block
        .transactions
        .iter()
        .filter(|tx| tx.processing_error.is_some())
        .count()
}

fn ensure_full_traces(block: &ProcessedBlock) -> Result<()> {
    let trace_count = block
        .transactions
        .iter()
        .filter(|tx| tx.trace.is_some())
        .count();
    if trace_count != block.transactions.len() {
        bail!(
            "processed block {} has {} transactions but {} traces",
            block.header.number,
            block.transactions.len(),
            trace_count
        );
    }
    Ok(())
}

fn trace_result_stats(traces: &[TraceResult]) -> (usize, u64) {
    let mut nodes = 0usize;
    let mut gas_used = 0u64;
    for trace in traces {
        if let TraceResult::Success {
            result: GethTrace::CallTracer(frame),
            ..
        } = trace
        {
            nodes += alloy_call_frame_nodes(frame);
            gas_used = gas_used.saturating_add(frame.gas_used.try_into().unwrap_or(u64::MAX));
        }
    }
    (nodes, gas_used)
}

fn alloy_call_frame_nodes(frame: &alloy_rpc_types_trace::geth::CallFrame) -> usize {
    1 + frame
        .calls
        .iter()
        .map(alloy_call_frame_nodes)
        .sum::<usize>()
}

fn normalize_processed_block(block: &ProcessedBlock) -> Result<Value> {
    let txs = block
        .transactions
        .iter()
        .map(|tx| -> Result<Value> {
            let mut processed = serde_json::to_value(&tx.processed)?;
            canonicalize_processed_value(&mut processed);
            Ok(json!({
                "metadata": serde_json::to_value(&tx.metadata)?,
                "receipt": serde_json::to_value(&tx.receipt)?,
                "processed": processed,
                "trace": tx.trace.as_ref().map(normalize_trace).transpose()?,
                "processing_error": tx.processing_error,
            }))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(json!({
        "header": {
            "number": block.header.number,
            "hash": format!("{:#x}", block.header.hash),
            "parent_hash": format!("{:#x}", block.header.parent_hash),
            "timestamp": block.header.timestamp,
            "gas_limit": block.header.gas_limit,
            "gas_used": block.header.gas_used,
            "base_fee_per_gas": block.header.base_fee_per_gas,
            "withdrawals_root": block.header.withdrawals_root.map(|value| format!("{value:#x}")),
            "blob_gas_used": block.header.blob_gas_used,
            "excess_blob_gas": block.header.excess_blob_gas,
            "parent_beacon_block_root": block.header.parent_beacon_block_root.map(|value| format!("{value:#x}")),
            "requests_hash": block.header.requests_hash.map(|value| format!("{value:#x}")),
            "block_access_list_hash": block.header.block_access_list_hash.map(|value| format!("{value:#x}")),
            "slot_number": block.header.slot_number,
        },
        "transactions": txs,
    }))
}

fn normalize_trace(trace: &TransactionTrace) -> Result<Value> {
    Ok(json!({
        "call_frame": normalize_call_frame(&trace.call_frame),
        "gas_used": trace.gas_used,
        "output": format!("0x{}", hex::encode(&trace.output)),
        "error": trace.error,
    }))
}

fn normalize_call_frame(frame: &CallFrame) -> Value {
    json!({
        "from": format!("{:#x}", frame.from),
        "to": frame.to.map(|address| format!("{address:#x}")),
        "value": frame.value.to_string(),
        "input": format!("0x{}", hex::encode(&frame.input)),
        "output": format!("0x{}", hex::encode(&frame.output)),
        "gas_used": frame.gas_used,
        "gas_limit": frame.gas_limit,
        "depth": frame.depth,
        "call_type": frame.call_type.to_string(),
        "subcalls": frame.subcalls.iter().map(normalize_call_frame).collect::<Vec<_>>(),
    })
}

fn canonicalize_processed_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if matches!(
                    key.as_str(),
                    "unique_addresses"
                        | "erc20_contracts"
                        | "erc721_contracts"
                        | "erc1155_contracts"
                ) {
                    sort_json_array(child);
                } else {
                    canonicalize_processed_value(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                canonicalize_processed_value(item);
            }
        }
        _ => {}
    }
}

fn sort_json_array(value: &mut Value) {
    if let Value::Array(items) = value {
        items.sort_by(|left, right| left.to_string().cmp(&right.to_string()));
    }
}

fn first_json_diff(path: &str, left: &Value, right: &Value) -> Option<String> {
    if left == right {
        return None;
    }

    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            let mut keys = left_map.keys().chain(right_map.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
                let child_path = format!("{path}.{key}");
                match (left_map.get(key), right_map.get(key)) {
                    (Some(left_value), Some(right_value)) => {
                        if let Some(diff) = first_json_diff(&child_path, left_value, right_value) {
                            return Some(diff);
                        }
                    }
                    (Some(_), None) => return Some(format!("{child_path}: missing on right")),
                    (None, Some(_)) => return Some(format!("{child_path}: missing on left")),
                    (None, None) => {}
                }
            }
            Some(format!("{path}: object values differ"))
        }
        (Value::Array(left_items), Value::Array(right_items)) => {
            if left_items.len() != right_items.len() {
                return Some(format!(
                    "{path}: array length {} != {}",
                    left_items.len(),
                    right_items.len()
                ));
            }
            for (index, (left_value, right_value)) in
                left_items.iter().zip(right_items.iter()).enumerate()
            {
                let child_path = format!("{path}[{index}]");
                if let Some(diff) = first_json_diff(&child_path, left_value, right_value) {
                    return Some(diff);
                }
            }
            Some(format!("{path}: array values differ"))
        }
        _ => Some(format!("{path}: left={left} right={right}")),
    }
}

fn print_header() {
    println!(
        "scenario,iteration,block,engine,blocks,txs,gas_used,trace_nodes,fetch_ms,trace_ms,process_raw_ms,block_load_ms,tx_load_ms,receipt_load_ms,raw_tx_process_ms,trace_convert_ms,internal_extract_ms,balance_calc_ms,cache_hit,cache_read_ms,cache_write_ms,source,total_ms,avg_block_ms,processing_errors,internal_txs"
    );
}

fn print_row(row: TimedRow) {
    println!(
        "{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{:.3},{:.3},{},{:.3},{:.3},{},{}",
        row.scenario,
        row.iteration,
        row.block
            .map(|value| value.to_string())
            .unwrap_or_else(|| "range".to_string()),
        row.engine
            .map(|value| format!("{value:?}"))
            .unwrap_or_else(|| "none".to_string()),
        row.blocks,
        row.txs,
        row.gas_used,
        row.trace_nodes,
        row.fetch_ms,
        row.trace_ms,
        row.process_raw_ms,
        row.stages.block_load_ms,
        row.stages.tx_load_ms,
        row.stages.receipt_load_ms,
        row.stages.raw_tx_process_ms,
        row.stages.trace_convert_ms,
        row.stages.internal_extract_ms,
        row.stages.balance_calc_ms,
        row.cache_hit,
        row.cache_read_ms,
        row.cache_write_ms,
        row.source,
        row.total_ms,
        row.total_ms / row.blocks.max(1) as f64,
        row.processing_errors,
        row.internal_txs,
    );
}

fn print_summary(label: &str, count: usize, totals: &[f64]) {
    if totals.is_empty() {
        return;
    }
    println!(
        "# summary {label}: count={} rows={} avg_ms={:.3} median_ms={:.3} p95_ms={:.3} max_ms={:.3}",
        count,
        totals.len(),
        average(totals),
        percentile(totals.to_vec(), 0.50),
        percentile(totals.to_vec(), 0.95),
        totals.iter().copied().fold(0.0, f64::max),
    );
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn average(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn percentile(mut values: Vec<f64>, percentile: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    let idx = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[idx]
}

fn default_datadir() -> String {
    env::var("RETH_DATADIR")
        .or_else(|_| env::var("RETH_DATA_DIR"))
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string())
}
