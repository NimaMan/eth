use std::cmp::min;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use eyre::{bail, Result};
use tx_processor::{
    load_processed_block_range_with_options, prune_processed_block_disk_cache,
    should_prune_processed_block_disk_cache, BlockProcessor, ProcessedBlockDiskCacheStore,
    ProcessedBlockRangeLoadOptions, DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS,
    DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY, DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH,
};

const DEFAULT_BLOCKS: u64 = 100_000;
const DEFAULT_RETAIN_BLOCKS: u64 = 1_000_000;
const DISK_CACHE_DIR_ENV: &str = "ETH_TOKEN_SERVER_PROCESSED_BLOCK_DISK_CACHE_DIR";
const DISK_CACHE_DIR_NAME: &str = "processed-block-cache";

/// Refresh the processed-block disk cache for a block range.
///
/// Example:
/// cargo run -p tx_processor --release --example refresh_processed_block_disk_cache -- \
///   --blocks 100000 --chunk-size 250 --fill-batch-blocks 250 --fill-concurrency 4 --retain-blocks 1000000
#[derive(Debug, Parser)]
struct Args {
    /// Reth data directory. Defaults to RETH_DATADIR/RETH_DB_PATH/config.env via tx_simulator.
    #[arg(long)]
    reth_datadir: Option<PathBuf>,

    /// Processed-block disk cache directory. Defaults to ETH_TOKEN_SERVER_PROCESSED_BLOCK_DISK_CACHE_DIR or <reth_datadir>/processed-block-cache.
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// First block to refresh. If omitted, the range is derived from --blocks and the selected end block.
    #[arg(long)]
    start_block: Option<u64>,

    /// Last block to refresh. If omitted, defaults to latest block minus --latest-offset.
    #[arg(long)]
    end_block: Option<u64>,

    /// Number of blocks to refresh when one side of the range is omitted.
    #[arg(long, default_value_t = DEFAULT_BLOCKS)]
    blocks: u64,

    /// Blocks per cache-fill/read chunk.
    #[arg(long, default_value_t = DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH)]
    chunk_size: u64,

    /// Missing blocks per traced cache-fill batch.
    #[arg(long, default_value_t = DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS)]
    fill_batch_blocks: usize,

    /// Parallel traced block processing jobs per cache-fill batch.
    #[arg(long, default_value_t = DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY)]
    fill_concurrency: usize,

    /// Retain this many recent disk-cache blocks when pruning.
    #[arg(long, default_value_t = DEFAULT_RETAIN_BLOCKS)]
    retain_blocks: u64,

    /// Process up to latest - latest_offset when --end-block is omitted.
    #[arg(long, default_value_t = 0)]
    latest_offset: u64,

    /// Skip periodic pruning while refreshing.
    #[arg(long, default_value_t = false)]
    no_prune: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tx_processor=info".to_string()),
        )
        .init();

    let args = Args::parse();
    if args.blocks == 0 {
        bail!("--blocks must be greater than zero");
    }
    if args.chunk_size == 0 {
        bail!("--chunk-size must be greater than zero");
    }
    if args.fill_batch_blocks == 0 {
        bail!("--fill-batch-blocks must be greater than zero");
    }
    if args.fill_concurrency == 0 {
        bail!("--fill-concurrency must be greater than zero");
    }

    let reth_datadir = resolve_reth_datadir(args.reth_datadir.as_deref())?;
    let reth_datadir_str = reth_datadir.to_string_lossy();
    let provider = Arc::new(reth_chain_query::RethQueryProvider::new(&reth_datadir_str)?);
    let latest = provider
        .get_latest_block()?
        .saturating_sub(args.latest_offset);
    let (start_block, end_block) = resolve_range(&args, latest)?;
    let cache_dir = resolve_cache_dir(args.cache_dir.as_deref(), &reth_datadir);
    let store = ProcessedBlockDiskCacheStore::open(&cache_dir)?;
    let processor = BlockProcessor::new(provider.clone());

    println!(
        "Refreshing processed-block disk cache: chain_id={} range={}..={} blocks={} chunk_size={} fill_batch_blocks={} fill_concurrency={} cache_dir={}",
        provider.chain_id(),
        start_block,
        end_block,
        end_block - start_block + 1,
        args.chunk_size,
        args.fill_batch_blocks,
        args.fill_concurrency,
        cache_dir.display()
    );

    let started = Instant::now();
    let mut totals = RefreshTotals::default();
    let mut cursor = start_block;
    let load_options = ProcessedBlockRangeLoadOptions::default()
        .with_fill_batch_blocks(args.fill_batch_blocks)
        .with_fill_concurrency(args.fill_concurrency);
    while cursor <= end_block {
        let chunk_end = min(
            cursor.saturating_add(args.chunk_size.saturating_sub(1)),
            end_block,
        );
        let chunk_started = Instant::now();
        let loaded = load_processed_block_range_with_options(
            &processor,
            provider.as_ref(),
            cursor,
            chunk_end,
            Some(&store),
            load_options,
        )
        .await?;

        let mut chunk = RefreshTotals::default();
        for loaded_block in &loaded {
            chunk.blocks += 1;
            chunk.transactions += loaded_block.block.transactions.len() as u64;
            chunk.cache_hits += u64::from(loaded_block.disk_cache_metrics.disk_cache_hit);
            chunk.cache_writes += u64::from(!loaded_block.disk_cache_metrics.disk_cache_hit);
            chunk.disk_read_ms += loaded_block.disk_cache_metrics.disk_cache_read_ms;
            chunk.disk_write_ms += loaded_block.disk_cache_metrics.disk_cache_write_ms;
        }
        totals.add(&chunk);

        println!(
            "chunk {}..{} blocks={} hits={} writes={} txs={} elapsed_ms={} disk_read_ms={} disk_write_ms={}",
            cursor,
            chunk_end,
            chunk.blocks,
            chunk.cache_hits,
            chunk.cache_writes,
            chunk.transactions,
            chunk_started.elapsed().as_millis(),
            chunk.disk_read_ms,
            chunk.disk_write_ms
        );

        if !args.no_prune && should_prune_processed_block_disk_cache(chunk_end, end_block) {
            prune_processed_block_disk_cache(&store, provider.chain_id(), args.retain_blocks);
        }

        if chunk_end == u64::MAX {
            break;
        }
        cursor = chunk_end + 1;
    }

    let coverage = store.coverage()?;
    println!(
        "done blocks={} hits={} writes={} txs={} elapsed_ms={} cache_files={} cache_bytes={} trace_hash={}",
        totals.blocks,
        totals.cache_hits,
        totals.cache_writes,
        totals.transactions,
        started.elapsed().as_millis(),
        coverage.file_count,
        coverage.total_bytes,
        coverage.trace_config_hash
    );

    Ok(())
}

#[derive(Default)]
struct RefreshTotals {
    blocks: u64,
    transactions: u64,
    cache_hits: u64,
    cache_writes: u64,
    disk_read_ms: u128,
    disk_write_ms: u128,
}

impl RefreshTotals {
    fn add(&mut self, other: &Self) {
        self.blocks += other.blocks;
        self.transactions += other.transactions;
        self.cache_hits += other.cache_hits;
        self.cache_writes += other.cache_writes;
        self.disk_read_ms += other.disk_read_ms;
        self.disk_write_ms += other.disk_write_ms;
    }
}

fn resolve_reth_datadir(value: Option<&Path>) -> Result<PathBuf> {
    match value {
        Some(path) => Ok(path.to_path_buf()),
        None => Ok(PathBuf::from(tx_simulator::config::repo::eth_node_root()?)),
    }
}

fn resolve_cache_dir(value: Option<&Path>, reth_datadir: &Path) -> PathBuf {
    value
        .map(PathBuf::from)
        .or_else(|| non_empty_env(DISK_CACHE_DIR_ENV).map(PathBuf::from))
        .unwrap_or_else(|| reth_datadir.join(DISK_CACHE_DIR_NAME))
}

fn resolve_range(args: &Args, latest: u64) -> Result<(u64, u64)> {
    let (start, end) = match (args.start_block, args.end_block) {
        (Some(start), Some(end)) => (start, end),
        (Some(start), None) => (
            start,
            min(latest, start.saturating_add(args.blocks.saturating_sub(1))),
        ),
        (None, Some(end)) => (end.saturating_sub(args.blocks - 1), end),
        (None, None) => (latest.saturating_sub(args.blocks - 1), latest),
    };

    if start > end {
        bail!("invalid range: start block {start} is after end block {end}");
    }
    if end > latest {
        bail!("end block {end} is above selected latest block {latest}");
    }
    Ok((start, end))
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
