use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use eth_chain_server::app::config;
use eth_chain_server::ranges::{
    pipeline, RangeIndexJob, RangeIndexRetentionMode, RangeIndexStatus, ResolvedRangeIndexRequest,
};
use eth_chain_server::read_models::risk_atlas;
use eth_chain_server::ChainServerConfig;
use eth_risk_atlas::db::migrate;
use eth_risk_atlas::RiskAtlasWriter;
use eyre::{bail, eyre, Result, WrapErr};
use reth_chain_query::RethQueryProvider;
use tokio::time::sleep;
use tracing_subscriber::{fmt, EnvFilter};
use tx_processor::{
    ProcessedBlockDiskCacheStore, ProcessedBlockRangeLoadOptions, ProcessedBlockReplayStoreWriter,
    DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS,
    DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY,
};

const DEFAULT_BLOCK_COUNT: u64 = 200_000;
const DEFAULT_END_BLOCK_LAG: u64 = 256;
const DEFAULT_HISTORY_LIMIT: usize = 64;
const DEFAULT_PROGRESS_EVERY_SECS: u64 = 30;
const DEFAULT_HEADLESS_CHUNK_BLOCKS: u64 = 250;
const DEFAULT_HEADLESS_READ_CONCURRENCY: usize = 2;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse()?;
    if let Some(path) = args.config_path.as_ref() {
        config::set_config_path(path.clone())?;
    }

    let config = ChainServerConfig::from_config_file()?;
    let datadir = config
        .reth_datadir
        .to_str()
        .ok_or_else(|| eyre!("RETH_DATADIR is not valid UTF-8"))?;
    let provider = Arc::new(RethQueryProvider::new(datadir)?);
    let latest_block = provider.get_latest_block()?;
    let range = resolve_range(&args, latest_block)?;
    let atlas_run_id = args.run_id.unwrap_or_else(|| default_run_id(&range));
    let source_run_id = source_run_id(&atlas_run_id);
    let request = ResolvedRangeIndexRequest {
        start_block: range.start_block,
        end_block: range.end_block,
        history_limit: args.history_limit,
        retention_mode: args.retention_mode,
    };
    let run = Arc::new(RangeIndexJob::new(source_run_id.clone(), request));
    let replay_store =
        processed_block_replay_store(&config, provider.chain_id(), args.no_disk_cache)?;

    println!("Risk Atlas headless generation");
    println!("atlas_run_id:  {atlas_run_id}");
    println!("source_run_id: {source_run_id}");
    println!("datadir:       {}", config.reth_datadir.display());
    println!("latest local:  {latest_block}");
    println!("range:         {}..={}", range.start_block, range.end_block);
    println!("blocks:        {}", range.block_count());
    println!("history_limit: {}", args.history_limit);
    println!("retention:     {:?}", args.retention_mode);
    println!("chunk_blocks:  {}", args.chunk_blocks);
    println!("fill_batch:    {}", args.fill_batch_blocks);
    println!("fill_conc:     {}", args.fill_concurrency);
    println!("read_conc:     {}", args.read_concurrency);
    println!(
        "db_write:      {}",
        if args.write_db { "enabled" } else { "disabled" }
    );
    println!(
        "disk_cache:    {}",
        replay_store
            .as_ref()
            .map(|_| "enabled")
            .unwrap_or("disabled")
    );
    println!();

    let started = Instant::now();
    let reporter = tokio::spawn(progress_reporter(
        run.clone(),
        Duration::from_secs(args.progress_every_secs),
        started,
    ));
    let load_options = ProcessedBlockRangeLoadOptions::default()
        .with_fill_batch_blocks(args.fill_batch_blocks)
        .with_fill_concurrency(args.fill_concurrency)
        .with_read_concurrency(args.read_concurrency);
    pipeline::run_range_index_with_load_options(
        run.clone(),
        provider.clone(),
        replay_store,
        config.processed_block_disk_cache_blocks,
        load_options,
        args.chunk_blocks,
    )
    .await;
    reporter.abort();

    let progress = run.progress().await;
    print_progress(&progress, started);
    if progress.status != RangeIndexStatus::Completed {
        bail!(
            "range run ended with status {:?}: {}",
            progress.status,
            progress
                .last_error
                .unwrap_or_else(|| "no error detail".to_string())
        );
    }

    println!("building Risk Atlas import from completed range state");
    let mut import = risk_atlas::range_import(&run).await?;
    import.run.run_id = atlas_run_id.clone();
    risk_atlas::attach_mempool_arrivals(&mut import, provider.as_ref())
        .wrap_err("failed to attach Risk Atlas mempool arrivals")?;

    println!(
        "writing Risk Atlas DB rows: pools={} events={} observations={} distributions={} targets={}",
        import.pool_eligibility.len(),
        import.event_evidence.len(),
        import.observations.len(),
        import.distributions.len(),
        import.active_targets.len()
    );
    if args.write_db {
        let writer = RiskAtlasWriter::connect(&config.risk_atlas_database_url)
            .await
            .wrap_err("failed to connect Risk Atlas database")?;
        migrate::apply(writer.pool()).await?;
        writer.replace_report_import(&import).await?;
    } else {
        println!("skipping Risk Atlas DB write (--no-db-write)");
    }

    println!(
        "risk atlas headless import complete: run={} blocks={} pools={} observations={} elapsed={:.3?}",
        atlas_run_id,
        range.block_count(),
        import.pool_eligibility.len(),
        import.observations.len(),
        started.elapsed()
    );
    Ok(())
}

#[derive(Clone, Debug)]
struct Args {
    config_path: Option<PathBuf>,
    start_block: Option<u64>,
    end_block: Option<u64>,
    block_count: u64,
    history_limit: usize,
    progress_every_secs: u64,
    run_id: Option<String>,
    no_disk_cache: bool,
    write_db: bool,
    retention_mode: RangeIndexRetentionMode,
    chunk_blocks: u64,
    fill_batch_blocks: usize,
    fill_concurrency: usize,
    read_concurrency: usize,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut args = Self {
            config_path: None,
            start_block: None,
            end_block: None,
            block_count: DEFAULT_BLOCK_COUNT,
            history_limit: DEFAULT_HISTORY_LIMIT,
            progress_every_secs: DEFAULT_PROGRESS_EVERY_SECS,
            run_id: None,
            no_disk_cache: false,
            write_db: true,
            retention_mode: RangeIndexRetentionMode::KeepAll,
            chunk_blocks: DEFAULT_HEADLESS_CHUNK_BLOCKS,
            fill_batch_blocks: DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS,
            fill_concurrency: DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY,
            read_concurrency: DEFAULT_HEADLESS_READ_CONCURRENCY,
        };

        let mut raw = std::env::args_os().skip(1);
        while let Some(arg) = raw.next() {
            match arg.to_string_lossy().as_ref() {
                "--config" => {
                    args.config_path = Some(PathBuf::from(next_string(&mut raw, "--config")?));
                }
                "--start" => {
                    args.start_block = Some(next_parse(&mut raw, "--start")?);
                }
                "--end" => {
                    args.end_block = Some(next_parse(&mut raw, "--end")?);
                }
                "--blocks" => {
                    args.block_count = next_parse(&mut raw, "--blocks")?;
                }
                "--history-limit" => {
                    args.history_limit = next_parse(&mut raw, "--history-limit")?;
                }
                "--chunk-blocks" => {
                    args.chunk_blocks = next_parse(&mut raw, "--chunk-blocks")?;
                }
                "--fill-batch-blocks" => {
                    args.fill_batch_blocks = next_parse(&mut raw, "--fill-batch-blocks")?;
                }
                "--fill-concurrency" => {
                    args.fill_concurrency = next_parse(&mut raw, "--fill-concurrency")?;
                }
                "--read-concurrency" => {
                    args.read_concurrency = next_parse(&mut raw, "--read-concurrency")?;
                }
                "--progress-every-secs" => {
                    args.progress_every_secs = next_parse(&mut raw, "--progress-every-secs")?;
                }
                "--run-id" => {
                    args.run_id = Some(next_string(&mut raw, "--run-id")?);
                }
                "--no-disk-cache" => {
                    args.no_disk_cache = true;
                }
                "--no-db-write" => {
                    args.write_db = false;
                }
                "--ephemeral-terminal-scam" => {
                    args.retention_mode = RangeIndexRetentionMode::EphemeralTerminalScam;
                    args.write_db = false;
                }
                "--retention-mode" => {
                    args.retention_mode =
                        parse_retention_mode(&next_string(&mut raw, "--retention-mode")?)?;
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                other => bail!("unknown argument {other}; pass --help for usage"),
            }
        }

        if args.block_count == 0 {
            bail!("--blocks must be greater than zero");
        }
        if args.history_limit == 0 {
            bail!("--history-limit must be greater than zero");
        }
        if args.chunk_blocks == 0 {
            bail!("--chunk-blocks must be greater than zero");
        }
        if args.fill_batch_blocks == 0 {
            bail!("--fill-batch-blocks must be greater than zero");
        }
        if args.fill_concurrency == 0 {
            bail!("--fill-concurrency must be greater than zero");
        }
        if args.read_concurrency == 0 {
            bail!("--read-concurrency must be greater than zero");
        }
        if args.progress_every_secs == 0 {
            bail!("--progress-every-secs must be greater than zero");
        }

        Ok(args)
    }
}

#[derive(Clone, Copy, Debug)]
struct BlockRange {
    start_block: u64,
    end_block: u64,
}

impl BlockRange {
    fn block_count(&self) -> u64 {
        self.end_block - self.start_block + 1
    }
}

fn resolve_range(args: &Args, latest_block: u64) -> Result<BlockRange> {
    let end = match (args.start_block, args.end_block) {
        (_, Some(end)) => end,
        (Some(start), None) => start
            .checked_add(args.block_count - 1)
            .ok_or_else(|| eyre!("block range overflow"))?,
        (None, None) => latest_block.saturating_sub(DEFAULT_END_BLOCK_LAG),
    };
    let start = match (args.start_block, args.end_block) {
        (Some(start), _) => start,
        (None, Some(end)) => end
            .checked_sub(args.block_count - 1)
            .ok_or_else(|| eyre!("block range underflow"))?,
        (None, None) => end
            .checked_sub(args.block_count - 1)
            .ok_or_else(|| eyre!("block range underflow"))?,
    };

    if end < start {
        bail!("end block must be greater than or equal to start block");
    }

    Ok(BlockRange {
        start_block: start,
        end_block: end,
    })
}

fn default_run_id(range: &BlockRange) -> String {
    format!(
        "risk-atlas-eth-{}k-{}-{}-{}",
        range.block_count() / 1_000,
        range.start_block,
        range.end_block,
        Utc::now().format("%Y%m%d")
    )
}

fn source_run_id(atlas_run_id: &str) -> String {
    atlas_run_id
        .strip_prefix("risk-atlas-")
        .unwrap_or(atlas_run_id)
        .to_string()
}

fn processed_block_replay_store(
    config: &ChainServerConfig,
    chain_id: u64,
    no_disk_cache: bool,
) -> Result<Option<Arc<ProcessedBlockReplayStoreWriter>>> {
    if no_disk_cache {
        return Ok(None);
    }
    let Some(path) = config.processed_block_disk_cache_dir.as_ref() else {
        return Ok(None);
    };
    let store = ProcessedBlockDiskCacheStore::open(path)?;
    Ok(Some(Arc::new(ProcessedBlockReplayStoreWriter::new(
        store, chain_id, None,
    ))))
}

async fn progress_reporter(run: Arc<RangeIndexJob>, interval: Duration, started: Instant) {
    loop {
        sleep(interval).await;
        let progress = run.progress().await;
        print_progress(&progress, started);
        if progress.status.is_terminal() {
            break;
        }
    }
}

fn print_progress(progress: &eth_chain_server::ranges::RangeIndexProgress, started: Instant) {
    let pct = if progress.total_blocks == 0 {
        0.0
    } else {
        progress.blocks_processed as f64 * 100.0 / progress.total_blocks as f64
    };
    println!(
        "progress: status={:?} blocks={}/{} ({:.2}%) current={:?} tokens={} pools={} cache_hits={} cache_misses={} elapsed={:.3?}",
        progress.status,
        progress.blocks_processed,
        progress.total_blocks,
        pct,
        progress.current_block,
        progress.tracked_tokens,
        progress.tracked_pools,
        progress.processed_block_disk_cache_hits,
        progress.processed_block_disk_cache_misses,
        started.elapsed()
    );
}

fn parse_retention_mode(value: &str) -> Result<RangeIndexRetentionMode> {
    match value {
        "keep_all" => Ok(RangeIndexRetentionMode::KeepAll),
        "bounded_index" => Ok(RangeIndexRetentionMode::BoundedIndex),
        "ephemeral_terminal_scam" => Ok(RangeIndexRetentionMode::EphemeralTerminalScam),
        _ => bail!("--retention-mode must be keep_all, bounded_index, or ephemeral_terminal_scam"),
    }
}

fn next_string(raw: &mut impl Iterator<Item = std::ffi::OsString>, flag: &str) -> Result<String> {
    raw.next()
        .map(|value| value.to_string_lossy().to_string())
        .ok_or_else(|| eyre!("{flag} requires a value"))
}

fn next_parse<T>(raw: &mut impl Iterator<Item = std::ffi::OsString>, flag: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(next_string(raw, flag)?.parse()?)
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,token_range_apply_profile=warn"));
    let _ = fmt().with_env_filter(filter).try_init();
}

fn print_help() {
    println!(
        "\
usage: risk_atlas_headless [options]

Options:
  --config <path>                 Config file path, defaults to blockchains/eth/config.env
  --run-id <id>                   Final risk_atlas_runs.run_id
  --start <block>                 Start block
  --end <block>                   End block
  --blocks <count>                Block count, default 200000
  --history-limit <count>         Token history limit, default 64
  --retention-mode <mode>         keep_all, bounded_index, or ephemeral_terminal_scam; default keep_all
  --ephemeral-terminal-scam       Shortcut for --retention-mode ephemeral_terminal_scam --no-db-write
  --chunk-blocks <count>          Processed-block range chunk size, default 250
  --fill-batch-blocks <count>     Processed-block fill batch, default 250
  --fill-concurrency <count>      Processed-block fill concurrency, default 4
  --read-concurrency <count>      Processed-block cache read concurrency, default 2
  --progress-every-secs <secs>    Progress print interval, default 30
  --no-disk-cache                 Disable processed-block disk cache reads/writes
  --no-db-write                   Build the import in memory but do not write Risk Atlas DB rows
"
    );
}
