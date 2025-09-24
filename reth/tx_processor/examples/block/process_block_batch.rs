use std::sync::Arc;
use std::time::{Duration, Instant};

use eyre::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use rand::{seq::SliceRandom, thread_rng};
use tx_processor::{BlockBatchOptions, BlockProcessor};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let datadir = std::env::var("RETH_DATADIR")?;
    let provider = Arc::new(reth_chain_query::RethQueryProvider::new(&datadir)?);
    let processor = BlockProcessor::new(provider.clone());

    let latest = provider.get_latest_block()?;
    let start = latest.saturating_sub(499);

    let mut candidates: Vec<u64> = (start..=latest).collect();
    let mut rng = thread_rng();
    candidates.shuffle(&mut rng);
    candidates.truncate(200);
    candidates.sort_unstable();

    println!(
        "Sampling 200 blocks out of [{}, {}] with default max_concurrency={}",
        start,
        latest,
        BlockBatchOptions::default().max_concurrency,
    );
    println!("Selected blocks: {:?}", candidates);

    let options = BlockBatchOptions::default();
    let max_concurrency = options.max_concurrency;
    let include_traces = options.include_traces;

    let now = Instant::now();
    let mut processed_blocks = stream::iter(candidates.iter().copied().map(|block_number| {
        let processor = processor.clone();
        async move {
            let block_start = Instant::now();
            let processed_block = processor
                .process_block_with_options(block_number, include_traces)
                .await?;
            let duration = block_start.elapsed();
            Ok::<_, eyre::Report>((processed_block, duration))
        }
    }))
    .buffer_unordered(max_concurrency)
    .try_collect::<Vec<_>>()
    .await?;
    let elapsed = now.elapsed();

    processed_blocks.sort_by_key(|(block, _)| block.header.number);

    let durations: Vec<Duration> = processed_blocks.iter().map(|(_, d)| *d).collect();
    let median = median_duration(durations);

    for (block, duration) in &processed_blocks {
        let failed = block
            .transactions
            .iter()
            .filter(|tx| tx.processed.status != "1")
            .count();
        println!(
            "Block {} -> txs: {}, failed: {}, duration: {:.3?}",
            block.header.number,
            block.transactions.len(),
            failed,
            duration,
        );
    }

    println!(
        "Processed {} blocks in {:.3?} (avg {:.3?} per block, median {:.3?})",
        processed_blocks.len(),
        elapsed,
        elapsed / processed_blocks.len() as u32,
        median,
    );

    Ok(())
}

fn median_duration(mut durations: Vec<Duration>) -> Duration {
    if durations.is_empty() {
        return Duration::from_secs(0);
    }

    durations.sort();
    let mid = durations.len() / 2;

    if durations.len() % 2 == 1 {
        durations[mid]
    } else {
        let lower = durations[mid - 1].as_nanos();
        let upper = durations[mid].as_nanos();
        let avg = (lower + upper) / 2;
        Duration::from_nanos(avg as u64)
    }
}
