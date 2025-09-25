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

    let token_arg = args.next().expect("Token address required");
    let token = Address::from_str(&token_arg)?;

    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    let latest = provider.get_latest_block()?;

    let start_block = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let end_block = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(latest);

    println!("Scanning token contract 0x{}", token);
    println!("Block range: [{} ..= {}]", start_block, end_block);
    println!("Latest block in DB: {}", latest);

    let blocks = provider.get_account_history_blocks_for_address(token, start_block, end_block)?;

    println!(
        "\nFound {} blocks with token state changes.",
        blocks.len()
    );
    if !blocks.is_empty() {
        println!("First 20 blocks: {:?}", blocks.iter().take(20).collect::<Vec<_>>());
    }

    Ok(())
}
