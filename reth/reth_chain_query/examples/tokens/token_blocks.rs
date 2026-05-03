use alloy_primitives::Address;
use reth_chain_query::{Result, RethQueryProvider};
use std::str::FromStr;

/// Token block discovery example
///
/// Lists block numbers where the given token contract touched state, which is a
/// practical starting point before loading full transactions with the
/// `TokenProcessedTxProvider`.
///
/// Run with: cargo run --example token_blocks [token_address] [start_block] [end_block]
#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let default_token = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    let token_arg = args.next().unwrap_or_else(|| default_token.to_string());
    let used_default = token_arg.eq_ignore_ascii_case(default_token);
    let token = Address::from_str(&token_arg)?;

    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let provider = RethQueryProvider::new(&reth_datadir)?;
    let latest = provider.get_latest_block()?;

    let start_block = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let end_block = args.next().and_then(|s| s.parse().ok()).unwrap_or(latest);

    if used_default {
        println!("No token provided on CLI; defaulting to USDC ({default_token}).");
    }

    println!("Scanning token contract 0x{}", token);
    println!("Block range: [{} ..= {}]", start_block, end_block);
    println!("Latest block in DB: {}", latest);

    let blocks = provider
        .get_address_account_history_blocks(token, start_block, end_block)
        .await?;

    println!("\nFound {} blocks with token state changes.", blocks.len());
    if !blocks.is_empty() {
        println!(
            "First 20 blocks: {:?}",
            blocks.iter().take(20).collect::<Vec<_>>()
        );
    }

    Ok(())
}
