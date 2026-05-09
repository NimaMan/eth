use crate::provider::RethQueryProvider;
use alloy_primitives::{address, keccak256, Address, Bytes, B256, U256};
use eyre::Result;

pub const UNISWAP_V4_MAINNET_POOL_MANAGER: Address =
    address!("000000000004444C5DC75cB358380d2E3de08a90");
pub const UNISWAP_V4_MAINNET_STATE_VIEW: Address =
    address!("7ffe42c4a5deea5b0fec41c94c136cf115597227");

impl RethQueryProvider {
    /// Read Uniswap V4 pool state via StateView view functions.
    /// Returns (sqrtPriceX96, tick, liquidity, block_timestamp)
    pub async fn uni_v4_get_slot0_and_liquidity(
        &self,
        pool_manager: Address,
        pool_id: B256,
        block: Option<u64>,
    ) -> Result<(U256, i32, U256, u64)> {
        let block_number = block.unwrap_or(self.get_latest_block()?);
        let state_view = state_view_for_pool_manager(pool_manager).unwrap_or(pool_manager);

        // Build calldata for getSlot0(bytes32)
        let sig_slot0 = b"getSlot0(bytes32)";
        let selector_slot0 = &keccak256(sig_slot0)[0..4];
        let mut params = Vec::with_capacity(4 + 32);
        params.extend_from_slice(selector_slot0);
        params.extend_from_slice(pool_id.as_slice());
        let call_data_slot0 = Bytes::from(params);

        let slot0_res = self
            .simulator()
            .simulate_view_function(state_view, call_data_slot0, Some(block_number))
            .await?;

        // Expect ABI-encoded (uint160 sqrtPriceX96, int24 tick, uint8 protocolFee, uint8 hookFee)
        if !slot0_res.success || slot0_res.output.len() < 64 {
            return Err(eyre::eyre!(
                "getSlot0 failed via Uniswap V4 StateView {state_view:#x}"
            ));
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
            .simulate_view_function(state_view, call_data_liq, Some(block_number))
            .await?;
        if !liq_res.success || liq_res.output.len() < 32 {
            return Err(eyre::eyre!(
                "getLiquidity failed via Uniswap V4 StateView {state_view:#x}"
            ));
        }
        let liquidity = U256::from_be_bytes::<32>(liq_res.output[0..32].try_into().unwrap());

        let timestamp = self
            .simulator()
            .block_context_loader()
            .load_block_header(block_number, None)
            .await?
            .timestamp;

        Ok((sqrt_price_x96, tick, liquidity, timestamp))
    }
}

fn state_view_for_pool_manager(pool_manager: Address) -> Option<Address> {
    if pool_manager == UNISWAP_V4_MAINNET_POOL_MANAGER {
        Some(UNISWAP_V4_MAINNET_STATE_VIEW)
    } else {
        None
    }
}
