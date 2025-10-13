use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use reth_primitives::SealedHeader;

use crate::provider::RethQueryProvider;

/// Function selectors used by Uniswap V2 pair contract
const SELECTOR_TOKEN0: [u8; 4] = [0x0d, 0xfe, 0x16, 0x81]; // token0()
const SELECTOR_TOKEN1: [u8; 4] = [0xd2, 0x12, 0x20, 0xa7]; // token1()
const SELECTOR_GET_RESERVES: [u8; 4] = [0x09, 0x02, 0xf1, 0xac]; // getReserves()

impl RethQueryProvider {
    /// Read Uniswap V2 pair's token0/token1 at a given block using a local view call
    pub async fn uni_v2_get_tokens(
        &self,
        pair: Address,
        block: Option<u64>,
        header: Option<&SealedHeader>,
    ) -> Result<(Address, Address)> {
        let token0_res = self
            .tx_simulator
            .simulate_view_function(
                pair,
                Bytes::from(SELECTOR_TOKEN0.to_vec()),
                block,
                header.cloned(),
            )
            .await?;

        let token1_res = self
            .tx_simulator
            .simulate_view_function(
                pair,
                Bytes::from(SELECTOR_TOKEN1.to_vec()),
                block,
                header.cloned(),
            )
            .await?;

        if !token0_res.success || token0_res.output.len() < 32 {
            return Err(eyre::eyre!("token0() view call failed or empty output"));
        }
        if !token1_res.success || token1_res.output.len() < 32 {
            return Err(eyre::eyre!("token1() view call failed or empty output"));
        }

        // token0/1 are returned as 32 byte words; the address is right-aligned (last 20 bytes)
        let token0 = Address::from_slice(&token0_res.output[12..32]);
        let token1 = Address::from_slice(&token1_res.output[12..32]);

        Ok((token0, token1))
    }

    /// Read Uniswap V2 pair reserves at a given block using a local view call.
    /// Returns (reserve0, reserve1, block_timestamp_last)
    pub async fn uni_v2_get_reserves(
        &self,
        pair: Address,
        block: Option<u64>,
        header: Option<&SealedHeader>,
    ) -> Result<(U256, U256, u32)> {
        let res = self
            .tx_simulator
            .simulate_view_function(
                pair,
                Bytes::from(SELECTOR_GET_RESERVES.to_vec()),
                block,
                header.cloned(),
            )
            .await?;

        if !res.success || res.output.len() < 96 {
            return Err(eyre::eyre!(
                "getReserves() view call failed or insufficient output"
            ));
        }

        // ABI: (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        // Values are encoded in 32-byte slots, so we can parse as U256/u32
        let reserve0 = U256::from_be_slice(&res.output[0..32]);
        let reserve1 = U256::from_be_slice(&res.output[32..64]);
        let ts_bytes = &res.output[64..96];
        let block_timestamp_last =
            u32::from_be_bytes([ts_bytes[28], ts_bytes[29], ts_bytes[30], ts_bytes[31]]);

        Ok((reserve0, reserve1, block_timestamp_last))
    }

    /// Compute Uniswap V2 amountOut given amountIn and reserves (uses 0.30% fee)
    pub fn uni_v2_calc_amount_out(
        &self,
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> U256 {
        if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
            return U256::ZERO;
        }
        // Uniswap V2 fee model: amountInWithFee = amountIn * 997; denominator = reserveIn*1000 + amountInWithFee
        let amount_in_with_fee = amount_in * U256::from(997u64);
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * U256::from(1000u64) + amount_in_with_fee;
        if denominator.is_zero() {
            U256::ZERO
        } else {
            numerator / denominator
        }
    }
}
