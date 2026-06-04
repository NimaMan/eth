use std::{env, path::PathBuf, sync::Arc, time::Instant};

use alloy_primitives::Address;
use eth_token::{erc20::ERC20TokenMetadata, tracking::TokenStateBuilder};
use eth_pnl_store::{EthConfigFile, PnlCalculationRun, TokenPnlStore, TokenPnlStoreConfig};
use eyre::{bail, eyre, Result};
use reth_chain_query::RethQueryProvider;
use serde_json::json;
use tx_processor::BlockProcessor;

const DEFAULT_HISTORY_LIMIT: usize = 100_000;
const ALGORITHM_VERSION: &str = "eth_token_pnl_v1";

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse()?;
    if args.end_block < args.start_block {
        bail!("end block must be >= start block");
    }
    let config = load_config(args.config_path.clone())?;
    let datadir = args
        .datadir
        .clone()
        .or_else(|| config.optional("RETH_DATADIR"))
        .ok_or_else(|| eyre!("missing RETH_DATADIR in {}", config.path().display()))?;

    let provider = Arc::new(RethQueryProvider::new(&datadir)?);
    let latest = provider.get_latest_block()?;
    if args.end_block > latest {
        bail!(
            "end block {} is above local latest block {}",
            args.end_block,
            latest
        );
    }

    let (token0, token1) = provider
        .uni_v2_get_tokens(args.pool, Some(args.end_block))
        .await?;
    let (denom, token1_is_denom) = orient_pair(args.token, token0, token1)?;
    let token_decimals = provider
        .get_token_decimals(args.token, Some(args.end_block))
        .await?;
    let denom_decimals = provider
        .get_token_decimals(denom, Some(args.end_block))
        .await?;
    let metadata = token_metadata(
        provider.as_ref(),
        args.token,
        args.end_block,
        token_decimals,
    )
    .await?;

    println!("Persisting pool PnL");
    println!("run_id:  {}", args.run_id);
    println!("token:   {:#x}", args.token);
    println!("pool:    {:#x}", args.pool);
    println!("denom:   {:#x}", denom);
    println!("range:   {}..={}", args.start_block, args.end_block);
    println!("traces:  {}", args.include_traces);
    println!(
        "decimals: token={} denom={} token1_is_denom={}",
        token_decimals, denom_decimals, token1_is_denom
    );

    let processor = BlockProcessor::new(provider.clone());
    let started = Instant::now();
    let mut blocks = Vec::new();
    let mut txs_scanned = 0usize;

    for block_number in args.start_block..=args.end_block {
        let block = processor
            .process_block_with_options(block_number, args.include_traces)
            .await?;
        txs_scanned += block.transactions.len();
        blocks.push(block);
    }

    let builder = TokenStateBuilder::new(metadata, args.history_limit);
    let token = builder.build_from_processed_blocks(blocks)?;
    let pool = token
        .uniswap_v2_pool(format!("{:#x}", args.pool))
        .ok_or_else(|| eyre!("pool {:#x} missing after token builder replay", args.pool))?;

    let pnl_pool = token
        .pnl
        .pool(format!("{:#x}", args.pool))
        .ok_or_else(|| eyre!("pool {:#x} missing from pnl tracker", args.pool))?;
    let mark_price = if pool.base.price().is_finite() && pool.base.price() > 0.0 {
        Some(pool.base.price())
    } else {
        None
    };
    let export = pnl_pool.export(Some("uniswap_v2"), mark_price);

    let store_config = TokenPnlStoreConfig::from_eth_config(&config)?;
    let store = TokenPnlStore::connect(&store_config).await?;
    store.run_migrations().await?;

    let mut run = PnlCalculationRun::historical(
        args.run_id.clone(),
        ALGORITHM_VERSION,
        args.start_block,
        args.end_block,
        args.include_traces,
    );
    run.status = "complete".to_string();
    run.metadata = json!({
        "token_address": format!("{:#x}", args.token),
        "pool_id": export.pool_id.clone(),
        "denom_address": format!("{:#x}", denom),
        "token0": format!("{:#x}", token0),
        "token1": format!("{:#x}", token1),
        "history_limit": args.history_limit,
        "txs_scanned": txs_scanned
    });

    store.upsert_run(&run).await?;
    store.write_pool_export(&args.run_id, &export).await?;

    let conservation = pnl_pool.conservation_summary();
    println!();
    println!("Persisted");
    println!("elapsed:           {:.3?}", started.elapsed());
    println!("txs_scanned:       {}", txs_scanned);
    println!("pnl_txs:           {}", pnl_pool.tx_count);
    println!("positions:         {}", pnl_pool.positions.len());
    println!("movements:         {}", export.movements.len());
    println!("token_conserved:   {}", conservation.token_is_conserved);
    println!("denom_conserved:   {}", conservation.denom_is_conserved);
    println!("token_delta_raw:   {}", conservation.token_delta_raw);
    println!("denom_delta_raw:   {}", conservation.denom_delta_raw);

    Ok(())
}

#[derive(Debug)]
struct Args {
    config_path: Option<PathBuf>,
    run_id: String,
    token: Address,
    pool: Address,
    start_block: u64,
    end_block: u64,
    datadir: Option<String>,
    history_limit: usize,
    include_traces: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut config_path = None;
        let mut run_id = None;
        let mut token = None;
        let mut pool = None;
        let mut start_block = None;
        let mut end_block = None;
        let mut history_limit = DEFAULT_HISTORY_LIMIT;
        let mut include_traces = true;
        let mut datadir = None;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" => config_path = Some(PathBuf::from(next_arg("--config", &mut args)?)),
                "--run-id" => run_id = Some(next_arg("--run-id", &mut args)?),
                "--token" => token = Some(parse_address_arg("--token", args.next())?),
                "--pool" => pool = Some(parse_address_arg("--pool", args.next())?),
                "--start" => start_block = Some(parse_u64_arg("--start", args.next())?),
                "--end" => end_block = Some(parse_u64_arg("--end", args.next())?),
                "--history-limit" => {
                    history_limit = parse_usize_arg("--history-limit", args.next())?;
                }
                "--no-traces" => include_traces = false,
                "--datadir" => datadir = Some(next_arg("--datadir", &mut args)?),
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                _ => bail!("unknown argument {arg}; pass --help for usage"),
            }
        }

        Ok(Self {
            config_path,
            run_id: run_id.ok_or_else(|| eyre!("missing --run-id"))?,
            token: token.ok_or_else(|| eyre!("missing --token"))?,
            pool: pool.ok_or_else(|| eyre!("missing --pool"))?,
            start_block: start_block.ok_or_else(|| eyre!("missing --start"))?,
            end_block: end_block.ok_or_else(|| eyre!("missing --end"))?,
            datadir,
            history_limit,
            include_traces,
        })
    }
}

fn load_config(path: Option<PathBuf>) -> Result<EthConfigFile> {
    Ok(match path {
        Some(path) => EthConfigFile::load(path)?,
        None => EthConfigFile::load_default()?,
    })
}

async fn token_metadata(
    provider: &RethQueryProvider,
    token: Address,
    end_block: u64,
    token_decimals: u8,
) -> Result<ERC20TokenMetadata> {
    Ok(provider
        .get_token_metadata(token, Some(end_block), None)
        .await?
        .map(|meta| {
            ERC20TokenMetadata::new(
                format!("{:#x}", token),
                meta.name,
                meta.symbol,
                token_decimals,
                meta.total_supply.to_string(),
            )
        })
        .unwrap_or_else(|| {
            ERC20TokenMetadata::new(
                format!("{:#x}", token),
                "unknown",
                "UNKNOWN",
                token_decimals,
                "0",
            )
        }))
}

fn orient_pair(token: Address, token0: Address, token1: Address) -> Result<(Address, bool)> {
    if token == token0 {
        Ok((token1, true))
    } else if token == token1 {
        Ok((token0, false))
    } else {
        bail!(
            "token {:#x} is not part of pair token0={:#x} token1={:#x}",
            token,
            token0,
            token1
        );
    }
}

fn parse_address_arg(name: &str, value: Option<String>) -> Result<Address> {
    value
        .ok_or_else(|| eyre!("{name} needs a value"))?
        .parse()
        .map_err(|e| eyre!("invalid {name}: {e}"))
}

fn parse_u64_arg(name: &str, value: Option<String>) -> Result<u64> {
    value
        .ok_or_else(|| eyre!("{name} needs a value"))?
        .parse()
        .map_err(|e| eyre!("invalid {name}: {e}"))
}

fn parse_usize_arg(name: &str, value: Option<String>) -> Result<usize> {
    value
        .ok_or_else(|| eyre!("{name} needs a value"))?
        .parse()
        .map_err(|e| eyre!("invalid {name}: {e}"))
}

fn next_arg(name: &str, args: &mut impl Iterator<Item = String>) -> Result<String> {
    args.next().ok_or_else(|| eyre!("{name} needs a value"))
}

fn print_usage() {
    println!(
        "Usage:\n  cargo run -p eth_pnl_store --example persist_uniswap_v2_pool_pnl -- \\\n    --run-id <run_id> --token <erc20> --pool <uniswap-v2-pair> \\\n    --start <block> --end <block> [--config /path/to/config.toml] [--datadir <reth-datadir>] [--no-traces]"
    );
}
