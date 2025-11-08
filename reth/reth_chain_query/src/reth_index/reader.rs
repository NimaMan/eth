use crate::reth_index::{
    database::RethIndexDB,
    models::{AddressMetrics, PoolData, TokenMetadata, TradeData},
    tables::address_index::Txumber,
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

    /// Get all transactions for an address
    pub fn get_transactions(&self, address: Address) -> Result<Vec<Txumber>> {
        self.db.get_transactions(address)
    }

    /// Get transaction count for an address
    pub fn get_transaction_count(&self, address: Address) -> Result<usize> {
        self.get_transactions(address).map(|txs| txs.len())
    }

    /// Get latest N transactions for an address
    pub fn get_latest_transactions(&self, address: Address, count: usize) -> Result<Vec<Txumber>> {
        let mut txs = self.get_transactions(address)?;
        if count >= txs.len() {
            return Ok(txs);
        }
        let start = txs.len() - count;
        Ok(txs.split_off(start))
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
