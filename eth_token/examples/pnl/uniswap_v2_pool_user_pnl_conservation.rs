use std::{env, sync::Arc, time::Instant};

use alloy_primitives::Address;
use eth_token::{
    erc20::{ERC20Token, ERC20TokenMetadata},
    tracking::TokenStateBuilder,
};
use eyre::{bail, eyre, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::BlockProcessor;

mod fixtures;

use fixtures::{find_known_pnl_fixture, KnownPnlFixture, KNOWN_PNL_FIXTURES};

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_HISTORY_LIMIT: usize = 10_000;

#[derive(Debug)]
struct Args {
    token: Address,
    pool: Address,
    start_block: u64,
    end_block: u64,
    datadir: String,
    history_limit: usize,
    include_traces: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    if args.end_block < args.start_block {
        bail!("end block must be >= start block");
    }

    let provider = Arc::new(RethQueryProvider::new(&args.datadir)?);
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

    println!("PnL conservation seed");
    println!("datadir: {}", args.datadir);
    println!("token:   {:#x}", args.token);
    println!("pool:    {:#x}", args.pool);
    println!("token0:  {:#x}", token0);
    println!("token1:  {:#x}", token1);
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

    println!();
    println!("Token builder replay");
    println!("status:          {}", token_status_label(&token));
    println!("blocks:          {}", args.end_block - args.start_block + 1);
    println!("txs_scanned:     {}", txs_scanned);
    println!("elapsed:         {:.3?}", started.elapsed());
    println!("token_reserve:   {:.18}", pool.base.token_reserve());
    println!("denom_reserve:   {:.18}", pool.base.denom_reserve());
    println!("price:           {:.18}", pool.base.price());
    println!(
        "buy_volume:      {:?}",
        token.activity.total_buy_volume_by_denom()
    );
    println!(
        "sell_volume:     {:?}",
        token.activity.total_sell_volume_by_denom()
    );
    println!("bribes_eth:      {:.18}", token.activity.total_bribe_eth());

    let pnl_pool = token
        .pnl
        .pool(format!("{:#x}", args.pool))
        .ok_or_else(|| eyre!("pool {:#x} missing from pnl tracker", args.pool))?;
    let conservation = pnl_pool.conservation_summary();
    let mark_price = if pool.base.price().is_finite() && pool.base.price() > 0.0 {
        Some(pool.base.price())
    } else {
        None
    };

    println!();
    println!("Pool PnL ledger");
    println!("positions:       {}", pnl_pool.positions.len());
    println!("pnl_txs:         {}", pnl_pool.tx_count);
    println!(
        "token_conserved: {} delta_raw={} delta={:.18}",
        conservation.token_is_conserved, conservation.token_delta_raw, conservation.token_delta
    );
    println!(
        "denom_conserved: {} delta_raw={} delta={:.18}",
        conservation.denom_is_conserved, conservation.denom_delta_raw, conservation.denom_delta
    );
    println!(
        "pool_token_net:  raw={} scaled={:.18}",
        conservation.pool_token_delta_raw, conservation.pool_token_delta
    );
    println!(
        "pool_denom_net:  raw={} scaled={:.18}",
        conservation.pool_denom_delta_raw, conservation.pool_denom_delta
    );
    println!();
    println!("Top address positions by denom movement");
    for summary in pnl_pool.top_positions_by_denom_volume(10, false, mark_price) {
        println!(
            "address={} denom_cashflow={:.18} token_balance={:.18} marked_pnl={}",
            summary.address,
            summary.denom_cashflow,
            summary.token_balance,
            summary
                .pnl_proxy_denom
                .map(|value| format!("{value:.18}"))
                .unwrap_or_else(|| "n/a".to_string())
        );
    }

    Ok(())
}

fn token_status_label(token: &ERC20Token) -> &'static str {
    if token.hidden_mint_detected() {
        "inactive_hidden_mint"
    } else if token.liquidity_removal_pool_count() > 0 {
        "inactive_other"
    } else if token.trading_enabled() {
        "active"
    } else {
        "creation"
    }
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

fn parse_args() -> Result<Args> {
    let mut token = None;
    let mut pool = None;
    let mut start_block = None;
    let mut end_block = None;
    let mut known = None;
    let mut history_limit = DEFAULT_HISTORY_LIMIT;
    let mut include_traces = true;
    let mut datadir = env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string());

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--known" => known = Some(args.next().ok_or_else(|| eyre!("--known needs a value"))?),
            "--token" => token = Some(parse_address_arg("--token", args.next())?),
            "--pool" => pool = Some(parse_address_arg("--pool", args.next())?),
            "--start" => start_block = Some(parse_u64_arg("--start", args.next())?),
            "--end" => end_block = Some(parse_u64_arg("--end", args.next())?),
            "--history-limit" => {
                history_limit = parse_usize_arg("--history-limit", args.next())?;
            }
            "--no-traces" => {
                include_traces = false;
            }
            "--datadir" => {
                datadir = args
                    .next()
                    .ok_or_else(|| eyre!("--datadir needs a value"))?
            }
            "--list-known" => {
                print_known_fixtures();
                std::process::exit(0);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => bail!("unknown argument {arg}; pass --help for usage"),
        }
    }

    if known.is_none() && token.is_none() && pool.is_none() && start_block.is_none() {
        known = Some("hodl_weth_may_2026".to_string());
    }

    if let Some(known) = known {
        let fixture = find_known_pnl_fixture(&known)
            .ok_or_else(|| eyre!("unknown --known fixture {known}"))?;
        apply_fixture_defaults(
            fixture,
            &mut token,
            &mut pool,
            &mut start_block,
            &mut end_block,
        );
    }

    Ok(Args {
        token: token.ok_or_else(|| eyre!("missing --token"))?,
        pool: pool.ok_or_else(|| eyre!("missing --pool"))?,
        start_block: start_block.ok_or_else(|| eyre!("missing --start"))?,
        end_block: end_block.ok_or_else(|| eyre!("missing --end"))?,
        datadir,
        history_limit,
        include_traces,
    })
}

fn apply_fixture_defaults(
    fixture: KnownPnlFixture,
    token: &mut Option<Address>,
    pool: &mut Option<Address>,
    start_block: &mut Option<u64>,
    end_block: &mut Option<u64>,
) {
    token.get_or_insert(fixture.token);
    pool.get_or_insert(fixture.pool);
    start_block.get_or_insert(fixture.start_block);
    end_block.get_or_insert(fixture.end_block);
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

fn parse_usize_arg(name: &str, value: Option<String>) -> Result<usize> {
    value
        .ok_or_else(|| eyre!("{name} needs a value"))?
        .parse()
        .map_err(|err| eyre!("invalid {name}: {err}"))
}

fn print_usage() {
    println!(
        "Usage:\n  cargo run -p eth_token --example uniswap_v2_pool_user_pnl_conservation -- \\\n    --known hodl_weth_may_2026 [--datadir <reth-datadir>] [--no-traces]\n\n  cargo run -p eth_token --example uniswap_v2_pool_user_pnl_conservation -- \\\n    --token <erc20> --pool <uniswap-v2-pair> --start <block> --end <block> [--datadir <reth-datadir>] [--no-traces]\n\n  cargo run -p eth_token --example uniswap_v2_pool_user_pnl_conservation -- --list-known"
    );
}

fn print_known_fixtures() {
    for fixture in KNOWN_PNL_FIXTURES {
        println!(
            "{} token={}({:#x}) pool={:#x} denom={}({:#x}) start={} end={} protocol={} source={} note={}",
            fixture.name,
            fixture.token_symbol,
            fixture.token,
            fixture.pool,
            fixture.denom_symbol,
            fixture.denom,
            fixture.start_block,
            fixture.end_block,
            fixture.protocol,
            fixture.source_url,
            fixture.note
        );
    }
}
