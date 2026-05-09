use alloy_primitives::{keccak256, Address, Bytes, U256};
use eyre::Result;

use crate::provider::RethQueryProvider;

impl RethQueryProvider {
    /// Read Uniswap V3 pool slot0 and liquidity through live-aware view calls.
    /// Returns (sqrtPriceX96, tick, liquidity, block_timestamp)
    pub async fn uni_v3_get_slot0_and_liquidity(
        &self,
        pool: Address,
        block: Option<u64>,
    ) -> Result<(U256, i32, U256, u64)> {
        let block_number = block.unwrap_or(self.get_latest_block()?);

        let slot0_selector = &keccak256(b"slot0()")[0..4];
        let slot0_res = self
            .simulator()
            .simulate_view_function(
                pool,
                Bytes::from(slot0_selector.to_vec()),
                Some(block_number),
            )
            .await?;
        if !slot0_res.success || slot0_res.output.len() < 64 {
            return Err(eyre::eyre!(
                "UniswapV3 slot0() failed for pool {} at block {}",
                pool,
                block_number
            ));
        }

        // slot0() returns (uint160 sqrtPriceX96, int24 tick, ...).
        let sqrt_price_x96 = U256::from_be_bytes::<32>(
            slot0_res.output[0..32]
                .try_into()
                .expect("slot0 output length checked"),
        );
        let tick = decode_abi_int24(&slot0_res.output[32..64]);

        let liquidity_selector = &keccak256(b"liquidity()")[0..4];
        let liquidity_res = self
            .simulator()
            .simulate_view_function(
                pool,
                Bytes::from(liquidity_selector.to_vec()),
                Some(block_number),
            )
            .await?;
        if !liquidity_res.success || liquidity_res.output.len() < 32 {
            return Err(eyre::eyre!(
                "UniswapV3 liquidity() failed for pool {} at block {}",
                pool,
                block_number
            ));
        }
        let liquidity = U256::from_be_bytes::<32>(
            liquidity_res.output[0..32]
                .try_into()
                .expect("liquidity output length checked"),
        );

        let timestamp = self
            .simulator()
            .block_context_loader()
            .load_block_header(block_number, None)
            .await?
            .timestamp;

        Ok((sqrt_price_x96, tick, liquidity, timestamp))
    }
}

fn decode_abi_int24(word: &[u8]) -> i32 {
    let raw = ((word[29] as u32) << 16) | ((word[30] as u32) << 8) | word[31] as u32;
    if (raw & (1 << 23)) != 0 {
        (raw as i32) - (1 << 24)
    } else {
        raw as i32
    }
}
