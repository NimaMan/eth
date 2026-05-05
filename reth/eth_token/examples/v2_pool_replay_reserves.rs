use std::{env, sync::Arc, time::Instant};

use alloy_primitives::{Address, U256};
use eth_token::{
    erc20::{ERC20Token, ERC20TokenMetadata},
    pools::{BasePoolConfig, UniswapV2Pool},
};
use eyre::{bail, eyre, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, ProcessedTransaction};

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";

#[derive(Debug)]
struct Args {
    token: Address,
    pool: Address,
    start_block: u64,
    end_block: u64,
    datadir: String,
}

#[derive(Debug)]
struct ReserveView {
    token_reserve: f64,
    denom_reserve: f64,
    reserve0: U256,
    reserve1: U256,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    if args.end_block < args.start_block {
        bail!("end block must be >= start block");
    }

    let provider = Arc::new(RethQueryProvider::new(&args.datadir)?);
    let processor = BlockProcessor::new(provider.clone());
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

    let metadata = provider
        .get_token_metadata(args.token, Some(args.end_block), None)
        .await?
        .map(|meta| {
            ERC20TokenMetadata::new(
                format!("{:#x}", args.token),
                meta.name,
                meta.symbol,
                meta.decimals,
                meta.total_supply.to_string(),
            )
        })
        .unwrap_or_else(|| {
            ERC20TokenMetadata::new(
                format!("{:#x}", args.token),
                "unknown",
                "UNKNOWN",
                token_decimals,
                "0",
            )
        });

    let mut token = ERC20Token::new(metadata);
    token.create_uniswap_v2_pool(
        format!("{:#x}", args.pool),
        format!("{:#x}", denom),
        BasePoolConfig {
            denom_decimals: Some(denom_decimals),
            token1_is_denom: Some(token1_is_denom),
            history_limit: 10_000,
            denom_threshold: 0.0,
            threshold_unit: None,
            ..BasePoolConfig::new(token_decimals)
        },
        std::iter::empty::<&str>(),
    );

    let seed_block = args.start_block.saturating_sub(1);
    seed_pool_from_chain(
        provider.as_ref(),
        token
            .uniswap_v2_pool_mut(format!("{:#x}", args.pool))
            .ok_or_else(|| eyre!("pool was not inserted"))?,
        args.pool,
        seed_block,
        token_decimals,
        denom_decimals,
    )
    .await?;

    println!("V2 pool reserve replay");
    println!("datadir: {}", args.datadir);
    println!("token:   {:#x}", args.token);
    println!("pool:    {:#x}", args.pool);
    println!("token0:  {:#x}", token0);
    println!("token1:  {:#x}", token1);
    println!("denom:   {:#x}", denom);
    println!(
        "range:   {}..={} (seeded from {})",
        args.start_block, args.end_block, seed_block
    );
    println!(
        "decimals: token={} denom={} token1_is_denom={}",
        token_decimals, denom_decimals, token1_is_denom
    );

    let started = Instant::now();
    let mut blocks_processed = 0usize;
    let mut txs_scanned = 0usize;
    let mut txs_applied = 0usize;
    let mut syncs_applied = 0usize;

    for block_number in args.start_block..=args.end_block {
        let block = processor
            .process_block_with_options(block_number, false)
            .await?;
        blocks_processed += 1;
        txs_scanned += block.transactions.len();

        for tx in block.transactions {
            let processed = &tx.processed;
            if !touches_v2_pool(processed, args.pool) {
                continue;
            }

            let before_syncs = token
                .uniswap_v2_pool(format!("{:#x}", args.pool))
                .map(|pool| pool.base.sync_events.len())
                .unwrap_or_default();

            token.update_uniswap_v2_pool_from_processed_transaction(
                format!("{:#x}", args.pool),
                processed,
            )?;

            let after_syncs = token
                .uniswap_v2_pool(format!("{:#x}", args.pool))
                .map(|pool| pool.base.sync_events.len())
                .unwrap_or_default();

            txs_applied += 1;
            syncs_applied += after_syncs.saturating_sub(before_syncs);
        }
    }

    let replayed_pool = token
        .uniswap_v2_pool(format!("{:#x}", args.pool))
        .ok_or_else(|| eyre!("pool missing after replay"))?;
    let chain = chain_reserve_view(
        provider.as_ref(),
        args.pool,
        args.end_block,
        token_decimals,
        denom_decimals,
        token1_is_denom,
    )
    .await?;

    let replay_token = replayed_pool.base.token_reserve();
    let replay_denom = replayed_pool.base.denom_reserve();
    let token_delta = replay_token - chain.token_reserve;
    let denom_delta = replay_denom - chain.denom_reserve;
    let token_ok = approximately_equal(replay_token, chain.token_reserve);
    let denom_ok = approximately_equal(replay_denom, chain.denom_reserve);

    println!();
    println!("Replay summary");
    println!("blocks_processed: {}", blocks_processed);
    println!("txs_scanned:      {}", txs_scanned);
    println!("txs_applied:      {}", txs_applied);
    println!("syncs_applied:    {}", syncs_applied);
    println!("elapsed:          {:.3?}", started.elapsed());
    println!();
    println!("Reserve comparison at block {}", args.end_block);
    println!("chain reserve0/raw: {}", chain.reserve0);
    println!("chain reserve1/raw: {}", chain.reserve1);
    println!(
        "token reserve: replay={:.18} chain={:.18} delta={:.18}",
        replay_token, chain.token_reserve, token_delta
    );
    println!(
        "denom reserve: replay={:.18} chain={:.18} delta={:.18}",
        replay_denom, chain.denom_reserve, denom_delta
    );
    println!(
        "match: token={} denom={}",
        if token_ok { "yes" } else { "no" },
        if denom_ok { "yes" } else { "no" }
    );

    if !(token_ok && denom_ok) {
        bail!("replayed reserves do not match chain reserves");
    }

    Ok(())
}

async fn seed_pool_from_chain(
    provider: &RethQueryProvider,
    pool: &mut UniswapV2Pool,
    pool_address: Address,
    block_number: u64,
    token_decimals: u8,
    denom_decimals: u8,
) -> Result<()> {
    let chain = match chain_reserve_view(
        provider,
        pool_address,
        block_number,
        token_decimals,
        denom_decimals,
        pool.base.config.token1_is_denom.unwrap_or(true),
    )
    .await
    {
        Ok(chain) => chain,
        Err(_) => ReserveView {
            token_reserve: 0.0,
            denom_reserve: 0.0,
            reserve0: U256::ZERO,
            reserve1: U256::ZERO,
        },
    };
    let timestamp = provider.get_block_timestamp(block_number)?;
    pool.base.update_reserves(
        chain.token_reserve,
        chain.denom_reserve,
        block_number,
        timestamp,
        "chain-seed",
    );
    Ok(())
}

async fn chain_reserve_view(
    provider: &RethQueryProvider,
    pool: Address,
    block_number: u64,
    token_decimals: u8,
    denom_decimals: u8,
    token1_is_denom: bool,
) -> Result<ReserveView> {
    let (reserve0, reserve1, _) = provider
        .uni_v2_get_reserves(pool, Some(block_number))
        .await?;
    let reserve0_scaled = u256_to_scaled_f64(
        reserve0,
        if token1_is_denom {
            token_decimals
        } else {
            denom_decimals
        },
    )?;
    let reserve1_scaled = u256_to_scaled_f64(
        reserve1,
        if token1_is_denom {
            denom_decimals
        } else {
            token_decimals
        },
    )?;
    let (token_reserve, denom_reserve) = if token1_is_denom {
        (reserve0_scaled, reserve1_scaled)
    } else {
        (reserve1_scaled, reserve0_scaled)
    };

    Ok(ReserveView {
        token_reserve,
        denom_reserve,
        reserve0,
        reserve1,
    })
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

fn touches_v2_pool(tx: &ProcessedTransaction, pool: Address) -> bool {
    tx.uniswap_v2_syncs
        .iter()
        .any(|event| event.pair_address == pool)
        || tx
            .uniswap_v2_swaps
            .iter()
            .any(|event| event.pair_address == pool)
        || tx
            .uniswap_v2_mints
            .iter()
            .any(|event| event.pair_address == pool)
        || tx
            .uniswap_v2_burns
            .iter()
            .any(|event| event.pair_address == pool)
        || tx
            .erc20_transfers
            .iter()
            .any(|event| event.token_address == pool)
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| event.token_address == pool)
}

fn u256_to_scaled_f64(value: U256, decimals: u8) -> Result<f64> {
    let raw = value.to_string().parse::<f64>()?;
    Ok(raw / 10_f64.powi(i32::from(decimals)))
}

fn approximately_equal(left: f64, right: f64) -> bool {
    let tolerance = 1e-9_f64.max(right.abs() * 1e-12);
    (left - right).abs() <= tolerance
}

fn parse_args() -> Result<Args> {
    let mut token = None;
    let mut pool = None;
    let mut start_block = None;
    let mut end_block = None;
    let mut datadir = env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string());

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--token" => token = Some(parse_address_arg("--token", args.next())?),
            "--pool" => pool = Some(parse_address_arg("--pool", args.next())?),
            "--start" => start_block = Some(parse_u64_arg("--start", args.next())?),
            "--end" => end_block = Some(parse_u64_arg("--end", args.next())?),
            "--datadir" => {
                datadir = args
                    .next()
                    .ok_or_else(|| eyre!("--datadir needs a value"))?
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => bail!("unknown argument {arg}; pass --help for usage"),
        }
    }

    Ok(Args {
        token: token.ok_or_else(|| eyre!("missing --token"))?,
        pool: pool.ok_or_else(|| eyre!("missing --pool"))?,
        start_block: start_block.ok_or_else(|| eyre!("missing --start"))?,
        end_block: end_block.ok_or_else(|| eyre!("missing --end"))?,
        datadir,
    })
}

fn parse_address_arg(name: &str, value: Option<String>) -> Result<Address> {
    value
        .ok_or_else(|| eyre!("{name} needs a value"))?
        .parse()
        .map_err(|err| eyre!("invalid {name} address: {err}"))
}

fn parse_u64_arg(name: &str, value: Option<String>) -> Result<u64> {
    value
        .ok_or_else(|| eyre!("{name} needs a value"))?
        .parse()
        .map_err(|err| eyre!("invalid {name} block: {err}"))
}

fn print_usage() {
    println!(
        "Usage:\n  cargo run -p eth_token --example v2_pool_replay_reserves -- \\\n    --token <erc20> --pool <uniswap-v2-pair> --start <block> --end <block> [--datadir <reth-datadir>]"
    );
}
