use std::{env, path::PathBuf};

use eth_token_store::{EthConfigFile, TokenPnlReader, TokenPnlStoreConfig};
use eyre::{bail, eyre, Result};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse()?;
    let config = load_config(args.config_path)?;
    let store_config = TokenPnlStoreConfig::from_eth_config(&config)?;

    let pool = PgPool::connect(&store_config.database_url).await?;
    let reader = TokenPnlReader::from_pool(pool);

    let state = reader
        .pool_state(&args.run_id, &args.pool_id)
        .await?
        .ok_or_else(|| eyre!("no pnl state for run={} pool={}", args.run_id, args.pool_id))?;
    println!("Pool PnL");
    println!("run_id:        {}", state.run_id);
    println!("pool_id:       {}", state.pool_id);
    println!("token:         {}", state.token_address);
    println!("denom:         {}", state.denom_address);
    println!(
        "protocol:      {}",
        state.protocol.unwrap_or_else(|| "-".to_string())
    );
    println!("tx_count:      {}", state.tx_count);
    println!("latest_block:  {:?}", state.latest_block);
    println!(
        "token_raw:     in={} out={}",
        state.token_in_raw, state.token_out_raw
    );
    println!(
        "denom_raw:     in={} out={}",
        state.denom_in_raw, state.denom_out_raw
    );

    println!();
    println!("Top addresses by denom cashflow");
    for row in reader
        .top_addresses_by_denom_cashflow(&args.run_id, &args.pool_id, args.limit)
        .await?
    {
        println!(
            "address={} denom_cashflow={} token_balance={} movements={}",
            row.address, row.denom_cashflow_raw, row.token_balance_raw, row.movement_count
        );
    }

    Ok(())
}

#[derive(Debug)]
struct Args {
    config_path: Option<PathBuf>,
    run_id: String,
    pool_id: String,
    limit: i64,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut config_path = None;
        let mut run_id = None;
        let mut pool_id = None;
        let mut limit = 10_i64;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" => config_path = Some(PathBuf::from(next_arg("--config", &mut args)?)),
                "--run-id" => run_id = Some(next_arg("--run-id", &mut args)?),
                "--pool" => pool_id = Some(next_arg("--pool", &mut args)?.to_ascii_lowercase()),
                "--limit" => {
                    limit = next_arg("--limit", &mut args)?
                        .parse()
                        .map_err(|e| eyre!("invalid --limit: {e}"))?;
                }
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
            pool_id: pool_id.ok_or_else(|| eyre!("missing --pool"))?,
            limit,
        })
    }
}

fn load_config(path: Option<PathBuf>) -> Result<EthConfigFile> {
    Ok(match path {
        Some(path) => EthConfigFile::load(path)?,
        None => EthConfigFile::load_default()?,
    })
}

fn next_arg(name: &str, args: &mut impl Iterator<Item = String>) -> Result<String> {
    args.next().ok_or_else(|| eyre!("{name} needs a value"))
}

fn print_usage() {
    println!(
        "Usage:\n  cargo run -p eth_token_store --example inspect_pool_pnl_db -- \\\n    --run-id <run_id> --pool <pool_id> [--config /path/to/config.toml] [--limit 10]"
    );
}
