use crate::reth_index::{
    database::RethIndexDB,
    models::{AddressMetrics, PoolData, TokenMetadata, TradeData},
    tables::address_block_participation::ParticipationBlockNumber,
};
/// Reader interface for RethIndex
///
/// Provides read-only access to all RethIndex tables with convenient query methods.
use alloy_primitives::Address;
use eyre::Result;

/// Read-only interface to RethIndex database
pub struct RethIndexReader {
    db: RethIndexDB,
}

impl RethIndexReader {
    /// Create a new reader
    pub fn new(db: RethIndexDB) -> Self {
        Self { db }
    }

    /// Get all indexed block numbers for an address.
    pub fn get_address_participation_blocks(
        &self,
        address: Address,
    ) -> Result<Vec<ParticipationBlockNumber>> {
        self.db.get_address_participation_blocks(address)
    }

    /// Get indexed block count for an address.
    pub fn get_address_participation_block_count(&self, address: Address) -> Result<usize> {
        self.get_address_participation_blocks(address)
            .map(|blocks| blocks.len())
    }

    /// Get latest N indexed blocks for an address.
    pub fn get_latest_address_participation_blocks(
        &self,
        address: Address,
        count: usize,
    ) -> Result<Vec<ParticipationBlockNumber>> {
        let mut blocks = self.get_address_participation_blocks(address)?;
        if count >= blocks.len() {
            return Ok(blocks);
        }
        let start = blocks.len() - count;
        Ok(blocks.split_off(start))
    }

    /// Get trade data for an address-token pair
    pub fn get_trade(&self, _address: Address, _token: Address) -> Result<Option<TradeData>> {
        // TODO: Implement
        Ok(None)
    }

    /// Get address metrics
    pub fn get_address_metrics(&self, _address: Address) -> Result<Option<AddressMetrics>> {
        // TODO: Implement
        Ok(None)
    }

    /// Get token metadata
    pub fn get_token(&self, _token: Address) -> Result<Option<TokenMetadata>> {
        // TODO: Implement
        Ok(None)
    }

    /// Get pool data
    pub fn get_pool(&self, _pool: Address) -> Result<Option<PoolData>> {
        // TODO: Implement
        Ok(None)
    }
}
