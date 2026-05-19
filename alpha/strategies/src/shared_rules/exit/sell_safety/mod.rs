use eth_alpha_core::{amount::DecimalAmount, market::PoolSnapshot};

pub const LOW_DENOM_RESERVE_REASON: &str = "pool_denom_reserve_below_min_sell_threshold";

/// Return a hold reason when the current pool denomination reserve is too low
/// to justify submitting an exit transaction.
pub fn dust_pool_exit_reason(
    pool: &PoolSnapshot,
    min_denom_reserve: DecimalAmount,
    exit_reason: &str,
) -> Option<String> {
    if min_denom_reserve <= DecimalAmount::ZERO || pool.denom_reserve >= min_denom_reserve {
        return None;
    }

    Some(format!(
        "{exit_reason}:{LOW_DENOM_RESERVE_REASON}:{}<{}",
        pool.denom_reserve, min_denom_reserve
    ))
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use eth_alpha_core::{
        ids::TokenPoolId,
        market::{PoolProtocol, PoolSnapshot},
    };
    use rust_decimal::Decimal;

    use super::*;

    fn pool_with_reserve(denom_reserve: DecimalAmount) -> PoolSnapshot {
        let token_address = Address::from([1u8; 20]);
        PoolSnapshot {
            address: TokenPoolId::new(token_address, "0x0202020202020202020202020202020202020202"),
            token_address,
            protocol: PoolProtocol::UniswapV2,
            denom_address: Some(Address::from([2u8; 20])),
            denom_symbol: Some("WETH".to_string()),
            denom_reserve,
            token_reserve: Decimal::from(1_000u64),
            price_denom_per_token: None,
            initial_price_denom_per_token: None,
            price_ratio_to_initial: None,
            token_decimals: Some(18),
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 1,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        }
    }

    #[test]
    fn returns_reason_when_pool_reserve_is_below_sell_floor() {
        let pool = pool_with_reserve(Decimal::new(1, 3));

        let reason = dust_pool_exit_reason(&pool, Decimal::new(1, 2), "exit.max_hold");

        assert_eq!(
            reason.as_deref(),
            Some("exit.max_hold:pool_denom_reserve_below_min_sell_threshold:0.001<0.01")
        );
    }

    #[test]
    fn allows_sell_when_pool_reserve_meets_floor() {
        let pool = pool_with_reserve(Decimal::new(1, 2));

        let reason = dust_pool_exit_reason(&pool, Decimal::new(1, 2), "exit.max_hold");

        assert_eq!(reason, None);
    }

    #[test]
    fn zero_floor_disables_dust_guard() {
        let pool = pool_with_reserve(Decimal::ZERO);

        let reason = dust_pool_exit_reason(&pool, Decimal::ZERO, "exit.max_hold");

        assert_eq!(reason, None);
    }
}
