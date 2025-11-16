use alloy_primitives::B256;
use eyre::Result;
use reth_primitives::SealedHeader;
use serde_json::Value;
use tracing::{debug, warn};

use crate::RethQueryProvider;
use tx_simulator::{
    tx_builders::unsigned_tx_builder::build_unsigned_transaction_from_processed_tx_json,
    UnsignedTxChainSimulation,
};

/// Replays pending transactions on top of the resolved block so metadata view
/// calls can observe live state even when MDBX has not advanced yet.
pub(super) async fn prepare_state_for_metadata(
    provider: &RethQueryProvider,
    block_number: Option<u64>,
    block_header: Option<SealedHeader>,
    pending_tx_hashes: &[B256],
) -> Result<Option<UnsignedTxChainSimulation>> {
    if pending_tx_hashes.is_empty() {
        return Ok(None);
    }

    let simulator = provider.simulator();
    let Some(cache) = simulator.live_chain_cache() else {
        return Ok(None);
    };

    let resolved_block = block_header
        .as_ref()
        .map(|header| header.number)
        .or(block_number)
        .unwrap_or(simulator.get_latest_block()?);

    let mut chain = simulator
        .start_simulation_chain(Some(resolved_block), block_header.clone())
        .await?;

    for hash in pending_tx_hashes {
        let hash_hex = format!("0x{}", hex::encode(hash.as_slice()));
        let Some(tx_value) = find_pending_transaction(&cache, resolved_block, &hash_hex).await? else {
            warn!(
                target: "reth_chain_query::erc20",
                block = resolved_block,
                pending_block = resolved_block + 1,
                hash = hash_hex,
                "pending transaction not found in live cache"
            );
            continue;
        };

        let unsigned_tx = build_unsigned_transaction_from_processed_tx_json(&tx_value)?;
        debug!(
            target: "reth_chain_query::erc20",
            block = resolved_block,
            pending_block = resolved_block + 1,
            hash = hash_hex,
            "Replaying pending tx before fetching token metadata"
        );
        chain.step(unsigned_tx).await?;
    }

    Ok(Some(chain))
}

async fn find_pending_transaction(
    cache: &tx_simulator::LiveChainCache,
    base_block: u64,
    hash: &str,
) -> Result<Option<Value>> {
    let latest_live = cache
        .latest_block_number()
        .await?
        .unwrap_or(base_block.saturating_add(1));

    for candidate in (base_block + 1)..=latest_live {
        if let Some(tx) = cache.find_processed_transaction(candidate, hash).await? {
            return Ok(Some(tx));
        }
    }

    Ok(None)
}
