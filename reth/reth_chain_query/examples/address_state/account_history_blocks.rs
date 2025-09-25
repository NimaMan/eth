use alloy_primitives::Address;
/// Find blocks where an address' *account state* changed (balance, nonce, code)
///
/// Usage:
///   cargo run --example account_history_blocks [start_block] [end_block]
///
/// The lookup reads the `AccountsHistory` table, so it captures only blocks that
/// mutated the address' own account entry. Pure log-only/token events are not
/// included; fetch the blocks first, then use the processed transaction pipeline
/// to filter for richer participation.
use reth_chain_query::{Result, RethQueryProvider};
use reth_db::tables;
use reth_db::transaction::DbTx;
use reth_prune_types::PruneSegment;
use reth_stages_types::StageId;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    // Address to analyze
    let addr = Address::from_str("0x2348e8a3a21dbe64ace84853d7b4b696e8a1fc27")?;

    // Open provider against your local reth datadir
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;

    let latest = provider.get_latest_block()?;

    // Optional CLI args: [start_block] [end_block]
    let mut args = std::env::args().skip(1);
    let start_block: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let end_block: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(latest);

    // Print archive indexing and pruning status
    let pf = provider.provider_factory();
    let p = pf.provider()?;
    let tx = p.tx_ref();
    let stage_ah = tx.get::<tables::StageCheckpoints>(StageId::IndexAccountHistory.to_string())?;
    let stage_sh = tx.get::<tables::StageCheckpoints>(StageId::IndexStorageHistory.to_string())?;
    let prune_ah = tx.get::<tables::PruneCheckpoints>(PruneSegment::AccountHistory)?;
    let prune_sh = tx.get::<tables::PruneCheckpoints>(PruneSegment::StorageHistory)?;

    println!("Latest block in DB: {}", latest);
    println!("IndexAccountHistory stage: {:?}", stage_ah);
    println!("IndexStorageHistory stage: {:?}", stage_sh);
    println!("AccountHistory prune checkpoint: {:?}", prune_ah);
    println!("StorageHistory prune checkpoint: {:?}", prune_sh);

    println!(
        "\nScanning AccountsHistory [{}..={}] for address {:?} ...",
        start_block, end_block, addr
    );

    let blocks = provider.get_account_history_blocks_for_address(addr, start_block, end_block)?;

    println!(
        "\nFound {} blocks with at least one tx involving the address.",
        blocks.len()
    );
    if !blocks.is_empty() {
        println!(
            "First 20 blocks: {:?}",
            &blocks.iter().take(20).cloned().collect::<Vec<_>>()
        );
        println!("\nAll blocks ({} total):", blocks.len());
        for b in &blocks {
            println!("{}", b);
        }
    }

    Ok(())
}
