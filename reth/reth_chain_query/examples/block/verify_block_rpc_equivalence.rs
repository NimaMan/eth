use reth_chain_query::{
    provider::{BlockDataFetcher, RpcBlockDataFetcher},
    Result, RethQueryProvider,
};
use std::{env, sync::Arc};

/// cargo run --example block/verify_block_rpc_equivalence -- <block_number?>
#[tokio::main]
async fn main() -> Result<()> {
    let datadir = match env::var("RETH_DATA_DIR") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => tx_simulator::config::repo::reth_datadir()?,
    };
    let rpc_url = env::var("EXECUTION_RPC").unwrap_or_else(|_| "http://127.0.0.1:8545".into());
    let args: Vec<String> = env::args().collect();

    let provider = Arc::new(RethQueryProvider::new(&datadir)?);
    let latest = provider.get_latest_block()?;
    let block_number = if args.len() > 1 {
        args[1].parse::<u64>().unwrap_or(latest)
    } else {
        latest
    };

    println!(
        "🔍 Comparing block {} via MDBX vs RPC (rpc={})",
        block_number, rpc_url
    );

    let fetcher =
        BlockDataFetcher::new(provider).with_rpc_fetcher(RpcBlockDataFetcher::new(&rpc_url)?);
    let db_block = fetcher.fetch_db_block(block_number).await?;
    let rpc_block = fetcher
        .fetch_rpc_block_by_hash(db_block.header.hash, block_number)
        .await?;

    compare_blocks(&db_block, &rpc_block)?;

    Ok(())
}

fn compare_blocks(
    db: &reth_chain_query::provider::RawBlockData,
    rpc: &reth_chain_query::provider::RawBlockData,
) -> Result<()> {
    use std::io::Write;

    let transactions_match =
        normalized_transactions(&db.transactions) == normalized_transactions(&rpc.transactions);

    if db.header == rpc.header
        && transactions_match
        && db.receipts == rpc.receipts
        && db.traces == rpc.traces
    {
        println!("✅ Blocks match after RPC normalization");
        return Ok(());
    }

    let log_path = env::var("BLOCK_FETCHER_DIFF_LOG").unwrap_or_else(|_| {
        "/home/nima/code/crypto/eth/logs/dev/block_fetcher_diffs.log".to_string()
    });
    if let Some(parent) = std::path::Path::new(&log_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    if db.header != rpc.header {
        println!(
            "❌ Header mismatch:\n  db: {:?}\n  rpc: {:?}",
            db.header, rpc.header
        );
        writeln!(
            log_file,
            "[header] block={} db_hash={:#x} rpc_hash={:#x}",
            db.header.number, db.header.hash, rpc.header.hash
        )
        .ok();
    }

    if !transactions_match {
        println!(
            "❌ Transaction metadata differ (db={} rpc={})",
            db.transactions.len(),
            rpc.transactions.len()
        );
        for (idx, (a, b)) in db
            .transactions
            .iter()
            .zip(rpc.transactions.iter())
            .enumerate()
        {
            let mut filtered_a = a.clone();
            let mut filtered_b = b.clone();
            filtered_a.tx_number = 0;
            filtered_b.tx_number = 0;
            if filtered_a != filtered_b {
                println!("  ↳ first mismatch at tx {} hash {:?}", idx, a.hash);
                writeln!(
                    log_file,
                    "[tx_meta] block={} idx={} hash={:#x}",
                    db.header.number, idx, a.hash
                )
                .ok();
                println!(
                    "    • db gas_price={} rpc gas_price={}",
                    a.gas_price, b.gas_price
                );
                println!("    • db nonce={} rpc nonce={}", a.nonce, b.nonce);
                writeln!(log_file, "[tx_meta_db] {:?}", filtered_a).ok();
                writeln!(log_file, "[tx_meta_rpc] {:?}", filtered_b).ok();
                break;
            }
        }
    }

    if db.receipts != rpc.receipts {
        println!(
            "❌ Receipts differ (db={} rpc={})",
            db.receipts.len(),
            rpc.receipts.len()
        );
        for (idx, (a, b)) in db.receipts.iter().zip(rpc.receipts.iter()).enumerate() {
            if a != b {
                println!("  ↳ first receipt mismatch at tx {}", idx);
                writeln!(
                    log_file,
                    "[receipt] block={} idx={} hash={:#x}",
                    db.header.number, idx, db.transactions[idx].hash
                )
                .ok();
                println!("    • status: db={} rpc={}", a.status, b.status);
                println!(
                    "    • cumulative gas: db={} rpc={}",
                    a.cumulative_gas_used, b.cumulative_gas_used
                );
                println!(
                    "    • effective gas price: db={} rpc={}",
                    a.effective_gas_price, b.effective_gas_price
                );
                println!(
                    "    • contract address: db={:?} rpc={:?}",
                    a.contract_address, b.contract_address
                );
                let logs_equal = a.logs == b.logs;
                println!("    • logs equal: {}", logs_equal);
                if !logs_equal {
                    for (log_idx, (log_a, log_b)) in a.logs.iter().zip(b.logs.iter()).enumerate() {
                        if log_a != log_b {
                            println!(
                                "      ↳ log {} differs\n        db: {:?}\n        rpc: {:?}",
                                log_idx, log_a, log_b
                            );
                            break;
                        }
                    }
                }
                break;
            }
        }
    }

    match (&db.traces, &rpc.traces) {
        (Some(a), Some(b)) if a != b => {
            println!("❌ Traces differ (db={} rpc={})", a.len(), b.len());
            for (idx, (da, rb)) in a.iter().zip(b.iter()).enumerate() {
                if da != rb {
                    println!(
                        "  ↳ first trace mismatch at tx {} (gas_used db={} rpc={})",
                        idx, da.gas_used, rb.gas_used
                    );
                    writeln!(
                        log_file,
                        "[trace] block={} idx={} hash={:#x}",
                        db.header.number, idx, db.transactions[idx].hash
                    )
                    .ok();
                    println!("    • error: db={:?} rpc={:?}", da.error, rb.error);
                    println!(
                        "    • call frames equal: {}",
                        da.call_frame == rb.call_frame
                    );
                    writeln!(log_file, "[trace_db] {:?}", da.call_frame).ok();
                    writeln!(log_file, "[trace_rpc] {:?}", rb.call_frame).ok();
                    break;
                }
            }
        }
        (None, Some(b)) => println!("❌ DB traces missing, rpc has {}", b.len()),
        (Some(a), None) => println!("❌ RPC traces missing, db has {}", a.len()),
        _ => {}
    }

    println!("⚠️ Block comparison finished with differences.");
    println!("Diff log: {log_path}");
    Ok(())
}

fn normalized_transactions(
    transactions: &[reth_chain_query::provider::TransactionMetadata],
) -> Vec<reth_chain_query::provider::TransactionMetadata> {
    transactions
        .iter()
        .cloned()
        .map(|mut tx| {
            // RPC block responses do not carry Reth's global transaction number.
            tx.tx_number = 0;
            tx
        })
        .collect()
}
