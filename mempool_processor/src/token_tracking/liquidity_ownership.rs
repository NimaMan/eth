use super::types::{Address, Pool, PoolType};

pub(crate) fn liquidity_ownership_token_addresses(pool: &Pool) -> Vec<Address> {
    let mut tokens = Vec::new();

    if let Some(lp_token_address) = pool.lp_token_address.as_ref() {
        push_unique(&mut tokens, lp_token_address);
    }

    if pool_address_is_ownership_token_candidate(pool) {
        push_unique(&mut tokens, &pool.address);
    }

    tokens
}

fn pool_address_is_ownership_token_candidate(pool: &Pool) -> bool {
    matches!(
        pool.pool_type,
        PoolType::UniswapV2
            | PoolType::SushiSwapV2
            | PoolType::PancakeSwapV2
            | PoolType::ShibaSwapV2
            | PoolType::FraxswapV2
            | PoolType::Curve
            | PoolType::Balancer
            | PoolType::Unknown
    ) && is_evm_address(&pool.address)
}

pub(crate) fn is_evm_address(value: &str) -> bool {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    hex.len() == 40 && hex.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit())
}

fn push_unique(tokens: &mut Vec<Address>, address: &str) {
    let address = address.trim();
    if address.is_empty() {
        return;
    }
    if !is_evm_address(address) {
        return;
    }

    let normalized = address.to_ascii_lowercase();
    if !tokens.iter().any(|existing| existing == &normalized) {
        tokens.push(normalized);
    }
}
