use clap::Parser;
use eyre::{eyre, Result};
use reth_chain_query::{common_addresses::DENOM_ADDRESSES, RethQueryProvider};
use std::sync::Arc;

#[derive(Debug, Parser)]
#[command(
    name = "denom_token_metadata",
    about = "Fetch metadata for all denom tokens tracked in reth_chain_query"
)]
struct Args {
    /// Optional block number to query against (defaults to latest)
    #[arg(long)]
    block: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

    let provider = Arc::new(RethQueryProvider::new(&datadir)?);
    let block = args.block;

    let rt = tokio::runtime::Runtime::new()?;
    let task_provider = Arc::clone(&provider);
    let task = async move {
        println!(
            "Fetching denom token metadata ({} entries)...",
            DENOM_ADDRESSES.len()
        );
        for (address, symbol) in DENOM_ADDRESSES.iter() {
            let meta = task_provider
                .get_token_metadata(*address, block, None)
                .await
                .map_err(|err| eyre!("metadata fetch failed for {symbol} ({address:?}): {err}"))?;

            match meta {
                Some(token) => {
                    println!(
                        "{} ({address:?}): name={}, symbol={}, decimals={}, total_supply={}",
                        symbol, token.name, token.symbol, token.decimals, token.total_supply
                    );
                }
                None => {
                    println!(
                        "{} ({address:?}): no metadata returned (not ERC-20?)",
                        symbol
                    );
                }
            }
        }
        Ok::<(), eyre::Report>(())
    };
    rt.block_on(task)?;
    drop(rt);
    drop(provider);

    Ok(())
}
