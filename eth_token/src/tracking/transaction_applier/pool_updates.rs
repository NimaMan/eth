use eyre::Result;
use tx_processor::ProcessedTransaction;

use crate::erc20::ERC20Token;

use super::touches::{touches_v2_pool, touches_v3_pool, touches_v4_pool};

pub(super) fn update_touched_v2_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
) -> Result<Vec<String>> {
    let pool_addresses = token.uniswap_v2_pool_addresses();
    let mut updated = Vec::new();
    for pool_address in pool_addresses {
        if touches_v2_pool(tx, &pool_address) {
            token.update_uniswap_v2_pool_from_processed_transaction(&pool_address, tx)?;
            updated.push(pool_address);
        }
    }
    Ok(updated)
}

pub(super) fn update_touched_v3_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
) -> Result<Vec<String>> {
    let pool_addresses = token.uniswap_v3_pool_addresses();
    let mut updated = Vec::new();
    for pool_address in pool_addresses {
        if touches_v3_pool(tx, &pool_address) {
            token.update_uniswap_v3_pool_from_processed_transaction(&pool_address, tx)?;
            updated.push(pool_address);
        }
    }
    Ok(updated)
}

pub(super) fn update_touched_v4_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
) -> Result<Vec<String>> {
    let pool_keys = token.uniswap_v4_pool_keys();
    let mut updated = Vec::new();
    for pool_key in pool_keys {
        if touches_v4_pool(tx, &pool_key) {
            token.update_uniswap_v4_pool_from_processed_transaction(&pool_key, tx)?;
            updated.push(pool_key);
        }
    }
    Ok(updated)
}
pub(super) fn has_protocol_pool_changes(changes: &[&Vec<String>]) -> bool {
    changes.iter().any(|change| !change.is_empty())
}
