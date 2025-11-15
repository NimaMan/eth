use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use reth_primitives::SealedHeader;
use reth_provider::BlockReader;

use crate::provider::RethQueryProvider;

impl RethQueryProvider {
    /// Read Uniswap V3 pool slot0 and liquidity using direct state reads.
    /// Returns (sqrtPriceX96, tick, liquidity, block_timestamp)
    pub async fn uni_v3_get_slot0_and_liquidity(
        &self,
        pool: Address,
        block: Option<u64>,
        header: Option<&SealedHeader>,
    ) -> Result<(U256, i32, U256, u64)> {
        let block_number = block.unwrap_or(self.get_latest_block()?);
        let header_present = header.is_some();

        let state = self.tx_simulator.get_chain_state_at_block(block_number)?;

        // slot0 at storage slot 0
        let slot0_storage = state
            .storage(pool, B256::from(U256::ZERO))?
            .unwrap_or_default();
        let slot0_packed = U256::from_be_bytes(slot0_storage.to_be_bytes::<32>());

        // Decode sqrtPriceX96 (low 160 bits) and tick (next 24 bits, signed)
        let sqrt_price_mask = U256::MAX >> (256 - 160);
        let sqrt_price_x96 = slot0_packed & sqrt_price_mask;

        let tick_mask = U256::from((1u32 << 24) - 1);
        let tick_raw: U256 = (slot0_packed >> 160) & tick_mask;
        let tick = if tick_raw >= U256::from(1u32 << 23) {
            let tick_u32 = tick_raw.to::<u32>();
            let tick_signed = (tick_u32 as i64) - (1i64 << 24);
            tick_signed as i32
        } else {
            tick_raw.to::<i32>()
        };

        // liquidity at storage slot 4
        let liquidity_storage = state
            .storage(pool, B256::from(U256::from(4)))?
            .unwrap_or_default();
        let liquidity = U256::from_be_bytes(liquidity_storage.to_be_bytes::<32>());

        // Fetch timestamp for the block
        let timestamp = if let Some(h) = header {
            h.header().timestamp
        } else {
            self.provider_factory
                .block_by_number(block_number)
                .map_err(|e| eyre::eyre!(e.to_string()))?
                .ok_or_else(|| {
                    eyre::eyre!(
                        "Invalid block {} while reading UniswapV3 state (header supplied: {})",
                        block_number,
                        header_present
                    )
                })?
                .timestamp
        };

        Ok((sqrt_price_x96, tick, liquidity, timestamp))
    }
}
