use alloy_primitives::B256;
use eyre::Result;
use tracing::debug;
use tx_simulator::UnsignedTxChainSimulation;

use crate::RethQueryProvider;

/// Pending metadata replay used to depend on a live cache replay path.
/// The live block processor now tracks block and token state in-process, so
/// reth_chain_query no longer asks tx_simulator to hydrate pending tx payloads
/// before metadata reads.
pub(super) async fn prepare_state_for_metadata(
    _provider: &RethQueryProvider,
    block_number: Option<u64>,
    pending_tx_hashes: &[B256],
) -> Result<Option<UnsignedTxChainSimulation>> {
    if !pending_tx_hashes.is_empty() {
        debug!(
            target: "reth_chain_query::erc20",
            ?block_number,
            pending_tx_count = pending_tx_hashes.len(),
            "pending metadata replay skipped; live-cache replay has been removed"
        );
    }

    Ok(None)
}
