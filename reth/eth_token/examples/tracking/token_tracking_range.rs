use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Instant;

use eth_token::manager::{BlockTokenProcessor, RethChainDiscoveryProvider, TrackedTokenStatus};
use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::BlockProcessor;

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_BLOCK_COUNT: u64 = 1_000;
const DEFAULT_HISTORY_LIMIT: usize = 1_000;

#[derive(Debug)]
struct Args {
    start_block: Option<u64>,
    end_block: Option<u64>,
    block_count: u64,
    datadir: String,
    history_limit: usize,
    progress_every: usize,
}

#[derive(Debug)]
struct Range {
    start_block: u64,
    end_block: u64,
}

#[derive(Debug, Default)]
struct StatusCounts {
    creation: usize,
    active: usize,
    inactive_scam: usize,
    inactive_other: usize,
}

#[derive(Debug, Default)]
struct ErrorSummary {
    count: usize,
    samples: Vec<ErrorSample>,
}

#[derive(Debug)]
struct ErrorSample {
    block_number: u64,
    tx_index: u64,
    tx_hash: String,
}

impl ErrorSummary {
    fn record(&mut self, block_number: u64, tx_index: u64, tx_hash: String) {
        self.count += 1;
        if self.samples.len() < 5 {
            self.samples.push(ErrorSample {
                block_number,
                tx_index,
                tx_hash,
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let provider = Arc::new(RethQueryProvider::new(&args.datadir)?);
    let tx_processor = BlockProcessor::new(provider.clone());
    let discovery_provider = RethChainDiscoveryProvider::new(provider.as_ref());
    let mut token_processor = BlockTokenProcessor::new(args.history_limit);

    let latest = provider.get_latest_block()?;
    let range = resolve_range(&args, latest)?;

    println!("Token tracking range");
    println!("datadir:       {}", args.datadir);
    println!("latest local:  {}", latest);
    println!("range:         {}..={}", range.start_block, range.end_block);
    println!("blocks:        {}", range.block_count());
    println!("history_limit: {}", args.history_limit);
    println!("progress_every: {}", args.progress_every);
    println!();

    let started = Instant::now();
    let mut blocks_processed = 0usize;
    let mut txs_scanned = 0usize;
    let mut txs_processed = 0usize;
    let mut tx_failures = 0usize;
    let mut token_update_reports = 0usize;
    let mut unique_created_tokens = BTreeSet::new();
    let mut unique_updated_tokens = BTreeSet::new();
    let mut unique_discovered_v2_pools = BTreeSet::new();
    let mut unique_updated_v2_pools = BTreeSet::new();
    let mut transaction_errors = BTreeMap::<String, ErrorSummary>::new();

    for block_number in range.start_block..=range.end_block {
        let block_started = Instant::now();
        let block = tx_processor
            .process_block_with_options(block_number, false)
            .await?;
        let upstream_elapsed = block_started.elapsed();
        if args.progress_every == 1 {
            println!(
                "processed_block: block={} txs={} upstream_elapsed={:.3?}",
                block_number,
                block.transactions.len(),
                upstream_elapsed,
            );
            io::stdout().flush().ok();
        }

        let token_apply_started = Instant::now();
        let report = token_processor
            .process_block_with_discovery_provider(&block, &discovery_provider)
            .await;
        let token_apply_elapsed = token_apply_started.elapsed();

        blocks_processed += 1;
        txs_scanned += report.transaction_count;
        txs_processed += report.processed_transaction_count;
        tx_failures += report.failed_transaction_count;
        token_update_reports += report.token_updates.len();
        unique_created_tokens.extend(report.created_token_addresses);
        unique_updated_tokens.extend(report.updated_token_addresses);
        for error in report.transaction_errors {
            transaction_errors
                .entry(error.message.clone())
                .or_default()
                .record(report.block_number, error.tx_index, error.tx_hash);
        }
        for update in report.token_updates {
            unique_discovered_v2_pools.extend(update.discovered_uniswap_v2_pools);
            unique_updated_v2_pools.extend(update.updated_uniswap_v2_pools);
        }

        if blocks_processed == 1
            || blocks_processed % args.progress_every == 0
            || block_number == range.end_block
        {
            println!(
                "progress: {:>5}/{} block={} block_elapsed={:.3?} upstream={:.3?} token_apply={:.3?} tracked_tokens={} indexed_pools={} elapsed={:.3?}",
                blocks_processed,
                range.block_count(),
                block_number,
                block_started.elapsed(),
                upstream_elapsed,
                token_apply_elapsed,
                token_processor.registry.tokens.len(),
                token_processor.token_index.pool_to_token.len(),
                started.elapsed(),
            );
            io::stdout().flush().ok();
        }
    }

    let status_counts = status_counts(&token_processor);
    let v2_pool_count = token_processor
        .registry
        .tokens
        .values()
        .map(|token| token.v2_pools.len())
        .sum::<usize>();

    println!();
    println!("Summary");
    println!("blocks_processed:       {}", blocks_processed);
    println!("txs_scanned:            {}", txs_scanned);
    println!("txs_processed:          {}", txs_processed);
    println!("tx_failures:            {}", tx_failures);
    println!("token_update_reports:   {}", token_update_reports);
    println!("created_tokens_unique:  {}", unique_created_tokens.len());
    println!("updated_tokens_unique:  {}", unique_updated_tokens.len());
    println!(
        "tracked_tokens:         {}",
        token_processor.registry.tokens.len()
    );
    println!(
        "indexed_tokens:         {}",
        token_processor.token_index.entries.len()
    );
    println!(
        "indexed_v2_pools:       {}",
        token_processor.token_index.pool_to_token.len()
    );
    println!("tracked_v2_pools:       {}", v2_pool_count);
    println!(
        "discovered_v2_pools:    {}",
        unique_discovered_v2_pools.len()
    );
    println!("updated_v2_pools:       {}", unique_updated_v2_pools.len());
    println!("status_creation:        {}", status_counts.creation);
    println!("status_active:          {}", status_counts.active);
    println!("status_inactive_scam:   {}", status_counts.inactive_scam);
    println!("status_inactive_other:  {}", status_counts.inactive_other);
    println!(
        "latest_processed_block: {:?}",
        token_processor.latest_processed_block
    );
    println!("elapsed:                {:.3?}", started.elapsed());
    print_transaction_errors(&transaction_errors);

    Ok(())
}

impl Range {
    fn block_count(&self) -> u64 {
        self.end_block - self.start_block + 1
    }
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        start_block: None,
        end_block: None,
        block_count: DEFAULT_BLOCK_COUNT,
        datadir: env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string()),
        history_limit: DEFAULT_HISTORY_LIMIT,
        progress_every: 100,
    };

    let mut raw_args = env::args().skip(1);
    while let Some(arg) = raw_args.next() {
        match arg.as_str() {
            "--start" => {
                args.start_block = Some(parse_next(&mut raw_args, "--start")?);
            }
            "--end" => {
                args.end_block = Some(parse_next(&mut raw_args, "--end")?);
            }
            "--blocks" => {
                args.block_count = parse_next(&mut raw_args, "--blocks")?;
            }
            "--datadir" => {
                args.datadir = raw_args
                    .next()
                    .ok_or_else(|| eyre::eyre!("--datadir requires a value"))?;
            }
            "--history-limit" => {
                args.history_limit = parse_next(&mut raw_args, "--history-limit")?;
            }
            "--progress-every" => {
                args.progress_every = parse_next(&mut raw_args, "--progress-every")?;
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
    if args.progress_every == 0 {
        bail!("--progress-every must be greater than zero");
    }

    Ok(args)
}

fn parse_next<T>(args: &mut impl Iterator<Item = String>, name: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    args.next()
        .ok_or_else(|| eyre::eyre!("{name} requires a value"))?
        .parse::<T>()
        .map_err(Into::into)
}

fn resolve_range(args: &Args, latest: u64) -> Result<Range> {
    let end_block = args.end_block.unwrap_or(latest);
    let start_block = args
        .start_block
        .unwrap_or_else(|| end_block.saturating_sub(args.block_count - 1));

    if end_block > latest {
        bail!("end block {end_block} is above local latest block {latest}");
    }
    if end_block < start_block {
        bail!("end block must be >= start block");
    }

    Ok(Range {
        start_block,
        end_block,
    })
}

fn status_counts(processor: &BlockTokenProcessor) -> StatusCounts {
    let mut counts = StatusCounts::default();
    for entry in processor.token_index.entries.values() {
        match &entry.token_status {
            TrackedTokenStatus::Creation => counts.creation += 1,
            TrackedTokenStatus::Active => counts.active += 1,
            TrackedTokenStatus::InactiveScam => counts.inactive_scam += 1,
            TrackedTokenStatus::InactiveOther => counts.inactive_other += 1,
        }
    }
    counts
}

fn print_transaction_errors(errors: &BTreeMap<String, ErrorSummary>) {
    if errors.is_empty() {
        return;
    }

    println!();
    println!("Transaction errors");
    for (message, summary) in errors {
        println!("- count={} message={}", summary.count, message);
        for sample in &summary.samples {
            println!(
                "  sample block={} tx_index={} tx_hash={}",
                sample.block_number, sample.tx_index, sample.tx_hash
            );
        }
    }
}

fn print_help() {
    println!(
        "Usage: cargo run -p eth_token --example token_tracking_range -- [OPTIONS]\n\
\n\
Options:\n\
  --blocks <N>          Number of blocks to process ending at --end/latest (default: 1000)\n\
  --start <BLOCK>      Start block. If omitted, derived from --end/latest and --blocks\n\
  --end <BLOCK>        End block. If omitted, latest local Reth block\n\
  --datadir <PATH>     Reth datadir. Defaults to RETH_DATADIR or local repo default\n\
  --history-limit <N>  Per-token bounded history limit (default: 1000)\n\
  --progress-every <N> Print progress every N blocks (default: 100)\n\
  -h, --help           Show this help"
    );
}
