use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tx_processor::processed_tx_provider::block::disk_cache::store::{
    cache_file_name, ProcessedBlockDiskCacheEntry,
};
use tx_processor::{address_participations_from_processed_block, BlockProcessor, ProcessedBlock};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(cache_root) = args.next().map(PathBuf::from) else {
        eprintln!("Usage: address_participation_stats <cache-root> <block> [block...]");
        std::process::exit(1);
    };
    let blocks = args
        .map(|value| value.parse::<u64>())
        .collect::<Result<Vec<_>, _>>()?;
    if blocks.is_empty() {
        eprintln!("Usage: address_participation_stats <cache-root> <block> [block...]");
        std::process::exit(1);
    }

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/storage/samsung8tb/ethereum/reth".to_string());
    let provider = Arc::new(reth_chain_query::RethQueryProvider::new(&reth_datadir)?);
    let processor = BlockProcessor::new(provider);

    let mut total_blocks = 0u64;
    let mut total_txs = 0u64;
    let mut total_per_tx_addresses = 0u64;
    let mut total_unique_block_addresses = 0u64;

    for block_number in blocks {
        let (block, source) = load_block(&cache_root, block_number, &processor).await?;
        let participations = address_participations_from_processed_block(&block);
        let per_tx_addresses = participations
            .iter()
            .map(|participation| participation.addresses.len())
            .sum::<usize>();
        let unique_block_addresses = participations
            .iter()
            .flat_map(|participation| participation.addresses.iter().copied())
            .collect::<BTreeSet<_>>()
            .len();

        total_blocks += 1;
        total_txs += block.transactions.len() as u64;
        total_per_tx_addresses += per_tx_addresses as u64;
        total_unique_block_addresses += unique_block_addresses as u64;

        println!(
            "block={} source={} txs={} per_tx_addresses={} unique_block_addresses={} avg_per_tx_addresses={:.2}",
            block.header.number,
            source,
            block.transactions.len(),
            per_tx_addresses,
            unique_block_addresses,
            per_tx_addresses as f64 / block.transactions.len().max(1) as f64
        );
    }

    println!(
        "summary blocks={} txs={} per_tx_addresses={} unique_block_addresses={} avg_unique_block_addresses={:.2} avg_per_tx_addresses={:.2}",
        total_blocks,
        total_txs,
        total_per_tx_addresses,
        total_unique_block_addresses,
        total_unique_block_addresses as f64 / total_blocks.max(1) as f64,
        total_per_tx_addresses as f64 / total_txs.max(1) as f64
    );

    Ok(())
}

async fn load_block(
    cache_root: &Path,
    block_number: u64,
    processor: &BlockProcessor,
) -> eyre::Result<(ProcessedBlock, &'static str)> {
    for path in cache_paths(cache_root, block_number) {
        if !path.exists() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(decoded) = zstd::stream::decode_all(bytes.as_slice()) else {
            continue;
        };
        let Ok(entry) = bincode::deserialize::<ProcessedBlockDiskCacheEntry>(&decoded) else {
            continue;
        };
        let Ok(block) = entry.into_processed_block() else {
            continue;
        };
        return Ok((block, "cache"));
    }

    let block = processor.process_block(block_number).await?;
    Ok((block, "reth"))
}

fn cache_paths(cache_root: &Path, block_number: u64) -> Vec<PathBuf> {
    ["", ".v2", ".v1"]
        .into_iter()
        .map(|version| {
            if version.is_empty() {
                cache_root.join(cache_file_name(block_number))
            } else {
                cache_root.join(format!("{block_number}{version}.pblock.zst"))
            }
        })
        .chain(std::iter::once(
            cache_root.join(format!("{block_number}.pblock.zst")),
        ))
        .collect()
}
