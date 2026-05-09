use eyre::{eyre, Result};
use reth_chain_query::reth_index::RethIndexDB;
use reth_chain_query::{RethQueryProvider, TransactionData};
use std::collections::HashSet;

#[derive(Debug)]
struct Args {
    datadir: String,
    index_dir: String,
    start_block: Option<u64>,
    end_block: Option<u64>,
}

#[derive(Debug, Clone)]
struct BlockCoverage {
    block_number: u64,
    tx_count: u64,
    seen_count: u64,
}

#[derive(Debug, Default)]
struct CoverageStats {
    blocks_scanned: u64,
    block_errors: u64,
    mined_txs: u64,
    seen_txs: u64,
    first_block: Option<BlockCoverage>,
    last_block: Option<BlockCoverage>,
    worst_block: Option<BlockCoverage>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let arrival_db = RethIndexDB::open_read_only(&args.index_dir)?;
    let mut arrival_entries = arrival_db.tx_arrival_entries()?;
    arrival_entries.sort_by_key(|(tx_number, _)| *tx_number);

    if arrival_entries.is_empty() {
        println!("No mempool arrival entries found in {}", args.index_dir);
        return Ok(());
    }

    let provider = RethQueryProvider::new(&args.datadir)?;
    let Some((first_idx, first_arrival_tx_number, first_tx)) =
        find_first_mapped_arrival(&provider, &arrival_entries).await?
    else {
        return Err(eyre!(
            "arrival DB has {} rows, but none mapped to a transaction in the Reth DB",
            arrival_entries.len()
        ));
    };
    let Some((last_idx, last_arrival_tx_number, last_tx)) =
        find_last_mapped_arrival(&provider, &arrival_entries).await?
    else {
        return Err(eyre!("failed to resolve the last mapped arrival"));
    };

    let start_block = args.start_block.unwrap_or(first_tx.block_number);
    let end_block = args.end_block.unwrap_or(last_tx.block_number);
    if start_block > end_block {
        return Err(eyre!(
            "invalid block range: start_block={} > end_block={}",
            start_block,
            end_block
        ));
    }

    let arrival_tx_numbers: HashSet<u64> = arrival_entries
        .iter()
        .map(|(tx_number, _)| *tx_number)
        .collect();
    let stats = measure_coverage(&provider, &arrival_tx_numbers, start_block, end_block)?;

    let skipped_front = first_idx;
    let skipped_back = arrival_entries.len().saturating_sub(last_idx + 1);
    let missed_txs = stats.mined_txs.saturating_sub(stats.seen_txs);

    println!("Mempool arrival coverage");
    println!("  Reth datadir: {}", args.datadir);
    println!("  Arrival index: {}", args.index_dir);
    println!("  Arrival rows: {}", arrival_entries.len());
    println!(
        "  Mapped arrival tx_number range: {}..{}",
        first_arrival_tx_number, last_arrival_tx_number
    );
    println!(
        "  Mapped arrival block range: {}..{}",
        first_tx.block_number, last_tx.block_number
    );
    if skipped_front > 0 || skipped_back > 0 {
        println!(
            "  Unmapped boundary rows skipped: front={} back={}",
            skipped_front, skipped_back
        );
    }
    println!("  Scanned block range: {}..{}", start_block, end_block);
    println!("  Blocks scanned: {}", stats.blocks_scanned);
    println!("  Block errors: {}", stats.block_errors);
    println!("  Mined txs: {}", stats.mined_txs);
    println!("  Seen in public mempool: {}", stats.seen_txs);
    println!("  Missing from public mempool: {}", missed_txs);
    println!("  Coverage: {:.2}%", pct(stats.seen_txs, stats.mined_txs));

    if let Some(block) = stats.first_block.as_ref() {
        print_block("First scanned block", block);
    }
    if let Some(block) = stats.last_block.as_ref() {
        print_block("Last scanned block", block);
    }
    if let Some(block) = stats.worst_block.as_ref() {
        print_block("Worst non-empty block", block);
    }

    println!();
    println!("Note: misses include private/builder-only transactions and any arrivals lost before the recorder resolved mined tx_numbers.");

    Ok(())
}

fn measure_coverage(
    provider: &RethQueryProvider,
    arrival_tx_numbers: &HashSet<u64>,
    start_block: u64,
    end_block: u64,
) -> Result<CoverageStats> {
    let mut stats = CoverageStats::default();

    for block_number in start_block..=end_block {
        match provider.get_block_tx_indices(block_number) {
            Ok(indices) => {
                let first_tx_number = indices.first_tx_num;
                let tx_count = indices.tx_count;
                let end_tx_number = first_tx_number.saturating_add(tx_count);
                let seen_count = (first_tx_number..end_tx_number)
                    .filter(|tx_number| arrival_tx_numbers.contains(tx_number))
                    .count() as u64;

                let block = BlockCoverage {
                    block_number,
                    tx_count,
                    seen_count,
                };

                if stats.first_block.is_none() {
                    stats.first_block = Some(block.clone());
                }
                stats.last_block = Some(block.clone());
                if tx_count > 0
                    && stats
                        .worst_block
                        .as_ref()
                        .map(|worst| {
                            pct(block.seen_count, block.tx_count)
                                < pct(worst.seen_count, worst.tx_count)
                        })
                        .unwrap_or(true)
                {
                    stats.worst_block = Some(block);
                }

                stats.blocks_scanned += 1;
                stats.mined_txs += tx_count;
                stats.seen_txs += seen_count;
            }
            Err(error) => {
                stats.block_errors += 1;
                eprintln!(
                    "failed to read block {} tx indices: {}",
                    block_number, error
                );
            }
        }
    }

    Ok(stats)
}

async fn find_first_mapped_arrival(
    provider: &RethQueryProvider,
    arrival_entries: &[(u64, u64)],
) -> Result<Option<(usize, u64, TransactionData)>> {
    for (idx, (tx_number, _)) in arrival_entries.iter().copied().enumerate() {
        if let Ok(tx) = provider.get_transaction_by_number(tx_number).await {
            return Ok(Some((idx, tx_number, tx)));
        }
    }
    Ok(None)
}

async fn find_last_mapped_arrival(
    provider: &RethQueryProvider,
    arrival_entries: &[(u64, u64)],
) -> Result<Option<(usize, u64, TransactionData)>> {
    for (idx, (tx_number, _)) in arrival_entries.iter().copied().enumerate().rev() {
        if let Ok(tx) = provider.get_transaction_by_number(tx_number).await {
            return Ok(Some((idx, tx_number, tx)));
        }
    }
    Ok(None)
}

fn print_block(label: &str, block: &BlockCoverage) {
    println!(
        "  {}: #{} {}/{} ({:.2}%)",
        label,
        block.block_number,
        block.seen_count,
        block.tx_count,
        pct(block.seen_count, block.tx_count)
    );
}

fn pct(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 * 100.0 / denominator as f64
    }
}

fn parse_args() -> Result<Args> {
    let mut datadir: Option<String> = None;
    let mut index_dir: Option<String> = None;
    let mut start_block: Option<u64> = None;
    let mut end_block: Option<u64> = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--datadir" => datadir = Some(next_value(&mut args, "--datadir")?),
            "--index-dir" => index_dir = Some(next_value(&mut args, "--index-dir")?),
            "--start-block" => start_block = Some(next_value(&mut args, "--start-block")?.parse()?),
            "--end-block" => end_block = Some(next_value(&mut args, "--end-block")?.parse()?),
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(eyre!("unknown argument: {}", other)),
        }
    }

    let datadir = match datadir {
        Some(path) => path,
        None => tx_simulator::config::repo::reth_datadir()?,
    };
    let index_dir =
        index_dir.unwrap_or_else(|| format!("{}/reth_index", datadir.trim_end_matches('/')));

    Ok(Args {
        datadir,
        index_dir,
        start_block,
        end_block,
    })
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String> {
    args.next()
        .ok_or_else(|| eyre!("{} requires a value", name))
}

fn print_usage() {
    println!("Usage:");
    println!("  cargo run -p reth_chain_query --example tx_arrival_coverage -- \\");
    println!("    [--datadir <reth_datadir>] [--index-dir <reth_index_dir>] \\");
    println!("    [--start-block <block>] [--end-block <block>]");
    println!();
    println!("Defaults:");
    println!("  --datadir uses the configured repo RETH_DATADIR");
    println!("  --index-dir defaults to <reth_datadir>/reth_index");
}
