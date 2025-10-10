use alloy_primitives::{Address, B256};
use eyre::Result;

use super::RethQueryProvider;

/// Lightweight transaction reference returned for address index lookups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressTransactionRef {
    pub tx_number: u64,
    pub tx_hash: B256,
    pub block_number: u64,
    pub tx_index: u64,
}

impl RethQueryProvider {
    /// Return ordered transaction references for an address using the optional address index.
    pub async fn transactions_for_address(
        &self,
        address: Address,
    ) -> Result<Vec<AddressTransactionRef>> {
        let index = self
            .reth_index()
            .ok_or_else(|| eyre::eyre!("RethIndex database not configured on this provider"))?;

        let tx_numbers = index.get_transactions(address)?;
        if tx_numbers.is_empty() {
            return Ok(Vec::new());
        }

        let mut refs = Vec::with_capacity(tx_numbers.len());
        for tx_number in tx_numbers {
            let tx_data = self.get_transaction_by_number(tx_number).await?;
            refs.push(AddressTransactionRef {
                tx_number,
                tx_hash: tx_data.hash,
                block_number: tx_data.block_number,
                tx_index: tx_data.tx_index,
            });
        }

        Ok(refs)
    }
}
