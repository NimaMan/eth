use alloy_primitives::Address;
use eyre::Result;

use super::RethQueryProvider;

impl RethQueryProvider {
    /// Return ordered processed block numbers for an address using the optional address index.
    pub fn participation_blocks_for_address(&self, address: Address) -> Result<Vec<u64>> {
        let index = self
            .reth_index()
            .ok_or_else(|| eyre::eyre!("RethIndex database not configured on this provider"))?;

        index.get_address_participation_blocks(address)
    }
}
