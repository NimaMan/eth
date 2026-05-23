use std::fs;
use std::path::PathBuf;
use tx_processor::processed_tx_provider::block::disk_cache::store::ProcessedBlockDiskCacheEntry;

fn main() -> eyre::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <cache-root> <block-number> <token-address>",
            args[0]
        );
        std::process::exit(1);
    }
    let cache_root = PathBuf::from(&args[1]);
    let block_number: u64 = args[2].parse()?;
    let target_token = args[3].to_lowercase();
    let weth = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

    let path = cache_root.join(format!("{}.pblock.zst", block_number));
    let bytes = fs::read(&path)?;
    let decoded = zstd::stream::decode_all(bytes.as_slice())?;
    let entry: ProcessedBlockDiskCacheEntry = bincode::deserialize(&decoded)?;
    let block = entry.into_processed_block()?;

    println!("Block: {}", block.header.number);
    println!("Target token: {}", target_token);
    println!();

    for (i, tx) in block.transactions.iter().enumerate() {
        let p = &tx.processed;
        let mut matched = false;

        for t in &p.erc20_transfers {
            if format!("{}", t.token_address).to_lowercase() == target_token {
                matched = true;
            }
        }

        for a in &p.erc20_approval_events {
            if format!("{}", a.token_address).to_lowercase() == target_token {
                matched = true;
            }
        }

        if !matched {
            continue;
        }

        let hash = format!("{}", p.hash);
        let from_addr = format!("{}", p.from_address);
        let to_addr = p.to_address.map(|a| format!("{}", a)).unwrap_or_default();

        println!("[{}] hash={} from={} to={}", i, hash, from_addr, to_addr);
        println!(
            "  transfers={} approvals={} v2_swaps={} v3_swaps={} v4_swaps={}",
            p.erc20_transfers.len(),
            p.erc20_approval_events.len(),
            p.uniswap_v2_swaps.len(),
            p.uniswap_v3_swaps.len(),
            p.uniswap_v4_swaps.len()
        );

        for swap in &p.uniswap_v2_swaps {
            println!(
                "  V2_SWAP: pair={} sender={} to={} amount0_in={} amount1_in={} amount0_out={} amount1_out={} log_index={}",
                format!("{}", swap.pair_address),
                format!("{}", swap.sender),
                format!("{}", swap.to),
                swap.amount0_in,
                swap.amount1_in,
                swap.amount0_out,
                swap.amount1_out,
                swap.log_index
            );
        }

        for t in &p.erc20_transfers {
            let token_address = format!("{}", t.token_address).to_lowercase();
            if token_address == target_token || token_address == weth {
                println!(
                    "  TRANSFER: token={} from={} to={} amount={}",
                    format!("{}", t.token_address),
                    format!("{}", t.from_address),
                    format!("{}", t.to_address),
                    t.amount
                );
            }
        }

        for (address, changes) in &p.address_balance_changes {
            if !changes.currency_net.is_empty()
                || changes
                    .token_net
                    .keys()
                    .any(|token| token.to_lowercase() == target_token)
            {
                println!(
                    "  BALANCE: address={} currency_net={:?} token_net={:?}",
                    format!("{}", address),
                    changes.currency_net,
                    changes.token_net
                );
            }
        }

        for a in &p.erc20_approval_events {
            if format!("{}", a.token_address).to_lowercase() == target_token {
                println!(
                    "  APPROVAL: owner={} spender={} amount={}",
                    format!("{}", a.owner),
                    format!("{}", a.spender),
                    a.amount
                );
            }
        }

        println!();
    }

    Ok(())
}
