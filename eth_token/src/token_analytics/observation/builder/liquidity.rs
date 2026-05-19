use crate::pools::BasePool;
use crate::token_analytics::PoolLiquidityFeatures;

pub(super) fn liquidity_features(pool: &BasePool) -> PoolLiquidityFeatures {
    let initial = pool.reserve_tracker.reserve_history.first();
    let max_denom_reserve = pool
        .reserve_tracker
        .reserve_history
        .iter()
        .map(|snapshot| snapshot.denom_reserve)
        .filter(|value| value.is_finite())
        .fold(None, |max: Option<f64>, value| {
            Some(max.map_or(value, |current| current.max(value)))
        });

    let mut features = PoolLiquidityFeatures::new(
        pool.denom_reserve(),
        pool.token_reserve(),
        pool.state.total_liquidity,
        pool.price(),
    )
    .with_initial_reserves(
        initial.map(|snapshot| snapshot.denom_reserve),
        initial.map(|snapshot| snapshot.token_reserve),
        initial.map(|snapshot| snapshot.price),
    )
    .with_max_denom_reserve(max_denom_reserve);
    features.reserve_observation_count = pool.reserve_tracker.reserve_history.len() as u32;
    features
}
