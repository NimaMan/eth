use super::thresholds::threshold_for_symbol;
use super::types::{Pool, PoolLifecycle};

pub(super) fn pool_is_viable(pool: &Pool, fallback_eth_threshold: f64) -> bool {
    match pool.lifecycle {
        PoolLifecycle::LiquidityDeposited | PoolLifecycle::Active => true,
        PoolLifecycle::Scam | PoolLifecycle::Evicted => false,
        PoolLifecycle::Discovered | PoolLifecycle::Unknown => {
            pool.eth_reserve >= pool_liquidity_threshold(pool, fallback_eth_threshold)
        }
    }
}

pub(super) fn pool_liquidity_threshold(pool: &Pool, fallback_eth_threshold: f64) -> f64 {
    let symbol = pool.denom_currency.trim();
    if !symbol.is_empty() {
        if let Some(threshold) = threshold_for_symbol(symbol) {
            return threshold;
        }

        if symbol.eq_ignore_ascii_case("ETH") {
            return fallback_eth_threshold;
        }
    }

    if pool
        .denom_address
        .eq_ignore_ascii_case("0x0000000000000000000000000000000000000000")
    {
        return fallback_eth_threshold;
    }

    fallback_eth_threshold
}
