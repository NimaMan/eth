use std::collections::BTreeSet;
use std::sync::Arc;

use eyre::Result;
use tx_processor::{BlockBatchOptions, BlockProcessor};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    let provider = Arc::new(reth_chain_query::RethQueryProvider::new(&datadir)?);
    let processor = BlockProcessor::new(provider.clone());

    let requested_blocks: Vec<u64> = std::env::args()
        .skip(1)
        .map(|arg| arg.parse::<u64>())
        .collect::<Result<_, _>>()?;

    if requested_blocks.len() <= 1 {
        let block_number = requested_blocks
            .first()
            .copied()
            .unwrap_or(provider.get_latest_block()?);

        let processed_block = processor.process_block(block_number).await?;
        report_block(&processed_block);
    } else {
        let processed_blocks = processor
            .process_block_batch(requested_blocks.clone(), BlockBatchOptions::default())
            .await?;

        for block in processed_blocks {
            report_block(&block);
        }
    }

    Ok(())
}

fn report_block(block: &tx_processor::ProcessedBlock) {
    let failed = block
        .transactions
        .iter()
        .filter(|tx| !tx.processed.status)
        .count();

    println!(
        "Block {} ({} txs, {} failed)",
        block.header.number,
        block.transactions.len(),
        failed,
    );

    let mut v2_pair_created = 0usize;
    let mut v3_pool_created = 0usize;
    let mut v4_initialized = 0usize;
    let mut v2_pools = BTreeSet::new();
    let mut v3_pools = BTreeSet::new();
    let mut v4_pools = BTreeSet::new();
    for tx in &block.transactions {
        v2_pair_created += tx.processed.uniswap_v2_pair_created_events.len();
        v3_pool_created += tx.processed.uniswap_v3_pools.len();
        v4_initialized += tx.processed.uniswap_v4_initializes.len();
        for event in &tx.processed.uniswap_v2_pair_created_events {
            v2_pools.insert(event.pair_address);
        }
        for event in &tx.processed.uniswap_v3_pools {
            v3_pools.insert(event.pool);
        }
        for event in &tx.processed.uniswap_v4_initializes {
            v4_pools.insert((event.pool_manager_address, event.event_id));
        }
    }
    println!(
        "  Pool creations: v2={} ({} unique), v3={} ({} unique), v4={} ({} unique)",
        v2_pair_created,
        v2_pools.len(),
        v3_pool_created,
        v3_pools.len(),
        v4_initialized,
        v4_pools.len(),
    );

    if let Some(first) = block.transactions.first() {
        println!("  First transaction: {:?}", first.metadata.hash);
    }
}
