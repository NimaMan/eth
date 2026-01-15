use eth_env::data::PriceFeeds;
use eth_env::{telemetry, Eth15mConfig};
use std::fs;
use std::path::PathBuf;

const SNAPSHOT_COUNT: u64 = 100;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    telemetry::init_tracing();

    let config = Eth15mConfig::default();
    let feeds = PriceFeeds::new(&config.reth_datadir)?;

    let latest_block = feeds.latest_block_number()?;
    println!(
        "Collecting {} snapshots ending at block {}",
        SNAPSHOT_COUNT, latest_block
    );

    let mut snapshots = Vec::with_capacity(SNAPSHOT_COUNT as usize);
    for offset in 0..SNAPSHOT_COUNT {
        let block_number = latest_block.saturating_sub(offset);
        let snapshot = feeds.fetch_snapshot_at_block(block_number).await?;
        println!(
            "block={} chainlink=${:.2} usdc_sources={}",
            snapshot.block_number,
            snapshot.chainlink_price.price_as_f64(),
            snapshot.eth_usdc_prices.len()
        );
        snapshots.push(snapshot);
    }

    snapshots.reverse();

    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/eth15m/data");
    fs::create_dir_all(&data_dir)?;
    let output = data_dir.join("eth15m_last_100_blocks.json");
    let payload = serde_json::to_vec_pretty(&snapshots)?;
    fs::write(&output, payload)?;

    println!("Saved {} snapshots to {}", SNAPSHOT_COUNT, output.display());

    Ok(())
}
