use alloy_primitives::U256;

const Q96: f64 = 79_228_162_514_264_337_593_543_950_336.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VirtualReserves {
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub token0_reserve: f64,
    pub token1_reserve: f64,
}

pub fn u256_to_u128_saturating(value: U256) -> u128 {
    value.to_string().parse::<u128>().unwrap_or(u128::MAX)
}

pub fn scale_u256(value: U256, decimals: u8) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0) / decimal_scale(decimals)
}

pub fn scale_i128(value: i128, decimals: u8) -> f64 {
    (value as f64) / decimal_scale(decimals)
}

pub fn token_price_from_sqrt_price_x96(
    sqrt_price_x96: U256,
    token_decimals: u8,
    denom_decimals: u8,
    token1_is_denom: bool,
) -> Option<f64> {
    let token0_per_token1 = token1_per_token0_price(
        sqrt_price_x96,
        if token1_is_denom {
            token_decimals
        } else {
            denom_decimals
        },
        if token1_is_denom {
            denom_decimals
        } else {
            token_decimals
        },
    )?;

    if token1_is_denom {
        finite_positive(token0_per_token1)
    } else {
        finite_positive(1.0 / token0_per_token1)
    }
}

pub fn virtual_reserves_from_liquidity(
    liquidity: u128,
    sqrt_price_x96: U256,
    token_decimals: u8,
    denom_decimals: u8,
    token1_is_denom: bool,
) -> Option<VirtualReserves> {
    if liquidity == 0 {
        return Some(VirtualReserves::default());
    }

    let sqrt_price = u256_to_f64(sqrt_price_x96)? / Q96;
    if !sqrt_price.is_finite() || sqrt_price <= 0.0 {
        return None;
    }

    let liquidity = liquidity as f64;
    let token0_reserve = liquidity / sqrt_price;
    let token1_reserve = liquidity * sqrt_price;

    let token0_decimals = if token1_is_denom {
        token_decimals
    } else {
        denom_decimals
    };
    let token1_decimals = if token1_is_denom {
        denom_decimals
    } else {
        token_decimals
    };

    let token0_reserve = token0_reserve / decimal_scale(token0_decimals);
    let token1_reserve = token1_reserve / decimal_scale(token1_decimals);
    let (token_reserve, denom_reserve) = if token1_is_denom {
        (token0_reserve, token1_reserve)
    } else {
        (token1_reserve, token0_reserve)
    };

    Some(VirtualReserves {
        token_reserve,
        denom_reserve,
        token0_reserve,
        token1_reserve,
    })
}

pub fn update_tick_delta(
    ticks: &mut std::collections::BTreeMap<i32, i128>,
    tick: i32,
    delta: i128,
) {
    let next = ticks.get(&tick).copied().unwrap_or_default() + delta;
    if next == 0 {
        ticks.remove(&tick);
    } else {
        ticks.insert(tick, next);
    }
}

pub fn current_tick_in_range(current_tick: Option<i32>, tick_lower: i32, tick_upper: i32) -> bool {
    current_tick
        .map(|tick| tick >= tick_lower && tick < tick_upper)
        .unwrap_or(false)
}

fn token1_per_token0_price(
    sqrt_price_x96: U256,
    token0_decimals: u8,
    token1_decimals: u8,
) -> Option<f64> {
    let sqrt_price = u256_to_f64(sqrt_price_x96)? / Q96;
    let raw_price = sqrt_price * sqrt_price;
    let decimal_adjustment = 10_f64.powi(i32::from(token0_decimals) - i32::from(token1_decimals));
    finite_positive(raw_price * decimal_adjustment)
}

fn finite_positive(value: f64) -> Option<f64> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

fn u256_to_f64(value: U256) -> Option<f64> {
    value
        .to_string()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn decimal_scale(decimals: u8) -> f64 {
    10_f64.powi(i32::from(decimals))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_price_one_yields_unit_price_for_equal_decimals() {
        let sqrt = U256::from(1u128) << 96;

        assert_eq!(
            token_price_from_sqrt_price_x96(sqrt, 18, 18, true).unwrap(),
            1.0
        );
        assert_eq!(
            token_price_from_sqrt_price_x96(sqrt, 18, 18, false).unwrap(),
            1.0
        );
    }

    #[test]
    fn virtual_reserves_map_token_and_denom_orientation() {
        let sqrt = U256::from(1u128) << 96;
        let reserves =
            virtual_reserves_from_liquidity(1_000_000_000_000_000_000, sqrt, 18, 18, true).unwrap();

        assert_eq!(reserves.token_reserve, 1.0);
        assert_eq!(reserves.denom_reserve, 1.0);
    }
}
