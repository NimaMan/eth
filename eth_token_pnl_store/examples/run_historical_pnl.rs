use std::{
    collections::{BTreeSet, HashMap},
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use eth_token::{
    chain_metadata::RethChainMetadataProvider,
    erc20::ERC20Token,
    pnl::{PnlPoolExport, PnlPoolMeta},
    pools::classification::classify_pool,
    tracking::BlockTokenProcessor,
};
use eth_token_pnl_store::{EthConfigFile, PnlCalculationRun, TokenPnlStore, TokenPnlStoreConfig};
use eth_token_state_store::{
    PoolLatestState, TokenLatestState, TokenStateStore, TokenStateStoreConfig,
};
use eyre::{bail, eyre, Result};
use reth_chain_query::RethQueryProvider;
use serde_json::json;
use tx_processor::{
    load_processed_block_range_with_options, BlockProcessor, PoolBuySellSimulator,
    ProcessedBlockRangeLoadOptions, ProcessedBlockReplayStoreWriter,
};

const DEFAULT_BLOCK_COUNT: u64 = 50_000;
const DEFAULT_HISTORY_LIMIT: usize = 100_000;
const DEFAULT_CHUNK_BLOCKS: u64 = 250;
const DEFAULT_RETENTION_EVERY_BLOCKS: u64 = DEFAULT_CHUNK_BLOCKS;
const DEFAULT_READ_CONCURRENCY: usize = 2;
const DEFAULT_FILL_CONCURRENCY: usize = 4;
const ALGORITHM_VERSION: &str = "eth_token_pnl_v1_aggregate_terminal_or_idle_50k";

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse()?;
    let eth_config = load_eth_config(args.config_path.clone())?;
    let env_config = load_env_config(args.env_config_path.clone())?;

    let datadir = args
        .datadir
        .clone()
        .or_else(|| env_config.get("RETH_DATADIR").cloned())
        .ok_or_else(|| eyre!("missing --datadir and RETH_DATADIR in config.env"))?;
    let cache_dir = args.cache_dir.clone().or_else(|| {
        env_config
            .get("PROCESSED_BLOCK_DISK_CACHE_DIR")
            .map(PathBuf::from)
    });

    let provider = Arc::new(RethQueryProvider::new(&datadir)?);
    let latest = provider.get_latest_block()?;
    let end_block = args.end_block.unwrap_or(latest);
    if end_block > latest {
        bail!("end block {end_block} is above local latest block {latest}");
    }
    let start_block = match args.start_block {
        Some(start) => start,
        None => end_block
            .checked_sub(args.block_count.saturating_sub(1))
            .ok_or_else(|| {
                eyre!(
                    "block count {} is too large for end block {end_block}",
                    args.block_count
                )
            })?,
    };
    if end_block < start_block {
        bail!("end block must be >= start block");
    }

    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
    let state_scope_id = format!("historical:{run_id}");
    let started = Instant::now();

    println!("Historical token PnL aggregate run");
    println!("run_id:          {run_id}");
    println!("state_scope:     {state_scope_id}");
    println!("policy:          terminal_or_idle_50k");
    println!("algorithm:       {ALGORITHM_VERSION}");
    println!("range:           {start_block}..={end_block}");
    println!("history_limit:   {}", args.history_limit);
    println!("chunk_blocks:    {}", args.chunk_blocks);
    println!("retention_every: {}", args.retention_every_blocks);
    println!("trading_sim:     {}", args.with_trading_simulation);
    println!("datadir:         {datadir}");
    println!(
        "cache_dir:       {}",
        cache_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<disabled>".to_string())
    );
    println!("write_db:        {}", !args.dry_run);

    let store = if args.dry_run {
        None
    } else {
        let store_config = TokenPnlStoreConfig::from_eth_config(&eth_config)?;
        let store = TokenPnlStore::connect(&store_config).await?;
        store.run_migrations().await?;
        let mut run = PnlCalculationRun::historical(
            run_id.clone(),
            ALGORITHM_VERSION,
            start_block,
            end_block,
            true,
        );
        run.metadata = json!({
            "retention_policy": "terminal_or_idle_50k",
            "retention_blocks": 50_000,
            "history_limit": args.history_limit,
            "chunk_blocks": args.chunk_blocks,
            "retention_every_blocks": args.retention_every_blocks,
            "trading_simulation": args.with_trading_simulation,
            "aggregate_only": true,
            "movement_rows_persisted": false,
            "state_scope": state_scope_id.clone()
        });
        store.upsert_run(&run).await?;
        Some(store)
    };

    let state_store = if args.dry_run {
        None
    } else {
        let store_config = load_state_store_config(args.config_path.clone())?;
        let store = TokenStateStore::connect(&store_config).await?;
        store.run_migrations().await?;
        store.delete_scope(&state_scope_id).await?;
        Some(store)
    };

    let tx_processor = BlockProcessor::new(provider.clone());
    let discovery_provider = RethChainMetadataProvider::new(provider.as_ref());
    let pool_simulator = args
        .with_trading_simulation
        .then(|| PoolBuySellSimulator::from_simulator(provider.simulator().clone()));
    let replay_store = cache_dir
        .as_ref()
        .map(|path| {
            tx_processor::ProcessedBlockDiskCacheStore::open(path)
                .map(|store| ProcessedBlockReplayStoreWriter::new(store, provider.chain_id(), None))
        })
        .transpose()?;
    let load_options = ProcessedBlockRangeLoadOptions::default()
        .with_fill_batch_blocks(args.chunk_blocks as usize)
        .with_fill_concurrency(args.fill_concurrency)
        .with_read_concurrency(args.read_concurrency);

    let mut token_processor =
        BlockTokenProcessor::new_with_terminal_or_idle_50k_retention(args.history_limit);
    token_processor.disable_network_graphs();

    let mut stats = RunStats::default();
    let mut next_block = start_block;
    while next_block <= end_block {
        let chunk_end = next_block
            .saturating_add(args.chunk_blocks.max(1) - 1)
            .min(end_block);
        let loaded = load_processed_block_range_with_options(
            &tx_processor,
            provider.as_ref(),
            next_block,
            chunk_end,
            replay_store.as_ref(),
            load_options,
        )
        .await?;

        for loaded_block in loaded {
            let block_number = loaded_block.block.header.number;
            stats.blocks_processed += 1;
            stats.cache_hits += u64::from(loaded_block.disk_cache_metrics.disk_cache_hit);
            stats.cache_misses += u64::from(!loaded_block.disk_cache_metrics.disk_cache_hit);

            let report = match pool_simulator.as_ref() {
                Some(pool_simulator) => {
                    token_processor
                        .process_block_with_discovery_provider(
                            &loaded_block.block,
                            &discovery_provider,
                            pool_simulator,
                        )
                        .await
                }
                None => {
                    token_processor
                        .process_block_with_discovery_provider_without_trading_simulation(
                            &loaded_block.block,
                            &discovery_provider,
                        )
                        .await
                }
            };
            stats.txs_scanned += report.transaction_count as u64;
            stats.txs_processed += report.processed_transaction_count as u64;
            stats.transaction_failures += report.failed_transaction_count as u64;

            if should_apply_retention(
                start_block,
                end_block,
                block_number,
                args.retention_every_blocks,
            ) {
                stats.retention_passes += 1;
                let flush_plan = retention_flush_plan(&token_processor, block_number);
                flush_token_state_requests(
                    &state_store,
                    &state_scope_id,
                    &run_id,
                    Some(block_number),
                    &token_processor,
                    &flush_plan.token_addresses,
                    "retention_drop",
                    &mut stats,
                )
                .await?;
                flush_pool_state_requests(
                    &state_store,
                    &state_scope_id,
                    &run_id,
                    Some(block_number),
                    &token_processor,
                    &flush_plan.pools,
                    "retention_drop",
                    &mut stats,
                )
                .await?;
                flush_pool_requests(
                    &store,
                    &run_id,
                    &token_processor,
                    &flush_plan.pools,
                    "retention_drop",
                    &mut stats,
                )
                .await?;

                if let Some(retention_report) =
                    token_processor.apply_index_retention_policy(block_number)
                {
                    stats.retention_evaluated_tokens += retention_report.evaluated_tokens as u64;
                    stats.retention_dropped_tokens += retention_report.dropped_tokens as u64;
                    stats.retention_dropped_pools += retention_report.dropped_v2_pool_count as u64;
                }
            }
        }

        println!(
            "progress block={} / {} tracked_tokens={} tracked_pools={} flushed={} elapsed={:.1}s",
            chunk_end,
            end_block,
            token_processor.registry.tokens.len(),
            token_processor
                .registry
                .tokens
                .values()
                .map(|token| token.pool_count())
                .sum::<usize>(),
            stats.pool_flushes,
            started.elapsed().as_secs_f64()
        );

        if chunk_end == u64::MAX {
            break;
        }
        next_block = chunk_end + 1;
    }

    let final_flushes = final_flush_requests(&token_processor);
    let final_token_states = final_token_state_requests(&token_processor);
    flush_token_state_requests(
        &state_store,
        &state_scope_id,
        &run_id,
        Some(end_block),
        &token_processor,
        &final_token_states,
        "run_final",
        &mut stats,
    )
    .await?;
    let final_pool_states = final_pool_state_requests(&token_processor);
    flush_pool_state_requests(
        &state_store,
        &state_scope_id,
        &run_id,
        Some(end_block),
        &token_processor,
        &final_pool_states,
        "run_final",
        &mut stats,
    )
    .await?;
    flush_pool_requests(
        &store,
        &run_id,
        &token_processor,
        &final_flushes,
        "run_final",
        &mut stats,
    )
    .await?;
    stats.final_pool_flushes = final_flushes.len() as u64;

    if let Some(store) = &store {
        let mut run = PnlCalculationRun::historical(
            run_id.clone(),
            ALGORITHM_VERSION,
            start_block,
            end_block,
            true,
        );
        run.status = "complete".to_string();
        run.metadata = json!({
            "retention_policy": "terminal_or_idle_50k",
            "retention_blocks": 50_000,
            "history_limit": args.history_limit,
            "chunk_blocks": args.chunk_blocks,
            "trading_simulation": args.with_trading_simulation,
            "aggregate_only": true,
            "movement_rows_persisted": false,
            "state_scope": state_scope_id.clone(),
            "blocks_processed": stats.blocks_processed,
            "txs_scanned": stats.txs_scanned,
            "txs_processed": stats.txs_processed,
            "transaction_failures": stats.transaction_failures,
            "cache_hits": stats.cache_hits,
            "cache_misses": stats.cache_misses,
            "retention_passes": stats.retention_passes,
            "retention_evaluated_tokens": stats.retention_evaluated_tokens,
            "retention_dropped_tokens": stats.retention_dropped_tokens,
            "retention_dropped_pools": stats.retention_dropped_pools,
            "pool_flushes": stats.pool_flushes,
            "final_pool_flushes": stats.final_pool_flushes,
            "token_state_writes": stats.token_state_writes,
            "pool_state_writes": stats.pool_state_writes,
            "address_positions_written": stats.address_positions_written,
            "elapsed_secs": started.elapsed().as_secs_f64()
        });
        store.upsert_run(&run).await?;
    }

    println!();
    println!("Complete");
    println!("run_id:                    {run_id}");
    println!("elapsed:                   {:.3?}", started.elapsed());
    println!("blocks_processed:          {}", stats.blocks_processed);
    println!("txs_scanned:               {}", stats.txs_scanned);
    println!("txs_processed:             {}", stats.txs_processed);
    println!("transaction_failures:      {}", stats.transaction_failures);
    println!("cache_hits:                {}", stats.cache_hits);
    println!("cache_misses:              {}", stats.cache_misses);
    println!("retention_passes:          {}", stats.retention_passes);
    println!(
        "retention_evaluated_tokens: {}",
        stats.retention_evaluated_tokens
    );
    println!(
        "retention_dropped_tokens:  {}",
        stats.retention_dropped_tokens
    );
    println!(
        "retention_dropped_pools:   {}",
        stats.retention_dropped_pools
    );
    println!("pool_flushes:              {}", stats.pool_flushes);
    println!("final_pool_flushes:        {}", stats.final_pool_flushes);
    println!("token_state_writes:        {}", stats.token_state_writes);
    println!("pool_state_writes:         {}", stats.pool_state_writes);
    println!(
        "address_positions_written: {}",
        stats.address_positions_written
    );

    Ok(())
}

#[derive(Debug)]
struct Args {
    config_path: Option<PathBuf>,
    env_config_path: Option<PathBuf>,
    run_id: Option<String>,
    start_block: Option<u64>,
    end_block: Option<u64>,
    block_count: u64,
    datadir: Option<String>,
    cache_dir: Option<PathBuf>,
    history_limit: usize,
    chunk_blocks: u64,
    retention_every_blocks: u64,
    read_concurrency: usize,
    fill_concurrency: usize,
    dry_run: bool,
    with_trading_simulation: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut args = Self {
            config_path: None,
            env_config_path: None,
            run_id: None,
            start_block: None,
            end_block: None,
            block_count: DEFAULT_BLOCK_COUNT,
            datadir: None,
            cache_dir: None,
            history_limit: DEFAULT_HISTORY_LIMIT,
            chunk_blocks: DEFAULT_CHUNK_BLOCKS,
            retention_every_blocks: DEFAULT_RETENTION_EVERY_BLOCKS,
            read_concurrency: DEFAULT_READ_CONCURRENCY,
            fill_concurrency: DEFAULT_FILL_CONCURRENCY,
            dry_run: false,
            with_trading_simulation: false,
        };

        let mut raw = env::args().skip(1);
        while let Some(arg) = raw.next() {
            match arg.as_str() {
                "--config" => {
                    args.config_path = Some(PathBuf::from(next_arg(&mut raw, "--config")?))
                }
                "--env-config" => {
                    args.env_config_path = Some(PathBuf::from(next_arg(&mut raw, "--env-config")?));
                }
                "--run-id" => args.run_id = Some(next_arg(&mut raw, "--run-id")?),
                "--start" => args.start_block = Some(next_parse(&mut raw, "--start")?),
                "--end" => args.end_block = Some(next_parse(&mut raw, "--end")?),
                "--blocks" => args.block_count = next_parse(&mut raw, "--blocks")?,
                "--datadir" => args.datadir = Some(next_arg(&mut raw, "--datadir")?),
                "--cache-dir" => {
                    args.cache_dir = Some(PathBuf::from(next_arg(&mut raw, "--cache-dir")?))
                }
                "--history-limit" => args.history_limit = next_parse(&mut raw, "--history-limit")?,
                "--chunk-blocks" => args.chunk_blocks = next_parse(&mut raw, "--chunk-blocks")?,
                "--retention-every-blocks" => {
                    args.retention_every_blocks = next_parse(&mut raw, "--retention-every-blocks")?
                }
                "--read-concurrency" => {
                    args.read_concurrency = next_parse(&mut raw, "--read-concurrency")?
                }
                "--fill-concurrency" => {
                    args.fill_concurrency = next_parse(&mut raw, "--fill-concurrency")?
                }
                "--dry-run" => args.dry_run = true,
                "--with-trading-simulation" => args.with_trading_simulation = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => bail!("unknown argument {other}; pass --help for usage"),
            }
        }

        if args.block_count == 0 {
            bail!("--blocks must be greater than zero");
        }
        if args.chunk_blocks == 0 {
            bail!("--chunk-blocks must be greater than zero");
        }
        if args.retention_every_blocks == 0 {
            bail!("--retention-every-blocks must be greater than zero");
        }
        Ok(args)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct PoolFlushRequest {
    token_address: String,
    pool_id: String,
}

#[derive(Default)]
struct RetentionFlushPlan {
    token_addresses: Vec<String>,
    pools: Vec<PoolFlushRequest>,
}

#[derive(Default)]
struct RunStats {
    blocks_processed: u64,
    txs_scanned: u64,
    txs_processed: u64,
    transaction_failures: u64,
    cache_hits: u64,
    cache_misses: u64,
    retention_passes: u64,
    retention_evaluated_tokens: u64,
    retention_dropped_tokens: u64,
    retention_dropped_pools: u64,
    pool_flushes: u64,
    final_pool_flushes: u64,
    token_state_writes: u64,
    pool_state_writes: u64,
    address_positions_written: u64,
}

fn should_apply_retention(
    start_block: u64,
    end_block: u64,
    current_block: u64,
    retention_every_blocks: u64,
) -> bool {
    current_block == end_block
        || current_block.saturating_sub(start_block).saturating_add(1) % retention_every_blocks == 0
}

fn retention_flush_plan(processor: &BlockTokenProcessor, current_block: u64) -> RetentionFlushPlan {
    let Some(policy) = processor.token_index.live_retention_policy().cloned() else {
        return RetentionFlushPlan::default();
    };
    let mut pool_requests = BTreeSet::new();
    let mut token_requests = BTreeSet::new();
    let mut token_addresses = processor.registry.token_addresses();
    token_addresses.sort();

    for token_address in token_addresses {
        let Some(token) = processor.registry.token(&token_address) else {
            continue;
        };
        let mut token_for_decision = token.clone();
        let decision = policy.apply_to_token(&mut token_for_decision, current_block);
        if !decision.retain {
            token_requests.insert(token_address.clone());
            for (pool_id, _) in token.pnl.pools() {
                pool_requests.insert(PoolFlushRequest {
                    token_address: token_address.clone(),
                    pool_id: pool_id.clone(),
                });
            }
            continue;
        }

        for pool in decision.dropped_v2_pools {
            token_requests.insert(token_address.clone());
            if token.pnl.pool(&pool.pool_address).is_some() {
                pool_requests.insert(PoolFlushRequest {
                    token_address: token_address.clone(),
                    pool_id: pool.pool_address,
                });
            }
        }
    }

    RetentionFlushPlan {
        token_addresses: token_requests.into_iter().collect(),
        pools: pool_requests.into_iter().collect(),
    }
}

fn final_token_state_requests(processor: &BlockTokenProcessor) -> Vec<String> {
    let mut token_addresses = processor.registry.token_addresses();
    token_addresses.sort();
    token_addresses
}

fn final_flush_requests(processor: &BlockTokenProcessor) -> Vec<PoolFlushRequest> {
    let mut requests = Vec::new();
    let mut token_addresses = processor.registry.token_addresses();
    token_addresses.sort();
    for token_address in token_addresses {
        let Some(token) = processor.registry.token(&token_address) else {
            continue;
        };
        for (pool_id, _) in token.pnl.pools() {
            requests.push(PoolFlushRequest {
                token_address: token_address.clone(),
                pool_id: pool_id.clone(),
            });
        }
    }
    requests
}

fn final_pool_state_requests(processor: &BlockTokenProcessor) -> Vec<PoolFlushRequest> {
    let mut requests = Vec::new();
    let mut token_addresses = processor.registry.token_addresses();
    token_addresses.sort();
    for token_address in token_addresses {
        let Some(token) = processor.registry.token(&token_address) else {
            continue;
        };
        for pool in token.all_pool_bases() {
            requests.push(PoolFlushRequest {
                token_address: token_address.clone(),
                pool_id: pool.identity.pool_address.clone(),
            });
        }
    }
    requests
}

async fn flush_pool_requests(
    store: &Option<TokenPnlStore>,
    run_id: &str,
    processor: &BlockTokenProcessor,
    requests: &[PoolFlushRequest],
    reason: &str,
    stats: &mut RunStats,
) -> Result<()> {
    for request in requests {
        let Some(token) = processor.registry.token(&request.token_address) else {
            continue;
        };
        let Some(export) = export_pool(token, &request.pool_id) else {
            continue;
        };
        stats.pool_flushes += 1;
        stats.address_positions_written += export.address_positions.len() as u64;
        if let Some(store) = store {
            store.write_pool_aggregate_export(run_id, &export).await?;
        }
        if stats.pool_flushes % 100 == 0 {
            println!(
                "flushed_pools={} reason={} last_pool={} positions={}",
                stats.pool_flushes,
                reason,
                export.pool_id,
                export.address_positions.len()
            );
        }
    }
    Ok(())
}

async fn flush_token_state_requests(
    store: &Option<TokenStateStore>,
    scope_id: &str,
    run_id: &str,
    as_of_block: Option<u64>,
    processor: &BlockTokenProcessor,
    token_addresses: &[String],
    reason: &str,
    stats: &mut RunStats,
) -> Result<()> {
    let Some(store) = store else {
        return Ok(());
    };
    for token_address in token_addresses {
        let Some(token) = processor.registry.token(token_address) else {
            continue;
        };
        let state = TokenLatestState::from_token(
            scope_id.to_string(),
            Some(run_id.to_string()),
            as_of_block,
            reason,
            token,
        );
        store.write_token_latest(&state).await?;
        stats.token_state_writes += 1;
    }
    Ok(())
}

async fn flush_pool_state_requests(
    store: &Option<TokenStateStore>,
    scope_id: &str,
    run_id: &str,
    as_of_block: Option<u64>,
    processor: &BlockTokenProcessor,
    requests: &[PoolFlushRequest],
    reason: &str,
    stats: &mut RunStats,
) -> Result<()> {
    let Some(store) = store else {
        return Ok(());
    };
    for request in requests {
        let Some(token) = processor.registry.token(&request.token_address) else {
            continue;
        };
        let Some(pool) = token.pool_base(&request.pool_id) else {
            continue;
        };
        let state = PoolLatestState::from_pool(
            scope_id.to_string(),
            Some(run_id.to_string()),
            as_of_block,
            reason,
            pool,
        );
        store.write_pool_latest(&state).await?;
        stats.pool_state_writes += 1;
    }
    Ok(())
}

fn export_pool(token: &ERC20Token, pool_id: &str) -> Option<PnlPoolExport> {
    let pnl_pool = token.pnl.pool(pool_id)?;
    let pool_base = token.pool_base(pool_id);
    let mark_price = pool_base
        .map(|pool| pool.price())
        .filter(|price| price.is_finite() && *price > 0.0);
    let protocol = pool_base.map(|pool| pool.identity.protocol.as_str());
    let meta = pool_base
        .map(|pool| {
            let classification = classify_pool(&pool.classification_input());
            PnlPoolMeta {
                token_creator_address: token.creator_address.clone(),
                pool_creator_address: pool.creator_address.clone(),
                can_buy: pool.effective_can_buy(),
                can_sell: pool.effective_can_sell(),
                lifecycle: Some(pool.state.lifecycle.as_str().to_string()),
                is_scam: pool.is_scam() || pool.scam_mechanism.is_some(),
                scam_label: pool.scam_label.clone(),
                scam_mechanism: pool.scam_mechanism.clone(),
                eligible: classification.eligible,
                eligible_outcome: classification
                    .eligible_outcome
                    .map(|o| o.key().to_string()),
            }
        })
        .unwrap_or_default();
    let mut export = pnl_pool.export(protocol, mark_price);
    export.meta = meta;
    Some(export)
}

fn load_eth_config(path: Option<PathBuf>) -> Result<EthConfigFile> {
    Ok(match path {
        Some(path) => EthConfigFile::load(path)?,
        None => EthConfigFile::load_default()?,
    })
}

fn load_state_store_config(path: Option<PathBuf>) -> Result<TokenStateStoreConfig> {
    Ok(match path {
        Some(path) => TokenStateStoreConfig::from_shared_config_path(path)?,
        None => TokenStateStoreConfig::from_shared_config()?,
    })
}

fn load_env_config(path: Option<PathBuf>) -> Result<HashMap<String, String>> {
    let path = path.unwrap_or_else(default_env_config_path);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    parse_env_config(&path)
}

fn parse_env_config(path: &Path) -> Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)?;
    let mut values = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(values)
}

fn default_env_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../config.env")
}

fn default_run_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("historical-token-pnl-{now}")
}

fn next_arg(raw: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    raw.next().ok_or_else(|| eyre!("{flag} requires a value"))
}

fn next_parse<T>(raw: &mut impl Iterator<Item = String>, flag: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(next_arg(raw, flag)?.parse()?)
}

fn print_usage() {
    println!(
        "\
Usage:
  cargo run -p eth_token_pnl_store --example run_historical_pnl -- \\
    [--run-id <run_id>] [--start <block>] [--end <block>] [--blocks 50000]

Options:
  --config <path>            ETH config.toml path for databases.token_pnl.url
  --env-config <path>        ETH config.env path for RETH_DATADIR/cache path
  --datadir <path>           Reth datadir override
  --cache-dir <path>         Processed-block disk cache override
  --history-limit <count>    Token/pool history limit, default 100000
  --chunk-blocks <count>     Processed-block chunk size, default 250
  --retention-every-blocks <count>
                             Retention evaluation cadence, default 250
  --read-concurrency <n>     Disk-cache read concurrency, default 2
  --fill-concurrency <n>     Missing-cache fill concurrency, default 4
  --dry-run                  Run calculation without writing token_pnl tables
  --with-trading-simulation  Also run buy/sell simulation; disabled by default for PnL speed
"
    );
}
