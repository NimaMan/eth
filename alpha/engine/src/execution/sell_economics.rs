use alloy_primitives::{Address, U256, address};
use eth_alpha_core::market::PoolSnapshot;

const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

/// Return a cancellation reason when a simulated ETH/WETH-denominated sell
/// would recover less denomination value than the gas it costs.
///
/// This is execution economics, not strategy policy: strategies can decide
/// when to ask for an exit, but the execution adapter owns whether the final
/// simulated transaction is worth submitting.
pub(crate) fn uneconomic_sell_cancellation_reason(
    pool: &PoolSnapshot,
    denom_received: U256,
    gas_cost: U256,
) -> Option<String> {
    if !pool_has_eth_like_denom(pool) || denom_received > gas_cost {
        return None;
    }

    Some(format!(
        "uneconomic sell: simulated WETH proceeds {denom_received} wei <= gas cost {gas_cost} wei"
    ))
}

fn pool_has_eth_like_denom(pool: &PoolSnapshot) -> bool {
    if matches!(pool.denom_address, Some(address) if address.is_zero() || address == WETH_ADDRESS) {
        return true;
    }

    pool.denom_symbol
        .as_deref()
        .map(str::trim)
        .map(|symbol| {
            let symbol = symbol.to_ascii_uppercase();
            symbol == "ETH" || symbol == "WETH"
        })
        .unwrap_or(false)
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

    fn pool_with_denom(denom_address: Option<Address>, denom_symbol: Option<&str>) -> PoolSnapshot {
        let token_address = Address::from([1u8; 20]);
        PoolSnapshot {
            address: TokenPoolId::new(token_address, "0x0202020202020202020202020202020202020202"),
            token_address,
            protocol: PoolProtocol::UniswapV2,
            denom_address,
            denom_symbol: denom_symbol.map(str::to_string),
            denom_reserve: Decimal::from(1),
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
    fn cancels_eth_like_sell_when_proceeds_do_not_exceed_gas() {
        let pool = pool_with_denom(Some(WETH_ADDRESS), Some("WETH"));

        let reason =
            uneconomic_sell_cancellation_reason(&pool, U256::from(100u64), U256::from(100u64));

        assert_eq!(
            reason.as_deref(),
            Some("uneconomic sell: simulated WETH proceeds 100 wei <= gas cost 100 wei")
        );
    }

    #[test]
    fn allows_eth_like_sell_when_proceeds_exceed_gas() {
        let pool = pool_with_denom(Some(WETH_ADDRESS), Some("WETH"));

        let reason =
            uneconomic_sell_cancellation_reason(&pool, U256::from(101u64), U256::from(100u64));

        assert_eq!(reason, None);
    }

    #[test]
    fn skips_non_eth_denoms_because_gas_and_proceeds_are_not_same_unit() {
        let pool = pool_with_denom(None, Some("USDC"));

        let reason =
            uneconomic_sell_cancellation_reason(&pool, U256::from(1u64), U256::from(100u64));

        assert_eq!(reason, None);
    }
}
