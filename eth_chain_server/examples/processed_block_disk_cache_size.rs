use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use eth_token::chain_metadata::RethChainMetadataProvider;
use eth_token::tracking::BlockTokenProcessor;
use reth_chain_query::RethQueryProvider;
use tx_processor::{
    BlockProcessor, PoolBuySellSimulator, ProcessedBlockDiskCacheKey, ProcessedBlockDiskCacheStore,
    ProcessedBlockReplayStoreWriter,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse()?;
    let provider = Arc::new(RethQueryProvider::new(&args.datadir)?);
    let processor = BlockProcessor::new(provider.clone());
    let store = ProcessedBlockDiskCacheStore::open(&args.cache_dir)?;
    let replay_store_writer =
        ProcessedBlockReplayStoreWriter::new(store.clone(), provider.chain_id(), None);
    let discovery_provider = RethChainMetadataProvider::new(provider.as_ref());
    let pool_simulator = PoolBuySellSimulator::from_simulator(provider.simulator().clone());
    let mut full_token_processor = BlockTokenProcessor::new(args.history_limit);
    let mut cached_token_processor = BlockTokenProcessor::new(args.history_limit);

    let mut processed_ms = Vec::new();
    let mut write_ms = Vec::new();
    let mut read_ms = Vec::new();

    if args.read_only && args.verify_token_output {
        eyre::bail!("--verify-token-output cannot be used with --read-only");
    }
    if args.fill_missing_then_read && args.verify_token_output {
        eyre::bail!("--verify-token-output cannot be used with --fill-missing-then-read");
    }
    if args.read_only && args.fill_missing_then_read {
        eyre::bail!("--read-only cannot be used with --fill-missing-then-read");
    }

    if args.read_only || args.fill_missing_then_read {
        let reader = store.reader();
        let plan = reader
            .plan_range(provider.as_ref(), args.start, args.end)
            .await?;
        if !plan.is_complete() && args.read_only {
            let missing = plan.missing_block_numbers();
            let sample = missing
                .iter()
                .take(20)
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            eyre::bail!(
                "cache missing {} blocks in requested range; first missing: {}",
                missing.len(),
                sample
            );
        }
        if args.fill_missing_then_read && !plan.is_complete() {
            for key in &plan.missing_keys {
                let process_started = Instant::now();
                let block = processor.process_block(key.block_number).await?;
                processed_ms.push(ms(process_started.elapsed()));

                let write = replay_store_writer.write_processed_block(&block)?;
                if write.disk_cache.key.chain_id != key.chain_id
                    || write.disk_cache.key.block_number != key.block_number
                {
                    eyre::bail!(
                        "replay store writer produced unexpected key for {}: expected chain={} block={}, wrote {:?}",
                        key.block_number,
                        key.chain_id,
                        key.block_number,
                        write.disk_cache.key
                    );
                }
                write_ms.push(write.total_write_ms() as f64);

                println!(
                    "filled block={} txs={} process_ms={:.3} write_ms={:.3}",
                    key.block_number,
                    block.transactions.len(),
                    processed_ms.last().copied().unwrap_or_default(),
                    write_ms.last().copied().unwrap_or_default()
                );
            }
        }

        let read_wall_started = Instant::now();
        let reads = reader.get_many_parallel(&plan.keys)?;
        let read_wall_ms = ms(read_wall_started.elapsed());
        let read_count = reads.len();

        for read in reads {
            let block = read
                .block
                .ok_or_else(|| eyre::eyre!("cache miss for {}", read.key.block_number))?;
            read_ms.push(read.read_ms);
            println!(
                "block={} txs={} read_ms={:.3}",
                read.key.block_number,
                block.transactions.len(),
                read_ms.last().copied().unwrap_or_default()
            );
        }

        if !processed_ms.is_empty() {
            print_summary("process", &processed_ms);
        }
        if !write_ms.is_empty() {
            print_summary("write", &write_ms);
        }
        print_summary("read", &read_ms);
        println!(
            "# summary read_wall: count={} wall_ms={:.3} avg_wall_ms_per_block={:.3}",
            read_count,
            read_wall_ms,
            read_wall_ms / read_count.max(1) as f64
        );
        return Ok(());
    }

    for block_number in args.start..=args.end {
        let key = ProcessedBlockDiskCacheKey::for_block_number(provider.chain_id(), block_number);

        let process_started = Instant::now();
        let block = processor.process_block(block_number).await?;
        processed_ms.push(ms(process_started.elapsed()));

        let write_started = Instant::now();
        store.put(&key, &block)?;
        write_ms.push(ms(write_started.elapsed()));

        let read_started = Instant::now();
        let cached = store
            .get(&key)?
            .ok_or_else(|| eyre::eyre!("cache miss immediately after write for {block_number}"))?;
        read_ms.push(ms(read_started.elapsed()));

        if args.verify_token_output {
            let full_report = full_token_processor
                .process_block_with_discovery_provider(&block, &discovery_provider, &pool_simulator)
                .await;
            let cached_report = cached_token_processor
                .process_block_with_discovery_provider(
                    &cached,
                    &discovery_provider,
                    &pool_simulator,
                )
                .await;
            if full_report != cached_report {
                eyre::bail!("token report mismatch after cached block {block_number}");
            }
            if full_token_processor != cached_token_processor {
                eyre::bail!("token processor state mismatch after cached block {block_number}");
            }
        }

        println!(
            "block={} txs={} process_ms={:.3} write_ms={:.3} read_ms={:.3}",
            block_number,
            cached.transactions.len(),
            processed_ms.last().copied().unwrap_or_default(),
            write_ms.last().copied().unwrap_or_default(),
            read_ms.last().copied().unwrap_or_default()
        );
    }

    if !processed_ms.is_empty() {
        print_summary("process", &processed_ms);
    }
    if !write_ms.is_empty() {
        print_summary("write", &write_ms);
    }
    print_summary("read", &read_ms);
    Ok(())
}

struct Args {
    datadir: String,
    cache_dir: PathBuf,
    start: u64,
    end: u64,
    history_limit: usize,
    verify_token_output: bool,
    read_only: bool,
    fill_missing_then_read: bool,
}

impl Args {
    fn parse() -> eyre::Result<Self> {
        let mut datadir = std::env::var("RETH_DATADIR")
            .unwrap_or_else(|_| "/home/nima/storage/samsung8tb/ethereum/reth".to_string());
        let mut cache_dir = None;
        let mut start = None;
        let mut end = None;
        let mut history_limit = 1_000;
        let mut verify_token_output = false;
        let mut read_only = false;
        let mut fill_missing_then_read = false;

        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--datadir" => {
                    datadir = args
                        .next()
                        .ok_or_else(|| eyre::eyre!("--datadir requires a value"))?;
                }
                "--cache-dir" => {
                    cache_dir =
                        Some(PathBuf::from(args.next().ok_or_else(|| {
                            eyre::eyre!("--cache-dir requires a value")
                        })?));
                }
                "--start" => {
                    start = Some(
                        args.next()
                            .ok_or_else(|| eyre::eyre!("--start requires a value"))?
                            .parse()?,
                    );
                }
                "--end" => {
                    end = Some(
                        args.next()
                            .ok_or_else(|| eyre::eyre!("--end requires a value"))?
                            .parse()?,
                    );
                }
                "--history-limit" => {
                    history_limit = args
                        .next()
                        .ok_or_else(|| eyre::eyre!("--history-limit requires a value"))?
                        .parse()?;
                }
                "--verify-token-output" => {
                    verify_token_output = true;
                }
                "--read-only" => {
                    read_only = true;
                }
                "--fill-missing-then-read" => {
                    fill_missing_then_read = true;
                }
                _ => eyre::bail!("unknown argument: {arg}"),
            }
        }

        let start = start.ok_or_else(|| eyre::eyre!("--start is required"))?;
        let end = end.unwrap_or(start);
        if end < start {
            eyre::bail!("--end must be greater than or equal to --start");
        }

        Ok(Self {
            datadir,
            cache_dir: cache_dir.ok_or_else(|| eyre::eyre!("--cache-dir is required"))?,
            start,
            end,
            history_limit,
            verify_token_output,
            read_only,
            fill_missing_then_read,
        })
    }
}

fn print_summary(label: &str, values: &[f64]) {
    if values.is_empty() {
        println!("# summary {label}: count=0");
        return;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let avg = sorted.iter().sum::<f64>() / sorted.len().max(1) as f64;
    let median = sorted[sorted.len() / 2];
    let p95 = sorted[((sorted.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)];
    let max = sorted.last().copied().unwrap_or_default();
    println!(
        "# summary {label}: count={} avg_ms={avg:.3} median_ms={median:.3} p95_ms={p95:.3} max_ms={max:.3}",
        sorted.len()
    );
}

fn ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
