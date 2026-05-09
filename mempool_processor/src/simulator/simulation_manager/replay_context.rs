use alloy_primitives::Address as AlloyAddress;
use std::collections::HashSet;
use std::sync::Arc;
use tx_processor::ProcessedTransaction;

use crate::token_tracking::{Pool, PoolType as CachePoolType};

/// Candidate pool entry that can come either from the token cache or from replay logs.
#[derive(Clone)]
pub struct PoolCandidate {
    pub address: String,
    pub denom_address: String,
    pub denom_currency: String,
    pub pool_type: CachePoolType,
    pub fee_tier: Option<u32>,
    pub pool_id: Option<String>,
    pub eth_reserve_hint: f64,
}

impl PoolCandidate {
    pub fn from_cache(pool: Arc<Pool>) -> Self {
        Self {
            address: pool.address.clone(),
            denom_address: pool.denom_address.clone(),
            denom_currency: pool.denom_currency.clone(),
            pool_type: pool.pool_type.clone(),
            fee_tier: pool.fee_tier,
            pool_id: pool.pool_id.clone(),
            eth_reserve_hint: pool.eth_reserve,
        }
    }

    pub fn from_uniswap_v2_event(
        pair_address: AlloyAddress,
        denom_address: AlloyAddress,
        weth_address: &AlloyAddress,
    ) -> Self {
        let denom_currency = if denom_address == *weth_address {
            "ETH".to_string()
        } else {
            format!("{:#x}", denom_address)
        };

        Self {
            address: format!("{:#x}", pair_address),
            denom_address: format!("{:#x}", denom_address),
            denom_currency,
            pool_type: CachePoolType::UniswapV2,
            fee_tier: None,
            pool_id: None,
            eth_reserve_hint: 0.0,
        }
    }
}

/// Derive pool candidates directly from replayed helper transactions (Uniswap V2 only for now).
pub fn derive_pools_from_replay(
    token_address: AlloyAddress,
    replay_sequence: &[ProcessedTransaction],
    weth_address: AlloyAddress,
) -> Vec<PoolCandidate> {
    let mut seen_pairs = HashSet::new();
    let mut derived = Vec::new();

    for processed in replay_sequence.iter().rev() {
        for event in &processed.uniswap_v2_pair_created_events {
            if event.token0 == token_address || event.token1 == token_address {
                if seen_pairs.insert(event.pair_address) {
                    let denom = if event.token0 == token_address {
                        event.token1
                    } else {
                        event.token0
                    };
                    derived.push(PoolCandidate::from_uniswap_v2_event(
                        event.pair_address,
                        denom,
                        &weth_address,
                    ));
                }
            }
        }
    }

    derived
}
