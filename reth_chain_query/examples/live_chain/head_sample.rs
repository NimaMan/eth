use std::time::Duration;

use clap::Parser;
use color_eyre::{eyre::eyre, Result};
use reth_chain_query::{
    live_chain::{
        beacon::BeaconRestClient, engine::ExecutionClient, lighthouse::LighthouseHeadListener,
        utils::transaction_hashes_from_block,
    },
    provider::RpcBlockDataFetcher,
};

#[derive(Debug, Parser)]
struct Args {
    #[arg(
        long,
        default_value = "http://127.0.0.1:5052",
        help = "Beacon REST base URL"
    )]
    beacon_api: String,

    #[arg(
        long,
        default_value = "http://127.0.0.1:8545",
        help = "Execution RPC endpoint"
    )]
    execution_rpc: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let beacon = BeaconRestClient::new(&args.beacon_api)?;
    let events_url = beacon.sse_endpoint();
    let mut listener = LighthouseHeadListener::connect(&events_url).await?;
    let execution = ExecutionClient::new(
        &args.execution_rpc,
        Duration::from_millis(200),
        Duration::from_secs(10),
    )?;

    println!("Waiting for next head event via {}", events_url);
    let observation = listener.next_observation().await?;
    println!(
        "Received slot {} at {} (root={:?})",
        observation.slot, observation.arrival_time, observation.block_root
    );

    let exec_info = beacon.fetch_execution_info(observation.block_root).await?;
    println!(
        "Execution payload hash {:?}, number {}, timestamp {}",
        exec_info.block_hash, exec_info.block_number, exec_info.timestamp
    );

    match execution.wait_for_block(exec_info.block_hash).await? {
        Some(ready) => {
            println!(
                "Execution node served block {} with {} txs after {:.3}s",
                ready
                    .number
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "unknown".into()),
                ready.transaction_count,
                (ready.ready_at - observation.arrival_time)
                    .num_microseconds()
                    .unwrap_or(0) as f64
                    / 1_000_000f64
            );

            let fetcher = RpcBlockDataFetcher::new(&args.execution_rpc)?
                .with_debug_endpoint(&args.execution_rpc)?;
            let block = fetcher
                .fetch_block(exec_info.block_hash)
                .await?
                .ok_or_else(|| eyre!("block data missing for {:?}", exec_info.block_hash))?;
            println!(
                "Fetched block JSON with {} fields",
                block.as_object().map(|o| o.len()).unwrap_or(0)
            );

            let receipts = fetcher
                .fetch_receipts(exec_info.block_hash)
                .await?
                .ok_or_else(|| eyre!("receipts missing for {:?}", exec_info.block_hash))?;
            println!("Fetched {} receipts", receipts.len());

            let tx_hashes = transaction_hashes_from_block(&block)?;
            let traces = fetcher.trace_transactions(&tx_hashes).await?;
            println!("Fetched {} transaction traces", traces.len());
        }
        None => {
            println!("Execution node did not return the block within the timeout");
        }
    }

    Ok(())
}
