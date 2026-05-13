use alloy_primitives::{Address, Bytes, U256};
use eyre::{bail, eyre, Result};
use std::{env, process, str::FromStr};
use tx_simulator::{
    config::repo,
    contract_simulation::{
        decode_string_from_contract_output, decode_uint256_from_contract_output,
        decode_uint8_from_contract_output, encode_contract_read_call_no_args,
    },
    LiveChainCache, LiveChainCacheBuilder, TxSimulator,
};

const DEFAULT_TOKEN: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"; // USDC

const TOTAL_SUPPLY_SELECTOR: [u8; 4] = [0x18, 0x16, 0x0d, 0xdd];
const DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
const SYMBOL_SELECTOR: [u8; 4] = [0x95, 0xd8, 0x9b, 0x41];
const NAME_SELECTOR: [u8; 4] = [0x06, 0xfd, 0xde, 0x03];

#[derive(Debug)]
struct Config {
    reth_datadir: String,
    redis_url: String,
    window: u64,
    token: Address,
}

#[derive(Debug)]
struct TokenMetadata {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: U256,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::from_args()?;

    println!("Live block replay window diagnostic (Rust)");
    println!("Redis URL: {}", cfg.redis_url);
    println!("Reth data dir: {}", cfg.reth_datadir);
    println!("Token address: {:#x}", cfg.token);

    let cache = LiveChainCacheBuilder::new(&cfg.redis_url).build()?;
    let simulator = TxSimulator::new(&cfg.reth_datadir)?.with_live_chain_cache(cache);
    let cache = simulator
        .live_chain_cache()
        .ok_or_else(|| eyre!("live chain cache unavailable on simulator"))?;

    let Some(latest_block) = cache.latest_block_number().await? else {
        println!("No latest block recorded in Redis.");
        return Ok(());
    };
    println!("Latest block per Redis: {}", latest_block);

    let blocks = summarize_window(cache.as_ref(), latest_block, cfg.window).await?;
    if blocks.is_empty() {
        println!("Live cache did not yield any blocks.");
        return Ok(());
    }
    if blocks.len() < cfg.window as usize {
        println!(
            "⚠️ Requested window was {}, but only {} blocks were available.",
            cfg.window,
            blocks.len()
        );
    }

    let target_block = *blocks.last().expect("blocks not empty");
    println!(
        "\nReplaying block {} ({} most recent in window)...",
        target_block,
        ordinal_label(cfg.window)
    );

    let persisted = simulator.get_latest_block()?;
    println!("Latest persisted MDBX block: {}", persisted);
    if target_block > persisted {
        println!(
            "Block {} is ahead of MDBX by {} blocks.",
            target_block,
            target_block - persisted
        );
    }

    let resolved_header = simulator
        .block_context_loader()
        .load_block_header(target_block, None)
        .await?;
    println!(
        "BlockContextLoader resolved header hash {:#x} @ timestamp {}",
        resolved_header.hash(),
        resolved_header.timestamp
    );
    if target_block > persisted {
        println!("State is not yet in MDBX; live data replay is required for downstream calls.");
    } else {
        println!("Block {} already persisted locally.", target_block);
    }

    match fetch_token_metadata(&simulator, cfg.token, target_block).await {
        Ok(Some(metadata)) => {
            println!("\nToken metadata at block {}:", target_block);
            println!("  name: {}", metadata.name);
            println!("  symbol: {}", metadata.symbol);
            println!("  decimals: {}", metadata.decimals);
            println!("  total_supply (raw): {}", metadata.total_supply);
        }
        Ok(None) => {
            println!("\nToken metadata lookup returned None (token missing or state unavailable).");
        }
        Err(err) => {
            println!("\nMetadata lookup failed: {err}");
        }
    }

    Ok(())
}

async fn summarize_window(cache: &LiveChainCache, latest: u64, window: u64) -> Result<Vec<u64>> {
    if window == 0 {
        bail!("window size must be at least 1");
    }
    let mut blocks = Vec::with_capacity(window as usize);
    for offset in 0..window {
        if offset > latest {
            break;
        }
        let block = latest - offset;
        if block == 0 {
            break;
        }
        let header = cache.fetch_block_header(block).await?;
        let processed = cache.fetch_processed_block(block).await?;
        let tx_count = processed.as_ref().map(|entries| entries.len()).unwrap_or(0);
        println!(
            "Block {}: header={}, processed_tx={}, tx_count={}",
            block,
            yes_no(header.is_some()),
            yes_no(processed.is_some()),
            tx_count
        );
        blocks.push(block);
    }
    Ok(blocks)
}

async fn fetch_token_metadata(
    simulator: &TxSimulator,
    token: Address,
    block_number: u64,
) -> Result<Option<TokenMetadata>> {
    let total_supply = call_uint256(simulator, token, TOTAL_SUPPLY_SELECTOR, block_number).await?;
    let decimals = call_uint8(simulator, token, DECIMALS_SELECTOR, block_number).await?;
    let symbol = call_string(simulator, token, SYMBOL_SELECTOR, block_number).await?;
    let name = call_string(simulator, token, NAME_SELECTOR, block_number).await?;

    if total_supply.is_none() && decimals.is_none() && symbol.is_none() && name.is_none() {
        return Ok(None);
    }

    Ok(Some(TokenMetadata {
        name: name.unwrap_or_default(),
        symbol: symbol.unwrap_or_default(),
        decimals: decimals.unwrap_or_default(),
        total_supply: total_supply.unwrap_or(U256::ZERO),
    }))
}

async fn call_view_function(
    simulator: &TxSimulator,
    token: Address,
    selector: [u8; 4],
    block_number: u64,
) -> Result<Option<Bytes>> {
    let data = encode_contract_read_call_no_args(selector);
    let result = simulator
        .simulate_view_function(token, data, Some(block_number))
        .await?;
    if result.success {
        Ok(Some(result.output))
    } else {
        Ok(None)
    }
}

async fn call_uint256(
    simulator: &TxSimulator,
    token: Address,
    selector: [u8; 4],
    block_number: u64,
) -> Result<Option<U256>> {
    let Some(bytes) = call_view_function(simulator, token, selector, block_number).await? else {
        return Ok(None);
    };
    Ok(Some(decode_uint256_from_contract_output(&bytes)))
}

async fn call_uint8(
    simulator: &TxSimulator,
    token: Address,
    selector: [u8; 4],
    block_number: u64,
) -> Result<Option<u8>> {
    let Some(bytes) = call_view_function(simulator, token, selector, block_number).await? else {
        return Ok(None);
    };
    Ok(Some(decode_uint8_from_contract_output(&bytes)))
}

async fn call_string(
    simulator: &TxSimulator,
    token: Address,
    selector: [u8; 4],
    block_number: u64,
) -> Result<Option<String>> {
    let Some(bytes) = call_view_function(simulator, token, selector, block_number).await? else {
        return Ok(None);
    };
    let value = decode_string_from_contract_output(&bytes);
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

impl Config {
    fn from_args() -> Result<Self> {
        let mut reth_datadir = repo::reth_datadir()?;
        let mut redis_url = repo::live_data_redis_url()?;
        let mut window = 3u64;
        let mut token = DEFAULT_TOKEN.to_string();

        let mut args = env::args().skip(1).peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--reth" => {
                    reth_datadir = args
                        .next()
                        .ok_or_else(|| eyre!("--reth requires a path argument"))?;
                }
                "--redis-url" => {
                    redis_url = args
                        .next()
                        .ok_or_else(|| eyre!("--redis-url requires a value"))?;
                }
                "--window" => {
                    let value = args
                        .next()
                        .ok_or_else(|| eyre!("--window requires a numeric value"))?;
                    window = value.parse()?;
                }
                "--token" => {
                    token = args
                        .next()
                        .ok_or_else(|| eyre!("--token requires an address"))?;
                }
                "-h" | "--help" => {
                    print_usage();
                    process::exit(0);
                }
                other => {
                    return Err(eyre!("unrecognized argument: {}", other));
                }
            }
        }

        if window == 0 {
            bail!("window size must be at least 1");
        }

        let token = Address::from_str(&token)?;

        Ok(Config {
            reth_datadir,
            redis_url,
            window,
            token,
        })
    }
}

fn yes_no(flag: bool) -> &'static str {
    if flag {
        "yes"
    } else {
        "no"
    }
}

fn ordinal_label(window: u64) -> String {
    match window {
        1 => "latest".to_string(),
        2 => "second".to_string(),
        3 => "third".to_string(),
        4 => "fourth".to_string(),
        _ => format!("{}th", window),
    }
}

fn print_usage() {
    println!("Usage: cargo run --example live_block_replay_window -- [--reth <PATH>] [--redis-url <URL>] [--window <N>] [--token <ADDRESS>]");
    println!(
        "Defaults: reth datadir {}, redis {}, window 3, token USDC",
        repo::reth_datadir().unwrap_or_else(|_| repo::DEFAULT_RETH_DATADIR.to_string()),
        repo::live_data_redis_url()
            .unwrap_or_else(|_| repo::DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL.to_string())
    );
}
