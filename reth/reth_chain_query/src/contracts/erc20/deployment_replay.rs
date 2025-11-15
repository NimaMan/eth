use alloy_primitives::Address;
use eyre::Result;
use reth_primitives::SealedHeader;

use crate::RethQueryProvider;

/// Placeholder hook for replaying the deployment transaction before running
/// metadata view calls. Future work will hydrate the simulator using the
/// original creation transaction so contracts created in the latest block can
/// still be inspected before the MDBX state catches up.
pub(super) fn prepare_state_for_metadata(
    _provider: &RethQueryProvider,
    _address: Address,
    _block_number: Option<u64>,
    _block_header: Option<SealedHeader>,
) -> Result<()> {
    Ok(())
}
