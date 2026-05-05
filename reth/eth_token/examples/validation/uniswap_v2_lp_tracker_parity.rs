use std::{env, sync::Arc, time::Instant};

use alloy_primitives::{Address, Bytes, U256};
use eth_token::{
    erc20::{ERC20Token, ERC20TokenMetadata},
    pools::BasePoolConfig,
};
use eyre::{bail, eyre, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, ProcessedTransaction};

#[path = "../fixtures/uniswap_v2_known_pools.rs"]
mod uniswap_v2_known_pools;

use uniswap_v2_known_pools::{find_known_uniswap_v2_pool_range, KNOWN_UNISWAP_V2_POOL_RANGES};

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const LP_DECIMALS: u8 = 18;
const SELECTOR_ALLOWANCE: [u8; 4] = [0xdd, 0x62, 0xed, 0x3e];

#[derive(Debug)]
struct Args {
    token: Address,
    pool: Address,
    start_block: u64,
    end_block: u64,
    datadir: String,
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

    let seed_block = args.start_block.saturating_sub(1);
    let seed_supply = token_total_supply_or_zero(provider.as_ref(), args.pool, seed_block).await?;
    if !seed_supply.is_zero() {
        bail!(
            "LP totalSupply at start - 1 is nonzero ({} at block {}). This validator needs a pool-creation range or explicit LP holder seeding.",
            seed_supply,
            seed_block
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

    println!("Uniswap V2 LP tracker parity");
    println!("datadir: {}", args.datadir);
    println!("token:   {:#x}", args.token);
    println!("pool/lp: {:#x}", args.pool);
    println!("range:   {}..={}", args.start_block, args.end_block);
    println!("token0:  {:#x}", token0);
    println!("token1:  {:#x}", token1);

    let started = Instant::now();
    let mut blocks_processed = 0usize;
    let mut txs_scanned = 0usize;
    let mut txs_applied = 0usize;

    for block_number in args.start_block..=args.end_block {
        let block = processor
            .process_block_with_options(block_number, false)
            .await?;
        blocks_processed += 1;
        txs_scanned += block.transactions.len();

        for tx in block.transactions {
            let processed = &tx.processed;
            if !touches_lp_token(processed, args.pool) {
                continue;
            }

            token.update_uniswap_v2_pool_from_processed_transaction(
                format!("{:#x}", args.pool),
                processed,
            )?;
            txs_applied += 1;
        }
    }

    let pool = token
        .uniswap_v2_pool(format!("{:#x}", args.pool))
        .ok_or_else(|| eyre!("pool missing after replay"))?;
    let chain_supply = provider
        .get_token_total_supply(args.pool, Some(args.end_block))
        .await?;
    let chain_supply_scaled = u256_to_scaled_f64(chain_supply, LP_DECIMALS)?;
    let tracked_supply = pool.lp_tracker.total_supply;
    let mut mismatches = Vec::new();

    if !approximately_equal(tracked_supply, chain_supply_scaled) {
        mismatches.push(format!(
            "totalSupply mismatch tracked={tracked_supply:.18} chain={chain_supply_scaled:.18}"
        ));
    }

    let holders = pool.lp_holders();
    for holder in &holders {
        let holder_address: Address = holder.address.parse()?;
        let chain_balance = provider
            .get_token_balance(args.pool, holder_address, Some(args.end_block))
            .await?;
        let chain_balance_scaled = u256_to_scaled_f64(chain_balance, LP_DECIMALS)?;
        if !approximately_equal(holder.balance, chain_balance_scaled) {
            mismatches.push(format!(
                "balance mismatch holder={} tracked={:.18} chain={:.18}",
                holder.address, holder.balance, chain_balance_scaled
            ));
        }

        for (spender, approval) in &holder.approvals {
            let spender_address: Address = spender.parse()?;
            let chain_allowance = erc20_allowance(
                provider.as_ref(),
                args.pool,
                holder_address,
                spender_address,
                args.end_block,
            )
            .await?;
            let chain_allowance_scaled = u256_to_scaled_f64(chain_allowance, LP_DECIMALS)?;
            if !approximately_equal(approval.amount, chain_allowance_scaled) {
                mismatches.push(format!(
                    "allowance mismatch owner={} spender={} tracked={:.18} chain={:.18}",
                    holder.address, spender, approval.amount, chain_allowance_scaled
                ));
            }
        }
    }

    println!();
    println!("Replay summary");
    println!("blocks_processed: {}", blocks_processed);
    println!("txs_scanned:      {}", txs_scanned);
    println!("txs_applied:      {}", txs_applied);
    println!("holders_tracked:  {}", holders.len());
    println!("approvals_tracked: {}", approval_count(&holders));
    println!("elapsed:          {:.3?}", started.elapsed());
    println!();
    println!("LP total supply at block {}", args.end_block);
    println!(
        "tracked={:.18} chain={:.18} raw={}",
        tracked_supply, chain_supply_scaled, chain_supply
    );

    if !mismatches.is_empty() {
        println!();
        println!("Mismatches");
        for mismatch in &mismatches {
            println!("- {mismatch}");
        }
        bail!(
            "LP tracker parity failed with {} mismatches",
            mismatches.len()
        );
    }

    println!("match: yes");
    Ok(())
}

async fn token_total_supply_or_zero(
    provider: &RethQueryProvider,
    token: Address,
    block: u64,
) -> Result<U256> {
    match provider.get_token_total_supply(token, Some(block)).await {
        Ok(supply) => Ok(supply),
        Err(_) => Ok(U256::ZERO),
    }
}

async fn erc20_allowance(
    provider: &RethQueryProvider,
    token: Address,
    owner: Address,
    spender: Address,
    block: u64,
) -> Result<U256> {
    let mut data = SELECTOR_ALLOWANCE.to_vec();
    data.extend_from_slice(&encode_address_word(owner));
    data.extend_from_slice(&encode_address_word(spender));
    let result = provider
        .simulator()
        .simulate_view_function(token, Bytes::from(data), Some(block))
        .await?;
    if !result.success || result.output.len() < 32 {
        bail!(
            "allowance view call failed for token={token:#x} owner={owner:#x} spender={spender:#x}"
        );
    }
    Ok(U256::from_be_slice(&result.output[..32]))
}

fn encode_address_word(address: Address) -> [u8; 32] {
    let mut word = [0_u8; 32];
    word[12..].copy_from_slice(address.as_ref());
    word
}

fn touches_lp_token(tx: &ProcessedTransaction, pool: Address) -> bool {
    tx.erc20_transfers
        .iter()
        .any(|event| event.token_address == pool)
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| event.token_address == pool)
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

fn u256_to_scaled_f64(value: U256, decimals: u8) -> Result<f64> {
    let raw = value.to_string().parse::<f64>()?;
    Ok(raw / 10_f64.powi(i32::from(decimals)))
}

fn approximately_equal(left: f64, right: f64) -> bool {
    let tolerance = 1e-9_f64.max(right.abs() * 1e-12);
    (left - right).abs() <= tolerance
}

fn approval_count(holders: &[eth_token::pools::uniswap::v2::LPHolderSnapshot]) -> usize {
    holders.iter().map(|holder| holder.approvals.len()).sum()
}

fn parse_args() -> Result<Args> {
    let mut token = None;
    let mut pool = None;
    let mut start_block = None;
    let mut end_block = None;
    let mut known = None;
    let mut datadir = env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string());

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--known" => known = Some(args.next().ok_or_else(|| eyre!("--known needs a value"))?),
            "--token" => token = Some(parse_address_arg("--token", args.next())?),
            "--pool" => pool = Some(parse_address_arg("--pool", args.next())?),
            "--start" => start_block = Some(parse_u64_arg("--start", args.next())?),
            "--end" => end_block = Some(parse_u64_arg("--end", args.next())?),
            "--datadir" => {
                datadir = args
                    .next()
                    .ok_or_else(|| eyre!("--datadir needs a value"))?
            }
            "--list-known" => {
                print_known_ranges();
                std::process::exit(0);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => bail!("unknown argument {arg}; pass --help for usage"),
        }
    }

    if let Some(known) = known {
        let range = find_known_uniswap_v2_pool_range(&known)
            .ok_or_else(|| eyre!("unknown --known fixture {known}"))?;
        token.get_or_insert(range.token);
        pool.get_or_insert(range.pool);
        start_block.get_or_insert(range.start_block);
        end_block.get_or_insert(range.end_block);
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
        "Usage:\n  cargo run -p eth_token --example uniswap_v2_lp_tracker_parity -- \\\n    --token <erc20> --pool <uniswap-v2-pair> --start <pool-creation-block> --end <block> [--datadir <reth-datadir>]\n\n  cargo run -p eth_token --example uniswap_v2_lp_tracker_parity -- --known <fixture>\n\n  cargo run -p eth_token --example uniswap_v2_lp_tracker_parity -- --list-known"
    );
}

fn print_known_ranges() {
    for range in KNOWN_UNISWAP_V2_POOL_RANGES {
        println!(
            "{} token={:#x} pool={:#x} start={} end={} note={}",
            range.name, range.token, range.pool, range.start_block, range.end_block, range.note
        );
    }
}
