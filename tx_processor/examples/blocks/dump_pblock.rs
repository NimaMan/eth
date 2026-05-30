use std::fs;
use std::path::PathBuf;
use tx_processor::processed_tx_provider::block::disk_cache::store::{
    cache_file_name, ProcessedBlockDiskCacheEntry,
};

fn main() -> eyre::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <cache-root> <block-number>", args[0]);
        std::process::exit(1);
    }
    let cache_root = PathBuf::from(&args[1]);
    let block_number: u64 = args[2].parse()?;

    let path = cache_root.join(cache_file_name(block_number));
    let bytes = fs::read(&path)?;
    let decoded = zstd::stream::decode_all(bytes.as_slice())?;
    let entry: ProcessedBlockDiskCacheEntry = bincode::deserialize(&decoded)?;
    let block = entry.into_processed_block()?;

    println!("Block: {}", block.header.number);
    println!("Timestamp: {}", block.header.timestamp);
    println!("Tx count: {}", block.transactions.len());
    println!();

    for (i, tx) in block.transactions.iter().enumerate() {
        let p = &tx.processed;
        let hash = format!("{}", p.hash);
        let from_addr = format!("{}", p.from_address);
        let to_addr = p.to_address.map(|a| format!("{}", a)).unwrap_or_default();
        let contract = p
            .contract_address
            .map(|a| format!("{}", a))
            .unwrap_or_default();

        let v2_swaps = p.uniswap_v2_swaps.len();
        let v3_swaps = p.uniswap_v3_swaps.len();
        let v4_swaps = p.uniswap_v4_swaps.len();
        let transfers = p.erc20_transfers.len();
        let approvals = p.erc20_approval_events.len();
        let internal_erc20 = p.internal_erc20_calls.len();

        println!(
            "[{}] hash={} from={} to={} contract={} v2_swaps={} v3_swaps={} v4_swaps={} transfers={} approvals={} internal_erc20={}",
            i, hash, from_addr, to_addr, contract, v2_swaps, v3_swaps, v4_swaps, transfers, approvals, internal_erc20
        );
        for c in &p.internal_erc20_calls {
            println!(
                "      internal_erc20: {} token={} from={} to={} amount={}",
                c.kind.as_str(), c.token_address, c.from_address, c.to_address, c.amount
            );
        }
    }

    Ok(())
}
