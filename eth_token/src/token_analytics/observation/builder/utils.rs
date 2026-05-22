use std::collections::BTreeMap;

use crate::erc20::ERC20Token;
use crate::pools::BasePool;
use crate::pools::TaxBucket;
use crate::token_activity::TokenBlockActivity;

pub(super) const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
pub(super) const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
pub(super) const WETH_SYMBOL: &str = "WETH";
pub(super) const ETH_SYMBOL: &str = "ETH";
pub(super) const ROUTER_ADDRESSES: &[&str] = &[
    "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
    "0xe592427a0aece92de3edee1f18e0157c05861564",
    "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45",
];
pub(super) const USD_DENOM_ADDRESSES: &[&str] = &[
    "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "0xdac17f958d2ee523a2206206994597c13d831ec7",
    "0x6b175474e89094c44da98b954eedeac495271d0f",
    "0x4fabb145d64652a948d72533023f6e7a623c7c53",
    "0x8e870d67f660d95d5be530380d0ec0bd388289e1",
    "0x853d955acef822db058eb8505911ed77f175b99e",
    "0x5f98805a4e8be255a32880fdec7f6728c6568ba0",
    "0x0000000000085d4780b73119b644ae5ecd22b376",
    "0x4c9edd5852cd905f086c759e8383e09bff1e68b3",
    "0xa4bdb11dc0a2bec88d24a3aa1e6bb17201112ebe",
    "0x6c3ea9036406852006290770bedfcaba0e23a0e8",
    "0xc5f0f7b66764f6ec8c8dff7ba683102295e16409",
    "0x57ab1ec28d129707052df4df418d58a2d46d5f51",
    "0x5d3a536e4d6dbd6114cc1ead35777bab948e3643",
    "0x39aa39c021dfbae8fac545936693ac917d5e7563",
    "0xda816459f1ab5631232fe5e97a05bbbb94970c95",
    "0xa354f35829ae975e850e23e9615b11da1b3dc4de",
];
pub(super) const USD_DENOM_SYMBOLS: &[&str] = &[
    "USDC", "USDT", "DAI", "BUSD", "PAX", "FRAX", "LUSD", "TUSD", "USDE", "SUSDE", "USDS", "USDD",
    "PYUSD", "FDUSD", "GUSD", "USDP", "USDM", "USD0", "USD1", "SUSD", "CDAI", "CUSDC", "YVDAI",
    "YVUSDC",
];

pub(super) fn is_weth_denom(value: &str) -> bool {
    let normalized = normalize_address(value);
    normalized == WETH_ADDRESS
        || normalized.eq_ignore_ascii_case(WETH_SYMBOL)
        || normalized.eq_ignore_ascii_case(ETH_SYMBOL)
}

pub(super) fn is_usd_denom(value: &str) -> bool {
    let normalized = normalize_address(value);
    USD_DENOM_ADDRESSES.contains(&normalized.as_str())
        || USD_DENOM_SYMBOLS
            .iter()
            .any(|symbol| normalized.eq_ignore_ascii_case(symbol))
        || normalized.to_ascii_uppercase().starts_with("USD")
}

pub(super) fn is_evm_address(value: &str) -> bool {
    value.len() == 42
        && value.starts_with("0x")
        && value[2..].chars().all(|char| char.is_ascii_hexdigit())
}

pub(super) fn ratio_if_positive(numerator: f64, denominator: Option<f64>) -> Option<f64> {
    let denominator = denominator?;
    if numerator.is_finite() && numerator >= 0.0 && denominator.is_finite() && denominator > 0.0 {
        Some(numerator / denominator)
    } else {
        None
    }
}

pub(super) fn pool_token_reserve_ratio_denominator(
    token: &ERC20Token,
    pool: &BasePool,
    block_number: u64,
) -> Option<f64> {
    let total_supply = token.total_supply_scaled();
    let current = meaningful_pool_token_reserve(pool.token_reserve(), total_supply);
    let previous = pool
        .reserve_tracker
        .reserve_history
        .iter()
        .rev()
        .find(|snapshot| snapshot.block_number < block_number)
        .and_then(|snapshot| meaningful_pool_token_reserve(snapshot.token_reserve, total_supply));

    match (current, previous) {
        (Some(current), Some(previous)) => Some(current.max(previous)),
        (Some(current), None) => Some(current),
        (None, Some(previous)) => Some(previous),
        (None, None) => None,
    }
}

fn meaningful_pool_token_reserve(
    token_reserve: f64,
    total_supply_scaled: Option<f64>,
) -> Option<f64> {
    const MIN_ABSOLUTE_TOKEN_RESERVE: f64 = 1e-12;
    const MIN_SUPPLY_RATIO: f64 = 1e-12;

    if !token_reserve.is_finite() || token_reserve <= 0.0 {
        return None;
    }

    if let Some(total_supply) =
        total_supply_scaled.filter(|value| value.is_finite() && *value > 0.0)
    {
        if token_reserve / total_supply >= MIN_SUPPLY_RATIO {
            return Some(token_reserve);
        }
    } else if token_reserve >= MIN_ABSOLUTE_TOKEN_RESERVE {
        return Some(token_reserve);
    }

    None
}

pub(super) fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

pub(super) fn economic_sellable(can_sell: bool, sell_tax: Option<f64>) -> Option<bool> {
    if !can_sell {
        return Some(false);
    }
    TaxBucket::from_percent(sell_tax).economic_sellable()
}

pub(super) fn activity_has_signal_for_denom(activity: &TokenBlockActivity, denom: &str) -> bool {
    activity.num_tx > 0
        || activity.token_transfer_count > 0
        || activity.denom_transfer_count > 0
        || volume_for_denom(&activity.buy_volume_by_denom, denom) > 0.0
        || volume_for_denom(&activity.sell_volume_by_denom, denom) > 0.0
        || activity.total_bribe_eth > 0.0
}

pub(super) fn volume_for_denom(volumes: &BTreeMap<String, f64>, denom: &str) -> f64 {
    volumes
        .iter()
        .find(|(key, _)| denom_key_matches(key, denom))
        .map(|(_, value)| *value)
        .unwrap_or(0.0)
}

pub(super) fn denom_key_matches(key: &str, denom: &str) -> bool {
    if key.eq_ignore_ascii_case(denom) {
        return true;
    }
    is_weth_denom(denom) && is_weth_denom(key)
}

pub(super) fn tax_bucket_key(bucket: TaxBucket) -> String {
    bucket.key().to_string()
}

pub(super) fn display_tax(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

pub(super) fn same_optional_address(left: Option<&str>, right: Option<&str>) -> Option<bool> {
    left.zip(right)
        .map(|(left, right)| left.eq_ignore_ascii_case(right))
}

pub fn observation_pool_key(token_address: &str, pool_address: &str) -> String {
    format!(
        "{}:{}",
        normalize_address(token_address),
        normalize_address(pool_address)
    )
}

pub(super) fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(super) fn is_lp_burn_holder(address: &str) -> bool {
    reth_chain_query::common_addresses::is_burn_address_str(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erc20::ERC20TokenMetadata;
    use crate::pools::base::{BasePoolConfig, PoolIdentity};

    #[test]
    fn meaningful_pool_token_reserve_rejects_supply_dust() {
        assert_eq!(meaningful_pool_token_reserve(5.6e-17, Some(100.0)), None);
    }

    #[test]
    fn meaningful_pool_token_reserve_accepts_initial_pool_reserve() {
        assert_eq!(meaningful_pool_token_reserve(78.1, Some(100.0)), Some(78.1));
    }

    #[test]
    fn meaningful_pool_token_reserve_uses_absolute_floor_without_supply() {
        assert_eq!(meaningful_pool_token_reserve(5.6e-17, None), None);
        assert_eq!(meaningful_pool_token_reserve(1e-10, None), Some(1e-10));
    }

    #[test]
    fn pool_token_reserve_ratio_denominator_uses_previous_reserve_when_current_is_dust() {
        let token = ERC20Token::new(ERC20TokenMetadata::new(
            "0x1111111111111111111111111111111111111111",
            "Token",
            "TKN",
            18,
            "100000000000000000000",
        ));
        let mut pool = BasePool::new(
            PoolIdentity::new(
                "0x2222222222222222222222222222222222222222",
                token.contract_address.clone(),
                WETH_ADDRESS,
                "uniswap_v2",
            ),
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                history_limit: 10,
                ..BasePoolConfig::new(18)
            },
        );

        pool.update_reserves(78.1, 1.0, 10, 1_700, "0xPRE");
        pool.update_reserves(5.6e-17, 5.6e-17, 11, 1_712, "0xDRAIN");

        assert_eq!(
            pool_token_reserve_ratio_denominator(&token, &pool, 11),
            Some(78.1)
        );
    }
}
