use std::time::Instant;

use eyre::Result;
use rand::{seq::SliceRandom, thread_rng};
use tx_processor::{ProcessedBlock, ProcessedTxProvider};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let datadir = std::env::var("RETH_DATADIR")?;
    let provider = ProcessedTxProvider::new(&datadir)?;

    let cli_blocks: Vec<u64> = std::env::args()
        .skip(1)
        .map(|arg| arg.parse::<u64>())
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if cli_blocks.is_empty() {
        let latest = provider.get_latest_block().await?;
        let start = latest.saturating_sub(49);
        let mut sample: Vec<u64> = (start..=latest).collect();
        let mut rng = thread_rng();
        sample.shuffle(&mut rng);
        sample.truncate(5);
        sample.sort_unstable();

        println!(
            "No block numbers provided. Sampling {} blocks between {} and {}.",
            sample.len(),
            start,
            latest
        );
        process_blocks(&provider, sample).await?;
    } else if cli_blocks.len() == 1 {
        let block_number = cli_blocks[0];
        let started = Instant::now();
        let processed = provider.process_block(block_number).await?;
        print_block(&processed);
        println!("Elapsed: {:.3?}", started.elapsed());
    } else {
        let mut blocks = cli_blocks;
        blocks.sort_unstable();
        process_blocks(&provider, blocks).await?;
    }

    Ok(())
}

async fn process_blocks(provider: &ProcessedTxProvider, blocks: Vec<u64>) -> Result<()> {
    println!("Processing blocks: {:?}", blocks);
    let started = Instant::now();
    let results = provider.process_block_batch(blocks.clone(), None).await?;
    let total_elapsed = started.elapsed();

    for block in results {
        print_block(&block);
    }

    println!(
        "Processed {} blocks in {:.3?} (avg {:.3?} per block)",
        blocks.len(),
        total_elapsed,
        total_elapsed / blocks.len() as u32,
    );

    Ok(())
}

fn print_block(block: &ProcessedBlock) {
    let failed = block
        .transactions
        .iter()
        .filter(|tx| !tx.processed.status)
        .count();

    println!(
        "Block {} -> txs: {}, failed: {}",
        block.header.number,
        block.transactions.len(),
        failed,
    );
}
