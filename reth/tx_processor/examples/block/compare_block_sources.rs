use eyre::Result;
use reth_chain_query::provider::TransactionData;
use reth_chain_query::{
    provider::{BlockDataFetcher, RpcBlockDataFetcher},
    RethQueryProvider,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    sync::Arc,
};
use tracing::{error, info};
use tx_processor::{
    block_processor::{BlockProcessor, ProcessedBlock, ProcessedBlockTransactions},
    tx_processor::data_models::balance_changes::AddressBalanceChange,
};

/// cargo run --example block/compare_block_sources -- <block_number?>
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(std::io::stderr)
        .init();

    let datadir =
        env::var("RETH_DATA_DIR").unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".into());
    let rpc_url = env::var("EXECUTION_RPC").unwrap_or_else(|_| "http://127.0.0.1:8545".into());

    let provider = Arc::new(RethQueryProvider::new(&datadir)?);
    let latest = provider.get_latest_block()?;
    let fetcher = BlockDataFetcher::new(provider.clone())
        .with_rpc_fetcher(RpcBlockDataFetcher::new(&rpc_url)?);
    let fetcher = Arc::new(fetcher);
    let block_processor = BlockProcessor::with_block_fetcher(fetcher.clone());
    let block_number = env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| latest.saturating_sub(10));

    info!(
        "Comparing processed block {} using MDBX vs RPC (rpc={})",
        block_number, rpc_url
    );

    let db_raw = fetcher.fetch_db_block(block_number).await?;
    let db_processed = block_processor.process_raw_block(db_raw.clone()).await?;

    let rpc_raw = fetcher
        .fetch_rpc_block_by_hash(db_raw.header.hash, block_number)
        .await?;
    let rpc_processed = block_processor.process_raw_block(rpc_raw.clone()).await?;

    let identical = compare_balance_changes(&db_processed, &rpc_processed).await?;

    if identical {
        info!("✅ Processed blocks match for {}", block_number);
    } else {
        error!("❌ Processed blocks differ for {}", block_number);
    }

    Ok(())
}

async fn compare_balance_changes(db: &ProcessedBlock, rpc: &ProcessedBlock) -> Result<bool> {
    let mut identical = true;

    if db.header != rpc.header {
        error!(
            "Header mismatch\n  db: {:?}\n  rpc: {:?}",
            db.header, rpc.header
        );
        identical = false;
    }

    if db.transactions.len() != rpc.transactions.len() {
        error!(
            "Transaction count differs db={} rpc={}",
            db.transactions.len(),
            rpc.transactions.len()
        );
        return Ok(false);
    }

    for (idx, (db_tx, rpc_tx)) in db
        .transactions
        .iter()
        .zip(rpc.transactions.iter())
        .enumerate()
    {
        if !metadata_matches(&db_tx.metadata, &rpc_tx.metadata) {
            error!(
                "metadata mismatch for tx {} hash {:?}",
                idx, db_tx.metadata.hash
            );
            identical = false;
        }

        if !balance_changes_match(db_tx, rpc_tx)? {
            identical = false;
        }
    }

    Ok(identical)
}

fn metadata_matches(db: &TransactionData, rpc: &TransactionData) -> bool {
    let mut db_clone = db.clone();
    let mut rpc_clone = rpc.clone();
    db_clone.tx_number = 0;
    rpc_clone.tx_number = 0;
    db_clone == rpc_clone
}

fn balance_changes_match(
    db_tx: &ProcessedBlockTransactions,
    rpc_tx: &ProcessedBlockTransactions,
) -> Result<bool> {
    let db_map = normalize_balance_changes(&db_tx.processed.address_balance_changes)?;
    let rpc_map = normalize_balance_changes(&rpc_tx.processed.address_balance_changes)?;

    if db_map == rpc_map {
        return Ok(true);
    }

    error!(
        "balance change mismatch for hash {:?}: db entries={} rpc entries={}",
        db_tx.metadata.hash,
        db_map.len(),
        rpc_map.len()
    );
    log_balance_change_diff(&db_map, &rpc_map);
    Ok(false)
}

fn normalize_balance_changes(
    changes: &std::collections::HashMap<alloy_primitives::Address, AddressBalanceChange>,
) -> Result<BTreeMap<String, Value>> {
    let mut map = BTreeMap::new();
    for (address, change) in changes {
        let key = format!("{address:#x}");
        let value = serde_json::to_value(change)?;
        map.insert(key, value);
    }
    Ok(map)
}

fn log_balance_change_diff(db: &BTreeMap<String, Value>, rpc: &BTreeMap<String, Value>) {
    let db_only: Vec<_> = db
        .keys()
        .filter(|addr| !rpc.contains_key(*addr))
        .cloned()
        .collect();
    if !db_only.is_empty() {
        error!("  ↳ addresses only in MDBX result: {:?}", db_only);
    }

    let rpc_only: Vec<_> = rpc
        .keys()
        .filter(|addr| !db.contains_key(*addr))
        .cloned()
        .collect();
    if !rpc_only.is_empty() {
        error!("  ↳ addresses only in RPC result: {:?}", rpc_only);
    }

    let shared: BTreeSet<_> = db
        .keys()
        .filter(|addr| rpc.contains_key(*addr))
        .cloned()
        .collect();
    for addr in shared {
        if let (Some(db_val), Some(rpc_val)) = (db.get(&addr), rpc.get(&addr)) {
            if db_val != rpc_val {
                error!(
                    "  ↳ address {addr} differs\n    db: {}\n    rpc: {}",
                    db_val, rpc_val
                );
            }
        }
    }
}
