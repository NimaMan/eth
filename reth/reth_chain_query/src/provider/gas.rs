use eyre::Result;

use super::RethQueryProvider;

impl RethQueryProvider {
    /// Get the base fee (wei) for a specific block.
    pub fn get_base_fee_at_block(&self, block_number: u64) -> Result<u128> {
        self.tx_simulator.get_base_fee_at_block(block_number)
    }

    /// Get the latest block's base fee (wei).
    pub fn get_latest_base_fee(&self) -> Result<u128> {
        let latest = self.get_latest_block()?;
        self.get_base_fee_at_block(latest)
    }

    /// Get block gas metadata: (gas_limit, gas_used, base_fee_wei Option)
    pub fn get_block_gas_metadata(&self, block_number: u64) -> Result<(u64, u64, Option<u128>)> {
        let (_ts, gas_limit, gas_used, base_fee_opt) = self.tx_simulator.get_block_metadata(block_number)?;
        Ok((gas_limit, gas_used, base_fee_opt))
    }
}

