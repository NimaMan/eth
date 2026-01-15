use crate::provider::RethQueryProvider;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use eyre::Result;
use reth_provider::BlockReader;

impl RethQueryProvider {
    /// Read Uniswap V4 pool state via PoolManager view functions.
    /// Returns (sqrtPriceX96, tick, liquidity, block_timestamp)
    pub async fn uni_v4_get_slot0_and_liquidity(
        &self,
        pool_manager: Address,
        pool_id: B256,
        block: Option<u64>,
    ) -> Result<(U256, i32, U256, u64)> {
        let block_number = block.unwrap_or(self.get_latest_block()?);

        // Build calldata for getSlot0(bytes32)
        let sig_slot0 = b"getSlot0(bytes32)";
        let selector_slot0 = &keccak256(sig_slot0)[0..4];
        let mut params = Vec::with_capacity(4 + 32);
        params.extend_from_slice(selector_slot0);
        params.extend_from_slice(pool_id.as_slice());
        let call_data_slot0 = Bytes::from(params);

        let slot0_res = self
            .simulator()
            .simulate_view_function(pool_manager, call_data_slot0, Some(block_number))
            .await?;

        // Expect ABI-encoded (uint160 sqrtPriceX96, int24 tick, uint8 protocolFee, uint8 hookFee)
        if !slot0_res.success || slot0_res.output.len() < 64 {
            return Err(eyre::eyre!("getSlot0 failed"));
        }
        // First 32 bytes: sqrtPriceX96 padded
        let sqrt_price_x96 = U256::from_be_bytes::<32>(slot0_res.output[0..32].try_into().unwrap());
        // Second 32 bytes: tick padded (take last 3 bytes as int24 with sign)
        let tick_bytes = &slot0_res.output[32..64];
        let t2 =
            (tick_bytes[29] as u32) << 16 | (tick_bytes[30] as u32) << 8 | (tick_bytes[31] as u32);
        let tick = if (t2 & (1 << 23)) != 0 {
            // negative
            let signed = (t2 as i32) - (1 << 24);
            signed
        } else {
            t2 as i32
        };

        // Build calldata for getLiquidity(bytes32) -> uint128
        let sig_liq = b"getLiquidity(bytes32)";
        let selector_liq = &keccak256(sig_liq)[0..4];
        let mut params_liq = Vec::with_capacity(4 + 32);
        params_liq.extend_from_slice(selector_liq);
        params_liq.extend_from_slice(pool_id.as_slice());
        let call_data_liq = Bytes::from(params_liq);

        let liq_res = self
            .simulator()
            .simulate_view_function(pool_manager, call_data_liq, Some(block_number))
            .await?;
        if !liq_res.success || liq_res.output.len() < 32 {
            return Err(eyre::eyre!("getLiquidity failed"));
        }
        let liquidity = U256::from_be_bytes::<32>(liq_res.output[0..32].try_into().unwrap());

        // Fetch timestamp for the block
        let timestamp = self
            .provider_factory()
            .block_by_number(block_number)
            .map_err(|e| eyre::eyre!(e.to_string()))?
            .ok_or_else(|| {
                eyre::eyre!(
                    "Invalid block {} while reading UniswapV4 state",
                    block_number
                )
            })?
            .timestamp;

        Ok((sqrt_price_x96, tick, liquidity, timestamp))
    }
}
