use alloy_primitives::B256;
use eyre::Result;
use reth_primitives::SealedHeader;
use tracing::debug;

use crate::RethQueryProvider;
use tx_simulator::tx_builders::unsigned_tx_builder::build_unsigned_transaction_from_processed_tx_json;

/// Fetch the processed creation transaction from Redis so we can hydrate state
/// before querying metadata. Future work will execute the transaction against a
/// forked state; for now we simply verify the transaction exists.
pub(super) async fn prepare_state_for_metadata(
    provider: &RethQueryProvider,
    block_number: Option<u64>,
    _block_header: Option<SealedHeader>,
    tx_hash: Option<B256>,
) -> Result<()> {
    let (Some(block_number), Some(tx_hash)) = (block_number, tx_hash) else {
        return Ok(());
    };

    let simulator = provider.simulator();
    let Some(cache) = simulator.live_chain_cache() else {
        return Ok(());
    };

    let hash_str = format!("0x{}", hex::encode(tx_hash.as_slice()));
    if let Some(tx_value) = cache
        .find_processed_transaction(block_number, &hash_str)
        .await?
    {
        let unsigned_tx = build_unsigned_transaction_from_processed_tx_json(&tx_value)?;
        debug!(
            target: "reth_chain_query::erc20",
            block = block_number,
            hash = hash_str,
            "Resolved deployment tx for ERC20 metadata (to={:?})",
            unsigned_tx.to
        );
    }

    Ok(())
}
