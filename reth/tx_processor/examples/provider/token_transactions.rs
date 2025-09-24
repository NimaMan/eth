use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::{address, Address};
use eyre::Result;
use tx_processor::{ProcessedTxProvider, TokenProcessedTxProvider};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let datadir = std::env::var("RETH_DATADIR")?;
    let core = Arc::new(ProcessedTxProvider::new(&datadir)?);
    let token_provider = TokenProcessedTxProvider::new(core.clone())?;

    // Default to USDC if no argument is provided.
    let token_address = std::env::args()
        .nth(1)
        .map(|arg| Address::from_str(&arg).expect("invalid address"))
        .unwrap_or(address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"));

    let latest_block = core.get_latest_block().await?;
    let start_block = latest_block.saturating_sub(999);

    println!(
        "Processing token {:?} over blocks {} -> {}",
        token_address, start_block, latest_block
    );

    token_provider
        .load_blocks_for_token(token_address, start_block, latest_block)
        .await?;

    let transactions = token_provider.transactions_for(token_address).await;

    println!(
        "Found {} processed transactions involving token {:?}",
        transactions.len(),
        token_address
    );

    for tx in transactions.iter().take(10) {
        println!(
            "- Block {} Tx {:?} type {}",
            tx.block_number, tx.hash, tx.txn_type
        );
    }

    Ok(())
}
