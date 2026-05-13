use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use alloy_primitives::Address;
use eyre::{eyre, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::BlockProcessor;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse()?;
    if args.single_worker_runtime {
        return run_in_single_worker_runtime(args);
    }
    profile(args).await
}

async fn profile(args: Args) -> Result<()> {
    let provider = Arc::new(RethQueryProvider::new(&args.datadir)?);

    println!("pool metadata lookup profile");
    println!("datadir: {}", args.datadir);
    println!("pool:    {:#x}", args.pool);
    println!("block:   {}", args.block);
    println!("latest:  {}", provider.get_latest_block()?);

    let total_started = Instant::now();

    let token0_started = Instant::now();
    let (token0, token1) = provider
        .uni_v2_get_tokens(args.pool, Some(args.block))
        .await?;
    println!(
        "token0/token1: {:>8} ms token0={:#x} token1={:#x}",
        token0_started.elapsed().as_millis(),
        token0,
        token1
    );

    let token0_decimals_started = Instant::now();
    let token0_decimals = provider
        .get_token_decimals(token0, Some(args.block))
        .await?;
    println!(
        "token0 decimals: {:>6} ms decimals={}",
        token0_decimals_started.elapsed().as_millis(),
        token0_decimals
    );

    let token1_decimals_started = Instant::now();
    let token1_decimals = provider
        .get_token_decimals(token1, Some(args.block))
        .await?;
    println!(
        "token1 decimals: {:>6} ms decimals={}",
        token1_decimals_started.elapsed().as_millis(),
        token1_decimals
    );

    let joined_decimals_started = Instant::now();
    let joined = tokio::try_join!(
        provider.get_token_decimals(token0, Some(args.block)),
        provider.get_token_decimals(token1, Some(args.block)),
    )?;
    println!(
        "joined decimals: {:>5} ms decimals=({}, {})",
        joined_decimals_started.elapsed().as_millis(),
        joined.0,
        joined.1
    );

    println!("total:   {:>8} ms", total_started.elapsed().as_millis());

    if args.scan_block {
        scan_processed_block(provider, args.block, args.pool).await?;
    }

    Ok(())
}

async fn scan_processed_block(
    provider: Arc<RethQueryProvider>,
    block_number: u64,
    focus_pool: Address,
) -> Result<()> {
    let processor = BlockProcessor::new(provider);
    let started = Instant::now();
    let block = processor.process_block(block_number).await?;
    let mut unique_v2_pools = std::collections::BTreeSet::new();
    let mut pair_created = 0usize;
    let mut syncs = 0usize;
    let mut swaps = 0usize;
    let mut mints = 0usize;
    let mut burns = 0usize;
    let mut focus_events = 0usize;

    for tx in &block.transactions {
        let processed = &tx.processed;
        pair_created += processed.uniswap_v2_pair_created_events.len();
        syncs += processed.uniswap_v2_syncs.len();
        swaps += processed.uniswap_v2_swaps.len();
        mints += processed.uniswap_v2_mints.len();
        burns += processed.uniswap_v2_burns.len();

        for event in &processed.uniswap_v2_pair_created_events {
            unique_v2_pools.insert(event.pair_address);
            if event.pair_address == focus_pool {
                focus_events += 1;
            }
        }
        for event in &processed.uniswap_v2_syncs {
            unique_v2_pools.insert(event.pair_address);
            if event.pair_address == focus_pool {
                focus_events += 1;
            }
        }
        for event in &processed.uniswap_v2_swaps {
            unique_v2_pools.insert(event.pair_address);
            if event.pair_address == focus_pool {
                focus_events += 1;
            }
        }
        for event in &processed.uniswap_v2_mints {
            unique_v2_pools.insert(event.pair_address);
            if event.pair_address == focus_pool {
                focus_events += 1;
            }
        }
        for event in &processed.uniswap_v2_burns {
            unique_v2_pools.insert(event.pair_address);
            if event.pair_address == focus_pool {
                focus_events += 1;
            }
        }
    }

    println!("block scan: {:>5} ms", started.elapsed().as_millis());
    println!("block txs:  {}", block.transactions.len());
    println!("v2 unique pools: {}", unique_v2_pools.len());
    println!(
        "v2 events: pair_created={} syncs={} swaps={} mints={} burns={}",
        pair_created, syncs, swaps, mints, burns
    );
    println!("focus pool v2 events: {}", focus_events);
    Ok(())
}

fn run_in_single_worker_runtime(args: Args) -> Result<()> {
    thread::spawn(move || -> Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("live-pool-metadata-profile")
            .enable_all()
            .build()?;
        runtime.block_on(profile(args))
    })
    .join()
    .map_err(|_| eyre!("single-worker runtime thread panicked"))?
}

#[derive(Debug)]
struct Args {
    datadir: String,
    pool: Address,
    block: u64,
    single_worker_runtime: bool,
    scan_block: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let shared_config = load_shared_config()?;
        let mut datadir = required_shared_config_value(&shared_config, "RETH_DATADIR")?;
        let mut pool = None;
        let mut block = None;
        let mut single_worker_runtime = false;
        let mut scan_block = false;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--datadir" => {
                    datadir = args
                        .next()
                        .ok_or_else(|| eyre!("missing --datadir value"))?;
                }
                "--pool" => {
                    let value = args.next().ok_or_else(|| eyre!("missing --pool value"))?;
                    pool = Some(value.parse()?);
                }
                "--block" => {
                    let value = args.next().ok_or_else(|| eyre!("missing --block value"))?;
                    block = Some(value.parse()?);
                }
                "--single-worker-runtime" => {
                    single_worker_runtime = true;
                }
                "--scan-block" => {
                    scan_block = true;
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => return Err(eyre!("unknown argument: {other}")),
            }
        }

        Ok(Self {
            datadir,
            pool: pool.ok_or_else(|| eyre!("missing --pool"))?,
            block: block.ok_or_else(|| eyre!("missing --block"))?,
            single_worker_runtime,
            scan_block,
        })
    }
}

fn shared_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("config.env")
}

fn load_shared_config() -> Result<HashMap<String, String>> {
    let path = shared_config_path();
    let contents = fs::read_to_string(&path).map_err(|error| {
        eyre!(
            "failed to read shared config file {}: {error}",
            path.display()
        )
    })?;
    Ok(parse_shared_config(&contents))
}

fn parse_shared_config(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        values.insert(
            key.to_string(),
            unquote_config_value(value.trim()).to_string(),
        );
    }
    values
}

fn required_shared_config_value(config: &HashMap<String, String>, key: &str) -> Result<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eyre!(
                "{key} must be set in shared config file {}",
                shared_config_path().display()
            )
        })
}

fn unquote_config_value(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn print_usage() {
    println!(
        "Usage:\n  cargo run -p eth_chain_server --example live_pool_metadata_lookup_profile -- \\\n    --pool <uniswap-v2-pair> --block <block> [--datadir <reth-datadir>] [--single-worker-runtime] [--scan-block]"
    );
}
