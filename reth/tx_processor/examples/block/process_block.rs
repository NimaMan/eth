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
        .filter(|tx| tx.processed.status != "1")
        .count();

    println!(
        "Block {} ({} txs, {} failed)",
        block.header.number,
        block.transactions.len(),
        failed,
    );

    if let Some(first) = block.transactions.first() {
        println!("  First transaction: {:?}", first.metadata.hash);
    }
}
