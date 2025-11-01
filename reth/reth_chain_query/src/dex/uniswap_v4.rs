//! Utilities for discovering Uniswap V4 pools via event scanning.

use crate::TxSimulator;
use alloy_primitives::{Address, B256};
use eyre::Result;
use reth_provider::ReceiptProvider;
use std::collections::HashSet;

/// Information about a Uniswap V4 pool discovered from PoolCreated events.
#[derive(Debug, Clone)]
pub struct V4PoolInfo {
    pub pool_id: B256,
    pub pool_address: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
}

/// Find Uniswap V4 pools for a token pair (all fee tiers).
pub async fn find_uniswap_v4_pools_for_pair(
    simulator: &TxSimulator,
    pool_manager: Address,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<Vec<V4PoolInfo>> {
    let mut pools = Vec::new();
    let provider = simulator.provider_factory().provider()?;
    let start_block = block_number.unwrap_or(0);
    let end_block = block_number.unwrap_or(start_block);
    let receipts = provider.receipts_by_block_range(start_block..=end_block)?;

    for block_receipts in receipts {
        for receipt in block_receipts {
            for log in receipt.logs {
                if log.address == pool_manager {
                    let log_bytes = log.data.data;
                    if let Some(pool_info) =
                        decode_uniswap_v4_pool_created_event(log_bytes.as_ref(), token_a, token_b)
                    {
                        pools.push(pool_info);
                    }
                }
            }
        }
    }

    Ok(pools)
}

/// Collect unique pool addresses from a list of V4 pools.
pub fn collect_unique_tokens_from_pools(pools: &[V4PoolInfo]) -> HashSet<Address> {
    pools.iter().map(|pool| pool.pool_address).collect()
}

fn decode_uniswap_v4_pool_created_event(
    data: &[u8],
    token_a: Address,
    token_b: Address,
) -> Option<V4PoolInfo> {
    if data.len() < 224 {
        return None;
    }

    let pool_id = B256::from_slice(&data[0..32]);
    let token0 = Address::from_slice(&data[32 + 12..64]);
    let token1 = Address::from_slice(&data[64 + 12..96]);
    let fee = u32::from_be_bytes([data[96 + 28], data[96 + 29], data[96 + 30], data[96 + 31]]);
    let tick_spacing = i32::from_be_bytes([
        data[128 + 28],
        data[128 + 29],
        data[128 + 30],
        data[128 + 31],
    ]);
    let hooks = Address::from_slice(&data[160 + 12..192]);
    let pool_address = Address::from_slice(&data[192 + 12..224]);

    if (token0 == token_a && token1 == token_b) || (token0 == token_b && token1 == token_a) {
        Some(V4PoolInfo {
            pool_id,
            pool_address,
            fee,
            tick_spacing,
            hooks,
        })
    } else {
        None
    }
}
